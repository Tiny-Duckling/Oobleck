# Codebase Review — Oobleck / Quiz Zoo

Reviewed the full source tree (Rust/Axum backend, SvelteKit frontend, migrations, Docker setup) and ran the available checks:

- `cargo clippy --all-targets` → compiles, **11 style warnings** (no errors)
- `npm run check` → **0 errors, 0 warnings**
- No test files or CI configuration exist.

Overall this is a clean, well-structured small full-stack app. The main risks are secrets handling, abuse/rate limiting, unbounded generated-quiz growth, a couple of real correctness bugs, and the absence of tests.

---

## High priority

### 1. Committed secrets and a weak default JWT secret
- `backend/.env` is **tracked in git** and contains the DB password plus `JWT_SECRET=change-me-to-a-long-random-string`.
- `docker-compose.yml:21` defaults `JWT_SECRET` to a known constant (`dev-only-secret-change-me-...`).
- If deployed without the prod override, anyone can forge JWTs. The README even notes `.env` is committed.

**Fix:** untrack `backend/.env`, add `.env` to `.gitignore`, add `.env.example`, require `JWT_SECRET` in compose (`${JWT_SECRET:?}`), and validate a minimum secret length at startup.

### 2. Unbounded generated-quiz growth
`generate_quiz` (`backend/src/routes/quizzes.rs:249-322`) inserts a new quiz + N questions + 4N options on every request, never deletes them, and `list_quizzes` hides them (`WHERE NOT q.is_generated`). Any registered user can spam this; the DB grows invisibly.

**Fix:** add a TTL/cleanup job, reuse a per-user generated quiz, or avoid persisting (store question IDs in the attempt). Combine with rate limiting.

### 3. No rate limiting on auth endpoints
`register`/`login` (`backend/src/routes/auth.rs`) have no throttling. Argon2 slows brute force but doesn't prevent it, and registration is open.

**Fix:** per-IP/per-username rate limiting (e.g. `tower-governor` or nginx `limit_req`), plus backoff/lockout.

### 4. SPA param-change bug: quiz/attempt pages don't refetch
`frontend/src/routes/quiz/[id]/+page.svelte:44-58` and `frontend/src/routes/attempts/[id]/+page.svelte:33-58` fetch in `onMount`. SvelteKit reuses the page component when only `[id]` changes, so navigating `/quiz/A` → `/quiz/B` keeps A's data (and selections).

**Fix:** use an `$effect` keyed on `quizId`/`attemptId`, `afterNavigate`, or `{#key id}`.

### 5. `seed_gre` is not deterministic despite the comment
`backend/src/bin/seed_gre.rs:33-34` claims a deterministic RNG, but `build_questions` iterates a `HashMap` (`:239`) whose order is randomized per process, and the shared RNG (`:236`) is consumed in that order. `--reset` produces different quizzes each run.

**Fix:** use `BTreeMap` or sort group keys before iterating.

---

## Medium priority

### Backend

- **N+1 query in `get_quiz`** (`quizzes.rs:131-150`): one options query per question. A 50-question quiz = 51 round trips. Fetch questions + options in one join and group in Rust.
- **`submit_quiz` doesn't validate that the selected option belongs to the question** (`quizzes.rs:190-204`). A client can send an option ID from another question; the FK passes, and the review later shows the wrong "your answer" text. Validate `(question_id, option_id)` pairs.
- **No DB guarantee of exactly one correct option per question.** `submit_quiz` assumes one (`:170-181`); multiple correct options would duplicate results and inflate `total`. Add a partial unique index: `UNIQUE (question_id) WHERE is_correct`.
- **`attempt_answers` FKs lack delete behavior** (`migrations/0001_init.sql:39-45`). `question_id`/`selected_option_id` have no `ON DELETE`; deleting a question/option directly fails, and quiz deletion relies on cascade ordering. `seed_gre --reset` works around it by deleting `attempt_answers` first (`seed_gre.rs:449-465`). Add `ON DELETE CASCADE` (or `SET NULL` for `selected_option_id`).
- **Register race → 500** (`routes/auth.rs:65-82`): check-then-insert; concurrent duplicate usernames hit the unique constraint and return 500. Catch the unique violation and return 409.
- **Login timing side channel** (`routes/auth.rs:100-114`): unknown users return before Argon2; known users pay the hash cost. Verify a dummy hash for unknown users.
- **`make_cloze` byte-index panic risk** (`seed_gre.rs:148-169`): offsets come from `sentence.to_lowercase()` but are applied to `sentence`; `to_lowercase` can change byte length for non-ASCII (e.g. `İ`), causing a panic or wrong slice. Use char-boundary-safe matching or restrict to ASCII.
- **`AppError::Internal` returns its message to clients** (`error.rs:17-24`). Currently only generic messages are used, but it's an easy leak. Return a generic message and log details.
- **Per-row inserts** in `submit_quiz` (`:224-234`) and `generate_quiz` (`:292-317`). Use `UNNEST`/`QueryBuilder` for fewer round trips.
- **`ORDER BY random()`** (`:264`) is fine at this scale but slow on large tables; consider `TABLESAMPLE` or precomputed random keys.
- **No pagination** on `list_quizzes` (`:83-101`) or `my_history` (`history.rs:48-69`). Fine now, but history grows unbounded per user.

### Frontend

- **No expired-token / 401 handling** (`lib/api.ts:84-103`, `lib/auth.svelte.ts:20-30`). The UI stays "logged in" after expiry. Decode `exp` on load and clear; on 401, logout + redirect.
- **`jspdf` is in the initial bundle** (`routes/+page.svelte:6`, `routes/quiz/[id]/+page.svelte:6` import `#lib/pdf`, which imports `jspdf`). Lazy-load it inside `downloadPdf` to keep the home page light.

### Ops / quality

- **No tests and no CI.** Add unit tests for grading/validation and integration tests for auth/quiz flow; run `cargo test` + `npm run check` in CI.
- **Docker hardening:** backend runs as root (`backend/Dockerfile`), no backend healthcheck, frontend `depends_on` backend without a health condition, and changing `POSTGRES_PASSWORD` in prod won't apply to an existing `pgdata` volume. Add a non-root user, `/health`, healthchecks, and document volume reset.

---

## Low priority / polish

- **Dead code:** `_pool_type_check` + `use sqlx::PgPool` in `routes/auth.rs:125-127` and `routes/quizzes.rs:324-325`.
- **Unused direct deps:** `tower`, `rand` in `backend/Cargo.toml`; `@sveltejs/adapter-auto` in `frontend/package.json`.
- **Clippy warnings:** needless borrows, `map_or` simplification, `&mut Vec` → `&mut [_]` in `seed_gre.rs`.
- **README mismatches:** `GET /api/quizzes/:id` is listed as no-auth but requires `AuthUser` (`quizzes.rs:105`); the seed note says repeated runs duplicate quizzes, but the code skips existing titles (`seed.rs:78-104`).
- **Stale docs:** `plan.md` describes a different vanilla-JS static app; `frontend/README.md` is the default Svelte template. Update or remove.
- **Misleading comment:** `history.rs:21-23` says review works for deleted quizzes, but `attempts.quiz_id` cascades on quiz delete, so the attempt is gone; the frontend fallback is effectively dead.
- **`seed_gre` odd-one-out ambiguity:** the intruder is only checked against the target word, not the three synonyms, so it can be ambiguous.
- **`seed_gre --reset`** deletes by `title LIKE 'GRE Vocabulary%'`, which could hit a user-created quiz with that title.
- **A11y:** chips use `role="tablist"` but buttons lack `role="tab"`/`aria-selected`; add `aria-live` for errors; add a table caption.
- **Logout doesn't redirect**; the user stays on a protected page with stale data.
- **No warning before submitting with unanswered questions.**
- **`window.location.href` instead of `goto`** for login redirects (full reload).
- **`app.html`** has a nonstandard `<meta name="text-scale">`; no favicon link or description.
- **nginx:** add security headers (CSP, `X-Content-Type-Options`, `Referrer-Policy`), gzip; `proxy_set_header Authorization` is redundant.
- **`AuthUser` clones the JWT secret per request** (`auth.rs:52`); minor.
- **`AuthUser` doesn't check the user still exists**; deleted users' tokens remain valid until expiry.
- **Username validation** allows any characters and is case-sensitive; password has no max length (bound Argon2 cost).
- **`generate_quiz` doesn't bound `subject` length**; could create huge titles/descriptions.
- **`auth.svelte.ts` `load()`** doesn't validate the stored shape; `setAuth` assumes `localStorage` exists.
- **`api.ts` `API_BASE`** could produce `//api` if the env value has a trailing slash.
- **`pdf.ts`** doesn't handle a single line taller than the page (unlikely).
- **`AttemptReview`** uses emoji markers; add text/aria for screen readers.
- **`+page.svelte` `auth.token!`** non-null assertion; safe today but brittle.
- **`+page.svelte` `activeSubject`** can go stale if the subject disappears after reload.
- **`+layout.svelte`** no active-link styling or mobile nav.

---

## Strengths

- Server-side grading; `get_quiz` never selects `is_correct` (`quizzes.rs:133`).
- Argon2 password hashing, JWT with expiry, generic login error.
- Ownership check on attempts (`history.rs:89`) and 404 for foreign attempts.
- Typed API client and a shared `AttemptReview` component.
- Idempotent seeding; `--reset` for GRE; embedded word data.
- Docker Compose with a health-checked DB, same-origin nginx proxy, and prod overrides requiring secrets.
- Svelte 5 runes, strict TypeScript, `svelte-check` clean.
- Good README for setup and architecture.

---

## Suggested order of work

1. **Secrets:** untrack `.env`, require `JWT_SECRET`, add `.env.example`.
2. **Abuse controls:** rate limiting + generated-quiz cleanup.
3. **Fix the SPA refetch bug.**
4. **Fix `seed_gre` determinism and `make_cloze` safety.**
5. **Add tests and CI.**
6. **Performance:** `get_quiz` join, batch inserts.
7. **Polish:** docs, a11y, Docker hardening.

---

*Reviewed statically and via the two check commands; the app and a live database were not run, so DB-cascade and runtime findings are based on the schema and code paths.*

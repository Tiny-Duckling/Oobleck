# Quiz Zoo — Quiz-Taking Website

A full-stack quiz application: **Axum (Rust)** backend + **SvelteKit (Svelte 5)** frontend + **PostgreSQL**.

## Features

- User accounts (register/login) with JWT auth and argon2 password hashing
- Browse pre-loaded quizzes, tagged with subjects (e.g. Rust, Physics, GregMat GRE Vocabulary)
- Take a quiz (multiple-choice), graded server-side — correct answers never leak to the client
- Instant score + per-question review after submitting
- Personal score history
- Practice quiz generator: pick a subject (e.g. GregMat GRE Vocabulary) and get a
  quiz built from random questions of that subject
- Five GRE vocabulary question types: word→meaning, meaning→word (production
  recall), synonyms, cloze (fill the blank in an example sentence) and odd-one-out

## Project Layout

```
backend/            Axum API (Rust)
  migrations/       SQLx migrations (schema)
  data/             GregMat GRE vocabulary word list (used by seed_gre)
  src/
    main.rs         server entrypoint, routes, CORS
    auth.rs         JWT creation/verification + AuthUser extractor
    config.rs       shared AppState (DB pool + JWT secret)
    error.rs       AppError -> HTTP responses
    routes/         auth, quizzes, history handlers
  src/bin/seed.rs   seed binary: loads 3 sample quizzes
  src/bin/seed_gre.rs  seed binary: loads 37 GRE vocabulary quizzes (~2,050 questions)
  data/gregmat_words.json  GregMat GRE word list (1,109 words: synonyms, definitions, examples)
frontend/          SvelteKit app (Svelte 5, TypeScript)
  src/lib/api.ts   typed API client
  src/lib/auth.svelte.ts  auth store (runes, persisted to localStorage)
  src/routes/       pages: home, login, register, quiz/[id], history, attempts/[id]
```

## API

| Method | Path                      | Auth   | Description                              |
| ------ | ------------------------- | ------ | ---------------------------------------- |
| POST   | `/api/auth/register`      | —      | Create account, returns JWT              |
| POST   | `/api/auth/login`         | —      | Login, returns JWT                       |
| GET    | `/api/quizzes`            | —      | List quizzes with question counts        |
| POST   | `/api/quizzes/generate`   | Bearer | Create a practice quiz from N random questions of a subject |
| GET    | `/api/quizzes/:id`        | —      | Quiz detail (no correct answers exposed) |
| POST   | `/api/quizzes/:id/submit` | Bearer | Submit answers, grade, store attempt     |
| GET    | `/api/attempts/:id`       | Bearer | Review one of your past attempts         |
| GET    | `/api/me/history`         | Bearer | Current user's attempt history           |

## Setup

### 1. PostgreSQL

Create a database and user (pick your own password; use the same one in
`backend/.env` below):

```sql
CREATE USER quizadmin WITH PASSWORD '<your-password>';
CREATE DATABASE quizdevdb02 OWNER quizadmin;
```

### 2. Backend

```bash
cd backend
cp .env.example .env   # gitignored; never commit this file
# Edit backend/.env and set real values:
#   DATABASE_URL=postgres://quizadmin:<your-password>@localhost:5432/quizdevdb02
#   JWT_SECRET=<output of: openssl rand -base64 48>   (>= 32 chars, required)
#   PORT=4000
#   CORS_ORIGINS=http://localhost:5173,http://127.0.0.1:5173   (optional)

cargo run --bin seed      # run migrations + load 3 sample quizzes
cargo run --bin seed_gre  # optional: load 37 GregMat GRE vocabulary quizzes
cargo run                 # start API on http://localhost:4000
```

The backend validates `JWT_SECRET` at startup: it must be at least 32
characters, or the process refuses to start. The `CHANGE_ME` placeholder in
the `.env.example` templates is deliberately shorter than that, so copying a
template without editing it fails validation.

Migrations also run automatically on server startup.

### 3. Frontend

```bash
cd frontend
npm install
npm run dev            # http://localhost:5173 (proxies /api to localhost:4000)
```

The frontend calls the API at `http://localhost:4000` by default; override with
`VITE_API_BASE` (e.g. in `frontend/.env`).

## Docker

The whole stack (PostgreSQL + API + frontend) can run with Docker Compose:

```bash
cp .env.example .env     # root .env, gitignored; fill in POSTGRES_PASSWORD + JWT_SECRET
docker compose up --build      # frontend on http://localhost:8080, API on :4000
```

Compose reads secrets from the root `.env` (auto-loaded) — none are hardcoded
in the compose files, and every variable is required: if one is missing,
Compose fails fast with an error. For production, layer the
overrides (`docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d`).

> Note: the `postgres` image only sets `POSTGRES_PASSWORD` when the `pgdata`
> volume is first created. To rotate the password later, run
> `docker compose down -v` (wipes local data) before changing `.env`.

- `backend/Dockerfile` — multi-stage Rust build; the entrypoint seeds the
  database (idempotent) and starts the API.
- `frontend/Dockerfile` — builds the SvelteKit app as a static SPA and serves
  it with nginx, which also proxies `/api` to the backend (single origin, no
  CORS involved).
- `db` — PostgreSQL 16 with a named volume (`pgdata`), health-checked before
  the backend starts.

The frontend calls the API with a **relative** base (`/api`) by default: during
`npm run dev` the Vite dev server proxies `/api` to `http://localhost:4000`,
and in Docker nginx handles the proxying. Set `VITE_API_BASE` to target a
different origin instead.

## Secrets & Configuration

All secrets live in gitignored `.env` files; only `.env.example` templates are
committed. Never commit a real `.env`, and never bake secrets into images or
tracked files.

| File | Purpose | Gitignored |
| --- | --- | --- |
| `backend/.env` | Local (non-Docker) backend: `DATABASE_URL`, `JWT_SECRET`, `PORT`, `CORS_ORIGINS` | yes |
| `.env` (repo root) | Docker Compose: `POSTGRES_*`, `JWT_SECRET` | yes |
| `backend/.env.example` / `.env.example` | Templates to copy from | no |

- Backend config is read from the environment (loaded via `dotenvy`) and
  validated in `backend/src/config.rs` (`Config::from_env`) — bad/missing
  secrets fail fast at startup instead of surfacing later.
- `JWT_SECRET` must be ≥ 32 chars; generate with `openssl rand -base64 48`.
- The frontend has no secrets: `VITE_API_BASE` is a non-secret build-time
  override, and the API base defaults to a relative `/api`.
- Docker images never contain `.env` files (excluded via `.dockerignore`; the
  backend gets its config at runtime from Compose `environment`).

## Dev Notes

- Backend CORS allows `localhost:5173` / `127.0.0.1:5173` (SvelteKit dev server);
  in Docker the frontend is same-origin via the nginx proxy, so CORS is bypassed.
  `CORS_ORIGINS` entries are validated at startup (must be `scheme://host[:port]`)
  and normalized to the exact form browsers send — a typo fails startup instead
  of silently never matching. Preflight responses are cached for 24h.
- JWTs are stored in `localStorage` under the `quiz_auth` key and sent as
  `Authorization: Bearer <token>`.
- Grading is fully server-side: the quiz-fetch endpoint never selects `is_correct`.
- `cargo run --bin seed` is idempotent per-run but will duplicate quizzes if run
  repeatedly; truncate `quizzes` first if you want a clean re-seed.
- `cargo run --bin seed_gre` skips groups whose quiz title already exists, so it
  is safe to re-run. Pass `--reset` to wipe the GRE quizzes (and their attempts)
  and rebuild them from scratch. Word data is embedded from
  `backend/data/gregmat_words.json`, originally sourced from the GregMat
  vocabulary list via the community mirror
  https://github.com/Xatta-Trone/gregmatgrewordsdb.
- Heads-up: `sqlx::migrate!` reads `migrations/` at compile time but cargo does
  not track that dependency. After adding a migration, `touch` the crate sources
  (or `cargo clean -p quiz-backend`) before re-running a seed binary — the
  Dockerfile already does this.

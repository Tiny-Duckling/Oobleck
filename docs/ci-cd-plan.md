# CI/CD Plan — Oobleck

**Date:** 2026-11-21
**Status:** Proposed
**Decisions:** GitLab CI · deploy to a VPS over SSH + Docker Compose · strict quality gates

---

## 1. Current state

What the pipeline has to work with:

- **Backend:** Rust/Axum, SQLx with embedded migrations (`sqlx::migrate!`). No `query!` macros, so compiling does **not** need a live database. `Cargo.lock` is committed.
- **Frontend:** SvelteKit 5 + TypeScript, static SPA build via `adapter-static`, `package-lock.json` committed.
- **Runtime:** PostgreSQL 16, backend on `:4000`, frontend nginx on `:80` proxying `/api` to the backend.
- **Docker:** both services have multi-stage Dockerfiles; root `docker-compose.yml` builds locally and publishes `4000:4000`.
- **Tests today:** 8 backend tests (CORS parsing only), zero frontend tests.
- **Checks today:** `npm run check` and `npm run build` pass. `cargo fmt --check` and Clippy are **not clean** (11 warnings, formatting diffs), so strict gates cannot be switched on immediately.
- **Secrets:** only `.env.example` files are tracked; real `.env` files are gitignored. Good baseline.
- **Missing for CD:** no health endpoint, no backend healthcheck, no production compose file, no deploy/rollback scripts, no TLS.

## 2. Goals and non-goals

### Goals

1. Every commit on a `release-*` or `master` branch is gated by formatting, linting, tests, type-checking, builds, dependency audits, and image builds.
2. Direct push to any `release-*` branch (e.g. `release-v1.0.0`, `release-2026.11`) automatically builds an immutable container image and deploys to the **Pre-Production** server for verification and testing—no PR required.
3. Merging a `release-*` branch into `master` automatically promotes the verified image and deploys to the **Production** server with a pre-flight DB backup, health verification, and rollback capability.
4. "Build Once, Promote Everywhere": Production deploys the exact immutable image SHA verified on pre-production—never rebuilds from source.
5. No secret is ever committed, baked into an image, or printed in a job log.
6. Warm pipeline under ~10 minutes.

### Non-goals (for now)

- Kubernetes, autoscaling, multi-region, blue/green deployment.
- Full observability stack (Prometheus/Grafana). Start with uptime checks and structured logs.

## 3. Target flow

```text
[ Push to release-* branch ] ──► validate ─► test ─► security ─► build :sha ─► deploy pre-production (auto)
                                                                                    │
                                                                                    └─► smoke test & manual QA
                                                                                             │
                                                                                    [ Release Verified ]
                                                                                             │
                                                                                             ▼
[ Merge release-* to master ] ────────────────────────────────► promote :sha ──► deploy production (auto)
                                                                                    │
                                                                                    ├─► DB backup
                                                                                    ├─► smoke test
                                                                                    └─► record last-good tag
```

Registry: **GitLab Container Registry** (`registry.gitlab.com/tiny-duckling/oobleck`).

```text
SERVERS
├── Pre-Production (Staging VPS or isolated staging stack on port :8081)
│   └── /opt/oobleck-staging/{.env, docker-compose.yml, deploy.sh}
└── Production (Production VPS on port :8080 behind Caddy TLS :80/:443)
    └── /opt/oobleck/{.env, docker-compose.yml, deploy.sh}
```

## 4. Pipeline design

### 4.1 Workflow rules

Create pipelines for pushes to release branches (`release-*`), commits on `master`, and release tags. No PR/MR is required to deploy to pre-production.

```yaml
workflow:
  rules:
    - if: $CI_COMMIT_BRANCH =~ /^release-.*$/
    - if: $CI_COMMIT_BRANCH == "master" || $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
    - if: $CI_COMMIT_TAG =~ /^v\d+\.\d+\.\d+$/
```

### 4.2 Stages and jobs

| Stage | Job | Purpose | Trigger |
| --- | --- | --- | --- |
| validate | `fmt` | `cargo fmt --all -- --check` | all pipelines |
| validate | `clippy` | `cargo clippy --all-targets --all-features -- -D warnings` | all pipelines |
| validate | `frontend-check` | `npm ci` + `npm run check` | all pipelines |
| validate | `hadolint` | Lint both Dockerfiles | all pipelines |
| validate | `compose-config` | `docker compose ... config -q` | all pipelines |
| test | `backend-test` | `cargo test --all-targets` with a Postgres service | all pipelines |
| test | `frontend-test` | Vitest + coverage | all pipelines |
| security | `cargo-audit` | RustSec advisories against `Cargo.lock` | all pipelines |
| security | `npm-audit` | `npm audit --omit=dev --audit-level=high` | all pipelines |
| security | `trivy-fs` | Lockfile/config scan, fail on CRITICAL | all pipelines |
| security | GitLab SAST + Secret Detection | Security templates | all pipelines |
| build | `build-backend` | Buildx image, tag `:sha-$CI_COMMIT_SHA` | all pipelines |
| build | `build-frontend` | Buildx image, tag `:sha-$CI_COMMIT_SHA` | all pipelines |
| build | `trivy-image` | Scan both images, fail on CRITICAL | all pipelines |
| build | `compose-smoke` | Start ephemeral stack, hit `/api/health` | all pipelines |
| deploy | `deploy-preproduction` | Deploy image to Pre-Production (Staging) | Push to `release-*` |
| deploy | `smoke-preproduction` | Poll staging `/api/health` + e2e check | Push to `release-*` |
| deploy | `deploy-production` | Deploy verified image to Production | Merge to `master` / tag |
| deploy | `smoke-production` | Poll prod `/api/health`, rollback on fail | Merge to `master` / tag |
| deploy | `rollback-production` | Manual emergency redeploy of `.last_good_tag` | Manual button |

Notes:

- Use `needs:` to let `build-*` start as soon as `validate` + `test` finish, instead of waiting for the whole stage.
- Mark all non-deploy jobs `interruptible: true`; superseded MR pipelines should cancel.
- Deploy jobs use `resource_group: staging` / `resource_group: production` so two deploys never overlap.
- Retry only infrastructure failures (`runner_system_failure`, `stuck_or_timeout_failure`), never deploy jobs.

### 4.3 Caching and performance

- Rust: cache `$CI_PROJECT_DIR/.cargo/{registry,git}` and `backend/target`, keyed on `backend/Cargo.lock`. Set `CARGO_INCREMENTAL=0`.
- Tool-install jobs (`cargo-audit`, `cargo-llvm-cov`) need a linker and OpenSSL headers in the slim image (`build-essential pkg-config libssl-dev`), plus `rustup component add llvm-tools-preview` for coverage. Alternatively bake a small CI tools image (Phase 6).
- Node: cache `frontend/node_modules` and `frontend/.svelte-kit`, keyed on `frontend/package-lock.json`.
- Docker: use `buildx` registry cache (`type=registry,ref=...:buildcache,mode=max`) so Dockerfile layers survive runner changes.
- If `target/` makes caches too large or slow, move to `sccache` backed by object storage; do not block Phase 1 on it.
- Pin toolchains: add `rust-toolchain.toml` (`channel = "1.94.0"`, components `rustfmt`, `clippy`) and use `rust:1.94-slim-bookworm`; use Node 22 to match the Dockerfile.

### 4.4 Artifacts

- Backend coverage: Cobertura XML (`cargo llvm-cov --cobertura`), reported to GitLab.
- Frontend coverage: Cobertura XML from Vitest, reported to GitLab.
- SBOM: CycloneDX per image (`trivy image --format cyclonedx`), retained 30 days.
- Job logs must never include `.env` contents or tokens.

## 5. Strict quality gates

### 5.1 Exact checks

```bash
# Backend
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo llvm-cov --workspace --lcov --output-path lcov.info
cargo llvm-cov report --cobertura --output-path coverage.xml

# Frontend
npm ci
npm run check
npm run build
npx vitest run --coverage

# Security
cargo audit
npm audit --omit=dev --audit-level=high
trivy fs --severity CRITICAL --exit-code 1 .
trivy image --severity CRITICAL --exit-code 1 <image>
hadolint backend/Dockerfile frontend/Dockerfile
```

### 5.2 Ratchet policy

Strict does not mean "turn everything on while the repo is red". Sequence:

1. **Phase 0:** run `cargo fmt`, fix all Clippy warnings, add first frontend tests, measure coverage.
2. Enable `fmt` and `clippy -D warnings` as blocking once clean.
3. Set the coverage floor to the measured baseline, rounded down. Raise it by ~5 points per month toward an agreed target (suggest 70% backend, 60% frontend).
4. `cargo audit` and `npm audit` block on vulnerabilities. Any temporary ignore must be a tracked file entry with an expiry date and a linked issue.
5. Trivy blocks on CRITICAL immediately; HIGH starts as a report and becomes blocking after the first triage pass.

### 5.3 Required merge checks

In GitLab project settings, require the pipeline to succeed on `main` and require the validate/test/security/build jobs for MRs. Protect `main` (no direct pushes, no force pushes).

## 6. CD design: VPS + Docker Compose

### 6.1 Image tagging

| Trigger | Tags |
| --- | --- |
| Any pipeline | `backend:$CI_COMMIT_SHA`, `frontend:$CI_COMMIT_SHA` |
| `main` | plus `:latest` |
| Tag `v1.2.3` | plus `:v1.2.3` (and optionally `:1.2`) |

Production always deploys an immutable tag (`SHA` or semver), never `latest`. Registry cleanup: delete MR/SHA tags older than 30 days; keep all semver tags.

### 6.2 Single parameterized compose file

Rather than maintaining multiple compose files for different environments (which risks configuration drift and requires complex `-f` override flags), use a **single `docker-compose.yml`** parameterized with environment variable defaults:

- **Images:**
  ```yaml
  backend:
    image: ${BACKEND_IMAGE:-oobleck-backend:local}
    build: ./backend
  frontend:
    image: ${FRONTEND_IMAGE:-oobleck-frontend:local}
    build: ./frontend
  ```
  - In local dev: defaults to building from source (`./backend`, `./frontend`).
  - In pre-prod & prod: `.env` sets `BACKEND_IMAGE` and `FRONTEND_IMAGE` to registry tags (e.g. `$CI_REGISTRY_IMAGE/backend:$IMAGE_TAG`), so Compose pulls pre-built images without compiling on the server.
- **Port Exposure:**
  - Backend: Publish no host port in production. Set `BACKEND_PORT_MAPPING=""` in production `.env` (or map only to `127.0.0.1:4000:4000` in dev).
  - Frontend: Bind to localhost behind Caddy in production via `FRONTEND_BIND=127.0.0.1` and `FRONTEND_PORT=8080`.
  - Database: Publishes no host port in any environment; accessed only through internal Compose network.
- **Healthchecks:**
  - Embedded in the base `docker-compose.yml` with `pg_isready` for `db` and `/api/health` curl for `backend`.

Also harden the backend runtime image with a non-root user and `curl` (needed by the healthcheck).

### 6.3 Server layout

```text
/opt/oobleck/
├── .env                     # POSTGRES_*, JWT_SECRET, BACKEND_IMAGE, FRONTEND_IMAGE (mode 600)
├── docker-compose.yml
├── deploy.sh
├── rollback.sh
├── .current_tag
├── .last_good_tag
└── backups/                 # pre-deploy pg_dump, 7 retained
```

Staging uses the same scripts with a separate directory, `COMPOSE_PROJECT_NAME=oobleck-staging`, its own `.env`/volume, and port `8081`. Prefer a separate VPS; the same VPS is acceptable if resources allow.

### 6.4 Deploy flow

#### Phase A: Pre-Production Deployment (On Push to `release-*`)

1. Developer pushes directly to a release branch (e.g. `release-v1.2.0` or `release-2026.11`).
2. CI runs validation, tests, and security scans.
3. CI builds backend and frontend images tagged `:sha-$CI_COMMIT_SHA` and pushes to the GitLab Container Registry.
4. `deploy-preproduction` SSHes to the Pre-Production host (`/opt/oobleck-staging`):
   - Sets `IMAGE_TAG=$CI_COMMIT_SHA` and executes `./deploy.sh`.
   - Runs database migrations automatically on backend container startup.
   - Polls `http://127.0.0.1:8081/api/health` until healthy (or times out after 180s).
5. `smoke-preproduction` runs automated end-to-end checks against the live pre-production URL.
6. The team performs QA / acceptance testing against the live pre-production server.

#### Phase B: Production Deployment (On Merge of `release-*` to `master`)

1. The verified `release-*` branch is merged into `master`.
2. The pipeline skips rebuilding source code, promoting the existing `:sha-$CI_COMMIT_SHA` image verified on pre-production.
3. `deploy-production` SSHes to the Production host (`/opt/oobleck`):
   - Takes a pre-deploy `pg_dump` backup before any container changes:
     ```bash
     $COMPOSE exec -T db pg_dump -U "$POSTGRES_USER" "$POSTGRES_DB" | gzip > "backups/pre-${IMAGE_TAG}-$(date +%Y%m%d%H%M%S).sql.gz"
     ```
   - Pulls and deploys `:sha-$CI_COMMIT_SHA` via `docker compose up -d --wait`.
   - Backend automatically applies new migrations via `sqlx::migrate!`.
   - Polls `http://127.0.0.1:8080/api/health`.
4. `smoke-production` validates public HTTPS health and critical paths.
5. If the smoke test fails:
   - Automated rollback is immediately triggered, restarting the previous container tag saved in `.last_good_tag`.
   - A critical alert is dispatched.
6. If the smoke test passes:
   - `.last_good_tag` is updated with the current tag.
   - Old container images are pruned.

### 6.5 Database migration safety between environments

Because Pre-Production runs migrations before Production:
- **Forward-Only, Expand/Contract Migrations:** Never remove or rename columns in the same release that introduces their replacement.
  - *Step 1 (Expand):* Add the new nullable/defaulted column or table. Both old and new application versions work.
  - *Step 2 (Contract):* Once verified on production, submit a follow-up release to drop deprecated columns.
- **Pre-Deploy Dumps:** Production always executes a `pg_dump` immediately before applying container updates so a catastrophic data-altering migration can be manually restored if needed.
- **Rollback Limitation:** Rolling back a container reverts the application binary; it does not roll back database schema. Backward-compatible migrations ensure the old application binary can safely read the updated schema.

### 6.6 Migrations and seeding

- Migrations run on backend startup via `sqlx::migrate!`; keep that.
- With a single backend replica this is safe. If you later run multiple replicas, move migrations to a one-shot job and make them backward-compatible.
- The entrypoint seeds the sample data idempotently. **Never** run `seed_gre --reset` in the deploy path; it deletes GRE quizzes and attempts.
- Rotating `POSTGRES_PASSWORD` does not change an existing `pgdata` volume. Treat it as a manual maintenance task, not part of CD.

### 6.7 TLS and network

- Caddy on the host (or a Caddy compose service) terminates TLS and proxies to `127.0.0.1:8080`. Caddy handles certificates automatically.
- Firewall: allow only `22`, `80`, `443`. Do not expose `4000` or `5432`.
- Keep `restart: unless-stopped` on all services.
- Add an uptime check against `https://<domain>/api/health` (UptimeRobot, Healthchecks.io, or similar).

### 6.8 Secrets inventory

| Secret | Stored in | Used by |
| --- | --- | --- |
| `POSTGRES_USER/PASSWORD/DB` | VPS `/opt/oobleck/.env` (600) | compose `db` + backend `DATABASE_URL` |
| `JWT_SECRET` | VPS `.env` (600), generated with `openssl rand -base64 48` | backend |
| `SSH_PRIVATE_KEY` | GitLab CI **File** variable, protected + masked where possible | deploy jobs |
| `SSH_KNOWN_HOSTS` | GitLab CI variable | deploy jobs |
| `DEPLOY_HOST`, `DEPLOY_USER`, `DEPLOY_PATH` | GitLab CI variables | deploy jobs |
| Registry read token | VPS `docker login` config | `docker compose pull` |
| Registry push credentials | GitLab predefined `CI_REGISTRY_USER/PASSWORD` | build jobs |

Rules:

- Runtime secrets never enter GitLab CI or the image.
- Use a dedicated `deploy` user with key-only SSH. Optional hardening: `authorized_keys` forced command that only accepts a commit SHA or `rollback`.
- Never echo secrets; use `--password-stdin` for registry login.

## 7. GitLab project setup checklist

- [ ] Register a runner with a Docker executor. If using Docker-in-Docker for image builds, it must be privileged; alternatively use rootless Buildah. Prefer a dedicated runner over sharing a general-purpose one.
- [ ] Enable Container Registry and create a read-only deploy token for the VPS.
- [ ] Add protected CI/CD variables listed in §6.8.
- [ ] Protect `main`; require a green pipeline and (optionally) one approval.
- [ ] Create environments `staging` and `production`; mark `production` as protected.
- [ ] Configure registry cleanup (30-day SHA/MR tags).
- [ ] Enable GitLab SAST, Secret Detection, and Dependency Scanning if the tier includes them. If Dependency Scanning is unavailable (Ultimate feature), `cargo audit` + `npm audit` cover the gap.
- [ ] Add a weekly scheduled pipeline for security scans on `main`.
- [ ] Enable Renovate (or GitLab dependency updates) with grouped minor/patch MRs; auto-merge patch updates after a green pipeline.

## 8. Phased rollout

### Phase 0 — Make the repo gate-ready (0.5–1 day)

- Run `cargo fmt`; fix all Clippy warnings.
- Add `rust-toolchain.toml`.
- Add `GET /api/health` (no DB) and `GET /api/ready` (`SELECT 1`) to the backend.
- Add backend/frontend healthchecks and non-root backend user; add `curl` to the runtime image.
- Parameterize `docker-compose.yml` with environment defaults for dev vs prod.
- Add `deploy/deploy.sh` and `deploy/rollback.sh`.
- Add Vitest + `@vitest/coverage-v8` and the first unit tests (`lib/api.ts` is the easiest target).
- Measure backend and frontend coverage and record the baseline.

**Exit:** all checks in §5.1 pass locally; compose validates with dummy env vars.

### Phase 1 — CI validate + test (0.5–1 day)

- Add `.gitlab-ci.yml` with workflow rules, caches, `fmt`, `clippy`, `frontend-check`, `backend-test` (Postgres service), `frontend-test`.
- Enable required checks for MRs.

**Exit:** an MR with a formatting error, a Clippy warning, or a failing test cannot merge.

### Phase 2 — Security gates (0.5 day)

- Add `cargo-audit`, `npm-audit`, `trivy-fs`, `hadolint`, and GitLab templates.
- Triage the first scan; add expiring ignores only with a linked issue.

**Exit:** a known vulnerable dependency or committed secret fails the pipeline.

### Phase 3 — Image build + smoke (0.5 day)

- Add Buildx build/push jobs with registry cache, Trivy image scan, SBOM artifact, and `compose-smoke`.
- Tag policy per §6.1.

**Exit:** every `main` commit yields pullable `:sha` and `:latest` images; the full stack starts and answers `/api/health` in CI.

### Phase 4 — Pre-Production (Staging) CD (0.5 day)

- Provision pre-production environment (VPS or isolated compose project), TLS, `.env`, registry login.
- Add `deploy-preproduction` and `smoke-preproduction` on `release-*` branch push events.

**Exit:** any push to a `release-*` branch automatically builds `:sha` images, deploys to pre-production, and passes automated health/smoke checks without requiring a PR.

### Phase 5 — Production CD (1 day)

- Provision production VPS: Docker + Compose, deploy user, firewall, Caddy, DNS.
- Add `deploy-production` (automatic on merge of `release-*` into `master` or semver tag) with pre-deploy database backup.
- Add `rollback-production` with automated failure trigger and manual fallback.
- Run a rollback drill.

**Exit:** merging a `release-*` branch into `master` promotes the verified `:sha` image directly to production with zero image rebuild; rollback restores the previous tag in under five minutes.

### Phase 6 — Hardening (ongoing)

- Renovate + weekly security pipeline.
- Coverage ratchet.
- Optional Playwright smoke against staging.
- Custom CI image with `cargo-audit`, `cargo-llvm-cov`, and `cargo-nextest` preinstalled to cut install time.
- Consider two backend replicas behind nginx for near-zero-downtime restarts.
- Backups: weekly offsite copy + restore drill.

## 9. Acceptance criteria

- [ ] Every push to a `release-*` branch builds an immutable `:sha` image and deploys to the Pre-Production server automatically.
- [ ] Pre-production deployment executes database migrations and passes automated smoke checks without a PR.
- [ ] Merging a `release-*` branch into `master` promotes the verified `:sha` image and deploys to Production automatically without rebuilding.
- [ ] A pre-deploy database backup is captured before any production container update.
- [ ] Rollback to the previous image completes automatically if production smoke tests fail, or in under five minutes via manual trigger.
- [ ] Secret Detection is green; no real secret exists in git history or job logs.
- [ ] Warm pipeline completes in under ~10 minutes.
- [ ] Coverage is enforced at the agreed floor.
- [ ] No production port except `22`, `80`, `443` is reachable from the internet.

## 10. Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Strict gates start red (fmt/clippy/tests) | Phase 0 cleanup before enabling blocking jobs |
| Rust `target/` cache too large or slow | Registry cache for Docker; `sccache` if needed; keep `CARGO_INCREMENTAL=0` |
| Privileged DinD runner is a security risk | Dedicated runner, ephemeral VMs; or rootless Buildah |
| GitLab tier lacks Dependency Scanning | `cargo audit` + `npm audit` as the blocking baseline |
| CVE noise blocks unrelated MRs | Fail on CRITICAL, report HIGH; expiring ignores with issues |
| Single VPS is a single point of failure | Backups, restart policies, uptime monitoring; managed Postgres later |
| Compose recreate causes brief downtime | Accept for now; add replicas + nginx upstream later |
| Forward-only migrations make rollback unsafe | Expand/contract migrations; pre-deploy dump |
| Registry storage growth | Cleanup policy; keep semver, expire SHA/MR tags |
| Accidental destructive seed in production | Never call `seed_gre --reset` from deploy scripts |

## 11. Open decisions

1. Production domain and DNS provider.
2. Staging on a separate VPS or the same host with a separate compose project.
3. GitLab tier: is Dependency Scanning available, or do we rely on CLI audits?
4. TLS: Caddy in compose vs. host Caddy vs. host nginx + certbot.
5. Registry visibility: private with a deploy token (recommended) vs. public.
6. Notification channel for failed deploys (Slack, email, GitLab only).
7. Coverage targets and whether coverage is blocking from day one or after a month.

## Appendix A — Files to add/change

```text
.gitlab-ci.yml                  # pipeline
rust-toolchain.toml             # pinned Rust + fmt/clippy
deploy/deploy.sh                # pull, up --wait, smoke, tag bookkeeping
deploy/rollback.sh              # redeploy last-good tag
deploy/Caddyfile                # optional, if Caddy runs in compose
frontend/vitest.config.ts       # Vitest + coverage
frontend/src/lib/*.test.ts      # first frontend tests
backend/src/routes/mod.rs       # /api/health, /api/ready
backend/Dockerfile              # non-root user, curl for healthcheck
docker-compose.yml              # parameterized images, healthchecks, conditions
```

## Appendix B — Illustrative `.gitlab-ci.yml` skeleton

> Not copy-paste-final: pin image versions, fill in registry/env names, and adjust rules per §4.

```yaml
workflow:
  rules:
    - if: $CI_COMMIT_BRANCH =~ /^release-.*$/
    - if: $CI_COMMIT_BRANCH == "master" || $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
    - if: $CI_COMMIT_TAG =~ /^v\d+\.\d+\.\d+$/

stages: [validate, test, security, build, deploy]

default:
  interruptible: true
  retry:
    max: 1
    when: [runner_system_failure, stuck_or_timeout_failure]

variables:
  CARGO_HOME: $CI_PROJECT_DIR/.cargo
  CARGO_INCREMENTAL: "0"
  CARGO_TERM_COLOR: always

.rust:
  image: rust:1.94-slim-bookworm
  cache:
    key:
      files: [backend/Cargo.lock]
    paths: [.cargo/registry, .cargo/git, backend/target]

.node:
  image: node:22-alpine
  cache:
    key:
      files: [frontend/package-lock.json]
    paths: [frontend/node_modules, frontend/.svelte-kit]

fmt:
  extends: .rust
  stage: validate
  script: [cd backend, cargo fmt --all -- --check]

clippy:
  extends: .rust
  stage: validate
  script: [cd backend, cargo clippy --workspace --all-targets --all-features -- -D warnings]

frontend-check:
  extends: .node
  stage: validate
  script: [cd frontend, npm ci --prefer-offline --no-audit, npm run check]

hadolint:
  stage: validate
  image: hadolint/hadolint:v2.12.0-debian
  script: [hadolint backend/Dockerfile frontend/Dockerfile]

compose-config:
  stage: validate
  image: docker:29-cli
  variables:
    IMAGE_TAG: ci
    REGISTRY_IMAGE: registry.example.com/group/project
  script:
    - docker compose config -q

backend-test:
  extends: .rust
  stage: test
  services:
    - name: postgres:16-alpine
      alias: postgres
  variables:
    POSTGRES_USER: test
    POSTGRES_PASSWORD: test
    POSTGRES_DB: test
    DATABASE_URL: postgres://test:test@postgres:5432/test
    JWT_SECRET: ci-only-secret-at-least-32-characters-long
  script: [cd backend, cargo test --workspace --all-targets]

frontend-test:
  extends: .node
  stage: test
  script: [cd frontend, npm ci --prefer-offline --no-audit, npx vitest run --coverage]
  artifacts:
    when: always
    reports:
      coverage_report:
        coverage_format: cobertura
        path: frontend/coverage/cobertura-coverage.xml

cargo-audit:
  extends: .rust
  stage: security
  script:
    - cargo install cargo-audit --locked
    - cd backend && cargo audit

npm-audit:
  extends: .node
  stage: security
  script: [cd frontend, npm ci --prefer-offline --no-audit, npm audit --omit=dev --audit-level=high]

build:backend:
  stage: build
  image: docker:29-cli
  services: [docker:29-dind]
  variables:
    DOCKER_TLS_CERTDIR: "/certs"
  needs: [fmt, clippy, backend-test]
  before_script:
    - echo "$CI_REGISTRY_PASSWORD" | docker login -u "$CI_REGISTRY_USER" --password-stdin "$CI_REGISTRY"
  script:
    - |
      TAGS="--tag $CI_REGISTRY_IMAGE/backend:$CI_COMMIT_SHA"
      [ "$CI_COMMIT_BRANCH" = "$CI_DEFAULT_BRANCH" ] && TAGS="$TAGS --tag $CI_REGISTRY_IMAGE/backend:latest"
      [ -n "$CI_COMMIT_TAG" ] && TAGS="$TAGS --tag $CI_REGISTRY_IMAGE/backend:$CI_COMMIT_TAG"
      docker buildx build --push $TAGS \
        --cache-from "type=registry,ref=$CI_REGISTRY_IMAGE/backend:buildcache" \
        --cache-to "type=registry,ref=$CI_REGISTRY_IMAGE/backend:buildcache,mode=max" \
        backend

# build:frontend mirrors build:backend with frontend/.

deploy:preproduction:
  stage: deploy
  image: alpine:3.20
  environment:
    name: pre-production
    url: https://staging.example.com
  resource_group: preproduction
  rules:
    - if: $CI_COMMIT_BRANCH =~ /^release-.*$/
  needs: [build:backend, build:frontend]
  script:
    - apk add --no-cache openssh-client curl
    - eval $(ssh-agent -s)
    - chmod 600 "$SSH_PRIVATE_KEY"
    - ssh-add "$SSH_PRIVATE_KEY"
    - mkdir -p ~/.ssh && chmod 700 ~/.ssh
    - printf '%s\n' "$SSH_KNOWN_HOSTS" > ~/.ssh/known_hosts
    - ssh "$STAGING_DEPLOY_USER@$STAGING_DEPLOY_HOST" "cd $STAGING_DEPLOY_PATH && IMAGE_TAG=$CI_COMMIT_SHA ./deploy.sh"
    - curl -fsS --retry 10 --retry-delay 3 https://staging.example.com/api/health

deploy:production:
  stage: deploy
  image: alpine:3.20
  environment:
    name: production
    url: https://quiz.example.com
  resource_group: production
  rules:
    - if: $CI_COMMIT_BRANCH == "master" || $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
    - if: $CI_COMMIT_TAG =~ /^v\d+\.\d+\.\d+$/
  needs: [build:backend, build:frontend]
  script:
    - apk add --no-cache openssh-client curl
    - eval $(ssh-agent -s)
    - chmod 600 "$SSH_PRIVATE_KEY"
    - ssh-add "$SSH_PRIVATE_KEY"
    - mkdir -p ~/.ssh && chmod 700 ~/.ssh
    - printf '%s\n' "$SSH_KNOWN_HOSTS" > ~/.ssh/known_hosts
    - ssh "$PROD_DEPLOY_USER@$PROD_DEPLOY_HOST" "cd $PROD_DEPLOY_PATH && IMAGE_TAG=$CI_COMMIT_SHA ./deploy.sh"
    - curl -fsS --retry 10 --retry-delay 3 https://quiz.example.com/api/health
```

## Appendix C — Deploy script sketch

```bash
#!/usr/bin/env bash
# /opt/oobleck/deploy.sh — run on the VPS with IMAGE_TAG set.
set -euo pipefail
: "${IMAGE_TAG:?IMAGE_TAG is required}"
cd /opt/oobleck

COMPOSE="docker compose"
PREVIOUS="$(cat .current_tag 2>/dev/null || true)"

exec 9>/tmp/oobleck-deploy.lock
flock -n 9 || { echo "another deploy is running"; exit 1; }

# Pre-deploy backup (skip on first deploy).
if $COMPOSE ps --status running db | grep -q db; then
  mkdir -p backups
  $COMPOSE exec -T db pg_dump -U "$POSTGRES_USER" "$POSTGRES_DB" \
    | gzip > "backups/pre-${IMAGE_TAG}-$(date +%Y%m%d%H%M%S).sql.gz"
  ls -1t backups/*.sql.gz | tail -n +8 | xargs -r rm --
fi

$COMPOSE pull backend frontend
IMAGE_TAG="$IMAGE_TAG" $COMPOSE up -d --wait --wait-timeout 180

for _ in $(seq 1 30); do
  curl -fsS http://127.0.0.1:8080/api/health >/dev/null && break
  sleep 2
done
curl -fsS http://127.0.0.1:8080/api/health >/dev/null

printf '%s' "$PREVIOUS" > .last_good_tag
printf '%s' "$IMAGE_TAG" > .current_tag
docker image prune -f
```

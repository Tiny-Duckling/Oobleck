# ---- build stage ----
FROM rust:1-slim-bookworm AS build
WORKDIR /app

# Cache the dependency build: copy manifests first, then sources.
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY data ./data

# sqlx::migrate! reads ./migrations at compile time, but cargo does not track
# that dependency — a migration-only change would otherwise reuse stale cached
# binaries. Touching the crate sources forces workspace recompiles whenever the
# copied inputs change, while dependencies stay cached in the mount below.
RUN touch src/main.rs src/bin/*.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --bins \
    && cp target/release/oobleck-backend target/release/seed target/release/seed_gre /usr/local/bin/

# ---- runtime stage ----
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /usr/local/bin/oobleck-backend /usr/local/bin/oobleck-backend
COPY --from=build /usr/local/bin/seed /usr/local/bin/seed
COPY --from=build /usr/local/bin/seed_gre /usr/local/bin/seed_gre
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Seed (idempotent) then start the API. JWT_SECRET must be provided via env.
ENTRYPOINT ["docker-entrypoint.sh"]

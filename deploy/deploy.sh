#!/usr/bin/env bash
# Deploy script — run on VPS with IMAGE_TAG set.
set -euo pipefail
: "${IMAGE_TAG:?IMAGE_TAG is required}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

COMPOSE="docker compose"
PREVIOUS="$(cat .current_tag 2>/dev/null || true)"

exec 9>/tmp/oobleck-deploy.lock
flock -n 9 || { echo "another deploy is running"; exit 1; }

# Load .env if present
if [ -f .env ]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

# Pre-deploy backup (skip on first deploy).
if $COMPOSE ps --status running db 2>/dev/null | grep -q db; then
  mkdir -p backups
  $COMPOSE exec -T db pg_dump -U "$POSTGRES_USER" "$POSTGRES_DB" \
    | gzip > "backups/pre-${IMAGE_TAG}-$(date +%Y%m%d%H%M%S).sql.gz"
  ls -1t backups/*.sql.gz 2>/dev/null | tail -n +8 | xargs -r rm -- 2>/dev/null || true
fi

# Export image tags and pull
if [ -n "${REGISTRY_IMAGE:-}" ]; then
  export BACKEND_IMAGE="${REGISTRY_IMAGE}/backend:${IMAGE_TAG}"
  export FRONTEND_IMAGE="${REGISTRY_IMAGE}/frontend:${IMAGE_TAG}"
fi

$COMPOSE pull backend frontend
$COMPOSE up -d --wait --wait-timeout 180

# Verify health endpoint
HEALTH_URL="${HEALTH_URL:-http://127.0.0.1:${FRONTEND_PORT:-8080}/api/health}"
echo "Verifying health endpoint at ${HEALTH_URL}..."
HEALTH_OK=0
for _ in $(seq 1 30); do
  if curl -fsS "$HEALTH_URL" >/dev/null 2>&1; then
    HEALTH_OK=1
    break
  fi
  sleep 2
done

if [ "$HEALTH_OK" -ne 1 ]; then
  echo "Health check failed after 60 seconds!"
  exit 1
fi

[ -n "$PREVIOUS" ] && printf '%s' "$PREVIOUS" > .last_good_tag
printf '%s' "$IMAGE_TAG" > .current_tag

# Clean up dangling images
docker image prune -f
echo "Deployment of ${IMAGE_TAG} completed successfully."

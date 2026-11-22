#!/usr/bin/env bash
# Rollback script — redeploys the last known good tag.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

if [ ! -f .last_good_tag ] || [ ! -s .last_good_tag ]; then
  echo "Error: .last_good_tag does not exist or is empty. Cannot perform automated rollback."
  exit 1
fi

LAST_GOOD="$(cat .last_good_tag)"
echo "Initiating rollback to last known good tag: ${LAST_GOOD}"
IMAGE_TAG="$LAST_GOOD" "$SCRIPT_DIR/deploy.sh"

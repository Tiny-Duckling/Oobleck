#!/bin/sh
set -e

echo "seeding database (idempotent)..."
seed
seed_gre

echo "starting API on port ${PORT:-4000}..."
exec quiz-backend

#!/usr/bin/env bash
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_USER="${DB_USER:-dev}"
DB_PASSWORD="${DB_PASSWORD:-devpassword}"
DB_NAME="${DB_NAME:-kreek_db}"

cd "$SCRIPTS_DIR"

uv run seed_space_members.py \
    --host     "$DB_HOST" \
    --port     "$DB_PORT" \
    --user     "$DB_USER" \
    --password "$DB_PASSWORD" \
    --database "$DB_NAME" \
    "$@"

#!/usr/bin/env bash
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

ENV_FILE="${SCRIPTS_DIR}/../.env.legacy"
[ -f "$ENV_FILE" ] || { echo "Erreur : $ENV_FILE introuvable" >&2; exit 1; }
set -a; source "$ENV_FILE"; set +a

OUTPUT="${OUTPUT:-${SCRIPTS_DIR}/extracted_competitions.json}"

cd "$SCRIPTS_DIR"

uv run python extract_competitions.py \
    --host     "${LEGACY_DB_HOST}" \
    --port     "${LEGACY_DB_PORT}" \
    --user     "${LEGACY_DB_USER}" \
    --password "${LEGACY_DB_PASSWORD}" \
    --database "${LEGACY_DB_NAME}" \
    --output   "$OUTPUT" \
    "$@"

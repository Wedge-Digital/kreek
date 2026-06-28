#!/usr/bin/env bash
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -n "${EXEC_PROFILE:-}" ]; then
    :
elif [ -n "${1:-}" ] && [[ "${1}" != -* ]]; then
    EXEC_PROFILE="$1"; shift
else
    EXEC_PROFILE="dev"
fi

ENV_FILE="${SCRIPTS_DIR}/../.env.${EXEC_PROFILE}"
[ -f "$ENV_FILE" ] || { echo "Erreur : $ENV_FILE introuvable" >&2; exit 1; }
set -a; source "$ENV_FILE"; set +a

INPUT="${INPUT:-${SCRIPTS_DIR}/extracted_spaces.json}"

cd "$SCRIPTS_DIR"

uv run import_spaces.py \
    --host     "${DATABASE__HOST}" \
    --port     "${DATABASE__PORT}" \
    --user     "${DATABASE__USER}" \
    --password "${DATABASE__PWD}" \
    --database "${DATABASE__NAME}" \
    --input    "$INPUT" \
    "$@"
#!/usr/bin/env bash
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

ENV_FILE="${SCRIPTS_DIR}/../.env.legacy"
[ -f "$ENV_FILE" ] || { echo "Erreur : $ENV_FILE introuvable" >&2; exit 1; }
set -a; source "$ENV_FILE"; set +a

INPUT="${INPUT:-${SCRIPTS_DIR}/extracted_articles.json}"

cd "$SCRIPTS_DIR"

uv run migrate_articles_images.py \
    --cloud-name "${CLOUDINARY_CLOUD_NAME}" \
    --api-key    "${CLOUDINARY_API_KEY}" \
    --api-secret "${CLOUDINARY_API_SECRET}" \
    --input      "$INPUT" \
    "$@"

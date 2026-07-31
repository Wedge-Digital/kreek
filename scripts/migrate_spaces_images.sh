#!/usr/bin/env bash
# Migre les logos d'espaces vers Cloudinary et réécrit extracted_spaces.json.
#
# À lancer AVANT import_spaces.sh : l'import recopie le champ `logo` dans
# `spaces.space_icon_path`, que le value object CloudinaryImage refuse tant
# qu'il pointe sur un chemin legacy — auquel cas /app/space/all s'affiche vide.
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

ENV_FILE="${SCRIPTS_DIR}/../.env.legacy"
[ -f "$ENV_FILE" ] || { echo "Erreur : $ENV_FILE introuvable" >&2; exit 1; }
set -a; source "$ENV_FILE"; set +a

for var in CLOUDINARY_CLOUD_NAME CLOUDINARY_API_KEY CLOUDINARY_API_SECRET; do
    [ -n "${!var:-}" ] || { echo "Erreur : $var absent de $ENV_FILE" >&2; exit 1; }
done

cd "$SCRIPTS_DIR"

uv run python migrate_spaces_images.py \
    --cloud-name "${CLOUDINARY_CLOUD_NAME}" \
    --api-key    "${CLOUDINARY_API_KEY}" \
    --api-secret "${CLOUDINARY_API_SECRET}" \
    --input      "${SCRIPTS_DIR}/extracted_spaces.json" \
    "$@"

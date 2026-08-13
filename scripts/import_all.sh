#!/usr/bin/env bash
# Lance tous les imports dans l'ordre de dépendance :
#   1. images spaces    (réécrit extracted_spaces.json)
#   2. images articles  (réécrit extracted_articles.json)
#   3. users            (pas de dépendance)
#   4. spaces           (pas de dépendance)
#   5. articles         (dépend de users + spaces)
#   6. competitions     (dépend de spaces ; désactivé par défaut, cf. --with-competitions)
#
# Les deux migrations d'images passent d'abord : elles corrigent les JSON
# extraits, que les imports recopient ensuite tels quels en base. Sans elles,
# `spaces.space_icon_path` garde un chemin legacy que le value object
# CloudinaryImage refuse, et /app/space/all s'affiche vide. Aucune écriture en
# base n'a lieu avant que les deux aient réussi.
#
# Elles sont idempotentes à double titre : un JSON déjà migré n'occasionne
# aucun appel réseau, et un `public_id` déterministe plus un test d'existence
# garantissent qu'aucune image n'est dupliquée sur Cloudinary.
#
# Usage :
#   ./scripts/import_all.sh [profil] [--with-competitions] [args…]
#
#   --with-competitions  importe aussi les compétitions legacy, désactivé par
#                        défaut — non intégrées pour le moment.
#
# Les autres arguments sont passés tels quels à chaque script d'import (ex : --dry-run).
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -n "${EXEC_PROFILE:-}" ]; then
    :
elif [ -n "${1:-}" ] && [[ "${1}" != -* ]]; then
    EXEC_PROFILE="$1"; shift
else
    EXEC_PROFILE="dev"
fi
export EXEC_PROFILE

WITH_COMPETITIONS=0
ARGS=()
for arg in "$@"; do
    case "$arg" in
        --with-competitions) WITH_COMPETITIONS=1 ;;
        *)                   ARGS+=("$arg") ;;
    esac
done
set -- ${ARGS[@]+"${ARGS[@]}"}

TOTAL=5
[ "$WITH_COMPETITIONS" -eq 1 ] && TOTAL=6

echo "=== [1/${TOTAL}] Migration des logos d'espaces vers Cloudinary ==="
"${SCRIPTS_DIR}/migrate_spaces_images.sh" "$@"

echo ""
echo "=== [2/${TOTAL}] Migration des images d'articles vers Cloudinary ==="
"${SCRIPTS_DIR}/migrate_articles_images.sh" "$@"

echo ""
echo "=== [3/${TOTAL}] Import users ==="
"${SCRIPTS_DIR}/import_users.sh" "$@"

echo ""
echo "=== [4/${TOTAL}] Import spaces ==="
"${SCRIPTS_DIR}/import_spaces.sh" "$@"

echo ""
echo "=== [5/${TOTAL}] Import articles ==="
"${SCRIPTS_DIR}/import_articles.sh" "$@"

if [ "$WITH_COMPETITIONS" -eq 1 ]; then
    echo ""
    echo "=== [6/${TOTAL}] Import competitions ==="
    "${SCRIPTS_DIR}/import_competitions.sh" "$@"
else
    echo ""
    echo "Compétitions legacy non intégrées pour le moment (utiliser --with-competitions pour les importer)."
fi

echo ""
echo "Import terminé."

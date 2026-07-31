#!/usr/bin/env bash
# Lance tous les imports dans l'ordre de dépendance :
#   1. images spaces    (réécrit extracted_spaces.json)
#   2. images articles  (réécrit extracted_articles.json)
#   3. users            (pas de dépendance)
#   4. spaces           (pas de dépendance)
#   5. articles         (dépend de users + spaces)
#   6. competitions     (dépend de spaces ; sauté si le fichier extrait manque)
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
# Passe tous les arguments reçus à chaque script (ex : --dry-run).
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

echo "=== [1/6] Migration des logos d'espaces vers Cloudinary ==="
"${SCRIPTS_DIR}/migrate_spaces_images.sh" "$@"

echo ""
echo "=== [2/6] Migration des images d'articles vers Cloudinary ==="
"${SCRIPTS_DIR}/migrate_articles_images.sh" "$@"

echo ""
echo "=== [3/6] Import users ==="
"${SCRIPTS_DIR}/import_users.sh" "$@"

echo ""
echo "=== [4/6] Import spaces ==="
"${SCRIPTS_DIR}/import_spaces.sh" "$@"

echo ""
echo "=== [5/6] Import articles ==="
"${SCRIPTS_DIR}/import_articles.sh" "$@"

echo ""
echo "=== [6/6] Import competitions ==="
"${SCRIPTS_DIR}/import_competitions.sh" "$@"

echo ""
echo "Import terminé."

#!/usr/bin/env bash
# Lance toutes les extractions depuis la base legacy MySQL.
#
# Chaque wrapper lit lui-même `.env.legacy` (connexion legacy) et écrit son
# `scripts/extracted_<domaine>.json`, nom attendu par l'import correspondant.
# Une fois terminé : `./scripts/import_all.sh [profil]` pour charger le tout.
#
# L'ordre reprend celui de import_all.sh par symétrie — les extractions sont
# indépendantes entre elles, contrairement aux imports.
#
# La première extraction qui échoue interrompt tout (set -e), pour ne jamais
# importer un jeu de données partiel.
#
# Usage :
#   ./scripts/export_all.sh [--with-competitions] [args…]
#
#   --with-competitions  ajoute l'extraction des compétitions, désactivée par
#                        défaut. Sans elle, import_competitions.sh se contente
#                        de sauter l'étape (fichier absent).
#
# Les autres arguments sont passés tels quels à chaque script d'extraction.
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

WITH_COMPETITIONS=0
ARGS=()
for arg in "$@"; do
    case "$arg" in
        --with-competitions) WITH_COMPETITIONS=1 ;;
        *)                   ARGS+=("$arg") ;;
    esac
done
set -- ${ARGS[@]+"${ARGS[@]}"}

TOTAL=3
[ "$WITH_COMPETITIONS" -eq 1 ] && TOTAL=4

echo "=== [1/${TOTAL}] Extraction users ==="
"${SCRIPTS_DIR}/extract_users.sh" "$@"

echo ""
echo "=== [2/${TOTAL}] Extraction spaces ==="
"${SCRIPTS_DIR}/extract_spaces.sh" "$@"

echo ""
echo "=== [3/${TOTAL}] Extraction articles ==="
"${SCRIPTS_DIR}/extract_articles.sh" "$@"

if [ "$WITH_COMPETITIONS" -eq 1 ]; then
    echo ""
    echo "=== [4/${TOTAL}] Extraction competitions ==="
    "${SCRIPTS_DIR}/extract_competitions.sh" "$@"
else
    echo ""
    echo "Compétitions ignorées (utiliser --with-competitions pour les extraire)."
fi

echo ""
echo "Extraction terminée."

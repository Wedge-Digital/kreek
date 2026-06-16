#!/usr/bin/env bash
# Lance tous les imports dans l'ordre de dépendance :
#   1. users      (pas de dépendance)
#   2. spaces     (pas de dépendance)
#   3. articles   (dépend de users + spaces)
#   4. competitions (dépend de spaces)
#
# Passe tous les arguments reçus à chaque script (ex : --dry-run).
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== [1/4] Import users ==="
"${SCRIPTS_DIR}/import_users.sh" "$@"

echo ""
echo "=== [2/4] Import spaces ==="
"${SCRIPTS_DIR}/import_spaces.sh" "$@"

echo ""
echo "=== [3/4] Import articles ==="
"${SCRIPTS_DIR}/import_articles.sh" "$@"

echo ""
echo "=== [4/4] Import competitions ==="
"${SCRIPTS_DIR}/import_competitions.sh" "$@"

echo ""
echo "Import terminé."

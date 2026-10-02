#!/usr/bin/env bash
#
# Sauvegarde la base de production dans `dumps/`, sans rien restaurer.
#
#   make backup_prod_db
#
# Un `pg_dump`, donc uniquement des lectures sur la source : pas de base cible,
# donc ni garde de localité ni question. Le dump est celui d'`import_prod_db`
# (`scripts/lib/prod_db.sh`) — il se restaure de la même façon.
#
# ── Les données ─────────────────────────────────────────────────────────────
#
# Le dump contient les vraies données : adresses électroniques des coachs,
# empreintes de mots de passe. `dumps/` est ignoré par git, mais le fichier
# reste en clair sur le disque — c'est un secret, à traiter comme tel.
set -euo pipefail

RACINE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$RACINE"

SOURCE_PROFILE="${SOURCE_PROFILE:-remote.prod}"

# shellcheck source=lib/prod_db.sh
source scripts/lib/prod_db.sh

SOURCE_URL=$(url_du_profil "$SOURCE_PROFILE")
SOURCE_HOTE=$(hote_de "$SOURCE_URL")

echo ""
echo "  ${gras}Sauvegarder la base de production${nul}"
echo ""
echo "     Source  : $(sans_secret "$SOURCE_URL")"
echo ""
dumper_production "$SOURCE_URL" "$SOURCE_HOTE"

echo ""
echo "  ${vert}${gras}✓${nul} Production sauvegardée."
echo ""
echo "    Le dump garde les adresses électroniques et les empreintes de mots de"
echo "    passe des coachs. Il est dans dumps/, que git ignore — mais il reste"
echo "    en clair sur ce disque."
echo ""

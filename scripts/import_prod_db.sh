#!/usr/bin/env bash
#
# Recopie la base de production dans une base **locale**.
#
#   make import_prod_db                          # prod → dev
#   make import_prod_db TARGET_PROFILE=test      # prod → test
#   YES=1 make import_prod_db                    # sans la question
#
# ── Ce que ce script détruit, et ce qu'il ne touche jamais ───────────────────
#
# Il **détruit la base cible** et la remplace. Il ne fait sur la source qu'un
# `pg_dump`, c'est-à-dire des lectures — mais une inversion des deux URLs
# écraserait la production, et c'est le seul accident qui compte ici. Trois
# gardes, dans cet ordre :
#
#   1. la cible doit être locale — un hôte distant est refusé, sans dérogation
#      possible, contrairement au `I_KNOW_THIS_IS_REMOTE` des autres cibles ;
#   2. la source et la cible doivent différer ;
#   3. la question est posée, sauf `YES=1`.
#
# La garde 1 n'a **pas** d'échappatoire, et c'est délibéré : `reset_db` en a une
# parce qu'on peut vouloir réinitialiser la démo, tandis qu'écrire la production
# depuis ce script n'a aucun usage légitime.
#
# ── Les migrations ──────────────────────────────────────────────────────────
#
# La production est souvent en retard d'une ou deux migrations sur le dépôt —
# mesuré au moment d'écrire : 60 contre 61. Restaurer sans rejouer laisserait
# une base que le code local ne sait plus lire, et l'erreur tomberait bien plus
# tard, sur une colonne absente. `sqlx migrate run` termine donc le travail.
#
# ── Les données ─────────────────────────────────────────────────────────────
#
# Le dump contient les vraies données : adresses électroniques des coachs,
# empreintes de mots de passe. Il atterrit dans `dumps/`, que `.gitignore`
# couvre déjà — mais il reste sur le disque, en clair, jusqu'à ce qu'on l'efface.
set -euo pipefail

RACINE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$RACINE"

SOURCE_PROFILE="${SOURCE_PROFILE:-remote.prod}"
TARGET_PROFILE="${TARGET_PROFILE:-dev}"
YES="${YES:-0}"

rouge=$'\033[31m'; vert=$'\033[32m'; gras=$'\033[1m'; nul=$'\033[0m'

echec() { echo ""; echo "  ${rouge}${gras}/!\\  $*${nul}"; echo ""; exit 1; }

url_du_profil() {
    local fichier=".env.$1"
    [ -f "$fichier" ] || echec "Profil « $1 » introuvable : $fichier n'existe pas."
    local url
    url=$(grep -E '^DATABASE__URL=' "$fichier" | head -1 | cut -d= -f2- | tr -d '"'"'")
    [ -n "$url" ] || echec "Aucun DATABASE__URL dans $fichier."
    printf '%s' "$url"
}

hote_de() { printf '%s' "$1" | sed -E 's#^[^:]+://([^/]*@)?([^:/?]+).*#\2#'; }
sans_secret() { printf '%s' "$1" | sed -E 's#://[^@]*@#://***@#'; }

SOURCE_URL=$(url_du_profil "$SOURCE_PROFILE")
TARGET_URL=$(url_du_profil "$TARGET_PROFILE")
SOURCE_HOTE=$(hote_de "$SOURCE_URL")
TARGET_HOTE=$(hote_de "$TARGET_URL")

# ── Garde 1 : la cible est locale, sans dérogation ───────────────────────────
case "$TARGET_HOTE" in
    localhost|127.0.0.1|::1) ;;
    *) echec "Refus : la cible « $TARGET_HOTE » n'est pas locale.

     Profil cible : $TARGET_PROFILE
     Cible        : $(sans_secret "$TARGET_URL")

  Ce script détruit la base cible. Il ne l'écrira jamais ailleurs qu'en local,
  et cette garde n'a pas d'échappatoire." ;;
esac

# ── Garde 2 : source et cible distinctes ─────────────────────────────────────
[ "$SOURCE_URL" != "$TARGET_URL" ] || echec "Refus : la source et la cible sont la même base."

# ── Garde 3 : la question ────────────────────────────────────────────────────
echo ""
echo "  ${gras}Importer la base de production dans une base locale${nul}"
echo ""
echo "     Source  : $(sans_secret "$SOURCE_URL")"
echo "     Cible   : $(sans_secret "$TARGET_URL")   ${rouge}(sera détruite)${nul}"
echo ""
if [ "$YES" != "1" ]; then
    printf "  Détruire la base cible et la remplacer ? [oui/N] "
    read -r reponse < /dev/tty || reponse=""
    case "$reponse" in
        oui|OUI|o|O) ;;
        *) echo ""; echo "  Abandon."; echo ""; exit 1 ;;
    esac
fi

# ── Le dump ──────────────────────────────────────────────────────────────────
mkdir -p dumps
HORODATAGE=$(date +%Y_%m_%d_%H_%M_%S)
DUMP="dumps/${SOURCE_HOTE}_prod-${HORODATAGE}.dump"

echo ""
echo "  ${gras}1/4${nul}  Lecture de la production…"
# `-Fc` plutôt que du SQL : `pg_restore` peut alors ignorer ce qu'il ne sait pas
# rejouer sans avaler tout le fichier. `--no-owner` et `--no-privileges` parce
# que les rôles de production n'existent pas en local — sans eux, la
# restauration échoue sur chaque `ALTER TABLE ... OWNER TO`.
pg_dump --format=custom --no-owner --no-privileges --file="$DUMP" "$SOURCE_URL"
echo "      $DUMP ($(du -h "$DUMP" | cut -f1))"

# ── La cible ─────────────────────────────────────────────────────────────────
echo "  ${gras}2/4${nul}  Remise à zéro de la cible…"
DATABASE_URL="$TARGET_URL" sqlx database drop -y >/dev/null
DATABASE_URL="$TARGET_URL" sqlx database create

echo "  ${gras}3/4${nul}  Restauration…"
# `--no-owner` de nouveau, et pas de `--exit-on-error` : un dump de production
# porte parfois des objets qu'une base neuve refuse (extensions déjà là,
# commentaires sur des rôles absents). Ces avertissements ne compromettent pas
# les données, et s'arrêter dessus rendrait le script inutilisable.
pg_restore --no-owner --no-privileges --dbname="$TARGET_URL" "$DUMP" 2>&1 \
    | grep -vE "^$" | sed 's/^/      /' || true

# ── Les migrations ───────────────────────────────────────────────────────────
echo "  ${gras}4/4${nul}  Migrations manquantes…"
avant=$(psql "$TARGET_URL" -t -A -c 'SELECT count(*) FROM _sqlx_migrations' 2>/dev/null || echo 0)
DATABASE_URL="$TARGET_URL" sqlx migrate run
apres=$(psql "$TARGET_URL" -t -A -c 'SELECT count(*) FROM _sqlx_migrations' 2>/dev/null || echo 0)
echo "      $avant → $apres migrations"

echo ""
echo "  ${vert}${gras}✓${nul} Base « $TARGET_PROFILE » remplacée par la production."
echo ""
echo "    Le dump garde les adresses électroniques et les empreintes de mots de"
echo "    passe des coachs. Il est dans dumps/, que git ignore — mais il reste"
echo "    en clair sur ce disque :"
echo ""
echo "        rm $DUMP"
echo ""

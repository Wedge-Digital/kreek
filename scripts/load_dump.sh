#!/usr/bin/env bash
#
# Charge un des dumps de `dumps/` dans une base **locale**.
#
#   make load_dump                                  # menu → dev
#   make load_dump TARGET_PROFILE=test              # menu → test
#   make load_dump DUMP=dumps/reaper_prod-….dump    # sans le menu
#   YES=1 make load_dump DUMP=…                     # sans la question
#
# ── Ce que ce script détruit ────────────────────────────────────────────────
#
# Il **détruit la base cible** et la remplace par le contenu du dump. Mêmes
# gardes qu'`import_prod_db`, moins celle qui compare source et cible — la
# source est ici un fichier :
#
#   1. la cible doit être locale, sans dérogation possible ;
#   2. la question est posée, sauf `YES=1`.
#
# ── Les deux formats ────────────────────────────────────────────────────────
#
# `dumps/` contient des `.dump` (`pg_dump -Fc`, ceux de `backup_prod_db`) et des
# `.sql` en clair (exportés à la main). Le format se lit dans l'en-tête du
# fichier, pas dans son extension : `PGDMP` → `pg_restore`, sinon `psql`.
#
# ── Les migrations ──────────────────────────────────────────────────────────
#
# Un dump est presque toujours en retard sur le dépôt. `sqlx migrate run`
# termine le travail, comme dans `import_prod_db`.
set -euo pipefail

RACINE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$RACINE"

TARGET_PROFILE="${TARGET_PROFILE:-dev}"
YES="${YES:-0}"
DUMP="${DUMP:-}"

# shellcheck source=lib/prod_db.sh
source scripts/lib/prod_db.sh

TARGET_URL=$(url_du_profil "$TARGET_PROFILE")
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

# ── Le choix du dump ─────────────────────────────────────────────────────────
choisir_dump() {
    local dumps=()
    # Un fichier vide est un `pg_dump` interrompu : le charger viderait la base.
    while IFS= read -r f; do [ -s "$f" ] && dumps+=("$f"); done \
        < <(ls -t dumps/*.dump dumps/*.sql 2>/dev/null || true)
    [ ${#dumps[@]} -gt 0 ] || echec "Aucun dump dans dumps/ (*.dump, *.sql)."

    echo ""
    echo "  ${gras}Dumps disponibles${nul} (du plus récent au plus ancien)"
    echo ""
    local i
    for i in "${!dumps[@]}"; do
        printf "    %2d) %-55s %6s  %s\n" $((i + 1)) "${dumps[$i]#dumps/}" \
            "$(du -h "${dumps[$i]}" | cut -f1)" \
            "$(date -r "${dumps[$i]}" '+%Y-%m-%d %H:%M')"
    done
    echo ""
    printf "  Numéro du dump à charger [1] : "
    local choix
    read -r choix < /dev/tty || choix=""
    choix="${choix:-1}"
    case "$choix" in
        ''|*[!0-9]*) echec "Choix invalide : « $choix »." ;;
    esac
    [ "$choix" -ge 1 ] && [ "$choix" -le ${#dumps[@]} ] \
        || echec "Choix hors liste : $choix."
    DUMP="${dumps[$((choix - 1))]}"
}

if [ -z "$DUMP" ]; then
    choisir_dump
fi
[ -f "$DUMP" ] || echec "Dump introuvable : $DUMP"
[ -s "$DUMP" ] || echec "Dump vide : $DUMP"

if [ "$(head -c 5 "$DUMP")" = "PGDMP" ]; then FORMAT=custom; else FORMAT=sql; fi

# ── Garde 2 : la question ────────────────────────────────────────────────────
echo ""
echo "  ${gras}Charger un dump dans une base locale${nul}"
echo ""
echo "     Dump    : $DUMP  ($FORMAT)"
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

# ── La cible ─────────────────────────────────────────────────────────────────
echo ""
echo "  ${gras}1/3${nul}  Remise à zéro de la cible…"
DATABASE_URL="$TARGET_URL" sqlx database drop -y >/dev/null
DATABASE_URL="$TARGET_URL" sqlx database create

echo "  ${gras}2/3${nul}  Restauration…"
# Ni `--exit-on-error` ni `ON_ERROR_STOP` : un dump porte souvent des objets
# qu'une base neuve refuse (rôles absents, extensions déjà là). Ces erreurs ne
# compromettent pas les données — cf. `import_prod_db.sh`.
if [ "$FORMAT" = custom ]; then
    pg_restore --no-owner --no-privileges --dbname="$TARGET_URL" "$DUMP" 2>&1 \
        | grep -vE "^$" | sed 's/^/      /' || true
else
    psql --quiet --dbname="$TARGET_URL" --file="$DUMP" 2>&1 >/dev/null \
        | grep -vE "^$" | sed 's/^/      /' || true
fi

# ── Les migrations ───────────────────────────────────────────────────────────
echo "  ${gras}3/3${nul}  Migrations manquantes…"
avant=$(psql "$TARGET_URL" -t -A -c 'SELECT count(*) FROM _sqlx_migrations' 2>/dev/null || echo 0)
DATABASE_URL="$TARGET_URL" sqlx migrate run
apres=$(psql "$TARGET_URL" -t -A -c 'SELECT count(*) FROM _sqlx_migrations' 2>/dev/null || echo 0)
echo "      $avant → $apres migrations"

echo ""
echo "  ${vert}${gras}✓${nul} Base « $TARGET_PROFILE » remplacée par $DUMP."
echo ""

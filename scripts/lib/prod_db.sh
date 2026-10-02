#!/usr/bin/env bash
#
# Ce que partagent `import_prod_db.sh` et `backup_prod_db.sh` : lire l'URL d'un
# profil, la montrer sans son mot de passe, et faire le dump de la production.
#
# Se charge par `source`, depuis la racine du dépôt.

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

# Écrit le dump dans `dumps/` et pose son chemin dans `DUMP`. Pas de `$(…)` pour
# le récupérer : `set -e` n'y est pas hérité, et un `pg_dump` en échec passerait
# pour réussi.
dumper_production() {
    local SOURCE_URL="$1" SOURCE_HOTE="$2"
    mkdir -p dumps
    local HORODATAGE; HORODATAGE=$(date +%Y_%m_%d_%H_%M_%S)
    DUMP="dumps/${SOURCE_HOTE}_prod-${HORODATAGE}.dump"

    # `-Fc` plutôt que du SQL : `pg_restore` peut alors ignorer ce qu'il ne sait pas
    # rejouer sans avaler tout le fichier. `--no-owner` et `--no-privileges` parce
    # que les rôles de production n'existent pas en local — sans eux, la
    # restauration échoue sur chaque `ALTER TABLE ... OWNER TO`.
    #
    # Un `pg_dump` en échec laisse un fichier vide, qui passerait pour une
    # sauvegarde : il est retiré avant d'échouer.
    pg_dump --format=custom --no-owner --no-privileges --file="$DUMP" "$SOURCE_URL" \
        || { rm -f "$DUMP"; echec "Le dump a échoué — aucun fichier n'est conservé."; }
    echo "      $DUMP ($(du -h "$DUMP" | cut -f1))"
}

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
port_de() { printf '%s' "$1" | sed -nE 's#^[^:]+://([^/]*@)?[^:/?]+:([0-9]+).*#\2#p'; }
sans_secret() { printf '%s' "$1" | sed -E 's#://[^@]*@#://***@#'; }

# ── Le tunnel SSH ────────────────────────────────────────────────────────────
#
# Un profil qui porte `SSH_TUNNEL=<utilisateur>@<machine>` n'est pas joint en
# direct : on ouvre un tunnel d'un port local libre vers le port de la base, vu
# depuis cette machine (`localhost`). La destination vit dans le profil, jamais
# ici (règle 16 de CLAUDE.md). Sans `SSH_TUNNEL`, rien ne change.
#
# Le port local est tiré au moment même — un port fixe entrerait en collision
# avec une base locale ou un autre tunnel. `ExitOnForwardFailure` fait échouer
# `ssh -f` si la redirection ne s'établit pas : quand il rend la main, le tunnel
# est prêt. Le socket de contrôle sert à le refermer à la sortie, succès ou non.

tunnel_du_profil() {
    grep -E '^SSH_TUNNEL=' ".env.$1" 2>/dev/null | head -1 | cut -d= -f2- | tr -d '"'"'"
}

port_local_libre() {
    python3 -I -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])'
}

# Ouvre le tunnel du profil s'il en a un, et pose dans `SOURCE_URL` l'URL qui
# passe par lui. L'hôte d'origine reste dans `SOURCE_HOTE`, qui nomme le dump.
ouvrir_tunnel_si_besoin() {
    local destination; destination=$(tunnel_du_profil "$1")
    [ -n "$destination" ] || return 0
    local port_distant; port_distant=$(port_de "$SOURCE_URL"); port_distant="${port_distant:-5432}"
    local port_local; port_local=$(port_local_libre)
    TUNNEL_SOCKET="$(mktemp -u "${TMPDIR:-/tmp}/kreek-tunnel.XXXXXX")"
    TUNNEL_DESTINATION="$destination"
    trap fermer_tunnel EXIT
    echo "      Tunnel SSH : 127.0.0.1:$port_local → $destination:$port_distant"
    ssh -f -N -M -S "$TUNNEL_SOCKET" -o ExitOnForwardFailure=yes \
        -L "127.0.0.1:$port_local:localhost:$port_distant" "$destination" \
        || echec "Le tunnel SSH vers « $destination » n'a pas pu s'ouvrir."
    SOURCE_URL=$(printf '%s' "$SOURCE_URL" | sed -E "s#@[^/@]+/#@127.0.0.1:$port_local/#")
}

fermer_tunnel() {
    [ -n "${TUNNEL_SOCKET:-}" ] && [ -S "$TUNNEL_SOCKET" ] || return 0
    ssh -S "$TUNNEL_SOCKET" -O exit "$TUNNEL_DESTINATION" 2>/dev/null || true
}

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
    #
    # `--exclude-table-data=sessions` (carte 491) : la table est gardée, ses lignes
    # non. Elles ne sont rangées que sous l'empreinte des cookies, donc ne se
    # rejouent pas, mais chacune porte une copie de l'empreinte du mot de passe de
    # son coach — et une base locale n'a aucun usage des sessions de production.
    # Sans `--strict-names`, le motif est ignoré tant que la table n'existe pas.
    pg_dump --format=custom --no-owner --no-privileges --exclude-table-data=sessions \
        --file="$DUMP" "$SOURCE_URL" \
        || { rm -f "$DUMP"; echec "Le dump a échoué — aucun fichier n'est conservé."; }
    echo "      $DUMP ($(du -h "$DUMP" | cut -f1))"
}

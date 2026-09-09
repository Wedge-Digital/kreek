#!/usr/bin/env bash
# Échantillonne le pool de connexions du serveur pendant une suite e2e.
#
# Pourquoi : la page qui expire à 30 s dans les passages rouges répond en 3 ms au
# repos — un facteur dix mille. Le serveur n'est donc pas lent, il est bloqué, et
# le pool (20 connexions) est le suspect naturel. Une fuite ferait attendre les
# tests tardifs, ce qui empire au fil d'un passage et se répare après que sqlx a
# moissonné les connexions inactives. C'est exactement le profil observé
# (468 s → 525 s, puis vert après plusieurs heures d'inactivité).
#
# Ce que ça prouve ou infirme : si le compte grimpe vers 20 et que des
# `idle in transaction` apparaissent, la cause est là et se corrige. Si le compte
# reste plat pendant un passage rouge, l'hypothèse tombe — et c'est aussi utile.
set -euo pipefail

URL=$(grep -hE '^DATABASE__URL=' .env.e2e | cut -d= -f2- | tr -d "\"'")
BASE=$(basename "${URL%%\?*}")
SORTIE=${1:?usage: e2e_pool_watch.sh <fichier>}

printf 'horodatage\ttotal\tactives\tinactives\tidle_in_tx\tverrous_attente\n' > "$SORTIE"
while true; do
    psql "$URL" -tA -F$'\t' -c "
        SELECT to_char(now(), 'HH24:MI:SS'),
               count(*),
               count(*) FILTER (WHERE state = 'active'),
               count(*) FILTER (WHERE state = 'idle'),
               count(*) FILTER (WHERE state = 'idle in transaction'),
               (SELECT count(*) FROM pg_locks WHERE NOT granted)
        FROM pg_stat_activity WHERE datname = '$BASE'
    " >> "$SORTIE" 2>/dev/null || true
    sleep 5
done

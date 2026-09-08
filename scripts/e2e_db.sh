#!/usr/bin/env bash
#
# Rend la base e2e identique à son gabarit, avant chaque passage de la suite.
#
# # Pourquoi un gabarit plutôt qu'une réinitialisation
#
# `sqlx database reset` rejoue toutes les migrations : cinq secondes aujourd'hui,
# et ce coût grandit avec le schéma. `CREATE DATABASE … TEMPLATE` est une copie
# de fichiers — moins d'une seconde, insensible à l'histoire des migrations.
# Une réinitialisation qu'on paie à chaque lancement finit par se faire sauter ;
# un clonage, non.
#
# # Pourquoi une base à part
#
# La suite tournait dans la base de travail, que rien ne purgeait jamais :
# 155 Mo après une journée, dont 94 % de résidus, et 25 % de lenteur que rien ne
# signalait. La purger automatiquement aurait détruit le travail manuel du
# développeur à chaque lancement — d'où une base dédiée, que la suite peut
# écraser sans rien coûter à personne.
#
# Cf. carte 536.
set -euo pipefail

PROFIL="${PROFIL:-e2e}"
RACINE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$RACINE"

lire_env() {
  grep -E "^$1=" ".env.$PROFIL" 2>/dev/null | cut -d= -f2- | tr -d "\"'" | head -1
}

URL="$(lire_env DATABASE__URL)"
BASE="$(lire_env DATABASE__NAME)"
GABARIT="${BASE}_gabarit"

masquer() { sed -E 's#://[^@]*@#://***@#'; }

if [ -z "$URL" ] || [ -z "$BASE" ]; then
  echo "e2e_db: .env.$PROFIL ne définit pas DATABASE__URL et DATABASE__NAME" >&2
  exit 1
fi

# La même garde que le Makefile : on ne réinitialise jamais une base distante.
case "$URL" in
  *localhost*|*127.0.0.1*) ;;
  *) echo "e2e_db: refus — la base n'est pas locale : $(echo "$URL" | masquer)" >&2; exit 1 ;;
esac

ADMIN="${URL%/*}/postgres"
psql_admin() { psql "$ADMIN" -v ON_ERROR_STOP=1 -q -t -A "$@"; }

existe() {
  [ "$(psql_admin -c "SELECT 1 FROM pg_database WHERE datname='$1'")" = "1" ]
}

connexions() {
  psql_admin -c "SELECT count(*) FROM pg_stat_activity WHERE datname='$1' AND pid <> pg_backend_pid()"
}

# Le gabarit est périmé dès que le dossier `migrations/` a bougé. Un gabarit
# périmé ferait tourner la suite sur le schéma d'hier **sans rien dire** — c'est
# pire que la lenteur qu'on corrige, d'où cette comparaison plutôt qu'une
# reconstruction manuelle qu'on oublierait.
# `_sqlx_migrations` compte les migrations **appliquées** ; le dossier en compte
# les fichiers. Les deux ne coïncident que si chaque fichier vaut une migration
# — ce qui est le cas ici, `sqlx migrate` n'ayant pas de fichiers `.down`.
migrations_du_dossier() { find migrations -name '*.sql' -type f | wc -l | tr -d ' '; }

migrations_du_gabarit() {
  existe "$GABARIT" || { echo "-1"; return; }
  psql "${URL%/*}/$GABARIT" -v ON_ERROR_STOP=1 -q -t -A \
    -c "SELECT count(*) FROM _sqlx_migrations" 2>/dev/null || echo "-1"
}

construire_le_gabarit() {
  local url_gabarit="${URL%/*}/$GABARIT"
  echo "e2e_db: construction du gabarit $GABARIT (migrations + seed)"
  liberer "$GABARIT"
  psql_admin -c "DROP DATABASE IF EXISTS $GABARIT" > /dev/null
  DATABASE_URL="$url_gabarit" sqlx database create
  DATABASE_URL="$url_gabarit" sqlx migrate run > /dev/null

  # `sqlx` lit DATABASE_URL, l'application lit DATABASE__URL — deux variables
  # distinctes, et n'en poser qu'une envoie le seed dans la base de travail
  # sans le moindre message. C'est arrivé à l'écriture de ce script : le
  # gabarit s'est retrouvé migré mais vide, et `kreek_db` reseedée à sa place.
  DATABASE_URL="$url_gabarit" DATABASE__URL="$url_gabarit" \
    cargo run --quiet -- seed-e2e
}

liberer() {
  psql_admin -c \
    "SELECT pg_terminate_backend(pid) FROM pg_stat_activity
     WHERE datname='$1' AND pid <> pg_backend_pid()" > /dev/null
}

# Le seed fait partie du gabarit autant que le schéma. Ne vérifier que les
# migrations laisserait vivre un gabarit migré mais vide — c'est arrivé à
# l'écriture de ce script, et le clonage a alors reproduit ce vide sans un mot,
# passage après passage.
gabarit_seede() {
  existe "$GABARIT" || return 1
  [ "$(psql "${URL%/*}/$GABARIT" -v ON_ERROR_STOP=1 -q -t -A \
        -c "SELECT count(*) FROM spaces WHERE space_name = 'Espace E2E'" \
        2>/dev/null || echo 0)" != "0" ]
}

if [ "$(migrations_du_gabarit)" != "$(migrations_du_dossier)" ] || ! gabarit_seede; then
  construire_le_gabarit
fi

# Un clonage exige que le gabarit n'ait aucune connexion. Personne ne s'y
# connecte jamais — mais le dire vaut mieux que de laisser PostgreSQL rendre son
# message, qui n'apprendrait rien à qui n'a pas ce script sous les yeux.
if [ "$(connexions "$GABARIT")" != "0" ]; then
  echo "e2e_db: refus — $GABARIT a des connexions ouvertes, le clonage échouerait" >&2
  exit 1
fi

# Le serveur de dev tient des connexions à la base e2e : sans cette coupure, le
# `DROP` échoue sur « base en cours d'utilisation ». Son pool se reconnecte seul
# ensuite — vérifié.
liberer "$BASE"
psql_admin -c "DROP DATABASE IF EXISTS $BASE" > /dev/null
psql_admin -c "CREATE DATABASE $BASE TEMPLATE $GABARIT" > /dev/null
echo "e2e_db: $BASE clonée depuis $GABARIT"

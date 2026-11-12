#!/usr/bin/env bash
#
# Vide la base e2e et la reseede, sous le serveur e2e vivant (carte 641).
#
#   make reset_e2e_db      # seul
#   make e2e               # avant chaque suite complète
#
# ── Pourquoi ─────────────────────────────────────────────────────────────────
#
# Rien n'effaçait ce que chaque suite crée. Mesuré le 2026-11-12, même code :
# 9 534 équipes → 19 à 22 min ; base neuve, 362 équipes → 8 min 25 s.
#
# ── Pourquoi TRUNCATE, et pas DROP ni clonage ────────────────────────────────
#
# Le serveur e2e tourne pendant la remise à zéro (règle 8 : on ne le
# redémarre pas). Un `DROP DATABASE` couperait son pool, et la base recréée
# perdrait `applied_data_migrations` : les migrations de données se
# rejoueraient au redémarrage suivant (carte 664). Le `TRUNCATE` ne détruit que
# les lignes — pool, schéma, `_sqlx_migrations` et `applied_data_migrations`
# restent, et **le serveur n'a pas à redémarrer**.
#
# Aucune migration SQL n'insère de données : une base vidée puis seedée est
# donc identique à une base neuve.
#
# ── Les gardes ───────────────────────────────────────────────────────────────
#
#   1. la base doit être locale — aucune dérogation ;
#   2. elle ne doit pas être la base de travail (`.env.dev`) ;
#   3. aucune session pytest ne doit l'utiliser — verrou exclusif, cf.
#      `tests/e2e/e2e_lock.py`. Refus immédiat, pas d'attente.
set -euo pipefail

# Un arrêt par `set -e` est muet ; celui-ci dit au moins où il a eu lieu.
trap 'echo "reset_e2e_db : arrêt ligne $LINENO (code $?)" >&2' ERR

RACINE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$RACINE"

masquer() { sed -E 's#://[^@]*@#://***@#g'; }

# Garde 3 : on se relance sous le verrou exclusif. Le helper pose la variable,
# ce qui arrête la récursion.
if [ "${KREEK_E2E_LOCK_HELD:-}" != "exclusive" ]; then
  exec python3 tests/e2e/e2e_lock.py exclusive -- "$0" "$@"
fi

# Un fichier absent n'est pas une erreur : la CI n'a pas de `.env.dev`. Sans ce
# test, `grep` y rend 2, `pipefail` le propage et `set -e` arrête le script
# sans un mot — c'est ce qui a fait échouer la CI après la carte 641.
lire_url() {
  [ -f "$1" ] || return 0
  grep -E '^DATABASE__URL=' "$1" 2>/dev/null | head -1 | cut -d= -f2- | tr -d "\"'"
}

# `hôte:port/base`, sans identifiants — de quoi comparer deux URLs.
identite() {
  printf '%s' "$1" | sed -E 's#^[^:]+://([^@/]*@)?([^:/?]+)(:([0-9]+))?/([^?]*).*#\2:\4/\5#'
}

URL="$(lire_url .env.e2e)"
if [ -z "$URL" ]; then
  echo "reset_e2e_db : .env.e2e ne définit pas DATABASE__URL" >&2
  exit 1
fi

HOTE="$(printf '%s' "$URL" | sed -E 's#^[^:]+://([^/]*@)?([^:/?]+).*#\2#')"
case "$HOTE" in
  localhost|127.0.0.1|::1) ;;
  *) echo "reset_e2e_db : refus — la base e2e n'est pas locale (hôte $HOTE)" >&2
     exit 1 ;;
esac

URL_TRAVAIL="$(lire_url .env.dev)"
if [ -n "$URL_TRAVAIL" ] && [ "$(identite "$URL")" = "$(identite "$URL_TRAVAIL")" ]; then
  echo "reset_e2e_db : refus — .env.e2e désigne la base de travail de .env.dev" >&2
  exit 1
fi

DEBUT=$(date +%s)

# Toutes les tables, sauf les deux registres de migrations. La liste est lue à
# chaque passage : une table ajoutée demain est couverte sans y penser.
TABLES="$(psql "$URL" -v ON_ERROR_STOP=1 -q -t -A -c "
  SELECT coalesce(string_agg(format('%I', tablename), ', ' ORDER BY tablename), '')
  FROM pg_tables
  WHERE schemaname = 'public'
    AND tablename NOT IN ('_sqlx_migrations', 'applied_data_migrations')" \
  2> >(masquer >&2))"

# Sans CASCADE : une clé étrangère vers une table épargnée doit échouer, pas
# vider en silence. `lock_timeout` : une transaction ouverte du serveur fait
# échouer en le disant, au lieu de bloquer.
if [ -n "$TABLES" ]; then
  psql "$URL" -v ON_ERROR_STOP=1 -q -c \
    "SET lock_timeout = '10s'; TRUNCATE $TABLES RESTART IDENTITY" 2>&1 | masquer
fi

# Le binaire applique les migrations avant de seeder : une migration que le
# serveur n'a pas encore jouée est couverte aussi.
EXEC_PROFILE=e2e cargo run --quiet -- seed-e2e 2>&1 | masquer

echo "reset_e2e_db : base e2e vidée et reseedée en $(( $(date +%s) - DEBUT )) s"

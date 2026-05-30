#!/usr/bin/env python3
"""
Importe les compétitions et saisons depuis extracted_competitions.json
vers la base PostgreSQL kreek.

Comportement :
  - Résout space_id via spaces.legacy_id = club_legacy_id
  - Insère les compétitions (ON CONFLICT (legacy_id) DO NOTHING)
  - Après insertion, récupère le kreek id via legacy_id ou (space_id, name)
  - Insère les saisons (ON CONFLICT (legacy_id) DO NOTHING)
  - Le règlement ref_ruleset est stocké dans rules JSONB : {"ruleset": "BB2020"}
  - Idempotent : rejouer le script n'insère que les éléments manquants

Usage :
    python scripts/import_competitions.py \\
        --host localhost --port 5432 \\
        --user kreek --password secret --database kreek_db \\
        --input scripts/extracted_competitions.json \\
        [--dry-run]
"""

import argparse
import json
import os
import sys

_SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))

try:
    import psycopg2
    import psycopg2.extras
except ImportError:
    print("Dépendance manquante : pip install psycopg2-binary", file=sys.stderr)
    sys.exit(1)

try:
    from ulid import ULID
except ImportError as e:
    print(f"Erreur : {e}", file=sys.stderr)
    sys.exit(1)

# ── SQL ───────────────────────────────────────────────────────────────────────

LOAD_SPACES_SQL = "SELECT id, legacy_id FROM spaces WHERE legacy_id IS NOT NULL"

INSERT_COMPETITION_SQL = """
    INSERT INTO competitions (id, legacy_id, space_id, name, logo)
    VALUES (%s, %s, %s, %s, '')
    ON CONFLICT (legacy_id) DO NOTHING
"""

# Après INSERT ... DO NOTHING, RETURNING ne retourne rien sur conflit.
# On fait un SELECT séparé pour récupérer l'id existant ou nouvellement inséré.
FIND_COMPETITION_SQL = """
    SELECT id FROM competitions
    WHERE legacy_id = %s
"""

INSERT_SEASON_SQL = """
    INSERT INTO competition_seasons (id, legacy_id, competition_id, name, rules, status)
    VALUES (%s, %s, %s, %s, %s, %s)
    ON CONFLICT (legacy_id) DO NOTHING
"""


# ── Helpers ───────────────────────────────────────────────────────────────────

def parse_args():
    p = argparse.ArgumentParser(description="Importe les compétitions et saisons dans kreek")
    p.add_argument("--host",     default="localhost")
    p.add_argument("--port",     type=int, default=5432)
    p.add_argument("--user",     required=True)
    p.add_argument("--password", required=True)
    p.add_argument("--database", required=True)
    p.add_argument("--input",    default=os.path.join(_SCRIPTS_DIR, "extracted_competitions.json"))
    p.add_argument("--dry-run",  action="store_true",
                   help="Simule l'import sans écrire en base")
    return p.parse_args()


def make_rules(ref_ruleset: str) -> str | None:
    if not ref_ruleset:
        return None
    return json.dumps({"ruleset": ref_ruleset}, ensure_ascii=False)


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    args = parse_args()

    with open(args.input, encoding="utf-8") as f:
        competitions = json.load(f)

    total_seasons = sum(len(c["seasons"]) for c in competitions)
    print(f"{len(competitions)} compétitions, {total_seasons} saisons — depuis {args.input}")
    if args.dry_run:
        print("Mode DRY-RUN — aucune écriture en base.")
        for c in competitions:
            print(f"  [DRY] compétition #{c['legacy_id']} «{c['name']}» "
                  f"(club_legacy_id={c['club_legacy_id']}, {len(c['seasons'])} saisons)")
            for s in c["seasons"]:
                print(f"    └─ saison #{s['legacy_id']} «{s['season_name']}» "
                      f"status={s['status']} ruleset={s['ref_ruleset']!r}")
        return

    print(f"Connexion à {args.host}:{args.port}/{args.database} …")
    try:
        conn = psycopg2.connect(
            host=args.host, port=args.port,
            user=args.user, password=args.password,
            dbname=args.database,
        )
        conn.autocommit = False
        cursor = conn.cursor()
    except psycopg2.Error as e:
        print(f"Erreur de connexion : {e}", file=sys.stderr)
        sys.exit(1)

    # Charger le mapping spaces legacy_id → kreek id
    cursor.execute(LOAD_SPACES_SQL)
    spaces_map = {row[1]: row[0] for row in cursor.fetchall()}
    print(f"  {len(spaces_map)} espaces chargés")

    comp_inserted = comp_skipped = comp_errors = 0
    seas_inserted = seas_skipped = seas_errors = 0

    for c in competitions:
        comp_legacy_id = c["legacy_id"]
        name           = c["name"]
        club_legacy_id = c["club_legacy_id"]

        space_id = spaces_map.get(club_legacy_id)
        if not space_id:
            print(f"  SKIP compétition #{comp_legacy_id} «{name}» "
                  f"(espace introuvable club_legacy_id={club_legacy_id})")
            comp_skipped += 1
            seas_skipped += len(c["seasons"])
            continue

        # ── Insertion compétition ─────────────────────────────────────────
        new_comp_ulid = str(ULID())
        try:
            cursor.execute(INSERT_COMPETITION_SQL, (
                new_comp_ulid, comp_legacy_id, space_id, name,
            ))
            if cursor.rowcount > 0:
                comp_inserted += 1
                competition_id = new_comp_ulid
            else:
                # Déjà présent — récupérer l'id existant
                cursor.execute(FIND_COMPETITION_SQL, (comp_legacy_id,))
                row = cursor.fetchone()
                if not row:
                    print(f"  ERREUR compétition #{comp_legacy_id} «{name}» : "
                          f"INSERT ignoré mais SELECT retourne rien", file=sys.stderr)
                    comp_errors += 1
                    seas_skipped += len(c["seasons"])
                    continue
                competition_id = row[0]
                comp_skipped += 1
        except psycopg2.Error as e:
            print(f"  ERREUR compétition #{comp_legacy_id} «{name}» : {e}", file=sys.stderr)
            conn.rollback()
            comp_errors += 1
            seas_skipped += len(c["seasons"])
            continue

        # ── Insertion saisons ─────────────────────────────────────────────
        for s in c["seasons"]:
            season_legacy_id = s["legacy_id"]
            season_name      = s["season_name"] or name  # fallback sur le nom de la compétition
            status           = s["status"]
            rules            = make_rules(s["ref_ruleset"])

            new_season_ulid = str(ULID())
            try:
                cursor.execute(INSERT_SEASON_SQL, (
                    new_season_ulid, season_legacy_id,
                    competition_id, season_name, rules, status,
                ))
                if cursor.rowcount > 0:
                    seas_inserted += 1
                else:
                    seas_skipped += 1
            except psycopg2.Error as e:
                print(f"  ERREUR saison #{season_legacy_id} «{season_name}» : {e}", file=sys.stderr)
                conn.rollback()
                seas_errors += 1

    conn.commit()
    cursor.close()
    conn.close()

    print()
    print("Résultat compétitions :")
    print(f"  Insérées : {comp_inserted}  Ignorées : {comp_skipped}  Erreurs : {comp_errors}")
    print("Résultat saisons :")
    print(f"  Insérées : {seas_inserted}  Ignorées : {seas_skipped}  Erreurs : {seas_errors}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""
Importe les espaces depuis space.json vers la base PostgreSQL kreek.

Comportement :
  - Génère un ULID pour chaque espace (id kreek)
  - legacy_id    ← legacy_id (stocké en base pour les imports suivants)
  - space_name   ← space_name (ignoré si déjà présent dans la DB)
  - space_icon_path ← logo
  - Les espaces avec status="inactive" sont ignorés

Usage :
    python scripts/import_spaces.py \
        --host localhost --port 5432 \
        --user kreek --password secret --database kreek_db \
        --input scripts/space.json \
        [--dry-run]
"""

import argparse
import json
import os
import sys

_SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))

try:
    import psycopg2
except ImportError:
    print("Dépendance manquante : pip install psycopg2-binary", file=sys.stderr)
    sys.exit(1)

try:
    from ulid import ULID
except ImportError as e:
    print(f"Erreur : {e}", file=sys.stderr)
    sys.exit(1)

INSERT_SQL = """
    INSERT INTO spaces (id, legacy_id, space_name, space_icon_path)
    VALUES (%s, %s, %s, %s)
    ON CONFLICT (legacy_id) DO NOTHING
"""

CHECK_NAME_SQL = "SELECT id FROM spaces WHERE space_name = %s"


def parse_args():
    p = argparse.ArgumentParser(description="Importe les espaces BBClub dans kreek")
    p.add_argument("--host",     default="localhost")
    p.add_argument("--port",     type=int, default=5432)
    p.add_argument("--user",     required=True)
    p.add_argument("--password", required=True)
    p.add_argument("--database", required=True)
    p.add_argument("--input",    default=os.path.join(_SCRIPTS_DIR, "space.json"))
    p.add_argument("--dry-run",  action="store_true",
                   help="Simule l'import sans écrire en base")
    return p.parse_args()


def main():
    args = parse_args()

    with open(args.input, encoding="utf-8") as f:
        spaces = json.load(f)

    print(f"{len(spaces)} espaces à importer depuis {args.input}")
    if args.dry_run:
        print("Mode DRY-RUN — aucune écriture en base.")

    if not args.dry_run:
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
    else:
        conn = cursor = None

    inserted  = 0
    skipped   = 0
    inactive  = 0

    for s in spaces:
        legacy_id  = s["legacy_id"]
        space_name = s["space_name"].strip()
        logo       = s["logo"] or ""
        status     = s["status"]

        if status != "active":
            print(f"  SKIP (inactif) : {space_name}")
            inactive += 1
            continue

        new_ulid = str(ULID())

        if args.dry_run:
            inserted += 1
            print(f"  [DRY] {space_name} legacy_id={legacy_id} → {new_ulid}")
            continue

        cursor.execute(CHECK_NAME_SQL, (space_name,))
        if cursor.fetchone():
            print(f"  SKIP (nom existant) : {space_name}")
            skipped += 1
            continue

        try:
            cursor.execute(INSERT_SQL, (new_ulid, legacy_id, space_name, logo))
            inserted += 1
        except psycopg2.Error as e:
            print(f"  ERREUR pour {space_name} : {e}", file=sys.stderr)
            conn.rollback()
            skipped += 1
            continue

    if not args.dry_run and conn:
        conn.commit()
        cursor.close()
        conn.close()

    print()
    print(f"Résultat :")
    print(f"  Insérés  : {inserted}")
    print(f"  Ignorés  : {skipped}")
    print(f"  Inactifs : {inactive}")
    print()
    print("Import suivant : JOIN sur spaces.legacy_id pour résoudre les IDs.")


if __name__ == "__main__":
    main()
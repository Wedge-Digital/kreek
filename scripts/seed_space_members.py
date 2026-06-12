#!/usr/bin/env python3
"""
Affecte aléatoirement ~100 coaches à chaque espace.

Usage :
    python scripts/seed_space_members.py [--count 100] [--dry-run]
    python scripts/seed_space_members.py --host localhost --port 5326 --user root --database kreek_db
"""

import argparse
import random
import sys

import psycopg2
import psycopg2.extras

PROFILE = "SpaceUser"


def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("--host",     default="localhost")
    p.add_argument("--port",     type=int, default=5326)
    p.add_argument("--user",     default="root")
    p.add_argument("--password", default="root")
    p.add_argument("--database", default="kreek_db")
    p.add_argument("--count",    type=int, default=100, help="Nombre de coaches par espace")
    p.add_argument("--dry-run",  action="store_true")
    return p.parse_args()


def main():
    args = parse_args()

    conn = psycopg2.connect(
        host=args.host,
        port=args.port,
        user=args.user,
        password=args.password,
        dbname=args.database,
    )
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)

    cur.execute("SELECT id FROM spaces ORDER BY id")
    spaces = [row["id"] for row in cur.fetchall()]

    cur.execute("SELECT id FROM spaces__user_cache ORDER BY id")
    coaches = [row["id"] for row in cur.fetchall()]

    if not spaces:
        print("Aucun espace trouvé.")
        sys.exit(1)
    if not coaches:
        print("Aucun coach trouvé dans spaces__user_cache.")
        sys.exit(1)

    print(f"{len(spaces)} espaces, {len(coaches)} coaches disponibles, cible {args.count} par espace")

    total_inserted = 0
    total_skipped = 0

    for space_id in spaces:
        sample = random.sample(coaches, min(args.count, len(coaches)))
        rows = [(space_id, coach_id, PROFILE) for coach_id in sample]

        if args.dry_run:
            print(f"  [dry-run] espace {space_id} : {len(rows)} membres")
            continue

        cur.executemany(
            """
            INSERT INTO spaces__user_space (space_id, coach_id, profile)
            VALUES (%s, %s, %s)
            ON CONFLICT DO NOTHING
            """,
            rows,
        )
        inserted = cur.rowcount if cur.rowcount >= 0 else len(rows)
        skipped = len(rows) - inserted
        total_inserted += inserted
        total_skipped += skipped
        print(f"  espace {space_id} : +{inserted} membres ({skipped} déjà présents)")

    if not args.dry_run:
        conn.commit()
        print(f"\nTotal : {total_inserted} insertions, {total_skipped} doublons ignorés")

    cur.close()
    conn.close()


if __name__ == "__main__":
    main()
#!/usr/bin/env python3
"""
Extrait les espaces (ligues/clubs) de la base MySQL BBClub vers un fichier JSON.

Tables sources :
  - league_manager_club

Usage :
    python scripts/extract_spaces.py \
        --host localhost --port 3306 \
        --user root --password secret --database bbclub_db \
        --output scripts/space.json
"""

import argparse
import json
import os
import sys

_SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))

try:
    import mysql.connector
except ImportError:
    print("Dépendance manquante : pip install mysql-connector-python", file=sys.stderr)
    sys.exit(1)

SQL = """
    SELECT
        id          AS legacy_id,
        name        AS space_name,
        icon_path   AS logo,
        status      AS status
    FROM league_manager_club
    ORDER BY id
"""


def parse_args():
    p = argparse.ArgumentParser(description="Extrait les espaces BBClub vers JSON")
    p.add_argument("--host",     default="localhost")
    p.add_argument("--port",     type=int, default=3306)
    p.add_argument("--user",     required=True)
    p.add_argument("--password", required=True)
    p.add_argument("--database", required=True)
    p.add_argument("--output",   default=os.path.join(_SCRIPTS_DIR, "extracted_spaces.json"))
    return p.parse_args()


def main():
    args = parse_args()

    print(f"Connexion à {args.host}:{args.port}/{args.database} …")
    try:
        conn = mysql.connector.connect(
            host=args.host, port=args.port,
            user=args.user, password=args.password,
            database=args.database, charset="utf8mb4",
        )
    except mysql.connector.Error as e:
        print(f"Erreur de connexion : {e}", file=sys.stderr)
        sys.exit(1)

    cursor = conn.cursor(dictionary=True)
    cursor.execute(SQL)
    rows = cursor.fetchall()
    cursor.close()
    conn.close()

    spaces = []
    for row in rows:
        spaces.append({
            "legacy_id":  row["legacy_id"],
            "space_name": row["space_name"] or "",
            "logo":       row["logo"] or "",
            "status":     "active" if row["status"] == 1 else "inactive",
        })

    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(spaces, f, ensure_ascii=False, indent=2)

    active = sum(1 for s in spaces if s["status"] == "active")
    print(f"Terminé — {len(spaces)} espaces exportés vers {args.output}")
    print(f"  Actifs   : {active} / {len(spaces)}")
    print(f"  Inactifs : {len(spaces) - active} / {len(spaces)}")


if __name__ == "__main__":
    main()
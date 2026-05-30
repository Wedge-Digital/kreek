#!/usr/bin/env python3
"""
Extrait les compétitions et saisons depuis la base MySQL BBClub vers un fichier JSON.

Table source : league_manager_league
  - id           → legacy_id de la saison
  - name         → nom de la compétition (regroupement)
  - season       → nom de la saison
  - Club_id      → club_legacy_id (résolu via spaces.legacy_id à l'import)
  - status       → 0=draft / 1=active / 2=closed
  - ref_ruleset  → ex. "BB2020" (stocké dans rules JSONB à l'import)

Logique de regroupement :
  Plusieurs lignes peuvent partager le même (Club_id, name) pour des saisons différentes.
  → une compétition par (Club_id, name), son legacy_id = min(id) du groupe
  → une saison par ligne, avec legacy_id = id de la ligne

Usage :
    python scripts/extract_competitions.py \\
        --host localhost --port 3306 \\
        --user root --password secret --database bbclub_db \\
        --output scripts/extracted_competitions.json
"""

import argparse
import json
import os
import sys
from collections import defaultdict

_SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))

try:
    import mysql.connector
except ImportError:
    print("Dépendance manquante : pip install mysql-connector-python", file=sys.stderr)
    sys.exit(1)

SQL = """
    SELECT
        id,
        name,
        season        AS season_name,
        Club_id       AS club_legacy_id,
        status,
        ref_ruleset
    FROM league_manager_league
    ORDER BY Club_id, name, id
"""

STATUS_MAP = {
    0: "draft",
    1: "ready",
    2: "ready",  # saisons terminées : 'ready' est le statut le plus proche
}


def parse_args():
    p = argparse.ArgumentParser(description="Extrait les compétitions BBClub vers JSON")
    p.add_argument("--host",     default="localhost")
    p.add_argument("--port",     type=int, default=3306)
    p.add_argument("--user",     required=True)
    p.add_argument("--password", required=True)
    p.add_argument("--database", required=True)
    p.add_argument("--output",   default=os.path.join(_SCRIPTS_DIR, "extracted_competitions.json"))
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

    print(f"  {len(rows)} lignes récupérées")

    # Regroupement par (club_legacy_id, name) → une compétition
    groups = defaultdict(list)
    for row in rows:
        key = (row["club_legacy_id"], row["name"])
        groups[key].append(row)

    competitions = []
    for (club_legacy_id, name), seasons in groups.items():
        # legacy_id de la compétition = min id du groupe (stable et déterministe)
        comp_legacy_id = min(s["id"] for s in seasons)

        seasons_out = []
        for s in seasons:
            seasons_out.append({
                "legacy_id":   s["id"],
                "season_name": (s["season_name"] or "").strip(),
                "status":      STATUS_MAP.get(s["status"], "draft"),
                "ref_ruleset": (s["ref_ruleset"] or "").strip(),
            })

        competitions.append({
            "legacy_id":      comp_legacy_id,
            "name":           name.strip(),
            "club_legacy_id": club_legacy_id,
            "seasons":        seasons_out,
        })

    # Tri stable : par club puis par nom
    competitions.sort(key=lambda c: (c["club_legacy_id"], c["name"]))

    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(competitions, f, ensure_ascii=False, indent=2)

    total_seasons = sum(len(c["seasons"]) for c in competitions)
    print(f"Terminé — {len(competitions)} compétitions, {total_seasons} saisons")
    print(f"  Fichier : {args.output}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""
Importe les utilisateurs depuis users_export.json vers la base PostgreSQL kreek.

Comportement :
  - Génère un ULID pour chaque utilisateur (id kreek)
  - legacy_id   ← legacy_id (stocké en base pour les imports suivants)
  - coach_name  ← username  (dédupliqué avec suffixe _N si collision)
  - email       ← email     (ignoré si déjà présent dans la DB)
  - coach_icon  ← NULL      (les URLs BBC ne sont pas Cloudinary — à migrer séparément)
  - password_hash ← hash Django PBKDF2 tel quel
                    → login bloqué jusqu'à reset mot de passe

Usage :
    python scripts/import_users.py \
        --host localhost --port 5432 \
        --user kreek --password secret --database kreek_db \
        --input  scripts/users_export.json \
        [--dry-run]
"""

import argparse
import json
import os
import sys
import time
from datetime import timezone

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

INSERT_SQL = """
    INSERT INTO auth__users (id, legacy_id, coach_name, coach_icon, email, password_hash, created_at)
    VALUES (%s, %s, %s, NULL, %s, %s, %s)
    ON CONFLICT (legacy_id) DO NOTHING
"""

CHECK_EMAIL_SQL      = "SELECT id FROM auth__users WHERE email = %s"
CHECK_COACH_NAME_SQL = "SELECT id FROM auth__users WHERE coach_name = %s"


def parse_args():
    p = argparse.ArgumentParser(description="Importe les users BBClub dans kreek")
    p.add_argument("--host",     default="localhost")
    p.add_argument("--port",     type=int, default=5432)
    p.add_argument("--user",     required=True)
    p.add_argument("--password", required=True)
    p.add_argument("--database", required=True)
    p.add_argument("--input",    default=os.path.join(_SCRIPTS_DIR, "users_export.json"))
    p.add_argument("--dry-run",  action="store_true",
                   help="Simule l'import sans écrire en base")
    return p.parse_args()


def unique_coach_name(cursor, base_name: str) -> str:
    """Ajoute un suffixe _2, _3… si le coach_name est déjà pris."""
    candidate = base_name
    n = 2
    while True:
        cursor.execute(CHECK_COACH_NAME_SQL, (candidate,))
        if cursor.fetchone() is None:
            return candidate
        candidate = f"{base_name}_{n}"
        n += 1


def parse_created_at(date_str):
    """Convertit la date Django (string) en objet datetime UTC."""
    if not date_str:
        return None
    from datetime import datetime
    for fmt in ("%Y-%m-%d %H:%M:%S.%f", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S.%fZ"):
        try:
            dt = datetime.strptime(date_str, fmt)
            return dt.replace(tzinfo=timezone.utc)
        except ValueError:
            continue
    return None


def main():
    args = parse_args()

    with open(args.input, encoding="utf-8") as f:
        users = json.load(f)

    print(f"{len(users)} utilisateurs à importer depuis {args.input}")
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

    inserted     = 0
    skipped      = 0
    name_renamed = 0

    for u in users:
        legacy_id  = u["legacy_id"]
        email      = u["email"].strip().lower()
        base_name  = u["username"].strip()
        password   = u["django_password_hash"]
        created_at = parse_created_at(u.get("date_joined"))

        new_ulid = str(ULID())

        if args.dry_run:
            inserted += 1
            print(f"  [DRY] {base_name} <{email}> legacy_id={legacy_id} → {new_ulid}")
            continue

        # Vérifier doublon email
        cursor.execute(CHECK_EMAIL_SQL, (email,))
        if cursor.fetchone():
            print(f"  SKIP (email existant) : {email}")
            skipped += 1
            continue

        # Dédupliquer le coach_name si nécessaire
        coach_name = unique_coach_name(cursor, base_name)
        if coach_name != base_name:
            print(f"  RENAME coach_name : '{base_name}' → '{coach_name}'")
            name_renamed += 1

        try:
            cursor.execute(INSERT_SQL, (new_ulid, legacy_id, coach_name, email, password, created_at))
            inserted += 1
        except psycopg2.Error as e:
            print(f"  ERREUR pour {base_name} <{email}> : {e}", file=sys.stderr)
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
    print(f"  Renommés : {name_renamed}")
    print()
    print("⚠️  Mots de passe : hash Django PBKDF2 stocké tel quel.")
    print("   Les utilisateurs devront réinitialiser leur mot de passe.")
    print("   Import suivant : JOIN sur auth__users.legacy_id pour résoudre les IDs.")


if __name__ == "__main__":
    main()

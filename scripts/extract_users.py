#!/usr/bin/env python3
"""
Extrait les utilisateurs actifs de la base MySQL BBClub vers un fichier JSON.

Tables sources :
  - auth_user            (données principales)
  - account_emailaddress (statut de vérification email)
  - bbc_user_useravatar  (avatar)

Usage :
    python scripts/extract_users.py \
        --host localhost --port 3306 \
        --user root --password secret --database bbclub_db \
        --output scripts/extracted_users.json

En pratique, passer par le wrapper `extract_users.sh`, qui lit `.env.legacy`.
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
        u.id                                        AS legacy_id,
        u.username,
        u.email,
        u.password                                  AS django_password_hash,
        u.first_name,
        u.last_name,
        u.date_joined,
        u.last_login,
        COALESCE(ea.verified, 0)                    AS email_verified,
        av.avatar                                   AS avatar_url
    FROM auth_user u
    LEFT JOIN account_emailaddress ea
           ON ea.user_id = u.id AND ea.`primary` = 1
    LEFT JOIN bbc_user_useravatar av
           ON av.user_id = u.id
    WHERE u.is_active = 1
    ORDER BY u.id
"""


def parse_args():
    p = argparse.ArgumentParser(description="Extrait les users BBClub vers JSON")
    p.add_argument("--host",     default="localhost")
    p.add_argument("--port",     type=int, default=3306)
    p.add_argument("--user",     required=True)
    p.add_argument("--password", required=True)
    p.add_argument("--database", required=True)
    p.add_argument("--output",   default=os.path.join(_SCRIPTS_DIR, "extracted_users.json"))
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

    users = []
    for row in rows:
        users.append({
            "legacy_id":            row["legacy_id"],
            "username":             row["username"],
            "email":                row["email"],
            "django_password_hash": row["django_password_hash"],
            "first_name":           row["first_name"] or "",
            "last_name":            row["last_name"] or "",
            "date_joined":          str(row["date_joined"]) if row["date_joined"] else None,
            "last_login":           str(row["last_login"])  if row["last_login"]  else None,
            "email_verified":       bool(row["email_verified"]),
            "avatar_url":           row["avatar_url"],
        })

    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(users, f, ensure_ascii=False, indent=2)

    verified   = sum(1 for u in users if u["email_verified"])
    with_avatar = sum(1 for u in users if u["avatar_url"])
    print(f"Terminé — {len(users)} utilisateurs actifs exportés vers {args.output}")
    print(f"  Email vérifié : {verified} / {len(users)}")
    print(f"  Avec avatar   : {with_avatar} / {len(users)}")


if __name__ == "__main__":
    main()

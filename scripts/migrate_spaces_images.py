#!/usr/bin/env python3
"""
Migre les images des espaces vers Cloudinary et met à jour la DB kreek.

Pour chaque espace actif avec un logo :
  1. Télécharge l'image depuis back.bloodbowlclub.com
  2. Uploade vers Cloudinary dans le dossier kreek/spaces
  3. Met à jour spaces.space_icon_path avec l'URL Cloudinary

Usage :
    python scripts/migrate_spaces_images.py \
        --cloud-name moncloud --api-key 123 --api-secret abc \
        --host localhost --port 5432 \
        --user kreek --password secret --database kreek_db \
        --input scripts/extracted_spaces.json \
        [--dry-run]
"""

import argparse
import json
import os
import sys
import time

_SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))
BBC_BASE_URL  = "https://back.bloodbowlclub.com"

try:
    import requests
except ImportError:
    print("Dépendance manquante : pip install requests", file=sys.stderr)
    sys.exit(1)

try:
    import cloudinary
    import cloudinary.api
    import cloudinary.uploader
    import cloudinary.exceptions
except ImportError:
    print("Dépendance manquante : pip install cloudinary", file=sys.stderr)
    sys.exit(1)

try:
    import psycopg2
except ImportError:
    print("Dépendance manquante : pip install psycopg2-binary", file=sys.stderr)
    sys.exit(1)

UPDATE_SQL = "UPDATE spaces SET space_icon_path = %s WHERE legacy_id = %s"


def parse_args():
    p = argparse.ArgumentParser(description="Migre les images des espaces vers Cloudinary")
    p.add_argument("--cloud-name", required=True)
    p.add_argument("--api-key",    required=True)
    p.add_argument("--api-secret", required=True)
    p.add_argument("--host",       default="localhost")
    p.add_argument("--port",       type=int, default=5432)
    p.add_argument("--user",       required=True)
    p.add_argument("--password",   required=True)
    p.add_argument("--database",   required=True)
    p.add_argument("--input",      default=os.path.join(_SCRIPTS_DIR, "extracted_spaces.json"))
    p.add_argument("--dry-run",    action="store_true",
                   help="Simule sans uploader ni écrire en base")
    return p.parse_args()


def logo_url(logo_path: str) -> str:
    if logo_path.startswith("http"):
        return logo_path
    return f"{BBC_BASE_URL}{logo_path}"


def cloudinary_public_id(logo_path: str) -> str:
    filename = os.path.basename(logo_path)
    name, _ = os.path.splitext(filename)
    return f"kreek/spaces/{name}"


def main():
    args = parse_args()

    cloudinary.config(
        cloud_name=args.cloud_name,
        api_key=args.api_key,
        api_secret=args.api_secret,
        secure=True,
    )

    with open(args.input, encoding="utf-8") as f:
        spaces = json.load(f)

    active_with_logo = [
        s for s in spaces
        if s["status"] == "active" and s.get("logo")
    ]
    print(f"{len(active_with_logo)} espaces actifs avec logo à migrer")
    if args.dry_run:
        print("Mode DRY-RUN — aucune écriture.")

    if not args.dry_run:
        try:
            conn = psycopg2.connect(
                host=args.host, port=args.port,
                user=args.user, password=args.password,
                dbname=args.database,
            )
            conn.autocommit = False
            cursor = conn.cursor()
        except psycopg2.Error as e:
            print(f"Erreur de connexion DB : {e}", file=sys.stderr)
            sys.exit(1)
    else:
        conn = cursor = None

    done    = 0
    skipped = 0
    errors  = 0

    for s in active_with_logo:
        legacy_id  = s["legacy_id"]
        space_name = s["space_name"]
        src_url    = logo_url(s["logo"])
        public_id  = cloudinary_public_id(s["logo"])

        if args.dry_run:
            print(f"  [DRY] {space_name} : {src_url} → {public_id}")
            done += 1
            continue

        # Vérifier l'existence avant d'uploader
        try:
            resource = cloudinary.api.resource(public_id)
            cloudinary_url = resource["secure_url"]
            print(f"  SKIP (déjà sur Cloudinary) {space_name} → {cloudinary_url}")
            skipped += 1
        except cloudinary.exceptions.NotFound:
            try:
                result = cloudinary.uploader.upload(
                    src_url,
                    public_id=public_id,
                    overwrite=False,
                    resource_type="image",
                )
                cloudinary_url = result["secure_url"]
            except Exception as e:
                print(f"  ERREUR upload {space_name} : {e}", file=sys.stderr)
                errors += 1
                continue

        try:
            cursor.execute(UPDATE_SQL, (cloudinary_url, legacy_id))
            print(f"  OK {space_name} → {cloudinary_url}")
            done += 1
        except psycopg2.Error as e:
            print(f"  ERREUR DB {space_name} : {e}", file=sys.stderr)
            conn.rollback()
            errors += 1
            continue

        time.sleep(0.2)

    if not args.dry_run and conn:
        conn.commit()
        cursor.close()
        conn.close()

    print()
    print(f"Résultat :")
    print(f"  Migrés  : {done}")
    print(f"  Ignorés : {skipped}")
    print(f"  Erreurs : {errors}")


if __name__ == "__main__":
    main()
#!/usr/bin/env python3
"""
Migre les images des articles vers Cloudinary et met à jour le JSON extrait.

Pour chaque article avec une image non-Cloudinary :
  1. Uploade vers Cloudinary dans le dossier kreek/articles
  2. Remplace le champ "image" dans le JSON par l'URL Cloudinary

Le fichier JSON est mis à jour en place — l'import_articles.py utilisera
directement les URLs Cloudinary.

Usage :
    python scripts/migrate_articles_images.py \
        --cloud-name moncloud --api-key 123 --api-secret abc \
        --input scripts/extracted_articles.json \
        [--dry-run]
"""

import argparse
import json
import os
import sys
import time

_SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))

try:
    import cloudinary
    import cloudinary.api
    import cloudinary.uploader
    import cloudinary.exceptions
except ImportError:
    print("Dépendance manquante : pip install cloudinary", file=sys.stderr)
    sys.exit(1)


def parse_args():
    p = argparse.ArgumentParser(description="Migre les images des articles vers Cloudinary")
    p.add_argument("--cloud-name", required=True)
    p.add_argument("--api-key",    required=True)
    p.add_argument("--api-secret", required=True)
    p.add_argument("--input",      default=os.path.join(_SCRIPTS_DIR, "extracted_articles.json"))
    p.add_argument("--dry-run",    action="store_true",
                   help="Simule sans uploader ni modifier le JSON")
    return p.parse_args()


def is_cloudinary(url: str) -> bool:
    return "res.cloudinary.com" in url


def cloudinary_public_id(article_id: int) -> str:
    return f"kreek/articles/{article_id}"


def main():
    args = parse_args()

    cloudinary.config(
        cloud_name=args.cloud_name,
        api_key=args.api_key,
        api_secret=args.api_secret,
        secure=True,
    )

    with open(args.input, encoding="utf-8") as f:
        articles = json.load(f)

    to_migrate = [
        a for a in articles
        if a.get("image") and not is_cloudinary(a["image"])
    ]
    already_done = sum(1 for a in articles if a.get("image") and is_cloudinary(a["image"]))

    print(f"{len(articles)} articles au total")
    print(f"  À migrer       : {len(to_migrate)}")
    print(f"  Déjà Cloudinary : {already_done}")
    if args.dry_run:
        print("Mode DRY-RUN — aucune modification.")

    done    = 0
    skipped = 0
    errors  = 0

    for a in to_migrate:
        article_id = a["id"]
        titre      = a["titre"][:50]
        src_url    = a["image"]
        public_id  = cloudinary_public_id(article_id)

        if args.dry_run:
            print(f"  [DRY] #{article_id} {titre} → {public_id}")
            done += 1
            continue

        # Vérifier l'existence avant d'uploader
        try:
            resource = cloudinary.api.resource(public_id)
            a["image"] = resource["secure_url"]
            print(f"  SKIP (déjà sur Cloudinary) #{article_id} → {a['image']}")
            skipped += 1
            continue
        except cloudinary.exceptions.NotFound:
            pass

        try:
            result = cloudinary.uploader.upload(
                src_url,
                public_id=public_id,
                overwrite=False,
                resource_type="image",
            )
            a["image"] = result["secure_url"]
            print(f"  OK #{article_id} → {a['image']}")
            done += 1
        except Exception as e:
            print(f"  ERREUR #{article_id} {titre} : {e}", file=sys.stderr)
            errors += 1
            continue

        with open(args.input, "w", encoding="utf-8") as f:
            json.dump(articles, f, ensure_ascii=False, indent=2)

        time.sleep(0.1)

    print()
    print(f"Résultat :")
    print(f"  Migrés  : {done}")
    print(f"  Ignorés : {skipped}")
    print(f"  Erreurs : {errors}")


if __name__ == "__main__":
    main()
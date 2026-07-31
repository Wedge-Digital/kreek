#!/usr/bin/env python3
"""
Migre les images des articles vers Cloudinary et met à jour le JSON extrait.

Pour chaque article dont l'image n'est pas déjà sur Cloudinary :
  1. Uploade vers Cloudinary dans le dossier kreek/articles
  2. Remplace le champ "image" dans le JSON par l'URL Cloudinary

Le fichier JSON est mis à jour **au fil de l'eau** (toutes les FLUSH_EVERY
images) : une interruption — quota Cloudinary atteint, Ctrl-C — ne fait pas
perdre le travail déjà fait, et la relance ne retraite que le reste, puisque
les images résolues portent désormais une URL Cloudinary.

L'existence en ligne est déterminée par un index listé en une passe, et non par
un appel par article : l'API Admin est limitée à 500 opérations/heure.

À lancer AVANT import_articles.py.

Usage :
    python scripts/migrate_articles_images.py \
        --cloud-name moncloud --api-key 123 --api-secret abc \
        --input scripts/extracted_articles.json \
        [--dry-run] [--max-wait 900]
"""

import argparse
import os
import sys
import time

import cloudinary_migration as cm

_SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))
CLOUDINARY_FOLDER = "kreek/articles"


def parse_args():
    p = argparse.ArgumentParser(description="Migre les images des articles vers Cloudinary")
    cm.add_common_args(p)
    p.add_argument("--input", default=os.path.join(_SCRIPTS_DIR, "extracted_articles.json"))
    return p.parse_args()


def cloudinary_public_id(article_id: int) -> str:
    return f"{CLOUDINARY_FOLDER}/{article_id}"


def to_migrate(articles: list) -> list:
    return [
        a for a in articles
        if a.get("image") and not cm.is_cloudinary(a["image"])
    ]


def print_plan(articles: list, pending: list):
    already = sum(1 for a in articles if a.get("image") and cm.is_cloudinary(a["image"]))
    print(f"{len(articles)} articles au total")
    print(f"  À migrer        : {len(pending)}")
    print(f"  Déjà Cloudinary : {already}")


def print_dry_run(pending: list):
    print("Mode DRY-RUN — aucune modification.")
    for a in pending:
        print(f"  [DRY] #{a['id']} {a['titre'][:50]} → {cloudinary_public_id(a['id'])}")


def migrate_one(article: dict, index: dict, stats: dict, max_wait: int):
    """Résout l'URL Cloudinary de l'image et l'écrit dans l'article (en mémoire)."""
    public_id = cloudinary_public_id(article["id"])
    url = index.get(public_id)

    if url:
        stats["skipped"] += 1
    else:
        url = cm.upload(article["image"], public_id, max_wait)
        print(f"  OK #{article['id']} → {url}")
        stats["uploaded"] += 1
        time.sleep(0.1)

    article["image"] = url


def migrate_all(articles: list, pending: list, index: dict, args) -> dict:
    stats = {"uploaded": 0, "skipped": 0, "errors": 0}
    for done, a in enumerate(pending, start=1):
        try:
            migrate_one(a, index, stats, args.max_wait)
        except cm.RateLimitAbort as e:
            cm.save_json(args.input, articles)
            cm.fail_rate_limited(e, args.input)
        except Exception as e:
            print(f"  ERREUR #{a['id']} {a['titre'][:50]} : {e}", file=sys.stderr)
            stats["errors"] += 1

        if done % cm.FLUSH_EVERY == 0:
            cm.save_json(args.input, articles)
    return stats


def main():
    args = parse_args()
    cm.configure(args)

    articles = cm.load_json(args.input)
    pending = to_migrate(articles)
    print_plan(articles, pending)

    if not pending:
        print("Rien à migrer — toutes les images pointent déjà sur Cloudinary.")
        return

    if args.dry_run:
        print_dry_run(pending)
        return

    index = cm.fetch_index(f"{CLOUDINARY_FOLDER}/", args.max_wait)
    print(f"  Déjà en ligne sur Cloudinary : {len(index)}")

    stats = migrate_all(articles, pending, index, args)
    cm.save_json(args.input, articles)

    print()
    print("Résultat :")
    print(f"  Uploadés : {stats['uploaded']}")
    print(f"  Ignorés  : {stats['skipped']}")
    print(f"  Erreurs  : {stats['errors']}")
    print(f"  JSON mis à jour : {args.input}")


if __name__ == "__main__":
    try:
        main()
    except cm.RateLimitAbort as e:
        cm.fail_rate_limited(e)

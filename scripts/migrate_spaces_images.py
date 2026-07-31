#!/usr/bin/env python3
"""
Migre les logos des espaces vers Cloudinary et met à jour le JSON extrait.

Pour chaque espace actif dont le logo n'est pas déjà sur Cloudinary :
  1. Uploade vers Cloudinary dans le dossier kreek/spaces
  2. Remplace le champ "logo" dans le JSON par l'URL Cloudinary

Le fichier JSON est mis à jour **au fil de l'eau** (toutes les FLUSH_EVERY
images) : une interruption ne fait pas perdre le travail déjà fait, et la
relance ne retraite que le reste.

import_spaces.py écrit ce champ tel quel dans `spaces.space_icon_path`, que le
value object CloudinaryImage refuse tant qu'il ne pointe pas sur
res.cloudinary.com — auquel cas /app/space/all s'affiche vide.

À lancer AVANT import_spaces.py.

Usage :
    python scripts/migrate_spaces_images.py \
        --cloud-name moncloud --api-key 123 --api-secret abc \
        --input scripts/extracted_spaces.json \
        [--dry-run] [--max-wait 900]
"""

import argparse
import os
import sys
import time

import cloudinary_migration as cm

_SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))
BBC_BASE_URL = "https://back.bloodbowlclub.com"
CLOUDINARY_FOLDER = "kreek/spaces"


def parse_args():
    p = argparse.ArgumentParser(description="Migre les logos des espaces vers Cloudinary")
    cm.add_common_args(p)
    p.add_argument("--input", default=os.path.join(_SCRIPTS_DIR, "extracted_spaces.json"))
    return p.parse_args()


def logo_url(logo_path: str) -> str:
    if logo_path.startswith("http"):
        return logo_path
    return f"{BBC_BASE_URL}{logo_path}"


def cloudinary_public_id(logo_path: str) -> str:
    name, _ = os.path.splitext(os.path.basename(logo_path))
    return f"{CLOUDINARY_FOLDER}/{name}"


def to_migrate(spaces: list) -> list:
    return [
        s for s in spaces
        if s["status"] == "active" and s.get("logo") and not cm.is_cloudinary(s["logo"])
    ]


def print_plan(spaces: list, pending: list):
    already = sum(1 for s in spaces if s.get("logo") and cm.is_cloudinary(s["logo"]))
    print(f"{len(spaces)} espaces au total")
    print(f"  À migrer        : {len(pending)}")
    print(f"  Déjà Cloudinary : {already}")


def print_dry_run(pending: list):
    print("Mode DRY-RUN — aucune modification.")
    for s in pending:
        print(f"  [DRY] {s['space_name']} → {cloudinary_public_id(s['logo'])}")


def migrate_one(space: dict, index: dict, stats: dict, max_wait: int):
    """Résout l'URL Cloudinary du logo et l'écrit dans l'espace (en mémoire)."""
    public_id = cloudinary_public_id(space["logo"])
    url = index.get(public_id)

    if url:
        stats["skipped"] += 1
    else:
        url = cm.upload(logo_url(space["logo"]), public_id, max_wait)
        print(f"  OK {space['space_name']} → {url}")
        stats["uploaded"] += 1
        time.sleep(0.2)

    space["logo"] = url


def migrate_all(spaces: list, pending: list, index: dict, args) -> dict:
    stats = {"uploaded": 0, "skipped": 0, "errors": 0}
    for done, s in enumerate(pending, start=1):
        try:
            migrate_one(s, index, stats, args.max_wait)
        except cm.RateLimitAbort as e:
            cm.save_json(args.input, spaces)
            cm.fail_rate_limited(e, args.input)
        except Exception as e:
            print(f"  ERREUR {s['space_name']} : {e}", file=sys.stderr)
            stats["errors"] += 1

        if done % cm.FLUSH_EVERY == 0:
            cm.save_json(args.input, spaces)
    return stats


def main():
    args = parse_args()
    cm.configure(args)

    spaces = cm.load_json(args.input)
    pending = to_migrate(spaces)
    print_plan(spaces, pending)

    if not pending:
        print("Rien à migrer — tous les logos pointent déjà sur Cloudinary.")
        return

    if args.dry_run:
        print_dry_run(pending)
        return

    index = cm.fetch_index(f"{CLOUDINARY_FOLDER}/", args.max_wait)
    print(f"  Déjà en ligne sur Cloudinary : {len(index)}")

    stats = migrate_all(spaces, pending, index, args)
    cm.save_json(args.input, spaces)

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

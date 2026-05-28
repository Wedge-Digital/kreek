#!/usr/bin/env python3
"""
Met à jour extracted_articles.json :
  - Ajoute le champ "abstract" (< 200 caractères) depuis "contenu"
  - Supprime le champ "contenu_html"

Usage :
    python scripts/patch_articles_abstract.py \
        --input scripts/extracted_articles.json
"""

import argparse
import json
import os

_SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))


def make_abstract(text: str, max_len: int = 200) -> str:
    text = text.strip().replace("\n", " ")
    if len(text) <= max_len:
        return text
    truncated = text[:max_len]
    cut = truncated.rfind(" ")
    return (truncated[:cut] if cut > 0 else truncated).rstrip(".,;:!?") + "…"


def parse_args():
    p = argparse.ArgumentParser(description="Ajoute abstract et supprime contenu_html")
    p.add_argument("--input", default=os.path.join(_SCRIPTS_DIR, "extracted_articles.json"))
    return p.parse_args()


def main():
    args = parse_args()

    with open(args.input, encoding="utf-8") as f:
        articles = json.load(f)

    already_have_abstract = sum(1 for a in articles if "abstract" in a)
    have_html             = sum(1 for a in articles if "contenu_html" in a)
    print(f"{len(articles)} articles chargés depuis {args.input}")
    print(f"  Avec abstract déjà   : {already_have_abstract}")
    print(f"  Avec contenu_html    : {have_html}")

    for a in articles:
        if "abstract" not in a:
            a["abstract"] = make_abstract(a.get("contenu", ""))
        a.pop("contenu_html", None)

    with open(args.input, "w", encoding="utf-8") as f:
        json.dump(articles, f, ensure_ascii=False, indent=2)

    print(f"Fichier mis à jour : {args.input}")


if __name__ == "__main__":
    main()
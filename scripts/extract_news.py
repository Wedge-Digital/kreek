#!/usr/bin/env python3
"""
Extrait toutes les news de l'API bloodbowlclub.com et les sauvegarde en JSON.
"""
import json
import re
import time
import math
import requests
from html.parser import HTMLParser

BASE_URL = "https://back.bloodbowlclub.com/content/"
PAGE_SIZE = 100  # actual page size from API
OUTPUT_FILE = "scripts/news_export.json"


class HTMLTextExtractor(HTMLParser):
    def __init__(self):
        super().__init__()
        self.result = []

    def handle_data(self, d):
        text = d.strip()
        if text:
            self.result.append(text)

    def get_text(self):
        return "\n".join(self.result)


def strip_html(html_content: str) -> str:
    parser = HTMLTextExtractor()
    parser.feed(html_content)
    return parser.get_text()


def fetch_all_ids() -> list[dict]:
    """Fetches all pages of the news list and returns all article summaries."""
    print("Fetching total count...")
    resp = requests.get(f"{BASE_URL}?type=0&page=1", timeout=10)
    resp.raise_for_status()
    data = resp.json()
    total = data["count"]
    pages = math.ceil(total / PAGE_SIZE)
    print(f"Total articles: {total} ({pages} pages)")

    articles = list(data["results"])

    for page in range(2, pages + 1):
        print(f"  Fetching list page {page}/{pages}...", end="\r")
        resp = requests.get(f"{BASE_URL}?type=0&page={page}", timeout=10)
        if resp.status_code == 404:
            print(f"\n  Page {page} returned 404, stopping.")
            break
        resp.raise_for_status()
        page_data = resp.json()
        articles.extend(page_data["results"])
        if not page_data.get("next"):
            break
        time.sleep(0.1)

    print(f"\nFetched {len(articles)} article summaries.")
    return articles


def fetch_article_detail(article_id: int) -> dict:
    resp = requests.get(f"{BASE_URL}{article_id}/", timeout=10)
    resp.raise_for_status()
    return resp.json()


def clean_title(title: str) -> str:
    # Remove HTML entities like &nbsp;
    title = re.sub(r"&[a-z]+;", " ", title)
    return title.strip()


def main():
    summaries = fetch_all_ids()

    results = []
    total = len(summaries)

    for i, summary in enumerate(summaries):
        article_id = summary["id"]
        print(f"  [{i+1}/{total}] Fetching detail for article #{article_id}...", end="\r")

        try:
            detail = fetch_article_detail(article_id)
            content_html = detail.get("content", "")
            content_text = strip_html(content_html) if content_html else ""

            results.append({
                "id": article_id,
                "titre": clean_title(detail.get("title", "")),
                "image": detail.get("featured_image", ""),
                "contenu": content_text,
                "contenu_html": content_html,
                "date": detail.get("created", ""),
                "auteur": detail.get("user", {}).get("username", ""),
                "ligue": detail.get("featured_club", {}).get("name", "") if detail.get("featured_club") else "",
            })
        except Exception as e:
            print(f"\n  ERROR on article #{article_id}: {e}")
            results.append({
                "id": article_id,
                "titre": clean_title(summary.get("title", "")),
                "image": summary.get("featured_image", ""),
                "contenu": "",
                "contenu_html": "",
                "date": summary.get("created", ""),
                "auteur": summary.get("user", {}).get("username", ""),
                "ligue": summary.get("featured_club", {}).get("name", "") if summary.get("featured_club") else "",
                "error": str(e),
            })

        time.sleep(0.15)

    print(f"\nDone! Saving {len(results)} articles to {OUTPUT_FILE}...")
    with open(OUTPUT_FILE, "w", encoding="utf-8") as f:
        json.dump(results, f, ensure_ascii=False, indent=2)

    print(f"Saved to {OUTPUT_FILE}")


if __name__ == "__main__":
    main()
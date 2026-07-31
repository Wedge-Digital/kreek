#!/usr/bin/env python3
"""
Extrait les articles et commentaires de la base MySQL BBClub vers un fichier JSON.

Tables sources :
  - bbc_content_content  (articles)
  - auth_user            (auteurs)
  - league_manager_club  (ligue associée)
  - django_content_type  (pour retrouver le content_type_id des articles)
  - django_comments      (commentaires)

Usage :
    python scripts/extract_news.py \
        --host localhost --port 3306 \
        --user root --password secret --database bbclub_db \
        --output scripts/news_export.json
"""

import argparse
import json
import os
import sys
from collections import defaultdict
from html.parser import HTMLParser

_SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))
BBC_MEDIA_BASE = "https://back.bloodbowlclub.com/static/"

try:
    import mysql.connector
except ImportError:
    print("Dépendance manquante : pip install mysql-connector-python", file=sys.stderr)
    sys.exit(1)


CONTENT_TYPE_SQL = """
    SELECT id FROM django_content_type
    WHERE app_label = 'bbc_content' AND model = 'content'
"""

ARTICLES_SQL = """
    SELECT
        c.id,
        c.title,
        c.description,
        c.content          AS content_html,
        c.featured_image,
        c.created,
        c.updated,
        c.status,
        u.username         AS auteur,
        u.id               AS user_legacy_id,
        cl.name            AS ligue,
        cl.id              AS club_legacy_id
    FROM bbc_content_content c
    JOIN  auth_user           u  ON u.id  = c.user_id
    LEFT JOIN league_manager_club cl ON cl.id = c.featured_club_id
    WHERE c.type = 0
    ORDER BY c.id DESC
"""

COMMENTS_SQL = """
    SELECT
        dc.id,
        dc.object_pk,
        COALESCE(NULLIF(u.username, ''), NULLIF(dc.user_name, ''), 'Anonyme') AS auteur,
        dc.comment,
        dc.submit_date
    FROM django_comments dc
    LEFT JOIN auth_user u ON u.id = dc.user_id
    WHERE dc.content_type_id = %s
      AND dc.is_public  = 1
      AND dc.is_removed = 0
    ORDER BY dc.object_pk, dc.submit_date
"""


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


def strip_html(html: str) -> str:
    p = HTMLTextExtractor()
    p.feed(html)
    return p.get_text()


def make_abstract(text: str, max_len: int = 200) -> str:
    text = text.strip().replace("\n", " ")
    if len(text) <= max_len:
        return text
    truncated = text[:max_len]
    cut = truncated.rfind(" ")
    return (truncated[:cut] if cut > 0 else truncated).rstrip(".,;:!?") + "…"


def full_image_url(path: str) -> str:
    if not path:
        return ""
    if path.startswith("http"):
        return path
    return BBC_MEDIA_BASE + path.lstrip("/")


def fmt_datetime(dt) -> str | None:
    if dt is None:
        return None
    return dt.strftime("%Y-%m-%dT%H:%M:%S.%f") + "Z"


def parse_args():
    p = argparse.ArgumentParser(description="Extrait les articles BBClub vers JSON")
    p.add_argument("--host",     default="localhost")
    p.add_argument("--port",     type=int, default=3306)
    p.add_argument("--user",     required=True)
    p.add_argument("--password", required=True)
    p.add_argument("--database", required=True)
    p.add_argument("--output",   default=os.path.join(_SCRIPTS_DIR, "extracted_articles.json"))
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

    # Récupérer le content_type_id pour bbc_content | content
    cursor.execute(CONTENT_TYPE_SQL)
    row = cursor.fetchone()
    if not row:
        print("Erreur : content_type 'bbc_content | content' introuvable.", file=sys.stderr)
        sys.exit(1)
    content_type_id = row["id"]
    print(f"  content_type_id pour les articles : {content_type_id}")

    # Charger tous les commentaires en une seule requête
    print("Chargement des commentaires …")
    cursor.execute(COMMENTS_SQL, (content_type_id,))
    comments_by_article: dict[str, list] = defaultdict(list)
    for c in cursor.fetchall():
        comments_by_article[c["object_pk"]].append({
            "id":      c["id"],
            "auteur":  c["auteur"],
            "date":    fmt_datetime(c["submit_date"]),
            "comment": strip_html(c["comment"]),
        })
    total_comments = sum(len(v) for v in comments_by_article.values())
    print(f"  {total_comments} commentaires chargés pour {len(comments_by_article)} articles")

    # Charger les articles
    print("Chargement des articles …")
    cursor.execute(ARTICLES_SQL)
    rows = cursor.fetchall()
    cursor.close()
    conn.close()
    print(f"  {len(rows)} articles récupérés")

    results = []
    for r in rows:
        article_id   = r["id"]
        commentaires = comments_by_article.get(str(article_id), [])
        contenu      = strip_html(r["content_html"] or "")

        results.append({
            "id":              article_id,
            "titre":           r["title"] or "",
            "image":           full_image_url(r["featured_image"] or ""),
            "abstract":        make_abstract(contenu),
            "contenu":         contenu,
            "description":     r["description"] or "",
            "date":            fmt_datetime(r["created"]),
            "date_maj":        fmt_datetime(r["updated"]),
            "auteur":          r["auteur"] or "",
            "user_legacy_id":  r["user_legacy_id"],
            "ligue":           r["ligue"] or "",
            "club_legacy_id":  r["club_legacy_id"],
            "likes":           0,
            "nb_commentaires": len(commentaires),
            "commentaires":    commentaires,
        })

    print(f"Sauvegarde dans {args.output} …")
    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(results, f, ensure_ascii=False, indent=2)

    with_comments = sum(1 for a in results if a["commentaires"])
    print(f"Terminé — {len(results)} articles, {total_comments} commentaires "
          f"({with_comments} articles commentés)")


if __name__ == "__main__":
    main()
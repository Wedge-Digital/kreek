#!/usr/bin/env python3
"""
Importe les articles et leurs commentaires depuis extracted_articles.json
vers la base PostgreSQL kreek.

Comportement articles :
  - Génère un ULID pour chaque article
  - legacy_id    ← id legacy (ON CONFLICT DO NOTHING = idempotent)
  - space_id     ← résolu via spaces.legacy_id     = club_legacy_id
  - author_id    ← résolu via auth__users.legacy_id = user_legacy_id
  - Ignore les articles dont l'espace ou l'auteur est introuvable en base

Comportement commentaires :
  - Importés après les articles (ou seuls avec --only-comments)
  - article_id   ← résolu via articles.legacy_id
  - author_id    ← résolu via auth__users.coach_name (insensible à la casse)
  - ON CONFLICT (legacy_id) DO NOTHING → rejouer le script n'insère que les manquants
  - Commentaires dont l'article ou l'auteur est introuvable : ignorés

Usage :
    python scripts/import_articles.py \\
        --host localhost --port 5432 \\
        --user kreek --password secret --database kreek_db \\
        --input scripts/extracted_articles.json \\
        [--only-comments] [--dry-run]
"""

import argparse
import json
import os
import sys
from datetime import datetime, timezone
from html import unescape

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

# ── SQL ──────────────────────────────────────────────────────────────────────

LOAD_SPACES_SQL = "SELECT id, legacy_id FROM spaces WHERE legacy_id IS NOT NULL"
LOAD_USERS_SQL  = "SELECT id, legacy_id FROM auth__users WHERE legacy_id IS NOT NULL"
LOAD_ARTICLES_SQL = "SELECT id, legacy_id FROM articles WHERE legacy_id IS NOT NULL"
LOAD_USERS_BY_NAME_SQL = "SELECT id, LOWER(coach_name) FROM auth__users"

INSERT_ARTICLE_SQL = """
    INSERT INTO articles (id, legacy_id, space_id, author_id, title, abstract, image, content, created_at)
    VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s)
    ON CONFLICT (legacy_id) DO NOTHING
"""

INSERT_COMMENT_SQL = """
    INSERT INTO comments (id, legacy_id, article_id, author_id, content, created_at)
    VALUES (%s, %s, %s, %s, %s, %s)
    ON CONFLICT (legacy_id) DO NOTHING
"""


# ── Helpers ──────────────────────────────────────────────────────────────────

def parse_args():
    p = argparse.ArgumentParser(description="Importe les articles et commentaires BBClub dans kreek")
    p.add_argument("--host",          default="localhost")
    p.add_argument("--port",          type=int, default=5432)
    p.add_argument("--user",          required=True)
    p.add_argument("--password",      required=True)
    p.add_argument("--database",      required=True)
    p.add_argument("--input",         default=os.path.join(_SCRIPTS_DIR, "extracted_articles.json"))
    p.add_argument("--only-comments", action="store_true",
                   help="Importe uniquement les commentaires (articles ignorés)")
    p.add_argument("--dry-run",       action="store_true",
                   help="Simule l'import sans écrire en base")
    return p.parse_args()


def parse_date(date_str: str | None):
    if not date_str:
        return None
    for fmt in ("%Y-%m-%dT%H:%M:%S.%fZ", "%Y-%m-%dT%H:%M:%SZ"):
        try:
            return datetime.strptime(date_str, fmt).replace(tzinfo=timezone.utc)
        except ValueError:
            continue
    return None


def make_content(contenu: str) -> str:
    return json.dumps([{"type": "1_COLUMN", "title": "", "content": contenu}], ensure_ascii=False)


# ── Import articles ───────────────────────────────────────────────────────────

def import_articles(cursor, articles: list, spaces_map: dict, users_map: dict, dry_run: bool) -> dict:
    """Insère les articles. Retourne le mapping legacy_id → kreek id pour les commentaires."""
    print(f"\n── Articles ({len(articles)}) ──")

    inserted = skipped = errors = 0

    for a in articles:
        legacy_id      = a["id"]
        title          = unescape(a.get("titre", "")).strip()
        image          = a.get("image") or None
        abstract       = a.get("abstract", "")
        contenu        = a.get("contenu", "")
        created_at     = parse_date(a.get("date"))
        club_legacy_id = a.get("club_legacy_id")
        user_legacy_id = a.get("user_legacy_id")

        if dry_run:
            print(f"  [DRY] #{legacy_id} {title[:60]}")
            inserted += 1
            continue

        space_id = spaces_map.get(club_legacy_id)
        if not space_id:
            print(f"  SKIP (espace introuvable club_legacy_id={club_legacy_id}) : {title[:60]}")
            skipped += 1
            continue

        author_id = users_map.get(user_legacy_id)
        if not author_id:
            print(f"  SKIP (auteur introuvable user_legacy_id={user_legacy_id}) : {title[:60]}")
            skipped += 1
            continue

        new_ulid = str(ULID())
        content  = make_content(contenu)

        try:
            cursor.execute(INSERT_ARTICLE_SQL, (
                new_ulid, legacy_id, space_id, author_id,
                title, abstract, image, content, created_at,
            ))
            inserted += 1
        except psycopg2.Error as e:
            print(f"  ERREUR #{legacy_id} {title[:60]} : {e}", file=sys.stderr)
            cursor.connection.rollback()
            errors += 1

    print(f"  Insérés : {inserted}  Ignorés : {skipped}  Erreurs : {errors}")
    return {}


# ── Import commentaires ───────────────────────────────────────────────────────

def import_comments(cursor, articles: list, articles_map: dict, users_by_name: dict, dry_run: bool):
    """
    Importe les commentaires depuis le champ 'commentaires' de chaque article.
    articles_map  : legacy_id (int) → kreek article.id (str)
    users_by_name : coach_name lowercase → kreek user.id (str)
    """
    total = sum(len(a.get("commentaires", [])) for a in articles)
    print(f"\n── Commentaires ({total} au total) ──")

    inserted = skipped_article = skipped_author = errors = 0

    for a in articles:
        article_legacy_id = a["id"]
        commentaires = a.get("commentaires", [])
        if not commentaires:
            continue

        article_id = articles_map.get(article_legacy_id)
        if not article_id and not dry_run:
            skipped_article += len(commentaires)
            continue

        for c in commentaires:
            comment_legacy_id = c["id"]
            auteur     = c.get("auteur", "").strip()
            content    = c.get("comment", "").strip()
            created_at = parse_date(c.get("date"))

            if dry_run:
                print(f"  [DRY] comment #{comment_legacy_id} by {auteur!r} on article #{article_legacy_id}")
                inserted += 1
                continue

            author_id = users_by_name.get(auteur.lower())
            if not author_id:
                skipped_author += 1
                continue

            if not content:
                skipped_author += 1
                continue

            new_ulid = str(ULID())
            try:
                cursor.execute(INSERT_COMMENT_SQL, (
                    new_ulid, comment_legacy_id, article_id, author_id, content, created_at,
                ))
                inserted += 1
            except psycopg2.Error as e:
                print(f"  ERREUR comment #{comment_legacy_id} : {e}", file=sys.stderr)
                cursor.connection.rollback()
                errors += 1

    print(f"  Insérés : {inserted}  "
          f"Ignorés (article manquant) : {skipped_article}  "
          f"Ignorés (auteur inconnu) : {skipped_author}  "
          f"Erreurs : {errors}")


# ── Main ─────────────────────────────────────────────────────────────────────

def main():
    args = parse_args()

    with open(args.input, encoding="utf-8") as f:
        articles = json.load(f)

    print(f"{len(articles)} articles chargés depuis {args.input}")
    if args.dry_run:
        print("Mode DRY-RUN — aucune écriture en base.")

    if args.dry_run:
        if not args.only_comments:
            import_articles(None, articles, {}, {}, dry_run=True)
        import_comments(None, articles, {}, {}, dry_run=True)
        return

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

    # Précharger les mappings communs
    cursor.execute(LOAD_USERS_BY_NAME_SQL)
    users_by_name = {row[1]: row[0] for row in cursor.fetchall()}
    print(f"  {len(users_by_name)} utilisateurs chargés (par nom)")

    if not args.only_comments:
        cursor.execute(LOAD_SPACES_SQL)
        spaces_map = {row[1]: row[0] for row in cursor.fetchall()}
        cursor.execute(LOAD_USERS_SQL)
        users_by_legacy = {row[1]: row[0] for row in cursor.fetchall()}
        print(f"  {len(spaces_map)} espaces chargés, {len(users_by_legacy)} utilisateurs (par legacy_id)")
        import_articles(cursor, articles, spaces_map, users_by_legacy, dry_run=False)
        conn.commit()

    # Charger les articles après insertion éventuelle
    cursor.execute(LOAD_ARTICLES_SQL)
    articles_map = {row[1]: row[0] for row in cursor.fetchall()}
    print(f"  {len(articles_map)} articles chargés (par legacy_id)")

    import_comments(cursor, articles, articles_map, users_by_name, dry_run=False)
    conn.commit()

    cursor.close()
    conn.close()


if __name__ == "__main__":
    main()

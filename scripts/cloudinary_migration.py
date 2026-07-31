#!/usr/bin/env python3
"""
Briques communes aux migrations d'images vers Cloudinary
(`migrate_spaces_images.py` et `migrate_articles_images.py`).

Trois mécanismes y sont mutualisés :

  * `fetch_index()` — liste en une passe paginée tout ce qui est déjà en ligne
    sous un préfixe, au lieu d'un `api.resource()` par élément. L'API Admin est
    limitée à 500 opérations/heure : 649 articles vérifiés un par un dépassent
    le quota avant la fin, là où l'index coûte 2 appels.
  * `with_rate_limit_retry()` — en cas de quota atteint, attend l'heure de
    réarmement annoncée par Cloudinary puis réessaie, dans la limite de
    `--max-wait`.
  * `save_json()` — appelé périodiquement par les scripts pour que le travail
    déjà fait survive à une interruption.
"""

import json
import re
import sys
import time
from datetime import datetime, timezone

try:
    import cloudinary
    import cloudinary.api
    import cloudinary.exceptions
    import cloudinary.uploader
except ImportError:
    print("Dépendance manquante : pip install cloudinary", file=sys.stderr)
    sys.exit(1)

# Nombre d'éléments traités entre deux écritures du JSON.
FLUSH_EVERY = 10

# Marge ajoutée à l'heure de réarmement annoncée, pour ne pas retomber dessus.
RETRY_MARGIN_SECONDS = 5

# Attente par défaut si Cloudinary ne précise pas d'heure de réarmement.
DEFAULT_RETRY_SECONDS = 60

MAX_RATE_LIMIT_ATTEMPTS = 3

_RESET_RE = re.compile(r"Try again on (\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) UTC")


class RateLimitAbort(Exception):
    """Quota atteint et réarmement trop lointain pour être attendu."""


def add_common_args(parser):
    parser.add_argument("--cloud-name", required=True)
    parser.add_argument("--api-key",    required=True)
    parser.add_argument("--api-secret", required=True)
    parser.add_argument("--dry-run",    action="store_true",
                        help="Simule sans uploader ni modifier le JSON")
    # Le quota Cloudinary se réarme à l'heure ronde : 3700 s couvre n'importe
    # quel point de la fenêtre, pour que le script attende plutôt qu'échouer.
    parser.add_argument("--max-wait",   type=int, default=3700,
                        help="Attente maximale en secondes si le quota Cloudinary "
                             "est atteint (défaut : 3700, soit une fenêtre entière ; "
                             "0 pour échouer immédiatement)")
    return parser


def configure(args):
    cloudinary.config(
        cloud_name=args.cloud_name,
        api_key=args.api_key,
        api_secret=args.api_secret,
        secure=True,
    )


def is_cloudinary(url: str) -> bool:
    return "res.cloudinary.com" in url


def retry_delay(error) -> int:
    """Secondes à attendre avant de réessayer, d'après le message de Cloudinary."""
    match = _RESET_RE.search(str(error))
    if not match:
        return DEFAULT_RETRY_SECONDS
    reset = datetime.strptime(match.group(1), "%Y-%m-%d %H:%M:%S").replace(tzinfo=timezone.utc)
    delta = (reset - datetime.now(timezone.utc)).total_seconds()
    return max(1, int(delta) + RETRY_MARGIN_SECONDS)


def with_rate_limit_retry(action, max_wait: int):
    """Exécute `action`, en attendant le réarmement du quota si nécessaire."""
    for _ in range(MAX_RATE_LIMIT_ATTEMPTS):
        try:
            return action()
        except cloudinary.exceptions.RateLimited as e:
            delay = retry_delay(e)
            if delay > max_wait:
                raise RateLimitAbort(
                    f"quota atteint, réarmement dans {delay}s "
                    f"(> --max-wait={max_wait}s)"
                ) from e
            print(f"  Quota Cloudinary atteint — pause de {delay}s avant reprise…")
            time.sleep(delay)
    raise RateLimitAbort("quota toujours atteint après plusieurs tentatives")


def fetch_index(prefix: str, max_wait: int) -> dict:
    """public_id → secure_url de tout ce qui est déjà en ligne sous `prefix`."""
    index, cursor = {}, None
    while True:
        page = with_rate_limit_retry(
            lambda: cloudinary.api.resources(
                type="upload", prefix=prefix, max_results=500, next_cursor=cursor,
            ),
            max_wait,
        )
        index.update({r["public_id"]: r["secure_url"] for r in page["resources"]})
        cursor = page.get("next_cursor")
        if not cursor:
            return index


def upload(src_url: str, public_id: str, max_wait: int) -> str:
    """Uploade sans jamais écraser ni dupliquer (public_id explicite)."""
    result = with_rate_limit_retry(
        lambda: cloudinary.uploader.upload(
            src_url, public_id=public_id, overwrite=False, resource_type="image",
        ),
        max_wait,
    )
    return result["secure_url"]


def fail_rate_limited(error, saved_path: str = None):
    """Message lisible plutôt qu'une trace Python, et code de sortie non nul."""
    print(f"\nInterrompu : {error}", file=sys.stderr)
    if saved_path:
        print(f"Progression sauvegardée dans {saved_path}.", file=sys.stderr)
    print("Relancer plus tard — les images déjà traitées ne seront pas revérifiées.",
          file=sys.stderr)
    print("Ou augmenter --max-wait pour laisser le script attendre le réarmement.",
          file=sys.stderr)
    sys.exit(1)


def save_json(path: str, data):
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


def load_json(path: str):
    with open(path, encoding="utf-8") as f:
        return json.load(f)

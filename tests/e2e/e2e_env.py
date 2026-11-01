"""Où la suite e2e trouve son serveur et sa base — un seul endroit.

La suite a **son** serveur (`make e2e-server`, port 3211) et **sa** base
(`kreek_e2e`, lue dans `.env.e2e`), à côté du serveur de travail (port 3210,
`.env.dev`). Les deux tournent en même temps : les tests n'écrivent plus dans la
base qu'on consulte à la main.

`E2E_BASE_URL` surcharge l'adresse — la CI s'en passe, elle lance le même
profil.
"""

import os
from pathlib import Path

BASE_URL = os.environ.get("E2E_BASE_URL", "http://localhost:3211")

ENV_FILE = Path(__file__).resolve().parents[2] / ".env.e2e"


def database_url() -> str:
    """DATABASE__URL de `.env.e2e` — la base que le serveur e2e écrit."""
    for line in ENV_FILE.read_text().splitlines():
        if line.startswith("DATABASE__URL="):
            return line.split("=", 1)[1].strip().strip("\"'")
    raise RuntimeError(f"DATABASE__URL introuvable dans {ENV_FILE}")

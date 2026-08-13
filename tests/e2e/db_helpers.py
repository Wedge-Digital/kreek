"""Accès direct à la base de données pour les assertions e2e qui ne peuvent
pas être vérifiées via une requête HTTP (ex. contenu brut d'une projection).

Connexion TCP directe via `psql` (hôte), pas `docker exec` : évite de
dépendre des droits d'accès au socket Docker, non garantis selon
l'environnement de dev.
"""

import subprocess
from pathlib import Path


def dev_database_url() -> str:
    """Lit DATABASE__URL depuis .env.dev — même source de vérité que le
    Makefile (`grep -E '^DATABASE__URL=' .env.dev`)."""
    env_path = Path(__file__).resolve().parents[2] / ".env.dev"
    for line in env_path.read_text().splitlines():
        if line.startswith("DATABASE__URL="):
            return line.split("=", 1)[1].strip()
    raise RuntimeError(f"DATABASE__URL introuvable dans {env_path}")


def execute_db(sql: str) -> None:
    """Écriture directe en base — réservée aux états qu'aucun parcours
    utilisateur ne peut atteindre.

    Aujourd'hui un seul usage : vieillir un panier de customisation de plus de
    24 h pour observer sa péremption. Attendre réellement une journée n'est pas
    une option, et reculer l'horloge du serveur en toucherait bien d'autres.

    Volontairement distincte de `query_db` malgré un corps quasi identique :
    une fonction qui écrit doit se voir à l'appel. Un test qui fabrique un état
    impossible le déclare, il ne le glisse pas dans une lecture.
    """
    result = subprocess.run(
        ["psql", dev_database_url(), "-t", "-A", "-c", sql],
        capture_output=True, text=True, timeout=10,
    )
    assert result.returncode == 0, f"psql error: {result.stderr}"


def query_db(sql: str) -> list[str]:
    result = subprocess.run(
        ["psql", dev_database_url(), "-t", "-A", "-c", sql],
        capture_output=True, text=True, timeout=10,
    )
    assert result.returncode == 0, f"psql error: {result.stderr}"
    return [l.strip() for l in result.stdout.strip().splitlines() if l.strip()]

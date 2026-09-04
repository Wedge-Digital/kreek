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

    Deux usages aujourd'hui :

    - vieillir un panier de customisation de plus de 24 h pour observer sa
      péremption — attendre réellement une journée n'est pas une option, et
      reculer l'horloge du serveur en toucherait bien d'autres ;
    - reculer le statut d'une saison publiée pour retrouver l'état « en cours
      de configuration » (carte 407) — le magicien enchaîne ses cinq phases
      sans point d'arrêt.

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


def attendre_que(condition, timeout_s: float = 20.0, quoi: str = "la condition"):
    """Rend la main dès que `condition()` est vraie, ou échoue en le disant.

    **Pourquoi c'est nécessaire.** Publier un rapport de match déclenche des app
    events traités dans une tâche séparée : la projection n'est pas à jour au
    retour de la requête. Lire `players_proj` dans la foulée mesure la vitesse
    de la machine, pas le comportement du produit — et le message d'échec accuse
    alors le produit pour un défaut de test.

    **Pourquoi pas un `sleep`.** Une durée fixe n'a aucune marge sur une machine
    chargée, et c'est précisément là que la course s'ouvre : le même test passe
    en 6 minutes de suite et tombe en 8. Elle coûterait en plus son délai à
    chaque appel, y compris quand tout est déjà prêt. La boucle ci-dessous rend
    la main dès que c'est vrai.

    **Ce qu'elle ne remplace pas.** Pour une assertion *négative* — « rien ne
    doit arriver » — attendre ne prouve rien : la condition serait vraie aussi
    si le pipeline ne tournait jamais. Il faut d'abord attendre un marqueur de
    progression, puis vérifier l'absence.
    """
    import time

    limite = time.time() + timeout_s
    while time.time() < limite:
        if condition():
            return
        time.sleep(0.2)
    raise AssertionError(
        f"{quoi} n'est toujours pas vraie après {timeout_s:g} s — "
        "le bus d'app events n'a rien projeté, ou la chaîne est cassée"
    )

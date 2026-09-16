"""Une équipe joue au plus un match par journée (carte 551).

La règle existait déjà — mais comme détail de l'algorithme de tirage, dans le
`used: HashSet` de `generate_round_pairings`. Elle ne protégeait que ce
chemin-là : `add_match_use_case`, par lequel un commissaire ajoute une rencontre
à la main, ne vérifiait que l'enrôlement des deux équipes.

**Ces tests exercent le chemin de l'admin**, celui qui était troué. Les tests
unitaires du domaine et du use case couvrent la règle ; ceux-ci vérifient
qu'elle remonte bien jusqu'à la réponse HTTP, avec un message qui nomme
l'adversaire déjà prévu.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db

JSON = {"Content-Type": "application/json", "HX-Request": "true"}


@pytest.fixture(scope="module")
def competition(browser, space_id):
    """Quatre équipes, deux journées déjà pairées.

    Quatre et non deux : la contre-épreuve a besoin de deux équipes **libres**
    pendant que deux autres sont engagées. Avec deux équipes, tout refus serait
    indistinguable d'un refus systématique.
    """
    ctx = build_full_competition(browser, space_id, num_teams=4, num_rounds=2)
    return {"space_id": space_id, **ctx}


def _admin_url(ctx: dict, suffixe: str) -> str:
    return (
        f"{BASE_URL}/app/{ctx['space_id']}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/admin/{suffixe}"
    )


def _ajouter_match(ctx: dict, round_id: str, domicile: str, exterieur: str):
    return requests.post(
        _admin_url(ctx, "schedule/add-match"),
        json={
            "round_id": round_id,
            "home_team_id": domicile,
            "away_team_id": exterieur,
        },
        headers=JSON,
        timeout=15,
    )


def _appariements(round_id: str) -> list[tuple[str, str]]:
    lignes = query_db(
        "SELECT home_team_id || '|' || away_team_id "
        f"FROM competition_match_day_pairings WHERE match_day_id = '{round_id}'"
    )
    return [tuple(l.split("|")) for l in lignes]


def test_une_equipe_deja_engagee_est_refusee(competition):
    """Le cas de l'incident : une équipe qui a déjà un adversaire ce jour-là."""
    round_id = competition["round_ids"][0]
    engages = _appariements(round_id)
    assert engages, "la journée doit être pairée pour que le test ait un sens"

    domicile_engage = engages[0][0]
    # Un tiers, non engagé contre celle-là : le refus doit venir de la première
    # équipe, pas d'une collision entre les deux.
    autres = [t for t in competition["team_ids"] if t not in engages[0]]
    assert autres, "il faut une équipe hors de cette rencontre"

    reponse = _ajouter_match(competition, round_id, domicile_engage, autres[0])

    assert reponse.status_code == 422, reponse.text
    message = reponse.json()["error"]
    assert "affronte déjà" in message, message


def test_l_equipe_engagee_a_l_exterieur_est_refusee_aussi(competition):
    """**Le test qui compte.**

    Une implémentation ne regardant que `home_team_id` passerait le précédent et
    manquerait celui-ci — c'est-à-dire la moitié des rencontres, celle des
    déplacements.
    """
    round_id = competition["round_ids"][0]
    engages = _appariements(round_id)
    exterieur_engage = engages[0][1]
    autres = [t for t in competition["team_ids"] if t not in engages[0]]

    reponse = _ajouter_match(competition, round_id, autres[0], exterieur_engage)

    assert reponse.status_code == 422, reponse.text
    assert "affronte déjà" in reponse.json()["error"]


def test_le_refus_nomme_l_adversaire_deja_prevu(competition):
    """Un refus qui dit seulement « impossible » envoie l'admin fouiller le
    calendrier pour comprendre ce qui bloque — alors que le domaine vient de
    lire l'information."""
    round_id = competition["round_ids"][0]
    domicile, exterieur = _appariements(round_id)[0]
    autres = [t for t in competition["team_ids"] if t not in (domicile, exterieur)]

    attendu = query_db(
        f"SELECT team_name FROM team_proj WHERE team_id = '{exterieur}'"
    )[0]

    message = _ajouter_match(competition, round_id, domicile, autres[0]).json()["error"]

    assert attendu in message, f"« {attendu} » attendu dans : {message}"


def test_deux_equipes_libres_restent_appariables(competition):
    """Contre-épreuve. Sans elle, un refus systématique passerait les trois
    tests précédents."""
    round_id = competition["round_ids"][1]

    vide = requests.post(
        _admin_url(competition, "schedule/clear-round"),
        json={"round_id": round_id},
        headers=JSON,
        timeout=15,
    )
    assert vide.status_code < 400, vide.text
    assert _appariements(round_id) == [], "la journée doit être vidée"

    a, b, c = competition["team_ids"][:3]

    premier = _ajouter_match(competition, round_id, a, b)
    assert premier.status_code < 400, premier.text

    # Et l'invariant s'applique aussitôt à la rencontre qu'on vient de créer.
    second = _ajouter_match(competition, round_id, a, c)
    assert second.status_code == 422, second.text

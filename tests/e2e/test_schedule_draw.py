"""Le tirage des journées du Calendrier (épic E18).

Scénario n°9 — « Générer une journée complète » ne laisse aucune équipe sans
match hors l'exemptée. C'est la régression du défaut de l'ancien tirage
(`generate_round_pairings`), qui préférait renvoyer une équipe chez elle
plutôt que de programmer une revanche.

Les scénarios 1 à 8 — l'appariement des présents — arrivent avec la carte 619.

**Ce qui n'est pas testé : l'aléa.** On vérifie que chaque présente joue, pas
qui joue contre qui : un test qui dépendrait du tirage échouerait une fois sur
N (cf. cartes 532 et 580).

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import time

import pytest
import requests
from playwright.sync_api import Page

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from htmx_helpers import attendre_cablage

JSON = {"Content-Type": "application/json", "HX-Request": "true"}


@pytest.fixture(scope="module")
def competition(browser, space_id):
    """Cinq équipes, trois journées déjà générées.

    Cinq et non quatre : un nombre impair force une exemptée, et c'est la seule
    équipe qui a le droit de ne pas jouer. Trois journées : la dernière tirée
    porte un historique, donc des revanches à éviter.
    """
    ctx = build_full_competition(browser, space_id, num_teams=5, num_rounds=3)
    return {"space_id": space_id, **ctx}


def _admin_url(ctx: dict, suffixe: str) -> str:
    return (
        f"{BASE_URL}/app/{ctx['space_id']}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/admin/{suffixe}"
    )


def _equipes_engagees(round_id: str) -> list[str]:
    lignes = query_db(
        "SELECT home_team_id || '|' || away_team_id "
        f"FROM competition_match_day_pairings WHERE match_day_id = '{round_id}'"
    )
    return [equipe for ligne in lignes for equipe in ligne.split("|")]


def _attendre(condition, description: str, timeout_s: int = 15):
    deadline = time.time() + timeout_s
    dernier = None
    while time.time() < deadline:
        dernier = condition()
        if dernier:
            return dernier
        time.sleep(0.3)
    raise AssertionError(f"{description} — non satisfait après {timeout_s}s ({dernier})")


def test_generer_une_journee_complete_fait_jouer_toutes_les_equipes_sauf_l_exemptee(
    page: Page, competition
):
    round_id = competition["round_ids"][-1]
    vidage = requests.post(
        _admin_url(competition, "schedule/clear-round"),
        json={"round_id": round_id},
        headers=JSON,
        timeout=15,
    )
    assert vidage.ok, f"clear-round : {vidage.status_code} {vidage.text[:200]}"
    assert _equipes_engagees(round_id) == []

    page.goto(_admin_url(competition, "schedule"), wait_until="load")
    page.locator(".round-item").nth(len(competition["round_ids"]) - 1).click()
    page.wait_for_selector(".match-list-empty")
    # Le bouton appelle `fetch` en `onclick` : htmx ne le câble jamais. On
    # attend donc son voisin `hx-delete`, câblé avec le même fragment.
    attendre_cablage(page, ".round-detail-footer .btn-danger-sm")
    page.locator(".btn-primary-sm:has-text('Générer une journée complète')").click()

    engagees = _attendre(lambda: _equipes_engagees(round_id), "la journée doit être tirée")
    assert len(engagees) == 4, f"cinq équipes : deux rencontres attendues, obtenu {engagees}"
    assert len(set(engagees)) == 4, f"une équipe joue deux fois : {engagees}"
    assert set(engagees) <= set(competition["team_ids"])

    # R9 — la journée pourvue n'offre plus le bouton.
    page.wait_for_selector(".match-row")
    assert page.locator(".btn-primary-sm:has-text('Générer une journée complète')").count() == 0

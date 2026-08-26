"""Un rapport de match manuel apparaît aux résultats dès sa confirmation (427).

Le défaut : un rapport créé **hors calendrier** n'a pas d'appariement, et la
projection de l'onglet Résultats a `pairing_id` pour clef. Trois listeners
l'alimentent ; celui de la confirmation — le seul qui écrit « en cours » —
abandonnait en silence faute de pouvoir en fabriquer un.

Pendant toute la saisie, personne ne voyait que le match avait commencé.

Ce test traverse `match_report`, le bus d'app events et `competitions` : aucun
test unitaire ne peut le remplacer.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import re

import pytest
import requests
from playwright.sync_api import Page, expect

from db_helpers import query_db

BASE_URL = "http://localhost:3210"
_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")


@pytest.fixture(scope="module")
def contexte(browser, space_id):
    from competition_lifecycle import build_full_competition

    # Quatre équipes : `generate-all` en apparie deux couples, et il reste
    # donc des paires que rien ne programme — la matière d'un rapport manuel.
    return build_full_competition(browser, space_id, num_teams=4, num_rounds=1)


def _couple_non_apparie(ctx):
    """Deux équipes que `generate-all` n'a pas mises face à face sur la journée.

    Cherché en base plutôt que supposé : une première version de ce test prenait
    les équipes 0 et 1, que le générateur apparie justement ensemble. Le rapport
    était donc **programmé**, et le test passait sans rien prouver — il aurait
    passé aussi avant le correctif.
    """
    round_id = ctx["round_ids"][0]
    apparies = query_db(
        "SELECT home_team_id || '|' || away_team_id FROM competition_match_day_pairings "
        f"WHERE match_day_id = '{round_id}'"
    )
    couples = {tuple(sorted(ligne.split("|"))) for ligne in apparies}
    equipes = ctx["team_ids"]
    for i in range(len(equipes)):
        for j in range(i + 1, len(equipes)):
            if tuple(sorted((equipes[i], equipes[j]))) not in couples:
                return equipes[i], equipes[j]
    raise AssertionError("toutes les paires sont appariées : aucun rapport manuel possible")


def _rapport_manuel(space_id, ctx):
    """Un rapport **sans appariement préalable** — le cas que la carte 427 rend
    visible."""
    domicile, exterieur = _couple_non_apparie(ctx)
    champs = {
        "competition_id": ctx["competition_id"],
        "season_id": ctx["season_id"],
        "round_id": ctx["round_ids"][0],
        "home_team_id": domicile,
        "away_team_id": exterieur,
    }
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/new", data=champs, allow_redirects=False
    )
    assert resp.status_code in (302, 303), f"création : {resp.status_code}\n{resp.text[:300]}"
    m = _ULID_RE.search(resp.headers.get("Location", ""))
    assert m, f"identifiant introuvable : {resp.headers.get('Location')!r}"
    mr_id = m.group(1)

    if requests.get(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2", allow_redirects=False
    ).status_code != 200:
        resp = requests.post(
            f"{BASE_URL}/app/{space_id}/match-report/{mr_id}", data=champs, allow_redirects=False
        )
        assert resp.status_code in (302, 303), f"confirmation : {resp.status_code}"
    return mr_id


def test_un_rapport_en_cours_apparait_aux_resultats(page: Page, space_id, contexte):
    mr_id = _rapport_manuel(space_id, contexte)

    # Le préalable du test, asserté et non supposé : sans lui, un rapport
    # programmé passerait le reste sans rien démontrer.
    origine = query_db(
        f"SELECT origin FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )
    assert origine == ["Manual"], f"le rapport devait être manuel, il est {origine}"

    lignes = query_db(
        "SELECT match_status FROM competition_match_display_proj "
        f"WHERE match_report_id = '{mr_id}'"
    )
    assert lignes == ["in_progress"], (
        "la confirmation doit créer la ligne de résultats et l'ouvrir « en cours » ; "
        f"obtenu : {lignes}"
    )

    page.goto(
        f"{BASE_URL}/app/{space_id}/competitions/{contexte['competition_id']}"
        f"/{contexte['season_id']}/resultats",
        wait_until="load",
    )
    badge = page.locator(".match-status-badge--in-progress").first
    badge.wait_for(timeout=10000)
    expect(badge).to_contain_text("En cours de saisie")

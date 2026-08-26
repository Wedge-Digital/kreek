"""Annuler un rapport de match en cours (carte 433).

Le domaine savait annuler depuis toujours ; aucune route n'y menait. Sans ce
chemin, un rapport ouvert par erreur verrouillait les deux équipes jusqu'à ce
qu'un administrateur vide la journée — ce qui, pour un match programmé, efface
la rencontre du calendrier.

Le test traverse `match_report`, `teams` et `competitions` : l'annulation libère
le verrou de saisie par un bus d'événements qu'aucun test unitaire ne franchit.

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

    # Quatre équipes : `generate-all` apparie deux couples, et il en reste
    # d'autres que rien ne programme — la matière d'un rapport manuel.
    return build_full_competition(browser, space_id, num_teams=4, num_rounds=1)


def _confirmer(space_id, mr_id, champs):
    if requests.get(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2", allow_redirects=False
    ).status_code != 200:
        resp = requests.post(
            f"{BASE_URL}/app/{space_id}/match-report/{mr_id}", data=champs, allow_redirects=False
        )
        assert resp.status_code in (302, 303), f"confirmation : {resp.status_code}"


def _rapport(space_id, ctx, domicile, exterieur):
    """Ouvre un rapport sur ce couple et le confirme.

    `/new` déduplique : pour une rencontre programmée, il retrouve le brouillon
    que `pairing_created_listener` a déjà créé, et le rapport est donc
    **programmé**. Pour un couple non apparié, il en crée un **manuel**. C'est ce
    qui distingue les deux tests, et il faut l'asserter — une première version
    croyait tester le cas programmé dans les deux.
    """
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
    assert resp.status_code in (302, 303), f"création : {resp.status_code}"
    mr_id = _ULID_RE.search(resp.headers.get("Location", "")).group(1)
    _confirmer(space_id, mr_id, champs)
    return mr_id


def _couples(ctx):
    """Un couple programmé et un couple qui ne l'est pas, lus en base."""
    apparies = query_db(
        "SELECT home_team_id || '|' || away_team_id FROM competition_match_day_pairings "
        f"WHERE match_day_id = '{ctx['round_ids'][0]}'"
    )
    assert apparies, "la génération devait produire au moins un appariement"
    programme = tuple(apparies[0].split("|"))
    couples = {tuple(sorted(l.split("|"))) for l in apparies}
    equipes = ctx["team_ids"]
    for i in range(len(equipes)):
        for j in range(i + 1, len(equipes)):
            if tuple(sorted((equipes[i], equipes[j]))) not in couples:
                return programme, (equipes[i], equipes[j])
    raise AssertionError("aucun couple libre : impossible de créer un rapport manuel")


def _annuler(page, space_id, mr_id):
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2", wait_until="load")
    bouton = page.locator(".mr-cancel-btn")
    bouton.wait_for(timeout=10000)
    expect(bouton).to_contain_text("Annuler le rapport")
    page.on("dialog", lambda d: d.accept())
    bouton.click()
    page.wait_for_url(re.compile(r".*/resultats$"), timeout=10000)


def _phase(mr_id):
    lignes = query_db(f"SELECT phase FROM match_report_proj WHERE match_report_id = '{mr_id}'")
    return lignes[0] if lignes else None


def _origine(mr_id):
    return query_db(f"SELECT origin FROM match_report_proj WHERE match_report_id = '{mr_id}'")[0]


def test_annuler_un_rapport_programme_remet_la_rencontre_a_venir(page: Page, space_id, contexte):
    """La rencontre appartient au calendrier : elle y reste. La supprimer
    effacerait un match que l'administrateur avait posé (carte 427)."""
    (domicile, exterieur), _ = _couples(contexte)
    mr_id = _rapport(space_id, contexte, domicile, exterieur)
    assert _origine(mr_id) == "Pairing", "ce test doit porter sur une rencontre programmée"

    _annuler(page, space_id, mr_id)

    assert _phase(mr_id) == "Cancelled"
    statut = query_db(
        "SELECT match_status FROM competition_match_display_proj "
        f"WHERE home_team_id = '{domicile}' AND away_team_id = '{exterieur}'"
    )
    assert statut == ["upcoming"], f"la rencontre doit redevenir « à venir » : {statut}"


def test_annuler_un_rapport_manuel_efface_sa_ligne(page: Page, space_id, contexte):
    """L'appariement d'un rapport manuel n'a été fabriqué que pour lui : il s'en
    va avec lui."""
    _, (domicile, exterieur) = _couples(contexte)
    mr_id = _rapport(space_id, contexte, domicile, exterieur)
    assert _origine(mr_id) == "Manual", "ce test doit porter sur un rapport manuel"

    _annuler(page, space_id, mr_id)

    assert _phase(mr_id) == "Cancelled"
    restant = query_db(
        "SELECT match_status FROM competition_match_display_proj "
        f"WHERE home_team_id = '{domicile}' AND away_team_id = '{exterieur}'"
    )
    assert restant == [], f"la ligne fabriquée pour ce rapport doit partir : {restant}"

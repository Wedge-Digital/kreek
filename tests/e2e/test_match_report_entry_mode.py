"""« Qui saisit ? » au démarrage (carte 632, épic E21).

Celui qui démarre un match choisit qui saisit. L'écran de sélection porte une
carte « Qui saisit ce match ? », cochée sur « Je saisis tout » ; « Chacun son
équipe » y figure grisée et marquée « bientôt » (décision 1 du cadrage).

Le mode part dans le champ `entry_mode` du POST de la phase 1. Absent, il vaut
« Je saisis tout » (décision 2) ; inconnu, la requête est refusée en 400 avant
le moindre effet.
"""

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import attendre_que, query_db


@pytest.fixture(scope="module")
def competition(browser, space_id):
    return build_full_competition(browser, space_id, 2, 2)


def _rapport_de_la_journee(round_id: str) -> str:
    lignes = query_db(
        "SELECT match_report_id FROM match_report_proj "
        f"WHERE round_id = '{round_id}' LIMIT 1"
    )
    assert lignes, "la journée doit porter un rapport, créé avec son appariement"
    return lignes[0].strip()


def _mode_et_demarreur(mr_id: str) -> tuple[str, str, str]:
    ligne = query_db(
        "SELECT phase || '|' || entry_mode || '|' || COALESCE(started_by, '') "
        f"FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )[0]
    phase, mode, demarreur = ligne.split("|")
    return phase, mode, demarreur


def test_la_carte_est_visible_et_cochee_sur_je_saisis_tout(page: Page, space_id):
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/new", wait_until="load")

    carte = page.locator(".entry-mode-card")
    expect(carte).to_contain_text("Qui saisit ce match ?")
    expect(carte).to_contain_text("il ne pourra plus changer")
    expect(carte.locator("input[name='entry_mode'][value='SoloEntry']")).to_be_checked()

    bientot = carte.locator(".entry-mode-option--soon")
    expect(bientot).to_contain_text("Chacun son équipe")
    expect(bientot).to_contain_text("bientôt")
    expect(bientot.locator("input[name='entry_mode']")).to_be_disabled()


def test_un_mode_inconnu_est_refuse_sans_demarrer(space_id, competition):
    """Le refus précède tout effet : le brouillon reste un brouillon. Le corps
    porte les deux équipes du brouillon, pour que seul le mode soit en cause."""
    mr_id = _rapport_de_la_journee(competition["round_ids"][1])
    domicile, exterieur = query_db(
        "SELECT home_team_id || '|' || away_team_id FROM match_report_proj "
        f"WHERE match_report_id = '{mr_id}'"
    )[0].split("|")

    reponse = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}",
        data={"home_team_id": domicile, "away_team_id": exterieur, "entry_mode": "TeamEntry"},
        allow_redirects=False,
        timeout=30,
    )

    assert reponse.status_code == 400, f"obtenu {reponse.status_code}"
    assert _mode_et_demarreur(mr_id)[0] == "Draft"


def test_commencer_demarre_en_je_saisis_tout(page: Page, space_id, competition):
    """Le geste réel : le formulaire envoie le mode coché, et le match en sort
    démarré en « Je saisis tout », avec son démarreur."""
    mr_id = _rapport_de_la_journee(competition["round_ids"][0])
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}", wait_until="load")

    with page.expect_request(
        lambda r: r.method == "POST" and r.url.endswith(f"/match-report/{mr_id}")
    ) as envoyee:
        page.get_by_role("button", name="Commencer").click()
    assert "entry_mode=SoloEntry" in (envoyee.value.post_data or "")

    attendre_que(
        lambda: _mode_et_demarreur(mr_id)[0] != "Draft",
        quoi="le démarrage du match",
    )
    _, mode, demarreur = _mode_et_demarreur(mr_id)
    assert mode == "SoloEntry"
    assert demarreur, "le démarreur doit être enregistré"

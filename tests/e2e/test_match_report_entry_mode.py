"""« Qui saisit ? » au démarrage (carte 632, épic E21).

Celui qui démarre un match choisit qui saisit. L'écran de sélection porte une
carte « Qui saisit ce match ? », cochée sur « Je saisis tout » ; « Chacun son
équipe » y figure grisée et marquée « bientôt » (décision 1 du cadrage).

Le mode part dans le champ `entry_mode` du POST de la phase 1. Absent, il vaut
« Je saisis tout » (décision 2) ; inconnu, la requête est refusée en 400 avant
le moindre effet.

« Chacun son équipe » (`TeamEntry`, carte 652, épic E22) se démarre par un POST
direct : l'option reste grisée jusqu'à la carte 659. Les deux coachs du match
entrent alors dans l'avant-match ; un tiers reste dehors. Le membre simple de la
suite (`X-Bypass-Auth-Profile: simple`) coache la deuxième équipe que monte
`build_full_competition`, DevCoach la première (cf. `test_match_report_access.py`).
"""

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import attendre_que, query_db
from match_report_helpers import create_draft, draft_of_round, entry_mode_of

PROFIL_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}


@pytest.fixture(scope="module")
def competition(browser, space_id):
    return build_full_competition(browser, space_id, 2, 2)


@pytest.fixture(scope="module")
def chacun_son_equipe(browser, space_id):
    """Deux rapports démarrés par DevCoach en « Chacun son équipe » : l'un
    l'oppose au membre simple (équipes 1 et 2), l'autre n'implique pas le
    membre simple (équipes 3 et 4)."""
    ctx = build_full_competition(browser, space_id, num_teams=4, num_rounds=2)
    t, journees = ctx["team_ids"], ctx["round_ids"]
    return {
        "space_id": space_id,
        "adverses": create_draft(space_id, ctx, journees[0], t[0], t[1], entry_mode="TeamEntry"),
        "etranger": create_draft(space_id, ctx, journees[1], t[2], t[3], entry_mode="TeamEntry"),
    }


def _get(ctx: dict, rapport: str, suffixe: str, headers=None):
    return requests.get(
        f"{BASE_URL}/app/{ctx['space_id']}/match-report/{ctx[rapport]}{suffixe}",
        headers=headers or {},
        allow_redirects=False,
        timeout=30,
    )


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
    mr_id = draft_of_round(competition["round_ids"][1])
    domicile, exterieur = query_db(
        "SELECT home_team_id || '|' || away_team_id FROM match_report_proj "
        f"WHERE match_report_id = '{mr_id}'"
    )[0].split("|")

    reponse = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}",
        data={"home_team_id": domicile, "away_team_id": exterieur, "entry_mode": "UnknownEntry"},
        allow_redirects=False,
        timeout=30,
    )

    assert reponse.status_code == 400, f"obtenu {reponse.status_code}"
    assert entry_mode_of(mr_id)[0] == "Draft"


def test_commencer_demarre_en_je_saisis_tout(page: Page, space_id, competition):
    """Le geste réel : le formulaire envoie le mode coché, et le match en sort
    démarré en « Je saisis tout », avec son démarreur."""
    mr_id = draft_of_round(competition["round_ids"][0])
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}", wait_until="load")

    with page.expect_request(
        lambda r: r.method == "POST" and r.url.endswith(f"/match-report/{mr_id}")
    ) as envoyee:
        page.get_by_role("button", name="Commencer").click()
    assert "entry_mode=SoloEntry" in (envoyee.value.post_data or "")

    attendre_que(
        lambda: entry_mode_of(mr_id)[0] != "Draft",
        quoi="le démarrage du match",
    )
    _, mode, demarreur = entry_mode_of(mr_id)
    assert mode == "SoloEntry"
    assert demarreur, "le démarreur doit être enregistré"


def test_chacun_son_equipe_ouvre_l_avant_match_aux_deux_coachs(chacun_son_equipe):
    """En « Je saisis tout », l'autre coach serait renvoyé au direct (303) ;
    en « Chacun son équipe », il entre dans l'avant-match comme le démarreur."""
    phase, mode, _ = entry_mode_of(chacun_son_equipe["adverses"])
    assert (phase, mode) == ("PreMatch", "TeamEntry")

    for qui in ({}, PROFIL_SIMPLE):
        reponse = _get(chacun_son_equipe, "adverses", "/step2", headers=qui)
        assert reponse.status_code == 200, f"{qui} : obtenu {reponse.status_code}"

    arrivee = _get(chacun_son_equipe, "adverses", "", headers=PROFIL_SIMPLE)
    assert arrivee.status_code == 303, f"obtenu {arrivee.status_code}"
    assert arrivee.headers["Location"].endswith("/step2")


def test_chacun_son_equipe_reste_ferme_a_un_tiers(chacun_son_equipe):
    reponse = _get(chacun_son_equipe, "etranger", "/step2", headers=PROFIL_SIMPLE)
    assert reponse.status_code == 403, f"obtenu {reponse.status_code}"

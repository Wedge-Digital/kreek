"""« Qui saisit ? » au démarrage (carte 632, épic E21).

Celui qui démarre un match choisit qui saisit. L'écran de sélection porte une
carte « Qui saisit ce match ? », cochée sur « Je saisis tout » ; « Chacun son
équipe » s'y choisit depuis la carte 659-C (§ 12.1 de E22).

Le mode part dans le champ `entry_mode` du POST de la phase 1. Absent, il vaut
« Je saisis tout » (décision 2) ; inconnu, la requête est refusée en 400 avant
le moindre effet.

« Chacun son équipe » (`TeamEntry`, carte 652, épic E22) se démarre depuis la
carte (659-C) ; les fixtures de mode B le démarrent encore par un POST direct.
Les deux coachs du match entrent alors dans l'avant-match ; un tiers reste
dehors. Le membre simple de la suite (`X-Bypass-Auth-Profile: simple`) coache la
deuxième équipe que monte `build_full_competition`, DevCoach la première (cf.
`test_match_report_access.py`).

Deux équipes d'un même coach ne se voient pas offrir le mode B (§ 12.9) : les
fixtures donnent un coach distinct à chaque équipe, ce masquage est donc couvert
par les tests unitaires de `match_selection_controller.rs`. Hors calendrier, le
refus se vérifie ici en postant deux fois la même équipe — un seul coach.
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
    return build_full_competition(browser, space_id, 2, 3)


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

    # Carte 659-C : l'option s'ouvre, avec sa valeur et sans « bientôt ».
    equipe = carte.locator(".entry-mode-option--team")
    expect(equipe).to_contain_text("Chacun son équipe")
    expect(carte).not_to_contain_text("bientôt")
    choix = equipe.locator("input[name='entry_mode'][value='TeamEntry']")
    expect(choix).to_be_enabled()
    expect(choix).not_to_be_checked()


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


def test_commencer_en_chacun_son_equipe(page: Page, space_id, competition):
    """Carte 659-C : « Qui saisit ? » propose « Chacun son équipe » pour deux
    coachs distincts, et le match en sort démarré en mode B."""
    mr_id = draft_of_round(competition["round_ids"][2])
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}", wait_until="load")

    carte = page.locator(".entry-mode-card")
    carte.locator(".entry-mode-option--team").click()
    expect(carte.locator("input[name='entry_mode'][value='TeamEntry']")).to_be_checked()
    with page.expect_request(
        lambda r: r.method == "POST" and r.url.endswith(f"/match-report/{mr_id}")
    ) as envoyee:
        page.get_by_role("button", name="Commencer").click()
    assert "entry_mode=TeamEntry" in (envoyee.value.post_data or "")

    attendre_que(
        lambda: entry_mode_of(mr_id)[0] != "Draft",
        quoi="le démarrage du match",
    )
    _, mode, demarreur = entry_mode_of(mr_id)
    assert mode == "TeamEntry"
    assert demarreur, "le démarreur doit être enregistré"
    page.wait_for_url(f"**/{mr_id}/step2**", timeout=10000)


def test_hors_calendrier_le_refus_du_mode_b_se_lit(space_id, competition):
    """Hors calendrier, les équipes se choisissent dans le formulaire : un seul
    coach pour les deux camps — ici, la même équipe deux fois — fait réafficher
    le formulaire avec un message lisible, et non un 400 nu. Rien n'est
    programmé : le refus précède tout effet."""
    equipe = competition["team_ids"][0]
    avant = query_db("SELECT COUNT(*) FROM match_report_proj")[0]
    reponse = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/new",
        data={
            "competition_id": competition["competition_id"],
            "season_id": competition["season_id"],
            "round_id": competition["round_ids"][0],
            "home_team_id": equipe,
            "away_team_id": equipe,
            "entry_mode": "TeamEntry",
        },
        allow_redirects=False,
        timeout=30,
    )

    assert reponse.status_code == 422, f"obtenu {reponse.status_code}"
    assert "Les deux équipes ont le même coach" in reponse.text
    assert "chacun son équipe n" in reponse.text
    assert "entry-mode-card" in reponse.text, "le formulaire doit être réaffiché"
    assert query_db("SELECT COUNT(*) FROM match_report_proj")[0] == avant


def test_chacun_son_equipe_ouvre_l_avant_match_aux_deux_coachs(chacun_son_equipe):
    """En « Je saisis tout », l'autre coach serait renvoyé au direct (303) ;
    en « Chacun son équipe », il entre dans l'avant-match comme le démarreur
    — chacun dans celui de son camp (carte 654) : `/step2` le mène à
    `/step2/{side}`."""
    phase, mode, _ = entry_mode_of(chacun_son_equipe["adverses"])
    assert (phase, mode) == ("PreMatch", "TeamEntry")

    for qui, camp in (({}, "home"), (PROFIL_SIMPLE, "away")):
        reponse = _get(chacun_son_equipe, "adverses", "/step2", headers=qui)
        assert reponse.status_code == 303, f"{qui} : obtenu {reponse.status_code}"
        assert reponse.headers["Location"].endswith(f"/step2/{camp}")
        page = _get(chacun_son_equipe, "adverses", f"/step2/{camp}", headers=qui)
        assert page.status_code == 200, f"{qui} : obtenu {page.status_code}"

    arrivee = _get(chacun_son_equipe, "adverses", "", headers=PROFIL_SIMPLE)
    assert arrivee.status_code == 303, f"obtenu {arrivee.status_code}"
    assert arrivee.headers["Location"].endswith("/step2")


def test_chacun_son_equipe_reste_ferme_a_un_tiers(chacun_son_equipe):
    reponse = _get(chacun_son_equipe, "etranger", "/step2", headers=PROFIL_SIMPLE)
    assert reponse.status_code == 403, f"obtenu {reponse.status_code}"

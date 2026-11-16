"""Les gains, les fans et le paragraphe de son camp (carte 658, épic E22).

En « Chacun son équipe », chaque coach enregistre l'après-match de son équipe ;
le domicile y ajoute le titre. Le second ferme l'après-match, et le
récapitulatif montre le titre suivi des deux paragraphes.

Scénarios :
- P1 — Deux navigateurs : le membre simple, coach du visiteur, enregistre ses
       gains, ses fans et son paragraphe ; la page de DevCoach, sur
       l'après-match du domicile, les voit arriver sans rechargement et garde
       ce qu'il était en train d'écrire. DevCoach enregistre à son tour, avec
       le titre : le récapitulatif montre le titre et les deux paragraphes.
- P2 — `/step5` mène le membre à l'après-match de son camp ; un POST dans
       `/step5/home` lui est refusé (403).
- P3 — Un rapport « Je saisis tout » sans titre montre son corps au
       récapitulatif (il le masquait).

Le mode B se démarre par un POST direct (`entry_mode=TeamEntry`) : l'option
reste grisée jusqu'à la carte 659. DevCoach coache le domicile, le membre
simple (`X-Bypass-Auth-Profile: simple`) l'extérieur.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

import pytest
import requests
from playwright.sync_api import expect

from competition_lifecycle import BASE_URL, build_full_competition
from match_report_helpers import (
    PROFIL_SIMPLE,
    create_draft,
    ensure_inducements,
    ensure_mvps,
    ensure_pre_match,
    ensure_team_entry_inducements,
    post_step5,
)
from realtime_helpers import DIRECT_TIMEOUT_MS, goto_declared

WITNESS = "() => { window.__sansRechargement = true; }"
STILL_THERE = "() => window.__sansRechargement === true"
OPPONENT = ".team-step5-opponent .opponent-post-match-row"
DRAFT = "Brouillon du domicile, pas encore enregistré."


@pytest.fixture(scope="module")
def competition(browser, space_id):
    return build_full_competition(browser, space_id, num_teams=2, num_rounds=3)


def _membre():
    session = requests.Session()
    session.headers.update(PROFIL_SIMPLE)
    return session


def _url(space_id, mr_id, suffix):
    return f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{suffix}"


def _team_entry_with_mvps(space_id, ctx, round_index):
    """Un match en « Chacun son équipe », coup d'envoi donné, chaque camp
    avec son MVP."""
    domicile, exterieur = ctx["team_ids"][0], ctx["team_ids"][1]
    mr_id = create_draft(
        space_id, ctx, ctx["round_ids"][round_index], domicile, exterieur, entry_mode="TeamEntry"
    )
    ensure_team_entry_inducements(space_id, mr_id)
    ensure_mvps(space_id, mr_id)
    return mr_id


def _fill(page, gain, paragraph):
    page.locator("input[name=gain]").fill(str(gain))
    page.locator("textarea[name=summary_paragraph]").fill(paragraph)


def test_chacun_son_apres_match_et_le_recapitulatif_a_deux(browser, space_id, competition):
    mr_id = _team_entry_with_mvps(space_id, competition, 0)

    devcoach = browser.new_context().new_page()
    goto_declared(devcoach, _url(space_id, mr_id, "step5/home"))
    expect(devcoach.locator(OPPONENT)).to_contain_text("en attente")
    devcoach.locator("textarea[name=summary_paragraph]").fill(DRAFT)
    devcoach.evaluate(WITNESS)

    membre = browser.new_context(extra_http_headers=PROFIL_SIMPLE).new_page()
    membre.goto(_url(space_id, mr_id, "step5/away"), wait_until="load")
    expect(membre.locator("input[name=summary_title]")).to_have_count(0)
    _fill(membre, 45, "Les Rats ont tenu jusqu'au bout.")
    membre.locator(".mr-fan-mod-btns").get_by_text("+1", exact=True).click()
    with membre.expect_navigation():
        membre.get_by_role("button", name="Enregistrer").click()
    assert membre.url.endswith(f"/{mr_id}/step5/away"), membre.url

    expect(devcoach.locator(OPPONENT)).to_contain_text("45 kPo", timeout=DIRECT_TIMEOUT_MS)
    expect(devcoach.locator(OPPONENT)).to_contain_text("fans +1")
    expect(devcoach.locator("textarea[name=summary_paragraph]")).to_have_value(DRAFT)
    assert devcoach.evaluate(STILL_THERE), "la page de DevCoach s'est rechargée"

    devcoach.locator("input[name=summary_title]").fill("Les Nains l'emportent")
    _fill(devcoach, 60, "Les Nains ont dominé la première mi-temps.")
    with devcoach.expect_navigation():
        devcoach.get_by_role("button", name="Enregistrer").click()
    assert devcoach.url.endswith(f"/{mr_id}/recap"), devcoach.url
    article = devcoach.locator(".ms-card", has_text="Compte-rendu")
    expect(article).to_contain_text("Les Nains l'emportent")
    expect(article).to_contain_text("Les Nains ont dominé la première mi-temps.")
    expect(article).to_contain_text("Les Rats ont tenu jusqu'au bout.")


def test_le_coach_est_mene_a_son_camp_et_refuse_ailleurs(space_id, competition):
    mr_id = _team_entry_with_mvps(space_id, competition, 1)
    membre = _membre()

    arrivee = membre.get(_url(space_id, mr_id, "step5"), allow_redirects=False)
    assert arrivee.status_code in (302, 303), f"obtenu {arrivee.status_code}"
    assert arrivee.headers["Location"].endswith(f"/{mr_id}/step5/away"), arrivee.headers["Location"]

    chez_le_domicile = membre.post(
        _url(space_id, mr_id, "step5/home"),
        data={"gain": "50", "fan_mod": "0", "summary_paragraph": "Intrusion"},
        allow_redirects=False,
    )
    assert chez_le_domicile.status_code == 403, f"obtenu {chez_le_domicile.status_code}"


def test_un_corps_sans_titre_se_lit_au_recapitulatif(page, space_id, competition):
    ctx = competition
    round_id = ctx["round_ids"][2]
    domicile, exterieur = ctx["team_ids"][0], ctx["team_ids"][1]
    mr_id = create_draft(space_id, ctx, round_id, domicile, exterieur)
    ensure_pre_match(space_id, mr_id, ctx, round_id, domicile, exterieur)
    ensure_inducements(space_id, mr_id)
    post_step5(space_id, mr_id, summary_title="", summary_body="Un corps sans titre.")

    page.goto(_url(space_id, mr_id, "recap"), wait_until="load")
    expect(page.locator(".ms-card", has_text="Compte-rendu")).to_contain_text("Un corps sans titre.")

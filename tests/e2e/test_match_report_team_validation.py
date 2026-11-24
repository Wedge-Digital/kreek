"""Valider son camp, et retirer sa validation (cartes 659-A et 659-B, épic E22).

En « Chacun son équipe », chaque coach valide son équipe dès que son camp a
son MVP et son après-match ; la seconde validation publie. Un camp validé ne
s'écrit plus, tant que sa validation n'est pas retirée.

Scénarios :
- V1 — Deux navigateurs : le membre simple, coach du visiteur, enregistre son
       après-match et valide depuis l'écran d'attente, qui dit « Validé ✓ » ;
       une écriture directe dans son camp rend alors 409. DevCoach enregistre
       à son tour : le récapitulatif dit « Validé ✓ par … » pour le visiteur
       et offre « … et publier » ; sa validation publie.
- V2 — Un coach qui poste au récapitulatif le `team_side` de l'autre valide
       le sien : le domicile reste « À valider », et c'est à lui que
       revient « … et publier ».
- V3 — Le membre simple enregistre son après-match, valide depuis l'écran
       d'attente, retire sa validation, modifie son après-match (gain et fan)
       et revalide. Le coach adverse ne retire pas sa validation : 403, par
       l'écran d'attente comme par le récapitulatif. DevCoach enregistre, et
       sa validation publie.

Le mode B se démarre par un POST direct (`entry_mode=TeamEntry`) : l'option
reste grisée jusqu'à la carte 659-C. DevCoach coache le domicile, le membre
simple (`X-Bypass-Auth-Profile: simple`) l'extérieur.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

import re

import pytest
import requests
from playwright.sync_api import expect

from competition_lifecycle import BASE_URL, build_full_competition
from match_report_helpers import (
    PROFIL_SIMPLE,
    create_draft,
    ensure_mvps,
    ensure_team_entry_inducements,
)

VALIDATE = re.compile(r"^Valider les ")
PUBLISHES = re.compile(r"et publier")


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


def _post_match(session, space_id, mr_id, side, gain):
    return session.post(
        _url(space_id, mr_id, f"step5/{side}"),
        data={"gain": str(gain), "fan_mod": "0", "summary_paragraph": f"Le camp {side}."},
        allow_redirects=False,
    )


def _record(page, gain):
    """Enregistre l'après-match de son camp, avec un paragraphe : sans lui, le
    compte-rendu, et donc « Publié par », ne s'affiche pas."""
    page.locator("input[name=gain]").fill(str(gain))
    page.locator("textarea[name=summary_paragraph]").fill(f"Un paragraphe à {gain} kPo.")
    with page.expect_navigation():
        page.get_by_role("button", name="Enregistrer").click()


def test_chacun_valide_et_la_seconde_validation_publie(browser, space_id, competition):
    mr_id = _team_entry_with_mvps(space_id, competition, 0)

    membre = browser.new_context(extra_http_headers=PROFIL_SIMPLE).new_page()
    membre.goto(_url(space_id, mr_id, "step5/away"), wait_until="load")
    _record(membre, 45)
    assert membre.url.endswith(f"/{mr_id}/step5/away"), membre.url
    with membre.expect_navigation():
        membre.get_by_role("button", name=VALIDATE).click()
    assert membre.url.endswith(f"/{mr_id}/step5/away"), membre.url
    expect(membre.locator(".team-step5-validated")).to_contain_text("Validé ✓")
    expect(membre.get_by_role("button", name="Enregistrer")).to_have_count(0)

    fige = _post_match(_membre(), space_id, mr_id, "away", 80)
    assert fige.status_code == 409, f"obtenu {fige.status_code}"

    devcoach = browser.new_context().new_page()
    devcoach.goto(_url(space_id, mr_id, "step5/home"), wait_until="load")
    _record(devcoach, 60)
    assert devcoach.url.endswith(f"/{mr_id}/recap"), devcoach.url
    expect(devcoach.locator(".ms-validation-row").nth(1)).to_contain_text("Validé ✓ par")
    expect(devcoach.locator(".ms-validation-row").nth(0)).to_contain_text("À valider")
    publier = devcoach.locator(".ms-btn-primary", has_text=PUBLISHES)
    expect(publier).to_have_count(1)
    with devcoach.expect_navigation():
        publier.click()
    assert devcoach.url.endswith(f"/{mr_id}/recap"), devcoach.url
    byline = devcoach.locator(".ms-card-header-byline")
    expect(byline).to_contain_text("Publié par")
    expect(byline.locator("strong")).to_have_text("DevCoach")
    expect(devcoach.locator(".ms-validation-row")).to_have_count(0)


def test_un_coach_valide_le_sien_quoi_que_dise_le_formulaire(page, space_id, competition):
    mr_id = _team_entry_with_mvps(space_id, competition, 1)
    membre = _membre()
    assert _post_match(membre, space_id, mr_id, "away", 40).status_code in (302, 303)
    assert _post_match(requests.Session(), space_id, mr_id, "home", 50).status_code in (302, 303)

    valide = membre.post(
        _url(space_id, mr_id, "recap/publish"), data={"team_side": "home"}, allow_redirects=False
    )
    assert valide.status_code in (302, 303), f"obtenu {valide.status_code}"

    page.goto(_url(space_id, mr_id, "recap"), wait_until="load")
    expect(page.locator(".ms-validation-row").nth(0)).to_contain_text("À valider")
    expect(page.locator(".ms-validation-row").nth(1)).to_contain_text("Validé ✓")
    expect(page.locator(".ms-btn-primary", has_text=PUBLISHES)).to_have_count(1)


def _withdraw_as_membre(space_id, mr_id, suffix, data=None):
    return _membre().post(_url(space_id, mr_id, suffix), data=data or {}, allow_redirects=False)


def _validate_from_waiting_screen(page, mr_id):
    with page.expect_navigation():
        page.get_by_role("button", name=VALIDATE).click()
    assert page.url.endswith(f"/{mr_id}/step5/away"), page.url
    expect(page.locator(".team-step5-validated")).to_contain_text("Validé ✓")


def test_retirer_sa_validation_corriger_et_revalider(browser, space_id, competition):
    mr_id = _team_entry_with_mvps(space_id, competition, 2)

    membre = browser.new_context(extra_http_headers=PROFIL_SIMPLE).new_page()
    membre.goto(_url(space_id, mr_id, "step5/away"), wait_until="load")
    _record(membre, 45)
    _validate_from_waiting_screen(membre, mr_id)

    with membre.expect_navigation():
        membre.get_by_role("button", name="Retirer ma validation").click()
    assert membre.url.endswith(f"/{mr_id}/step5/away"), membre.url
    expect(membre.locator(".team-step5-validated")).to_have_count(0)
    expect(membre.get_by_role("button", name=VALIDATE)).to_have_count(1)

    membre.locator(".mr-fan-mod-btn", has_text="+1").click()
    _record(membre, 55)
    expect(membre.locator("input[name=gain]")).to_have_value("55")
    expect(membre.locator("input[name=fan_mod]")).to_have_value("1")
    _validate_from_waiting_screen(membre, mr_id)

    ecran = _withdraw_as_membre(space_id, mr_id, "step5/home/withdraw-validation")
    assert ecran.status_code == 403, f"obtenu {ecran.status_code}"

    devcoach = browser.new_context().new_page()
    devcoach.goto(_url(space_id, mr_id, "step5/home"), wait_until="load")
    _record(devcoach, 60)
    assert devcoach.url.endswith(f"/{mr_id}/recap"), devcoach.url

    recap = _withdraw_as_membre(
        space_id, mr_id, "recap/withdraw-validation", data={"team_side": "home"}
    )
    assert recap.status_code == 403, f"obtenu {recap.status_code}"

    devcoach.reload(wait_until="load")
    expect(devcoach.locator(".ms-validation-row").nth(1)).to_contain_text("Validé ✓ par")
    publier = devcoach.locator(".ms-btn-primary", has_text=PUBLISHES)
    with devcoach.expect_navigation():
        publier.click()
    expect(devcoach.locator(".ms-card-header-byline")).to_contain_text("Publié par")
    expect(devcoach.locator(".ms-validation-row")).to_have_count(0)

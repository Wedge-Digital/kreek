"""Valider son camp, retirer sa validation, le récapitulatif en direct, et
corriger à deux (cartes 659-A, 659-B, 659-C et 660, épic E22).

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
- V4 — Le direct (659-C), deux navigateurs : le membre simple enregistre et
       valide depuis l'écran d'attente ; DevCoach enregistre et arrive sur le
       récapitulatif ; l'écran d'attente du membre bascule seul vers le
       récapitulatif. Le membre retire sa validation puis revalide : la page
       de DevCoach, sans rechargement, voit « À valider », puis « Validé ✓ »
       et « … et publier », et reçoit le toast « Les … sont validés par … ».
       La validation de DevCoach publie ; la page du membre se recharge seule
       et montre le rapport publié.
- V5 — Corriger à deux (660) : le rapport publié par les deux validations,
       le membre simple le dépublie par « Corriger le rapport ». DevCoach
       recharge — pas de direct pour la dépublication — et lit « Dépublié par
       … — à revalider par les deux camps », les deux camps « À valider ». Le
       membre corrige son après-match (gain de 35 à 70) et revalide ; la
       validation de DevCoach republie (« Publié par »). Le classement perd
       puis retrouve ses deux lignes, et la trésorerie du visiteur reçoit le
       gain corrigé.

Le mode B se démarre ici par un POST direct (`entry_mode=TeamEntry`) ; la
carte « Qui saisit ? » est couverte par `test_match_report_entry_mode.py`.
DevCoach coache le domicile, le membre simple
(`X-Bypass-Auth-Profile: simple`) l'extérieur.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

import re

import pytest
import requests
from playwright.sync_api import expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import attendre_que, query_db
from match_report_helpers import (
    PROFIL_SIMPLE,
    create_draft,
    ensure_mvps,
    ensure_team_entry_inducements,
)
from realtime_helpers import (
    DIRECT_TIMEOUT_MS,
    goto_declared,
    presence_declared,
    recording_context,
    toasts_of_match,
    wait_for_match_toasts,
)

VALIDATE = re.compile(r"^Valider les ")
PUBLISHES = re.compile(r"et publier")
VALIDATED_TOAST = re.compile(r"^Les .+ sont validés par ")
WITNESS = "() => { window.__sansRechargement = true; }"
STILL_THERE = "() => window.__sansRechargement === true"


@pytest.fixture(scope="module")
def competition(browser, space_id):
    return build_full_competition(browser, space_id, num_teams=2, num_rounds=5)


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


# ── V4 — le récapitulatif en direct (carte 659-C) ─────────────────────────────

def _rows(page):
    return page.locator(".ms-validation-row")


def _declared(page, action):
    """Une action qui recharge la page, puis la présence déclarée de nouveau :
    sans elle, l'onglet ne recevrait plus le signal du match."""
    with page.expect_response(presence_declared):
        action()


def _click_and_reload(page, button):
    with page.expect_navigation():
        button.click()


def test_le_recapitulatif_en_direct(browser, space_id, competition):
    mr_id = _team_entry_with_mvps(space_id, competition, 3)

    membre = recording_context(browser, PROFIL_SIMPLE).new_page()
    goto_declared(membre, _url(space_id, mr_id, "step5/away"))
    _declared(membre, lambda: _record(membre, 45))
    valider = membre.get_by_role("button", name=VALIDATE)
    _declared(membre, lambda: _click_and_reload(membre, valider))
    expect(membre.locator(".team-step5-validated")).to_contain_text("Validé ✓")

    # DevCoach ferme l'après-match : l'écran d'attente du membre bascule seul.
    devcoach = recording_context(browser).new_page()
    goto_declared(devcoach, _url(space_id, mr_id, "step5/home"))
    with membre.expect_response(presence_declared, timeout=DIRECT_TIMEOUT_MS):
        _declared(devcoach, lambda: _record(devcoach, 60))
    assert devcoach.url.endswith(f"/{mr_id}/recap"), devcoach.url
    assert membre.url.endswith(f"/{mr_id}/recap"), membre.url
    expect(_rows(membre).nth(1)).to_contain_text("Validé ✓ par")
    expect(_rows(devcoach).nth(1)).to_contain_text("Validé ✓ par")
    devcoach.evaluate(WITNESS)

    # Le retrait arrive chez DevCoach, sans toast ni rechargement.
    retirer = membre.get_by_role("button", name="Retirer ma validation")
    _declared(membre, lambda: _click_and_reload(membre, retirer))
    expect(_rows(devcoach).nth(1)).to_contain_text("À valider", timeout=DIRECT_TIMEOUT_MS)
    expect(devcoach.locator(".ms-btn-primary", has_text=PUBLISHES)).to_have_count(0)
    # DevCoach est admin de l'espace : un bouton par camp non validé (§ 12.6).
    domicile = _rows(devcoach).nth(0).locator(".ms-validation-team").inner_text()
    valider_domicile = devcoach.get_by_role("button", name=f"Valider les {domicile}")
    expect(valider_domicile).to_have_count(1)

    # La validation aussi, avec son toast.
    revalider = membre.get_by_role("button", name=VALIDATE)
    _declared(membre, lambda: _click_and_reload(membre, revalider))
    expect(_rows(devcoach).nth(1)).to_contain_text("Validé ✓ par", timeout=DIRECT_TIMEOUT_MS)
    publier = devcoach.locator(".ms-btn-primary", has_text=PUBLISHES)
    expect(publier).to_have_count(1)
    wait_for_match_toasts(devcoach, mr_id, 1, timeout_ms=DIRECT_TIMEOUT_MS)
    textes = [t["text"] for t in toasts_of_match(devcoach, mr_id)]
    assert any(VALIDATED_TOAST.search(t.strip()) for t in textes), textes
    assert devcoach.evaluate(STILL_THERE), "la page de DevCoach s'est rechargée"
    assert toasts_of_match(membre, mr_id) == [], "l'auteur ne reçoit pas son toast"

    # La validation de DevCoach publie ; la page du membre se recharge seule.
    with membre.expect_navigation(timeout=DIRECT_TIMEOUT_MS):
        with devcoach.expect_navigation():
            publier.click()
    for page in (devcoach, membre):
        assert page.url.endswith(f"/{mr_id}/recap"), page.url
        expect(page.locator(".ms-card-header-byline")).to_contain_text("Publié par")
        expect(_rows(page)).to_have_count(0)


# ── V5 — corriger un compte rendu à deux (carte 660) ──────────────────────────

UNPUBLISHED_BY = re.compile(r"Dépublié par .+ — à revalider par les deux camps")


def _ranking_lines(mr_id):
    rows = query_db(f"SELECT count(*) FROM ranking_lines WHERE match_report_id = '{mr_id}'")
    return int(rows[0]) if rows else 0


def _team_phase(team_id):
    rows = query_db(f"SELECT game_phase FROM team_proj WHERE team_id = '{team_id}'")
    return rows[0] if rows else None


def _last_income(team_id):
    """Le gain du dernier match crédité à l'équipe : la trésorerie ne vit que
    dans l'event store de `teams`."""
    rows = query_db(
        "SELECT payload FROM team_event_store "
        f"WHERE team_id = '{team_id}' AND payload::text LIKE '%PostMatchSequenceStarted%' "
        "ORDER BY version DESC LIMIT 1"
    )
    found = re.search(r'"treasury_income":\s*(\d+)', rows[0]) if rows else None
    return int(found.group(1)) if found else None


def _published_by_both(browser, space_id, mr_id):
    """Chacun son après-match et sa validation ; celle de DevCoach publie."""
    membre = _membre()
    assert _post_match(membre, space_id, mr_id, "away", 35).status_code in (302, 303)
    assert _post_match(requests.Session(), space_id, mr_id, "home", 60).status_code in (302, 303)
    valide = membre.post(_url(space_id, mr_id, "recap/publish"), allow_redirects=False)
    assert valide.status_code in (302, 303), f"obtenu {valide.status_code}"
    devcoach = browser.new_context().new_page()
    devcoach.goto(_url(space_id, mr_id, "recap"), wait_until="load")
    with devcoach.expect_navigation():
        devcoach.locator(".ms-btn-primary", has_text=PUBLISHES).click()
    expect(devcoach.locator(".ms-card-header-byline")).to_contain_text("Publié par")
    return devcoach


def _wait_correctable(mr_id, team_ids):
    """La correction n'est permise que les deux équipes en amélioration : leur
    passage vient d'un app event, après la publication. Le gain du visiteur
    (35, qu'aucun scénario précédent n'a crédité) dit que `teams` a reçu
    celle-ci, et non la précédente."""
    attendre_que(lambda: _ranking_lines(mr_id) == 2, quoi="le classement compte le match")
    attendre_que(lambda: _last_income(team_ids[1]) == 35, quoi="le gain du visiteur crédité")
    for team_id in team_ids:
        attendre_que(
            lambda t=team_id: _team_phase(t) == "PlayerImprovement",
            quoi=f"l'équipe {team_id} en amélioration",
        )


def test_corriger_un_compte_rendu_a_deux(browser, space_id, competition):
    mr_id = _team_entry_with_mvps(space_id, competition, 4)
    exterieur = competition["team_ids"][1]
    devcoach = _published_by_both(browser, space_id, mr_id)
    _wait_correctable(mr_id, competition["team_ids"][:2])

    # Le visiteur dépublie, par le bouton du récapitulatif publié.
    membre = browser.new_context(extra_http_headers=PROFIL_SIMPLE).new_page()
    membre.goto(_url(space_id, mr_id, "recap"), wait_until="load")
    corriger = membre.locator(".ms-correct-btn")
    expect(corriger).to_be_enabled()
    membre.once("dialog", lambda dialog: dialog.accept())
    with membre.expect_navigation():
        corriger.click()
    expect(membre.locator(".ms-unpublished-banner")).to_be_visible()
    attendre_que(lambda: _ranking_lines(mr_id) == 0, quoi="le classement compensé")

    # Le domicile recharge : qui a dépublié, et les deux camps à revalider.
    devcoach.reload(wait_until="load")
    bandeau = devcoach.locator(".ms-unpublished-banner")
    expect(bandeau).to_contain_text(UNPUBLISHED_BY)
    expect(bandeau).not_to_contain_text("DevCoach")
    expect(_rows(devcoach)).to_have_count(2)
    expect(_rows(devcoach).nth(0)).to_contain_text("À valider")
    expect(_rows(devcoach).nth(1)).to_contain_text("À valider")

    # Le visiteur corrige son après-match, puis revalide depuis le récapitulatif.
    membre.goto(_url(space_id, mr_id, "step5/away"), wait_until="load")
    _record(membre, 70)
    assert membre.url.endswith(f"/{mr_id}/recap"), membre.url
    # Corrigé, le rapport reste en correction : le bandeau ne tombe pas.
    devcoach.reload(wait_until="load")
    expect(devcoach.locator(".ms-unpublished-banner")).to_contain_text(UNPUBLISHED_BY)
    with membre.expect_navigation():
        membre.get_by_role("button", name=VALIDATE).click()
    expect(_rows(membre).nth(1)).to_contain_text("Validé ✓ par")

    # La validation du domicile republie.
    devcoach.reload(wait_until="load")
    expect(_rows(devcoach).nth(1)).to_contain_text("Validé ✓ par")
    publier = devcoach.locator(".ms-btn-primary", has_text=PUBLISHES)
    with devcoach.expect_navigation():
        publier.click()
    expect(devcoach.locator(".ms-card-header-byline")).to_contain_text("Publié par")
    expect(devcoach.locator(".ms-unpublished-banner")).to_have_count(0)
    expect(_rows(devcoach)).to_have_count(0)

    # Le classement suit, et la trésorerie reçoit le gain corrigé.
    attendre_que(lambda: _ranking_lines(mr_id) == 2, quoi="le classement rejoué")
    attendre_que(lambda: _last_income(exterieur) == 70, quoi="le gain corrigé du visiteur")

"""Tests E2E de l'affichage des règles spéciales de roster et de leur
caractère obligatoire à la finalisation d'une équipe.

Un roster sans FAVOURED_OF_CHOOSE_* (ex. Orc, Skaven) affiche ses règles
fixes en lecture seule (chips) ou "Pas de règle spéciale". Un roster à choix
(ex. Chaos Renégats) garde son <select> interactif, et la finalisation de
l'équipe est bloquée tant qu'aucune règle n'a été choisie.

La sélection du roster ne passe pas par un clic UI sur le widget TomSelect
(cf. test_build_and_finalize_team.py, skip WIP : ce câblage précis est connu
pour être flaky en Playwright headless) mais déclenche directement
l'événement DOM `rosterSelected` que TomSelect émettrait normalement — ce
qui exerce le même rendu HTMX/Askama réel côté serveur, sans dépendre du
timing d'attachement du listener TomSelect.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import re
import time

from playwright.sync_api import Page, expect

BASE_URL = "http://localhost:3210"
FAKE_LOGO_URL = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg"


def _create_minimal_competition(page: Page, competition_create_url: str) -> str:
    """Crée une compétition avec un tier à 0 SPP (finalisation immédiate).

    Retourne le nom de la compétition, unique par exécution — nécessaire pour
    la retrouver ensuite dans le kreek-select de /team/create : la base de
    test accumule les compétitions des runs précédents, et ce <kreek-select>
    les liste par ordre alphabétique de nom (pas par date de création), donc
    "la première option" n'est pas fiablement la nôtre.
    """
    competition_name = f"SpecialRule E2E {time.time_ns()}"
    page.goto(competition_create_url, wait_until="load")
    page.fill("input[name='name']", competition_name)
    page.wait_for_selector(".coach-result-row", timeout=5000)
    page.locator(".coach-result-row").first.click()
    page.evaluate(f"document.getElementById('logo_url').value = '{FAKE_LOGO_URL}'")
    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    page.wait_for_selector(".tier-block", timeout=5000)
    xp_input = page.locator(".tier-block").first.locator("input.tier-xp")
    xp_input.fill("0")
    xp_input.dispatch_event("change")
    page.fill("#season_name", f"Saison SpecialRule E2E {time.time_ns()}")
    page.click("button[onclick='submitRules()']")
    page.wait_for_selector("#groups-config", timeout=10000)
    return competition_name


def _create_draft_team(page: Page, space_id: str, competition_name: str) -> str:
    """Crée un draft d'équipe et retourne le team_id (atterrit sur /build)."""
    draft_url = f"{BASE_URL}/app/{space_id}/team/create"
    page.goto(draft_url, wait_until="load")
    page.fill("input[name='team_name']", f"Team SpecialRule E2E {time.time_ns()}")

    page.wait_for_selector(".coach-select .ts-control", timeout=5000)
    page.locator(".coach-select .ts-control").click()
    page.wait_for_selector(".ts-dropdown .option", timeout=3000)
    page.locator(".ts-dropdown .option").first.click()
    page.wait_for_timeout(300)

    # competition_id est un <kreek-select> (input caché + .ks-control/.ks-option,
    # pas un <select> natif) — season_id se sélectionne automatiquement ensuite
    # via son attribut auto-select-first une fois la compétition choisie.
    comp_select = page.locator("kreek-select[name='competition_id']")
    comp_select.wait_for(timeout=5000)
    comp_select.locator(".ks-control").click()
    comp_option = comp_select.locator(".ks-option", has_text=competition_name)
    comp_option.wait_for(timeout=5000)
    comp_option.first.click()

    season_hidden = page.locator("kreek-select[name='season_id'] input[type='hidden']")
    expect(season_hidden).not_to_have_value("", timeout=5000)

    page.click("button[type='submit']")
    page.wait_for_url(re.compile(r".*/build$"), timeout=10000)

    match = re.search(r"/team/([0-9A-Za-z]+)/build", page.url)
    assert match, f"team_id introuvable dans l'URL {page.url}"
    return match.group(1)


def _select_roster(page: Page, uid: str, name: str) -> None:
    """Déclenche `rosterSelected` directement, sans passer par TomSelect."""
    page.evaluate(
        "([uid, name]) => htmx.trigger(document.body, 'rosterSelected', {uid, name})",
        [uid, name],
    )


def _hire_players(page: Page, count: int) -> int:
    page.wait_for_selector("#player-table-container .tbl-btn", timeout=10000)
    hired = 0
    attempts = 0
    while hired < count and attempts < 40:
        buttons = page.locator("#player-table-container .tbl-btn:not([disabled])").all()
        clicked = False
        for btn in buttons:
            if btn.inner_text().strip() == "+":
                btn.click()
                page.wait_for_timeout(200)
                hired += 1
                clicked = True
                break
        if not clicked:
            break
        attempts += 1
    return hired


def test_fixed_rules_display_as_chips(page: Page, space_id, competition_create_url):
    competition_name = _create_minimal_competition(page, competition_create_url)
    team_id = _create_draft_team(page, space_id, competition_name)
    page.goto(f"{BASE_URL}/app/{space_id}/team/{team_id}/build", wait_until="load")

    _select_roster(page, "ORC", "Orques")

    zone = page.locator("#special-rule-selector-zone")
    expect(zone.locator(".special-rule-chip")).to_have_count(2)
    expect(zone.locator(".special-rule-chip").nth(0)).to_have_text("Brutes Bagarreuses")
    expect(zone.locator(".special-rule-chip").nth(1)).to_have_text("Capitaine d'Équipe")
    expect(zone.locator("select")).to_have_count(0)


def test_no_rule_roster_shows_none_message(page: Page, space_id, competition_create_url):
    competition_name = _create_minimal_competition(page, competition_create_url)
    team_id = _create_draft_team(page, space_id, competition_name)
    page.goto(f"{BASE_URL}/app/{space_id}/team/{team_id}/build", wait_until="load")

    _select_roster(page, "SKAVEN", "Skavens")

    zone = page.locator("#special-rule-selector-zone")
    expect(zone.locator(".selector-none")).to_have_text("Pas de règle spéciale")


def test_choice_roster_keeps_interactive_select(page: Page, space_id, competition_create_url):
    competition_name = _create_minimal_competition(page, competition_create_url)
    team_id = _create_draft_team(page, space_id, competition_name)
    page.goto(f"{BASE_URL}/app/{space_id}/team/{team_id}/build", wait_until="load")

    _select_roster(page, "CHAOS_RENEGADE", "Renégats du Chaos")

    zone = page.locator("#special-rule-selector-zone")
    select = zone.locator("select.league-select")
    expect(select).to_be_visible()
    expect(select).to_be_enabled()
    expect(select.locator("option")).to_have_count(6)  # placeholder + 5 dieux


def test_finalize_blocked_when_choice_roster_has_no_special_rule(
    page: Page, space_id, competition_create_url
):
    competition_name = _create_minimal_competition(page, competition_create_url)
    team_id = _create_draft_team(page, space_id, competition_name)
    page.goto(f"{BASE_URL}/app/{space_id}/team/{team_id}/build", wait_until="load")

    _select_roster(page, "CHAOS_RENEGADE", "Renégats du Chaos")
    hired = _hire_players(page, 11)
    assert hired >= 11, f"N'a pu recruter que {hired} joueurs (11 requis)"

    page.click("text=Terminer la construction →")
    page.wait_for_timeout(1000)

    error = page.locator("#submit-errors")
    expect(error).to_contain_text("règle spéciale")
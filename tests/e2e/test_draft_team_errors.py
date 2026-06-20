"""Tests E2E des erreurs de validation du formulaire de création d'équipe.

Vérifie que les erreurs sont affichées avec le bon style quand :
- Le formulaire est soumis sans compétition/saison sélectionnée
- Le formulaire est soumis sans coach sélectionné
- Les erreurs s'affichent avec le style correct (classe table-error)

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

from playwright.sync_api import Page, expect


def _goto_draft(page: Page, space_id: str) -> None:
    url = f"http://localhost:3210/app/{space_id}/team/create"
    page.goto(url, wait_until="load")
    page.wait_for_selector("#draft-team-form", timeout=5000)


def _select_coach(page: Page) -> None:
    page.wait_for_selector(".coach-select .ts-control", timeout=5000)
    page.locator(".coach-select .ts-control").click()
    page.wait_for_selector(".ts-dropdown .option", timeout=3000)
    page.locator(".ts-dropdown .option").first.click()
    page.wait_for_timeout(300)


def _select_competition_and_season(page: Page) -> None:
    comp_select = page.locator("select[name='competition_id']")
    comp_select.wait_for(timeout=5000)
    page.wait_for_timeout(500)
    options = comp_select.locator("option:not([value=''])").all()
    if options:
        comp_select.select_option(options[0].get_attribute("value"))
        page.wait_for_timeout(500)
    season_select = page.locator("select[name='season_id']")
    season_select.wait_for(timeout=5000)
    page.wait_for_timeout(500)
    season_options = season_select.locator("option:not([value=''])").all()
    if season_options:
        season_select.select_option(season_options[0].get_attribute("value"))
        page.wait_for_timeout(300)


def test_submit_without_competition_shows_error(page: Page, space_id):
    _goto_draft(page, space_id)

    page.fill("input[name='team_name']", "Test sans compet")
    _select_coach(page)

    page.click("button[type='submit']")
    page.wait_for_timeout(500)

    error = page.locator("#draft-team-error")
    expect(error).not_to_be_empty()
    expect(error).to_contain_text("champs obligatoires")


def test_submit_without_coach_shows_error(page: Page, space_id):
    _goto_draft(page, space_id)

    page.fill("input[name='team_name']", "Test sans coach")
    _select_competition_and_season(page)

    page.click("button[type='submit']")
    page.wait_for_timeout(500)

    error = page.locator("#draft-team-error")
    expect(error).to_contain_text("coach")


def test_error_has_correct_style(page: Page, space_id):
    _goto_draft(page, space_id)

    page.fill("input[name='team_name']", "Test style erreur")
    _select_coach(page)

    page.click("button[type='submit']")
    page.wait_for_timeout(500)

    error_p = page.locator("#draft-team-error .table-error")
    expect(error_p).to_be_visible()

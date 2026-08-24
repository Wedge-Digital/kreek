"""Tests E2E du widget coach-selector (multi-select) sur la création de
compétition, phase 1.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) — le coach
« DevCoach » est connecté automatiquement et doit être membre du space
E2E_SPACE_ID (par défaut, le space seedé en données de migration).
"""

from playwright.sync_api import Page, expect


def _goto_create_page(page: Page, url: str) -> None:
    page.goto(url, wait_until="load")
    page.wait_for_selector(".coach-result-row", timeout=5000)


def test_widget_loads_search_results(page: Page, competition_create_url):
    _goto_create_page(page, competition_create_url)
    assert page.locator(".coach-result-row").count() > 0


def test_select_coach_adds_badge_and_hidden_input(page: Page, competition_create_url):
    _goto_create_page(page, competition_create_url)
    page.locator(".coach-result-row").first.click()

    expect(page.locator(".coach-selected-badge")).to_have_count(1)
    expect(page.locator("input[name='admin_ids[]']")).to_have_count(1)


def test_selected_coach_excluded_from_results(page: Page, competition_create_url):
    _goto_create_page(page, competition_create_url)
    rows = page.locator(".coach-result-row")
    first_name = rows.first.locator(".invited-name").inner_text()

    rows.first.click()
    page.wait_for_timeout(500)

    remaining_names = page.locator(".coach-result-row .invited-name").all_inner_texts()
    assert first_name not in remaining_names


def test_multi_select_then_remove_restores_count(page: Page, competition_create_url):
    _goto_create_page(page, competition_create_url)

    page.locator(".coach-result-row").first.click()
    page.wait_for_timeout(500)
    page.locator(".coach-result-row").first.click()
    page.wait_for_timeout(500)

    expect(page.locator(".coach-selected-badge")).to_have_count(2)
    expect(page.locator("input[name='admin_ids[]']")).to_have_count(2)

    page.locator(".coach-selected-badge__remove").first.click()
    page.wait_for_timeout(500)

    expect(page.locator(".coach-selected-badge")).to_have_count(1)
    expect(page.locator("input[name='admin_ids[]']")).to_have_count(1)


def test_admin_ids_are_sent_to_server_on_submit(page: Page, competition_create_url):
    _goto_create_page(page, competition_create_url)

    page.locator(".coach-result-row").first.click()
    page.wait_for_timeout(500)

    selected_id = page.locator("input[name='admin_ids[]']").first.input_value()
    page.fill("input[name='name']", "Test E2E coach selector")

    with page.expect_response(
        lambda r: "/competitions/create" in r.url and r.request.method == "POST"
    ) as resp_info:
        page.click("button[type='submit']")
    response = resp_info.value

    assert response.status == 200
    # Le serveur a bien reçu et conservé l'ID — vérifié via le re-render qui
    # réinitialise adminSelector() avec l'ID envoyé. (La validation s'arrête
    # ensuite sur le logo, requis par ailleurs et hors périmètre de ce widget.)
    assert selected_id in response.text()

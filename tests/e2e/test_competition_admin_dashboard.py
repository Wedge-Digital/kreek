"""Tests E2E du dashboard admin compétition.

Crée une compétition via le flow standard (phase 1 → phase 2),
puis navigue vers la page d'administration /admin et vérifie
que le dashboard se charge correctement.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import re
import time

import pytest
from playwright.sync_api import Page, expect

FAKE_LOGO_URL = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg"


def _create_competition_and_get_admin_url(page: Page, competition_create_url: str) -> str:
    """Crée une compétition phase 1 → phase 2, extrait les IDs et retourne l'URL admin."""
    page.goto(competition_create_url, wait_until="load")

    page.fill("input[name='name']", f"Admin E2E {time.time_ns()}")

    page.wait_for_selector(".coach-result-row", timeout=5000)

    # Sélectionner Bagouze (le bypass user) comme admin
    bagouze_row = page.locator(".coach-result-row", has_text="Bagouze")
    if bagouze_row.count() > 0:
        bagouze_row.first.click()
    else:
        page.locator(".coach-result-row").first.click()

    expect(page.locator(".coach-selected-badge")).to_have_count(1)

    page.evaluate(f"document.getElementById('logo_url').value = '{FAKE_LOGO_URL}'")

    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    expect(page).to_have_url(re.compile(r".*/rules$"))

    url = page.url
    match = re.search(r"/app/([^/]+)/competitions/create/([^/]+)/([^/]+)/rules", url)
    assert match, f"Impossible d'extraire les IDs depuis l'URL: {url}"
    space_id = match.group(1)
    competition_id = match.group(2)
    season_id = match.group(3)

    return f"http://localhost:3210/app/{space_id}/competitions/{competition_id}/{season_id}/admin"


def test_admin_page_loads_with_dashboard(page: Page, competition_create_url):
    admin_url = _create_competition_and_get_admin_url(page, competition_create_url)

    page.goto(admin_url, wait_until="load")

    expect(page.locator(".admin-banner")).to_be_visible()
    expect(page.locator(".admin-tabs")).to_be_visible()
    expect(page.locator("#admin-content")).to_be_visible()


def test_admin_dashboard_shows_stats(page: Page, competition_create_url):
    admin_url = _create_competition_and_get_admin_url(page, competition_create_url)

    page.goto(admin_url, wait_until="load")

    stat_chips = page.locator(".stat-chip")
    expect(stat_chips.first).to_be_visible()
    assert stat_chips.count() >= 3


def test_admin_dashboard_shows_empty_activity(page: Page, competition_create_url):
    admin_url = _create_competition_and_get_admin_url(page, competition_create_url)

    page.goto(admin_url, wait_until="load")

    expect(page.locator(".empty-state")).to_be_visible()

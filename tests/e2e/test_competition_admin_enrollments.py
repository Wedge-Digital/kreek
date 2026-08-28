"""Tests E2E de l'onglet Inscriptions de l'admin compétition.

Crée une compétition, navigue vers /admin, clique sur l'onglet
Inscriptions et vérifie que les widgets se chargent.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import re
import time

import pytest
from playwright.sync_api import Page, expect

FAKE_LOGO_URL = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg"


def _create_competition_and_get_admin_url(page: Page, competition_create_url: str) -> str:
    page.goto(competition_create_url, wait_until="load")
    page.fill("input[name='name']", f"Enroll E2E {time.time_ns()}")

    page.wait_for_selector(".coach-result-row", timeout=5000)
    bagouze_row = page.locator(".coach-result-row", has_text="Bagouze")
    if bagouze_row.count() > 0:
        bagouze_row.first.click()
    else:
        page.locator(".coach-result-row").first.click()

    expect(page.locator(".coach-selected-badge")).to_have_count(1)
    page.evaluate(f"document.getElementById('logo_url').value = '{FAKE_LOGO_URL}'")

    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    url = page.url
    match = re.search(r"/app/([^/]+)/competitions/create/([^/]+)/([^/]+)/rules", url)
    assert match, f"Impossible d'extraire les IDs depuis l'URL: {url}"
    space_id = match.group(1)
    competition_id = match.group(2)
    season_id = match.group(3)

    return f"http://localhost:3210/app/{space_id}/competitions/{competition_id}/{season_id}/admin"


def test_ouvrir_l_administration_mene_au_resume(page: Page, competition_create_url):
    """Carte 419 — le Résumé est l'onglet d'accueil.

    Le tableau de bord l'occupait ; il a quitté l'administration, et le défaut de
    l'aiguillage rend désormais le Résumé. Trois cas e2e le vérifiaient sous son
    ancien nom — ils partent avec lui, et celui-ci prend leur place.

    L'assertion porte sur le **contenu servi**, pas seulement sur le fait que la
    page réponde : c'est l'onglet actif et le fragment rendu qui distinguent le
    Résumé de ce qui l'a précédé.
    """
    admin_url = _create_competition_and_get_admin_url(page, competition_create_url)

    page.goto(admin_url, wait_until="load")

    expect(page.locator(".admin-summary")).to_be_visible()
    onglet_actif = page.locator(".admin-tab.active")
    expect(onglet_actif).to_have_count(1)
    expect(onglet_actif).to_contain_text("Résumé")


def test_les_onglets_retires_ne_repondent_plus(page: Page, competition_create_url):
    """Leurs routes ont disparu avec eux — un signet périmé rend `404`, il ne
    retombe pas sur une page à moitié rendue.

    Et la barre d'onglets ne les propose plus : masquer un lien sans retirer la
    route, ou l'inverse, laisserait la moitié du travail faite.
    """
    admin_url = _create_competition_and_get_admin_url(page, competition_create_url)

    for parti in ("dashboard", "results"):
        reponse = page.request.get(f"{admin_url}/{parti}")
        assert reponse.status == 404, f"/{parti} rend {reponse.status}"

    page.goto(admin_url, wait_until="load")
    barre = page.locator(".admin-tabs")
    expect(barre).not_to_contain_text("Tableau de bord")
    expect(barre).not_to_contain_text("Résultats")


def test_enrollments_tab_loads(page: Page, competition_create_url):
    admin_url = _create_competition_and_get_admin_url(page, competition_create_url)

    page.goto(admin_url + "/enrollments", wait_until="load")

    expect(page.locator(".admin-banner")).to_be_visible()
    page.wait_for_selector("#pending-container", timeout=5000)
    expect(page.locator("#pending-container")).to_be_visible()
    expect(page.locator("#enrolled-container")).to_be_visible()


def test_enrollments_shows_empty_states(page: Page, competition_create_url):
    admin_url = _create_competition_and_get_admin_url(page, competition_create_url)

    page.goto(admin_url + "/enrollments", wait_until="load")
    page.wait_for_selector("#pending-container .empty-state", timeout=5000)

    pending_empty = page.locator("#pending-container .empty-state")
    enrolled_empty = page.locator("#enrolled-container .empty-state")
    expect(pending_empty).to_be_visible()
    expect(enrolled_empty).to_be_visible()

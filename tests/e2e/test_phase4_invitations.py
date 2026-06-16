"""Tests E2E de la phase 4 ("Invitations") de la création de compétition.

Contrairement à test_phase2_pickers.py, cette phase ne peut PAS être testée
via un accès direct avec des identifiants fictifs : le bug qui a motivé ces
tests (un déséquilibre de balises `<div>` dans le template, qui cassait le
chargement du `<script>` de la page) ne se manifestait QUE lors de la
navigation SPA réelle (htmx.ajax + pushState) depuis la phase 3 — un accès
direct en navigation complète masquait le problème (le navigateur, plus
tolérant, parsait quand même le script). Chaque test ici traverse donc le
vrai parcours phase 1 → 2 → 3 → 4.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import time

from playwright.sync_api import Page, expect

FAKE_LOGO_URL = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg"


def _reach_phase4(page: Page, competition_create_url: str) -> None:
    competition_name = f"Ligue E2E Invitations {time.time_ns()}"

    page.goto(competition_create_url, wait_until="load")
    page.fill("input[name='name']", competition_name)
    page.wait_for_selector(".coach-result-row", timeout=5000)
    page.locator(".coach-result-row").first.click()
    expect(page.locator(".coach-selected-badge")).to_have_count(1)
    page.evaluate(f"document.getElementById('logo_url').value = '{FAKE_LOGO_URL}'")

    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    page.wait_for_selector(".tier-block [data-slot='star'] .roster-chip", timeout=5000)
    page.fill("#season_name", "Saison E2E 1")
    page.click("button[onclick='submitRules()']")
    page.wait_for_selector("#groups-config", timeout=5000)

    page.click("button[onclick='submitStructure()']")
    page.wait_for_selector("#access-mode-btns", timeout=5000)
    # Le widget coach-search met le plus de temps à charger — l'attendre
    # garantit que le script de la page a bien fini de s'exécuter.
    page.wait_for_selector("#invite-coach-widget .coach-result-row", timeout=5000)


def test_phase4_loads_with_two_access_modes_and_widget(page: Page, competition_create_url):
    _reach_phase4(page, competition_create_url)

    labels = page.locator("#access-mode-btns .choice-btn").all_text_contents()
    assert len(labels) == 2
    assert "Sur invitation (liste fermée)" in labels[0]
    assert "Inscription libre" in labels[1]

    assert page.locator("#validation-mode-btns .choice-btn").count() == 2
    assert page.locator("#invite-coach-widget input[name='q']").count() == 1


def test_invite_section_hidden_when_access_mode_is_open(page: Page, competition_create_url):
    _reach_phase4(page, competition_create_url)

    expect(page.locator("#invite-section")).to_be_visible()

    page.click("#access-mode-btns .choice-btn[data-val='open']")
    expect(page.locator("#invite-section")).to_be_hidden()

    page.click("#access-mode-btns .choice-btn[data-val='invitation']")
    expect(page.locator("#invite-section")).to_be_visible()


def test_selecting_a_coach_updates_count_and_badge(page: Page, competition_create_url):
    _reach_phase4(page, competition_create_url)

    expect(page.locator("#invited-count")).to_have_text("0")

    page.locator("#invite-coach-widget .coach-result-row").first.click()
    page.wait_for_timeout(300)

    expect(page.locator("#invited-count")).to_have_text("1")
    expect(page.locator("#invite-coach-widget .coach-selected-badge")).to_have_count(1)


def test_full_submission_reflects_access_mode_and_validation_in_recap(
    page: Page, competition_create_url
):
    _reach_phase4(page, competition_create_url)

    page.click("#validation-mode-btns .choice-btn[data-val='false']")
    page.locator("#invite-coach-widget .coach-result-row").first.click()
    page.wait_for_timeout(300)

    page.click("button[onclick='submitInvitations()']")
    page.wait_for_selector(".recap-row", timeout=5000)

    recap_text = " ".join(page.locator(".recap-row").all_text_contents())
    assert "Sur invitation (liste fermée)" in recap_text
    assert "Non (acceptation automatique)" in recap_text
    assert "1 coach" in recap_text

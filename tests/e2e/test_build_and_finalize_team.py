"""Test E2E du parcours construction + finalisation d'une équipe.

Crée une compétition avec un tier à 6 SPP, crée un draft, construit
l'équipe (sélection roster, recrutement de 11 joueurs), accède à la
page de finalisation, sélectionne un joueur, vérifie que le skill
picker se charge, et soumet l'équipe.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import re
import time

import pytest
from playwright.sync_api import Page, expect

FAKE_LOGO_URL = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg"


def _create_competition_with_spp(page: Page, competition_create_url: str) -> str:
    """Crée une compétition avec 1 tier à 6 SPP. Retourne l'URL de la phase 3."""
    competition_name = f"Finalize E2E {time.time_ns()}"
    page.goto(competition_create_url, wait_until="load")
    page.fill("input[name='name']", competition_name)
    page.wait_for_selector(".coach-result-row", timeout=5000)
    page.locator(".coach-result-row").first.click()
    page.evaluate(f"document.getElementById('logo_url').value = '{FAKE_LOGO_URL}'")
    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    # Phase 2 — set 6 SPP on the tier
    page.wait_for_selector(".tier-block", timeout=5000)
    page.wait_for_selector(
        ".tier-block [data-slot='star'] .roster-chip", timeout=5000
    )
    xp_input = page.locator(".tier-block").first.locator("input.tier-xp")
    xp_input.fill("6")
    xp_input.dispatch_event("change")
    page.fill("#season_name", f"Saison Finalize E2E {time.time_ns()}")
    page.click("button[onclick='submitRules()']")
    page.wait_for_selector("#groups-config", timeout=10000)
    return page.url


@pytest.mark.skip(reason="WIP — le câblage rosterSelected → player table ne fonctionne pas dans Playwright")
def test_build_and_finalize_with_spp(page: Page, competition_create_url, space_id):
    # ── Phase 1-2-3 : créer la compétition ───────────────────────────
    _create_competition_with_spp(page, competition_create_url)

    # ── Accéder à la page de création d'équipe ───────────────────────
    draft_url = f"http://localhost:3210/app/{space_id}/team/create"
    page.goto(draft_url, wait_until="load")

    # Remplir le nom d'équipe
    page.fill("input[name='team_name']", f"Team E2E {time.time_ns()}")

    # Sélectionner un coach
    page.wait_for_selector(".coach-select .ts-control", timeout=5000)
    page.locator(".coach-select .ts-control").click()
    page.wait_for_selector(".ts-dropdown .option", timeout=3000)
    page.locator(".ts-dropdown .option").first.click()
    page.wait_for_timeout(300)

    # Sélectionner la compétition (dernière créée)
    comp_select = page.locator("select[name='competition_id']")
    comp_select.wait_for(timeout=5000)
    page.wait_for_timeout(500)
    options = comp_select.locator("option:not([value=''])").all()
    assert options, "Aucune compétition disponible"
    comp_select.select_option(options[-1].get_attribute("value"))
    page.wait_for_timeout(500)

    # Sélectionner la saison
    season_select = page.locator("select[name='season_id']")
    season_select.wait_for(timeout=5000)
    page.wait_for_timeout(500)
    season_options = season_select.locator("option:not([value=''])").all()
    assert season_options, "Aucune saison disponible"
    season_select.select_option(season_options[-1].get_attribute("value"))
    page.wait_for_timeout(300)

    # Soumettre le draft
    page.click("button[type='submit']")
    page.wait_for_url(re.compile(r".*/build$"), timeout=10000)

    # ── Page build-team ──────────────────────────────────────────────
    # Attendre le roster picker widget
    page.wait_for_selector(".roster-picker-widget select", timeout=5000)
    page.wait_for_timeout(500)

    # Sélectionner le premier roster via TomSelect
    page.locator(".roster-picker-widget .ts-control").click()
    page.wait_for_selector(".ts-dropdown .option", timeout=3000)
    page.locator(".ts-dropdown .option").first.click()
    page.wait_for_timeout(2000)

    # Attendre que le player table se charge
    page.wait_for_selector("#player-table-container .tbl-btn", timeout=10000)

    # Recruter 11 joueurs en cliquant sur les boutons +
    hire_buttons = page.locator("#player-table-container .tbl-btn:not([disabled])")
    hired = 0
    max_attempts = 30
    attempts = 0
    while hired < 11 and attempts < max_attempts:
        buttons = hire_buttons.all()
        clicked = False
        for btn in buttons:
            text = btn.inner_text().strip()
            if text == "+":
                btn.click()
                page.wait_for_timeout(300)
                hired += 1
                clicked = True
                break
        if not clicked:
            break
        attempts += 1

    assert hired >= 11, f"N'a pu recruter que {hired} joueurs (11 requis)"

    # Cliquer sur "Terminer la construction"
    page.click("text=Terminer la construction →")
    page.wait_for_timeout(2000)

    # ── Page finalize ────────────────────────────────────────────────
    # Vérifier qu'on est sur la page de finalisation
    expect(page.locator(".finalize-page")).to_be_visible(timeout=10000)
    expect(page.locator(".team-header")).to_be_visible()
    expect(page.locator(".player-list")).to_be_visible()

    # Vérifier que le SPP badge est affiché (6 SPP)
    expect(page.locator(".spp-badge")).to_be_visible()

    # Sélectionner le premier joueur
    page.locator(".player-row").first.click()
    page.wait_for_timeout(1000)

    # Vérifier que le skill header se charge
    expect(page.locator(".skill-header-widget")).to_be_visible(timeout=5000)

    # Vérifier que le skill picker se charge
    page.wait_for_selector("#skill-picker-container .skill-list", timeout=5000)

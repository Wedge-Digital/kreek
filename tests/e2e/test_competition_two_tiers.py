"""Test E2E de création d'une compétition avec deux tiers dont un avec SPP.

Vérifie que :
- Le parcours complet phase 1 → 2 → 3 fonctionne avec deux tiers
- Les roster pickers se chargent correctement dans chaque tier
- Les star player pickers se chargent correctement dans chaque tier
- Les inducement pickers se chargent correctement dans chaque tier
- Le deuxième tier peut avoir un XP de départ (SPP) différent de zéro
- Les tiers ont des rosters distincts (pas de conflit de soumission)
- La phase 3 est atteinte après soumission

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import re
import time

from playwright.sync_api import Page, expect


FAKE_LOGO_URL = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg"


def _create_competition_phase1(page: Page, competition_create_url: str) -> None:
    """Phase 1 : crée une compétition avec un nom unique."""
    competition_name = f"Ligue 2 Tiers E2E {time.time_ns()}"
    page.goto(competition_create_url, wait_until="load")
    page.fill("input[name='name']", competition_name)
    page.wait_for_selector(".coach-result-row", timeout=5000)
    page.locator(".coach-result-row").first.click()
    expect(page.locator(".coach-selected-badge")).to_have_count(1)
    page.evaluate(f"document.getElementById('logo_url').value = '{FAKE_LOGO_URL}'")
    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")
    expect(page).to_have_url(re.compile(r".*/rules$"))


def test_two_tiers_with_spp_reaches_phase_3(page: Page, competition_create_url):
    _create_competition_phase1(page, competition_create_url)

    # ── Phase 2 : attendre que le tier par défaut soit chargé ────────────
    page.wait_for_selector(".tier-block", timeout=5000)
    page.wait_for_selector(
        ".tier-block [data-slot='roster'] .roster-chip", timeout=5000
    )
    page.wait_for_selector(
        ".tier-block [data-slot='star'] .roster-chip", timeout=5000
    )
    page.wait_for_selector(
        ".tier-block [data-slot='inducement'] .inducement-chip", timeout=5000
    )

    # Vérifier que les 3 pickers du tier 1 sont chargés
    tier1 = page.locator(".tier-block").nth(0)
    tier1_rosters = tier1.locator("[data-slot='roster'] .roster-chip")
    tier1_stars = tier1.locator("[data-slot='star'] .roster-chip")
    tier1_inducements = tier1.locator("[data-slot='inducement'] .inducement-chip")
    assert tier1_rosters.count() > 0, "Tier 1 : aucun roster chip chargé"
    assert tier1_stars.count() > 0, "Tier 1 : aucun star player chip chargé"
    assert tier1_inducements.count() > 0, "Tier 1 : aucun inducement chip chargé"

    # ── Ajouter un deuxième tier ─────────────────────────────────────────
    page.click("button[onclick='addTier()']")
    page.wait_for_selector(
        ".tier-block:nth-child(2) [data-slot='roster'] .roster-chip", timeout=5000
    )
    page.wait_for_selector(
        ".tier-block:nth-child(2) [data-slot='star'] .roster-chip", timeout=5000
    )
    page.wait_for_selector(
        ".tier-block:nth-child(2) [data-slot='inducement'] .inducement-chip",
        timeout=5000,
    )

    # Vérifier que les 3 pickers du tier 2 sont chargés
    tier2 = page.locator(".tier-block").nth(1)
    tier2_rosters = tier2.locator("[data-slot='roster'] .roster-chip")
    tier2_stars = tier2.locator("[data-slot='star'] .roster-chip")
    tier2_inducements = tier2.locator("[data-slot='inducement'] .inducement-chip")
    assert tier2_rosters.count() > 0, "Tier 2 : aucun roster chip chargé"
    assert tier2_stars.count() > 0, "Tier 2 : aucun star player chip chargé"
    assert tier2_inducements.count() > 0, "Tier 2 : aucun inducement chip chargé"

    # ── Configurer le tier 2 avec 6 SPP ──────────────────────────────────
    tier2_xp_input = tier2.locator("input.tier-xp")
    tier2_xp_input.fill("6")
    tier2_xp_input.dispatch_event("change")

    # ── Différencier les rosters pour éviter RosterInMultipleTiers ────────
    # Désélectionner tous les rosters du tier 1 sauf le premier
    tier1_roster_count = tier1_rosters.count()
    for i in range(1, tier1_roster_count):
        chip = tier1_rosters.nth(i)
        if "selected" in (chip.get_attribute("class") or ""):
            chip.click()
            page.wait_for_timeout(100)

    # Désélectionner le premier roster du tier 2 (garder les autres)
    first_tier2_roster = tier2_rosters.first
    if "selected" in (first_tier2_roster.get_attribute("class") or ""):
        first_tier2_roster.click()
        page.wait_for_timeout(100)

    # ── Remplir le nom de saison et soumettre ────────────────────────────
    page.fill("#season_name", "Saison 2 Tiers E2E")

    page.click("button[onclick='submitRules()']")
    page.wait_for_selector("#groups-config", timeout=10000)

    expect(page).to_have_url(re.compile(r".*/structure$"))
    expect(page.locator(".title").first).to_have_text(
        "Structure de la compétition"
    )

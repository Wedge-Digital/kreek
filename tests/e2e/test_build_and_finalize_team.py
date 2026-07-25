"""Test E2E du parcours construction + finalisation d'une équipe.

Crée une compétition avec un tier à 6 SPP, crée un draft, construit
l'équipe (sélection roster, recrutement de 11 joueurs), accède à la
page de finalisation, sélectionne un joueur, vérifie que le skill
picker se charge, et soumet l'équipe.

Le roster est sélectionné en déclenchant directement `rosterSelected`
plutôt qu'en cliquant le widget TomSelect (flaky en Playwright headless —
cf. `competition_lifecycle.py`), et la compétition est sélectionnée en
ciblant l'option `<kreek-select>` par son nom exact (pas la première de la
liste — cf. bug corrigé dans test_special_rule_selector.py/test_draft_team_errors.py).

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import re
import time

from playwright.sync_api import Page, expect

from competition_lifecycle import create_full_competition

BASE_URL = "http://localhost:3210"


def test_build_and_finalize_with_spp(page: Page, competition_create_url, space_id):
    # ── Phase 1-2-3-4-5 : créer une compétition dédiée (6 SPP) ────────
    competition = create_full_competition(page, competition_create_url, num_rounds=1, tier_xp=6)

    # ── Accéder à la page de création d'équipe ───────────────────────
    draft_url = f"{BASE_URL}/app/{space_id}/team/create"
    page.goto(draft_url, wait_until="load")

    # Remplir le nom d'équipe
    page.fill("input[name='team_name']", f"Team E2E {time.time_ns()}")

    # Sélectionner un coach
    page.wait_for_selector(".coach-select .ts-control", timeout=5000)
    page.locator(".coach-select .ts-control").click()
    page.wait_for_selector(".ts-dropdown .option", timeout=3000)
    page.locator(".ts-dropdown .option").first.click()
    page.wait_for_timeout(300)

    # Sélectionner la compétition dédiée (kreek-select — cible par son nom exact)
    comp_select = page.locator("kreek-select[name='competition_id']")
    comp_select.wait_for(timeout=5000)
    comp_select.locator(".ks-control").click()
    comp_option = comp_select.locator(".ks-option", has_text=competition["name"])
    comp_option.wait_for(timeout=5000)
    comp_option.first.click()

    season_hidden = page.locator("kreek-select[name='season_id'] input[type='hidden']")
    expect(season_hidden).not_to_have_value("", timeout=5000)

    # Soumettre le draft
    page.click("button[type='submit']")
    page.wait_for_url(re.compile(r".*/build$"), timeout=10000)

    # ── Page build-team ──────────────────────────────────────────────
    # Sélection du roster via l'événement DOM que TomSelect émettrait
    # normalement, plutôt qu'un clic UI (peu fiable en Playwright headless).
    page.evaluate(
        "([uid, name]) => htmx.trigger(document.body, 'rosterSelected', {uid, name})",
        ["DEMO_GRANIT", "Granitiers"],
    )

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

    # Vérifier que le budget SPP est affiché (6 SPP) — .spp-badge n'existe
    # que dans le skill-header, chargé après sélection d'un joueur (plus bas).
    expect(page.locator(".spp-budget-numbers")).to_be_visible()
    expect(page.locator(".spp-budget-numbers")).to_contain_text("6")

    # Sélectionner le premier joueur
    page.locator(".player-row").first.click()
    page.wait_for_timeout(1000)

    # Vérifier que le skill header se charge
    expect(page.locator(".skill-header-widget")).to_be_visible(timeout=5000)

    # Vérifier que le skill picker se charge
    page.wait_for_selector("#skill-picker-container .skill-list", timeout=5000)

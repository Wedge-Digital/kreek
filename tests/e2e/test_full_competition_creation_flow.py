"""Test E2E du parcours complet de création d'une compétition, phase 1
(informations + admins + logo) → phase 2 (règles + tiers) → phase 3
(structure).

Contrairement à test_coach_selector.py et test_phase2_pickers.py (qui
testent chaque phase isolément, parfois avec des identifiants de
compétition/saison fictifs), ce test exerce le parcours réel de bout en
bout : la phase 1 crée une vraie compétition + saison en base, et la
navigation entre phases utilise les redirections serveur réelles
(HX-Redirect natif d'htmx pour phase 1 → 2, navigation SPA via htmx.ajax
pour phase 2 → 3).

Le logo passe par le champ caché `logo_url` rempli directement en JS
plutôt que par une vraie interaction avec le widget Cloudinary (upload de
fichier réel hors de portée d'un test E2E local) — seule la validation
serveur du format d'URL est exercée ici, pas l'upload tiers lui-même.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import re
import time

from playwright.sync_api import Page, expect

FAKE_LOGO_URL = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg"


def test_full_creation_flow_reaches_phase_3(page: Page, competition_create_url):
    # Nom unique par exécution — la compétition est réellement persistée en
    # base (pas de mock), un nom répété déclencherait CompetitionNameAlreadyTaken.
    competition_name = f"Ligue E2E {time.time_ns()}"

    # ── Phase 1 : informations ──────────────────────────────────────────
    page.goto(competition_create_url, wait_until="load")
    page.fill("input[name='name']", competition_name)

    page.wait_for_selector(".coach-result-row", timeout=5000)
    page.locator(".coach-result-row").first.click()
    expect(page.locator(".coach-selected-badge")).to_have_count(1)

    page.evaluate(f"document.getElementById('logo_url').value = '{FAKE_LOGO_URL}'")

    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    expect(page).to_have_url(re.compile(r".*/rules$"))

    # ── Phase 2 : règles ─────────────────────────────────────────────────
    page.wait_for_selector(".tier-block [data-slot='star'] .roster-chip", timeout=5000)
    page.fill("#season_name", "Saison E2E 1")

    page.click("button[onclick='submitRules()']")
    page.wait_for_selector("#groups-config", timeout=5000)

    # Phase 2 → 3 est une navigation SPA (htmx.ajax + pushState, pas un
    # rechargement complet) — l'URL change quand même, vérifiable.
    expect(page).to_have_url(re.compile(r".*/structure$"))
    expect(page.locator(".title").first).to_have_text("Structure de la compétition")

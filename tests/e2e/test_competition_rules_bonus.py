"""Test E2E du round-trip de saisie des bonus de classement (phase 2 des
règles de compétition).

Couvre ce qu'aucun test unitaire ne voit : la sérialisation JS des nouveaux
champs bonus par `buildJSON()` (seuil défensif configurable + bonus agressif),
leur persistance, puis leur ré-hydratation par `hydrateBonuses()` au
rechargement du formulaire depuis `existing_rules_json`.

Le libellé récapitulatif (phase 5 / onglet admin) n'est pas exercé ici : son
formatage est couvert par les tests unitaires de `format_bonus_label`. Ce test
se concentre sur l'aller-retour formulaire → serveur → formulaire.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import re
import time

from playwright.sync_api import Page, expect

FAKE_LOGO_URL = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg"


def test_bonus_inputs_round_trip(page: Page, competition_create_url):
    competition_name = f"Ligue Bonus E2E {time.time_ns()}"

    # ── Phase 1 : informations (crée une vraie compétition + saison) ─────
    page.goto(competition_create_url, wait_until="load")
    page.fill("input[name='name']", competition_name)

    page.wait_for_selector(".coach-result-row", timeout=5000)
    page.locator(".coach-result-row").first.click()
    expect(page.locator(".coach-selected-badge")).to_have_count(1)

    page.evaluate(f"document.getElementById('logo_url').value = '{FAKE_LOGO_URL}'")

    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    expect(page).to_have_url(re.compile(r".*/rules$"))
    rules_url = page.url

    # ── Phase 2 : saisie des bonus ───────────────────────────────────────
    page.wait_for_selector(".tier-block [data-slot='star'] .roster-chip", timeout=5000)
    page.fill("#season_name", "Saison Bonus 1")

    # Seuil défensif ≠ 1 (valeur historique) + activation du bonus agressif.
    page.fill("#def_max_td", "0")
    page.check("#agg_activated")
    page.fill("#agg_points", "2")
    page.fill("#agg_min_cas", "3")

    page.click("button[onclick='submitRules()']")
    page.wait_for_selector("#groups-config", timeout=5000)
    expect(page).to_have_url(re.compile(r".*/structure$"))

    # ── Rechargement frais de la phase 2 → ré-hydratation serveur ────────
    page.goto(rules_url, wait_until="load")

    expect(page.locator("#agg_activated")).to_be_checked()
    expect(page.locator("#agg_points")).to_have_value("2")
    expect(page.locator("#agg_min_cas")).to_have_value("3")
    expect(page.locator("#def_max_td")).to_have_value("0")

    # Le bonus offensif (non modifié) reste intact — clé JSON `diff_td`.
    expect(page.locator("#off_activated")).to_be_checked()
    expect(page.locator("#off_diff_td")).to_have_value("3")

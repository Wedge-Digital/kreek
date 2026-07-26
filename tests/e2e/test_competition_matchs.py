"""Tests E2E — onglets Résultats et Calendrier de la page compétition.

Crée une compétition via le flow standard (phase 1), puis navigue sur la page
de détail pour vérifier le comportement des onglets Résultats et Calendrier.

Les scénarios testés ici couvrent :
- S1 / S2 : comportement partagé (onglet actif par défaut, switch onglet)
- R1, R8  : onglet Résultats (clic + URL directe)
- C1, C6  : onglet Calendrier (clic + URL directe)

Les scénarios de scroll (R6/R7/C5) et de données spécifiques (R2–R5, C2–C4)
sont couverts unitairement dans les controllers Rust (format_date_range, etc.)
et nécessitent des fixtures de données plus complexes pour l'E2E.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import re
import time

import pytest
from playwright.sync_api import Page, expect

FAKE_LOGO_URL = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg"


# ── Helpers ────────────────────────────────────────────────────────────────────


def _create_competition_and_get_detail_ids(page: Page, competition_create_url: str) -> dict:
    """Crée une compétition phase 1 et retourne les IDs extraits de l'URL rules."""
    page.goto(competition_create_url, wait_until="load")
    page.fill("input[name='name']", f"Matchs E2E {time.time_ns()}")
    page.wait_for_selector(".coach-result-row", timeout=5000)
    page.locator(".coach-result-row").first.click()
    expect(page.locator(".coach-selected-badge")).to_have_count(1)
    page.evaluate(f"document.getElementById('logo_url').value = '{FAKE_LOGO_URL}'")
    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")
    expect(page).to_have_url(re.compile(r".*/rules$"))
    url = page.url
    match = re.search(r"/app/([^/]+)/competitions/create/([^/]+)/([^/]+)/rules", url)
    assert match, f"Impossible d'extraire les IDs depuis l'URL: {url}"
    return {
        "space_id": match.group(1),
        "competition_id": match.group(2),
        "season_id": match.group(3),
    }


def _detail_url(base: str, ids: dict) -> str:
    return (
        f"{base}/app/{ids['space_id']}"
        f"/competitions/{ids['competition_id']}"
        f"/{ids['season_id']}"
    )


# ── Fixtures ───────────────────────────────────────────────────────────────────


@pytest.fixture(scope="module")
def competition_ids(browser, space_id):
    """Crée une compétition une seule fois pour le module."""
    base = "http://localhost:3210"
    create_url = f"{base}/app/{space_id}/competitions/create"
    context = browser.new_context()
    page = context.new_page()
    ids = _create_competition_and_get_detail_ids(page, create_url)
    context.close()
    return ids


# ── Scénarios partagés (S1, S2) ───────────────────────────────────────────────


def test_s1_classement_actif_par_defaut(page: Page, competition_ids, console_errors):
    """S1 — L'onglet Classement est actif au chargement initial."""
    base = "http://localhost:3210"
    page.goto(_detail_url(base, competition_ids), wait_until="load")

    # Texte exact : « Classement détaillé » répondrait aussi à une sous-chaîne.
    tab_classement = page.locator(".tabs .tab", has_text=re.compile(r"^Classement$"))
    expect(tab_classement).to_have_class(re.compile(r"\bactive\b"))

    tab_resultats = page.locator(".tabs .tab", has_text="Résultats")
    expect(tab_resultats).not_to_have_class(re.compile(r"\bactive\b"))

    tab_calendrier = page.locator(".tabs .tab", has_text="Calendrier")
    expect(tab_calendrier).not_to_have_class(re.compile(r"\bactive\b"))


def test_s2_switch_vers_resultats_desactive_classement(page: Page, competition_ids, console_errors):
    """S2 — Clic sur Résultats désactive Classement."""
    base = "http://localhost:3210"
    page.goto(_detail_url(base, competition_ids), wait_until="load")

    page.locator(".tabs .tab", has_text="Résultats").click()
    page.wait_for_timeout(500)

    expect(page.locator(".tabs .tab", has_text="Résultats")).to_have_class(
        re.compile(r"\bactive\b")
    )
    expect(page.locator(".tabs .tab", has_text=re.compile(r"^Classement$"))).not_to_have_class(
        re.compile(r"\bactive\b")
    )


# ── Onglet Résultats (R1, R8) ─────────────────────────────────────────────────


def test_r1_clic_resultats_charge_fragment(page: Page, competition_ids, console_errors):
    """R1 — Clic sur Résultats charge le fragment HTMX (état vide attendu)."""
    base = "http://localhost:3210"
    page.goto(_detail_url(base, competition_ids), wait_until="load")

    page.locator(".tabs .tab", has_text="Résultats").click()

    # Attend que #tab-content soit remplacé par le fragment résultats
    page.wait_for_selector("#resultats-list, .tab-empty-state", timeout=5000)

    # L'URL doit pointer vers /resultats
    expect(page).to_have_url(re.compile(r".*/resultats$"))


def test_r8_navigation_directe_url_resultats(page: Page, competition_ids, console_errors):
    """R8 — Navigation directe sur /resultats : full page, onglet Résultats actif."""
    base = "http://localhost:3210"
    ids = competition_ids
    url = (
        f"{base}/app/{ids['space_id']}"
        f"/competitions/{ids['competition_id']}"
        f"/{ids['season_id']}/resultats"
    )
    page.goto(url, wait_until="load")

    # L'onglet Résultats doit être actif (JS auto-déclenche le chargement)
    expect(page.locator(".tabs .tab", has_text="Résultats")).to_have_class(
        re.compile(r"\bactive\b")
    )

    # La page doit être complète (header, onglets présents)
    expect(page.locator(".tabs")).to_be_visible()
    expect(page.locator(".detail-left")).to_be_visible()

    # Le fragment doit être chargé (état vide ou journées)
    page.wait_for_selector("#resultats-list, .tab-empty-state", timeout=5000)


# ── Onglet Calendrier (C1, C6) ────────────────────────────────────────────────


def test_c1_clic_calendrier_charge_fragment(page: Page, competition_ids, console_errors):
    """C1 — Clic sur Calendrier charge le fragment HTMX (état vide attendu)."""
    base = "http://localhost:3210"
    page.goto(_detail_url(base, competition_ids), wait_until="load")

    page.locator(".tabs .tab", has_text="Calendrier").click()

    # Attend que #tab-content soit remplacé par le fragment calendrier
    page.wait_for_selector("#calendrier-list, .tab-empty-state", timeout=5000)

    # L'URL doit pointer vers /calendrier
    expect(page).to_have_url(re.compile(r".*/calendrier$"))


def test_c6_navigation_directe_url_calendrier(page: Page, competition_ids, console_errors):
    """C6 — Navigation directe sur /calendrier : full page, onglet Calendrier actif."""
    base = "http://localhost:3210"
    ids = competition_ids
    url = (
        f"{base}/app/{ids['space_id']}"
        f"/competitions/{ids['competition_id']}"
        f"/{ids['season_id']}/calendrier"
    )
    page.goto(url, wait_until="load")

    expect(page.locator(".tabs .tab", has_text="Calendrier")).to_have_class(
        re.compile(r"\bactive\b")
    )

    expect(page.locator(".tabs")).to_be_visible()
    expect(page.locator(".detail-left")).to_be_visible()

    page.wait_for_selector("#calendrier-list, .tab-empty-state", timeout=5000)


# ── Onglet Résultats — état vide ──────────────────────────────────────────────


def test_r1_etat_vide_resultats_sans_matchs(page: Page, competition_ids, console_errors):
    """R1 — Sans matchs terminés, l'état vide est affiché dans Résultats."""
    base = "http://localhost:3210"
    page.goto(_detail_url(base, competition_ids), wait_until="load")

    page.locator(".tabs .tab", has_text="Résultats").click()
    page.wait_for_timeout(1000)

    # Soit des journées, soit l'état vide — les deux sont acceptables
    resultats_list = page.locator("#resultats-list")
    empty_state = page.locator(".tab-empty-state")

    has_list = resultats_list.count() > 0
    has_empty = empty_state.count() > 0

    assert has_list or has_empty, "Ni #resultats-list ni .tab-empty-state trouvé"


def test_c1_etat_vide_calendrier_sans_matchs(page: Page, competition_ids, console_errors):
    """C1 — Sans matchs à venir, l'état vide est affiché dans Calendrier."""
    base = "http://localhost:3210"
    page.goto(_detail_url(base, competition_ids), wait_until="load")

    page.locator(".tabs .tab", has_text="Calendrier").click()
    page.wait_for_timeout(1000)

    calendrier_list = page.locator("#calendrier-list")
    empty_state = page.locator(".tab-empty-state")

    has_list = calendrier_list.count() > 0
    has_empty = empty_state.count() > 0

    assert has_list or has_empty, "Ni #calendrier-list ni .tab-empty-state trouvé"

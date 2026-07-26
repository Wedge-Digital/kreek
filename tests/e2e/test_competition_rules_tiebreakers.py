"""Tests E2E de l'activation des critères de départage (phase 2 des règles de
compétition).

Couvre ce qu'aucun test unitaire ne voit : le rendu JS de la liste depuis le
catalogue injecté par le serveur, la renumérotation des seuls critères actifs,
le garde-fou de soumission, la sérialisation par `buildJSON()` puis la
ré-hydratation par `initFromExistingRules()` au rechargement.

Les invariants (au moins un actif, pas de doublon) sont couverts unitairement
par `TiebreakConfig::try_new` ; ici on vérifie le comportement du formulaire.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) servant le référentiel
de démonstration.
"""

import re
import time

from playwright.sync_api import Page, expect

FAKE_LOGO_URL = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg"

# Libellés attendus, dans l'ordre canonique du catalogue (BC ranking).
EXPECTED_LABELS = [
    "Différence de touchdowns (marqués − encaissés)",
    "Nombre de touchdowns marqués",
    "Nombre de touchdowns encaissés",
    "Nombre de blessures infligées",
    "Nombre de victoires",
    "Nombre de fautes commises",
    "Nombre de réussites",
]


def _create_competition_and_reach_rules(page: Page, create_url: str, name: str) -> str:
    """Parcourt la phase 1 et retourne l'URL de la phase 2 (règles)."""
    page.goto(create_url, wait_until="load")
    page.fill("input[name='name']", name)

    page.wait_for_selector(".coach-result-row", timeout=5000)
    page.locator(".coach-result-row").first.click()
    expect(page.locator(".coach-selected-badge")).to_have_count(1)

    page.evaluate(f"document.getElementById('logo_url').value = '{FAKE_LOGO_URL}'")

    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    expect(page).to_have_url(re.compile(r".*/rules$"))
    page.wait_for_selector("#tiebreak-list .tiebreak-row", timeout=5000)
    return page.url


def _rows(page: Page):
    return page.locator("#tiebreak-list .tiebreak-row")


def _labels(page: Page) -> list[str]:
    return _rows(page).locator(".tiebreak-label").all_text_contents()


def _ranks(page: Page) -> list[str]:
    return [t.strip() for t in _rows(page).locator(".tiebreak-rank").all_text_contents()]


def _uncheck_row(page: Page, label: str) -> None:
    row = _rows(page).filter(has_text=label)
    row.locator(".tiebreak-check").uncheck()


def _drag_row(page: Page, source_label: str, target_label: str) -> None:
    """Glisse une ligne sur une autre. `drag_to` d'abord ; en cas d'échec du
    HTML5 drag-and-drop, on dispatche les événements à la main — les handlers
    exercés restent ceux du template."""
    source = _rows(page).filter(has_text=source_label)
    target = _rows(page).filter(has_text=target_label)
    try:
        source.drag_to(target)
    except Exception:
        page.evaluate(
            """([srcText, dstText]) => {
                const rows = [...document.querySelectorAll('#tiebreak-list .tiebreak-row')];
                const src = rows.find(r => r.textContent.includes(srcText));
                const dst = rows.find(r => r.textContent.includes(dstText));
                const dt = new DataTransfer();
                src.dispatchEvent(new DragEvent('dragstart', { bubbles: true, dataTransfer: dt }));
                dst.dispatchEvent(new DragEvent('dragover', { bubbles: true, dataTransfer: dt }));
                dst.dispatchEvent(new DragEvent('drop', { bubbles: true, dataTransfer: dt }));
            }""",
            [source_label, target_label],
        )


def test_catalogue_lists_the_seven_criteria_without_red_cards(
    page: Page, competition_create_url
):
    """Le catalogue rendu vient du BC ranking : 7 critères, pas de cartons rouges."""
    name = f"Ligue Departage Catalogue {time.time_ns()}"
    _create_competition_and_reach_rules(page, competition_create_url, name)

    assert _labels(page) == EXPECTED_LABELS
    assert "Nombre de cartons rouges" not in " ".join(_labels(page))
    # Défaut : les 7 critères actifs, numérotés 1 à 7 (règle 3).
    assert _ranks(page) == ["1", "2", "3", "4", "5", "6", "7"]


def test_unchecking_renumbers_only_active_criteria(page: Page, competition_create_url):
    """Décocher laisse la ligne en place, la grise, et recalcule les rangs."""
    name = f"Ligue Departage Rangs {time.time_ns()}"
    _create_competition_and_reach_rules(page, competition_create_url, name)

    _uncheck_row(page, "Différence de touchdowns")

    # La ligne n'a pas bougé : l'ordre est conservé (règle 2).
    assert _labels(page) == EXPECTED_LABELS
    # Le décoché affiche un tiret, les actifs se renumérotent à partir de 1.
    assert _ranks(page) == ["—", "1", "2", "3", "4", "5", "6"]
    expect(_rows(page).first).to_have_class(re.compile(r"is-off"))


def test_submit_is_blocked_when_no_criterion_is_active(
    page: Page, competition_create_url
):
    """Garde-fou de la règle 1 : aucun critère actif ⇒ soumission impossible."""
    name = f"Ligue Departage Garde Fou {time.time_ns()}"
    _create_competition_and_reach_rules(page, competition_create_url, name)

    expect(page.locator("#tiebreak-warning")).to_be_hidden()
    expect(page.locator("#rules-submit-btn")).to_be_enabled()

    for label in EXPECTED_LABELS:
        _uncheck_row(page, label)

    expect(page.locator("#tiebreak-warning")).to_be_visible()
    expect(page.locator("#rules-submit-btn")).to_be_disabled()

    # Recocher un seul critère relâche le garde-fou.
    _rows(page).last.locator(".tiebreak-check").check()
    expect(page.locator("#tiebreak-warning")).to_be_hidden()
    expect(page.locator("#rules-submit-btn")).to_be_enabled()


def test_order_and_activation_round_trip(page: Page, competition_create_url):
    """Aller-retour complet : formulaire → serveur → formulaire."""
    name = f"Ligue Departage Round Trip {time.time_ns()}"
    rules_url = _create_competition_and_reach_rules(page, competition_create_url, name)

    page.wait_for_selector(".tier-block [data-slot='star'] .roster-chip", timeout=5000)
    page.fill("#season_name", "Saison Departage 1")

    _uncheck_row(page, "Nombre de fautes commises")
    _uncheck_row(page, "Nombre de réussites")
    _drag_row(page, "Nombre de victoires", "Différence de touchdowns")

    expected_order = _labels(page)
    expected_ranks = _ranks(page)

    page.click("#rules-submit-btn")
    page.wait_for_selector("#groups-config", timeout=5000)
    expect(page).to_have_url(re.compile(r".*/structure$"))

    # ── Rechargement frais → ré-hydratation depuis existing_rules_json ───
    page.goto(rules_url, wait_until="load")
    page.wait_for_selector("#tiebreak-list .tiebreak-row", timeout=5000)

    assert _labels(page) == expected_order
    assert _ranks(page) == expected_ranks

    fouls = _rows(page).filter(has_text="Nombre de fautes commises")
    expect(fouls.locator(".tiebreak-check")).not_to_be_checked()
    reu = _rows(page).filter(has_text="Nombre de réussites")
    expect(reu.locator(".tiebreak-check")).not_to_be_checked()
    diff = _rows(page).filter(has_text="Différence de touchdowns")
    expect(diff.locator(".tiebreak-check")).to_be_checked()

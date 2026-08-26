"""Tests E2E de l'affichage des règles spéciales de roster et de leur
caractère obligatoire à la finalisation d'une équipe.

Un roster sans FAVOURED_OF_CHOOSE_* (jeu de démonstration : Granitiers,
Zéphyriens) affiche ses règles fixes en lecture seule (chips) ou "Pas de règle
spéciale". Un roster à choix (Lanterniers) garde son <select> interactif, et la
finalisation de l'équipe est bloquée tant qu'aucune règle n'a été choisie.

La sélection du roster ne passe pas par un clic UI sur le widget TomSelect
(cf. test_build_and_finalize_team.py, skip WIP : ce câblage précis est connu
pour être flaky en Playwright headless) mais déclenche directement
l'événement DOM `rosterSelected` que TomSelect émettrait normalement — ce
qui exerce le même rendu HTMX/Askama réel côté serveur, sans dépendre du
timing d'attachement du listener TomSelect.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import re
import time

from playwright.sync_api import Page, expect

from competition_lifecycle import create_full_competition
from htmx_helpers import attendre_cablage, cliquer_quand_cable

BASE_URL = "http://localhost:3210"


def _create_minimal_competition(page: Page, competition_create_url: str) -> str:
    """Crée une compétition **finalisée**, avec un tier à 0 SPP.

    Elle s'arrêtait jusqu'ici à la phase 2, et bâtir une équipe dessus
    fonctionnait : c'était précisément le défaut de la carte 407, encodé ici
    comme prémisse. Une saison non finalisée refuse désormais la création, donc
    ces tests passent par le magicien complet — leur sujet est le sélecteur de
    règle spéciale, pas l'état de la compétition.

    Retourne le nom de la compétition, unique par exécution — nécessaire pour
    la retrouver ensuite dans le kreek-select de /team/create : la base de
    test accumule les compétitions des runs précédents, et ce <kreek-select>
    les liste par ordre alphabétique de nom (pas par date de création), donc
    "la première option" n'est pas fiablement la nôtre.
    """
    return create_full_competition(page, competition_create_url, num_rounds=1, tier_xp=0)["name"]


def _create_draft_team(page: Page, space_id: str, competition_name: str) -> str:
    """Crée un draft d'équipe et retourne le team_id (atterrit sur /build)."""
    draft_url = f"{BASE_URL}/app/{space_id}/team/create"
    page.goto(draft_url, wait_until="load")
    page.fill("input[name='team_name']", f"Team SpecialRule E2E {time.time_ns()}")

    page.wait_for_selector(".coach-select .ts-control", timeout=5000)
    page.locator(".coach-select .ts-control").click()
    page.wait_for_selector(".ts-dropdown .option", timeout=3000)
    page.locator(".ts-dropdown .option").first.click()
    page.wait_for_timeout(300)

    # competition_id est un <kreek-select> (input caché + .ks-control/.ks-option,
    # pas un <select> natif) — season_id se sélectionne automatiquement ensuite
    # via son attribut auto-select-first une fois la compétition choisie.
    comp_select = page.locator("kreek-select[name='competition_id']")
    comp_select.wait_for(timeout=5000)
    comp_select.locator(".ks-control").click()
    comp_option = comp_select.locator(".ks-option", has_text=competition_name)
    comp_option.wait_for(timeout=5000)
    comp_option.first.click()

    season_hidden = page.locator("kreek-select[name='season_id'] input[type='hidden']")
    expect(season_hidden).not_to_have_value("", timeout=5000)

    page.click("button[type='submit']")
    page.wait_for_url(re.compile(r".*/build$"), timeout=10000)

    match = re.search(r"/team/([0-9A-Za-z]+)/build", page.url)
    assert match, f"team_id introuvable dans l'URL {page.url}"
    return match.group(1)


def _select_roster(page: Page, uid: str, name: str) -> None:
    """Déclenche `rosterSelected` directement, sans passer par TomSelect.

    L'attente de câblage est nécessaire : les trois conteneurs qui écoutent
    `rosterSelected from:body` ne le reçoivent que si htmx les a câblés, et un
    événement déclenché plus tôt se perd exactement comme un clic.

    **Elle ne suffit pas.** Ce test reste instable — environ un échec sur six,
    contre un sur deux avant —, et toujours au même endroit : le tableau des
    joueurs n'arrive jamais. La cause du reste n'est pas établie ; ne pas
    supposer que ce garde-fou l'a fermée.
    """
    attendre_cablage(page, "#player-table-container")
    page.evaluate(
        "([uid, name]) => htmx.trigger(document.body, 'rosterSelected', {uid, name})",
        [uid, name],
    )


def _hire_players(page: Page, count: int) -> int:
    page.wait_for_selector("#player-table-container .tbl-btn", timeout=10000)
    hired = 0
    attempts = 0
    while hired < count and attempts < 40:
        buttons = page.locator("#player-table-container .tbl-btn:not([disabled])").all()
        clicked = False
        for btn in buttons:
            if btn.inner_text().strip() == "+":
                btn.click()
                page.wait_for_timeout(200)
                hired += 1
                clicked = True
                break
        if not clicked:
            break
        attempts += 1
    return hired


def test_fixed_rules_display_as_chips(page: Page, space_id, competition_create_url):
    competition_name = _create_minimal_competition(page, competition_create_url)
    team_id = _create_draft_team(page, space_id, competition_name)
    page.goto(f"{BASE_URL}/app/{space_id}/team/{team_id}/build", wait_until="load")

    _select_roster(page, "DEMO_GRANIT", "Granitiers")

    # Trois règles depuis la carte 275 : « Brutes Bagarreuses » a été ajoutée au
    # corpus de démo pour que le barème SPP inversé soit exerçable — c'est le
    # seul roster de démo qui le porte. Les Zéphyriens ne pouvaient pas
    # l'accueillir : ce fichier a besoin d'eux comme roster sans aucune règle.
    zone = page.locator("#special-rule-selector-zone")
    expect(zone.locator(".special-rule-chip")).to_have_count(3)
    expect(zone.locator(".special-rule-chip").nth(0)).to_have_text("Gens de Rocaille")
    expect(zone.locator(".special-rule-chip").nth(1)).to_have_text("Meneur Né")
    expect(zone.locator(".special-rule-chip").nth(2)).to_have_text("Brutes Bagarreuses")
    expect(zone.locator("select")).to_have_count(0)


def test_no_rule_roster_shows_none_message(page: Page, space_id, competition_create_url):
    competition_name = _create_minimal_competition(page, competition_create_url)
    team_id = _create_draft_team(page, space_id, competition_name)
    page.goto(f"{BASE_URL}/app/{space_id}/team/{team_id}/build", wait_until="load")

    _select_roster(page, "DEMO_ZEPHYR", "Zéphyriens")

    zone = page.locator("#special-rule-selector-zone")
    expect(zone.locator(".selector-none")).to_have_text("Pas de règle spéciale")


def test_choice_roster_keeps_interactive_select(page: Page, space_id, competition_create_url):
    competition_name = _create_minimal_competition(page, competition_create_url)
    team_id = _create_draft_team(page, space_id, competition_name)
    page.goto(f"{BASE_URL}/app/{space_id}/team/{team_id}/build", wait_until="load")

    _select_roster(page, "DEMO_LANTERNE", "Lanterniers")

    zone = page.locator("#special-rule-selector-zone")
    select = zone.locator("select.league-select")
    expect(select).to_be_visible()
    expect(select).to_be_enabled()
    expect(select.locator("option")).to_have_count(6)  # placeholder + 5 dieux


def test_finalize_blocked_when_choice_roster_has_no_special_rule(
    page: Page, space_id, competition_create_url
):
    competition_name = _create_minimal_competition(page, competition_create_url)
    team_id = _create_draft_team(page, space_id, competition_name)
    page.goto(f"{BASE_URL}/app/{space_id}/team/{team_id}/build", wait_until="load")

    _select_roster(page, "DEMO_LANTERNE", "Lanterniers")
    hired = _hire_players(page, 11)
    assert hired >= 11, f"N'a pu recruter que {hired} joueurs (11 requis)"

    # Le bouton vient d'être injecté : visible avant d'être câblé, et un clic
    # tombé dans cette fenêtre se perd sans rien signaler — cf. `htmx_helpers`.
    cliquer_quand_cable(page, "text=Terminer la construction →")

    # Pas d'attente en durée : `to_contain_text` attend déjà l'état résultant,
    # et le `wait_for_timeout(1000)` qui était ici ne pouvait de toute façon
    # rien pour un clic déjà perdu.
    error = page.locator("#submit-errors")
    expect(error).to_contain_text("règle spéciale")
"""Tests E2E des erreurs de validation du formulaire de création d'équipe.

Vérifie que les erreurs sont affichées avec le bon style quand :
- Le formulaire est soumis sans compétition/saison sélectionnée
- Le formulaire est soumis sans coach sélectionné
- Les erreurs s'affichent avec le style correct (classe table-error)

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

from playwright.sync_api import Page, expect
from e2e_env import BASE_URL


def _goto_draft(page: Page, space_id: str) -> None:
    url = f"{BASE_URL}/app/{space_id}/team/create"
    page.goto(url, wait_until="load")
    page.wait_for_selector("#draft-team-form", timeout=5000)


def _select_coach(page: Page) -> None:
    page.wait_for_selector(".coach-select .ts-control", timeout=5000)
    page.locator(".coach-select .ts-control").click()
    page.wait_for_selector(".ts-dropdown .option", timeout=3000)
    page.locator(".ts-dropdown .option").first.click()
    # Le choix est pris quand la page l'a reçu par `coachSelected`, pas au bout
    # d'une durée.
    expect(page.locator("#coach_id_field")).not_to_have_value("")


def _select_competition_and_season(page: Page) -> None:
    """competition_id/season_id sont des <kreek-select> (input caché +
    .ks-control/.ks-option), pas des <select> natifs. season_id se sélectionne
    automatiquement une fois competition_id choisi (attribut auto-select-first).

    **Attendre une vraie option** (carte 668). Le menu s'ouvre avant que la
    liste des compétitions soit chargée, et montre alors « Aucun résultat »
    (`.ks-empty`). L'ancienne version attendait n'importe quelle `.ks-option`,
    trouvait celle-là, ne choisissait rien et laissait le menu ouvert ; la liste
    arrivait ensuite dans le menu ouvert et recouvrait le bouton d'envoi, dont le
    clic attendait 30 s. Mesuré : 3 ouvertures sur 10 tombaient avant la liste.
    """
    comp_select = page.locator("kreek-select[name='competition_id']")
    comp_select.wait_for(timeout=5000)
    comp_select.locator(".ks-control").click()
    comp_select.locator(".ks-option:not(.ks-empty)").first.click()
    expect(comp_select.locator(".ks-dropdown")).to_be_hidden()
    season_hidden = page.locator("kreek-select[name='season_id'] input[type='hidden']")
    expect(season_hidden).not_to_have_value("", timeout=5000)


def test_submit_without_competition_shows_error(page: Page, space_id):
    _goto_draft(page, space_id)

    page.fill("input[name='team_name']", "Test sans compet")
    _select_coach(page)

    page.click("button[type='submit']")

    error = page.locator("#draft-team-error")
    expect(error).not_to_be_empty()
    expect(error).to_contain_text("Sélectionnez une compétition et une saison")


def test_submit_without_coach_shows_error(page: Page, space_id):
    _goto_draft(page, space_id)

    page.fill("input[name='team_name']", "Test sans coach")
    _select_competition_and_season(page)

    page.click("button[type='submit']")

    error = page.locator("#draft-team-error")
    expect(error).to_contain_text("coach")


def test_error_has_correct_style(page: Page, space_id):
    _goto_draft(page, space_id)

    page.fill("input[name='team_name']", "Test style erreur")
    _select_coach(page)

    page.click("button[type='submit']")

    error_p = page.locator("#draft-team-error .table-error")
    expect(error_p).to_be_visible()

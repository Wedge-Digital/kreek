"""Une compétition non finalisée n'accepte pas de création d'équipe (carte 407).

En production, une équipe sur seize de la Saison 10 n'est jamais entrée dans la
compétition : elle avait été créée **avant** `CompetitionReady`. À ce moment la
configuration des invitations n'existe pas encore, et l'inscription automatique
retombait silencieusement sur « non » — équipe en attente, aucun `TeamEnrolled`,
aucune ligne de journal.

Le magicien enchaîne les phases 1 à 5 sans point d'arrêt : pour observer l'état
intermédiaire, on recule le statut de la saison en base. C'est exactement la
condition sous test — le statut est ce que la garde regarde.

Le second test compte autant que le premier : une garde qui refuse tout le monde
passerait le premier sans que rien ne le signale.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import build_and_submit_team, create_full_competition
from db_helpers import execute_db

# Le statut que porte une saison dont seules les règles sont posées : la
# première des quatre étapes, et déjà joignable avant la carte 407.
STATUT_INACHEVE = "structure_selected"


@pytest.fixture
def saison_inachevee(page: Page, competition_create_url):
    """Une compétition publiée, ramenée à l'état « en cours de configuration ».

    Rendue `ready` à la sortie : les autres tests de la suite partagent la base
    de développement, et une saison laissée inachevée les ferait échouer sur un
    symptôme sans rapport.
    """
    competition = create_full_competition(page, competition_create_url, num_rounds=1)
    season_id = competition["season_id"]
    execute_db(
        f"UPDATE competition_seasons SET status = '{STATUT_INACHEVE}' WHERE id = '{season_id}'"
    )
    yield competition
    execute_db(f"UPDATE competition_seasons SET status = 'ready' WHERE id = '{season_id}'")


def _ouvrir_creation(page: Page, space_id: str) -> None:
    page.goto(f"http://localhost:3210/app/{space_id}/team/create", wait_until="load")
    page.wait_for_selector("#draft-team-form", timeout=5000)


def _choisir_competition(page: Page, nom: str) -> None:
    """Sélectionne la compétition par son nom.

    Échoue si elle n'est pas proposée, au lieu de rendre la main : le sélecteur
    ne filtre pas sur le statut (`get_json_competitions` liste toute compétition
    ayant au moins une saison), donc une absence signalerait un changement de
    comportement, pas un refus. Une première version rendait `False` ici et le
    test se terminait en vert sans avoir rien soumis."""
    comp_select = page.locator("kreek-select[name='competition_id']")
    comp_select.wait_for(timeout=5000)
    comp_select.locator(".ks-control").click()
    comp_select.locator(".ks-option").first.wait_for(timeout=5000)
    option = comp_select.locator(f".ks-option:not(.ks-empty):has-text('{nom}')")
    assert option.count() > 0, f"la compétition « {nom} » n'est pas proposée au coach"
    option.first.click()
    season = page.locator("kreek-select[name='season_id'] input[type='hidden']")
    expect(season).not_to_have_value("", timeout=5000)


def _choisir_coach(page: Page) -> None:
    page.wait_for_selector(".coach-select .ts-control", timeout=5000)
    page.locator(".coach-select .ts-control").click()
    page.wait_for_selector(".ts-dropdown .option", timeout=3000)
    page.locator(".ts-dropdown .option").first.click()


def test_une_saison_non_finalisee_refuse_la_creation(page: Page, space_id, saison_inachevee):
    _ouvrir_creation(page, space_id)
    page.fill("input[name='team_name']", "Equipe Trop Pressee")
    _choisir_coach(page)

    _choisir_competition(page, saison_inachevee["name"])

    page.click("button[type='submit']")
    expect(page.locator("#draft-team-error")).to_contain_text(
        "pas encore ouverte aux inscriptions", timeout=5000
    )


def test_la_saison_redevient_joignable_une_fois_prete(page: Page, space_id, saison_inachevee):
    """Le garde-fou du garde-fou : sans ce test, une garde qui refuse *toutes*
    les saisons passerait la suite en vert."""
    execute_db(
        f"UPDATE competition_seasons SET status = 'ready' WHERE id = '{saison_inachevee['season_id']}'"
    )
    _ouvrir_creation(page, space_id)
    page.fill("input[name='team_name']", "Equipe Bien Elevee")
    _choisir_coach(page)

    _choisir_competition(page, saison_inachevee["name"])
    page.click("button[type='submit']")

    # Succès = `HX-Redirect` : le navigateur quitte la page de création. Guetter
    # l'absence d'un message d'erreur ne dirait rien — sur succès l'élément
    # n'existe pas, et l'attendre expire au lieu de conclure.
    expect(page).not_to_have_url(
        f"http://localhost:3210/app/{space_id}/team/create", timeout=10000
    )


def test_une_equipe_commencee_avant_ne_peut_pas_etre_soumise(page: Page, space_id, saison_inachevee):
    """Le cas que la garde d'ouverture seule laisserait passer.

    Une équipe peut être commencée pendant que la saison est joignable et
    soumise après — c'est même l'ordre naturel des choses le jour où un
    administrateur rouvre la configuration de sa compétition.

    L'équipe est construite entièrement : la page de finalisation exige un
    roster complet, et un brouillon nu y répond 404 sans jamais atteindre la
    garde. Une première version du test s'y est trompée.
    """
    season_id = saison_inachevee["season_id"]
    execute_db(f"UPDATE competition_seasons SET status = 'ready' WHERE id = '{season_id}'")

    build_and_submit_team(
        page, space_id, saison_inachevee["name"], 0, 0, soumettre=False
    )

    # L'administrateur rouvre la configuration : la saison n'est plus ouverte.
    execute_db(
        f"UPDATE competition_seasons SET status = '{STATUT_INACHEVE}' WHERE id = '{season_id}'"
    )

    page.locator(".submit-bar button").click()
    expect(page.locator("#submit-errors")).to_contain_text(
        "pas encore ouverte aux inscriptions", timeout=10000
    )

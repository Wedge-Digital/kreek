"""L'onglet Présences : la coquille s'ouvre, la barre latérale liste les journées.

**Le premier test e2e de l'épic E16.** Les treize cartes précédentes n'avaient
posé aucune route : la suite ne vérifiait jusqu'ici que l'absence de régression
sur le Calendrier.

Ce que ce test couvre et qu'aucun test unitaire ne voit : l'onglet est atteignable
par le chemin htmx du changement d'onglet, le fragment de barre latérale se charge
sur son `hx-trigger="load"`, et il liste les journées de la saison.

Ce qu'il ne couvre pas encore : le panneau, qui rend une invite jusqu'à la carte
520.

Prérequis : serveur lancé en `make dev-e2e`, base au gabarit (`make e2e_db`).
"""

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from htmx_helpers import cliquer_quand_cable

# Coach seedé sans droit d'administration (`seed_e2e.rs::SIMPLE_COACH_NAME`).
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}


@pytest.fixture(scope="module")
def competition(browser, space_id):
    """Une compétition finalisée, avec son calendrier — mais **sans appariements**.

    `with_pairings` reste à son défaut : la carte 508 a montré qu'un test qui
    n'en a pas besoin ne doit pas en créer, le tirage étant aléatoire (R17) et
    l'orientation des rapports en découlant.
    """
    # Quatre équipes et trois journées : le minimum pour que la barre latérale ait
    # de quoi lister sans allonger la construction de la fixture.
    return build_full_competition(browser, space_id, 4, 3)


def _url_onglet(space_id, competition):
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{competition['competition_id']}/{competition['season_id']}/admin/presences"
    )


# ── L'onglet s'ouvre, par les deux chemins ────────────────────────────────────


def test_l_onglet_presences_s_ouvre_en_chargement_complet(page: Page, space_id, competition):
    page.goto(_url_onglet(space_id, competition), wait_until="load")

    expect(page.locator(".competition-admin-presences").first).to_be_visible(timeout=10000)
    expect(page.locator("#presences-sidebar")).to_be_visible()
    expect(page.locator("#presences-panel")).to_be_visible()


def test_l_onglet_est_atteignable_depuis_le_calendrier_par_htmx(page: Page, space_id, competition):
    """Le chemin htmx, et non un rechargement : c'est celui que l'organisateur
    emprunte, et c'est aussi celui qui contournerait la garde si un fragment ne
    l'appelait pas (carte 416)."""
    schedule = _url_onglet(space_id, competition).replace("/presences", "/schedule")
    page.goto(schedule, wait_until="load")
    expect(page.locator(".competition-admin-schedule").first).to_be_visible(timeout=10000)

    cliquer_quand_cable(page, ".admin-tab:has-text('Présences')")

    expect(page.locator("#presences-sidebar")).to_be_visible(timeout=10000)


# ── La barre latérale liste les journées ──────────────────────────────────────


def test_la_barre_laterale_liste_les_journees_de_la_saison(page: Page, space_id, competition):
    page.goto(_url_onglet(space_id, competition), wait_until="load")

    journees = page.locator(".presences-round")
    expect(journees.first).to_be_visible(timeout=10000)
    assert journees.count() > 0, "la saison porte des journées, elles doivent être listées"
    # Aucune campagne n'est ouverte : toutes les journées jouables le disent.
    expect(page.locator(".presences-round--aucun").first).to_be_visible()
    expect(page.locator(".presences-round--aucun").first).to_contain_text("Aucun sondage")


def test_le_panneau_invite_a_choisir_une_journee(page: Page, space_id, competition):
    page.goto(_url_onglet(space_id, competition), wait_until="load")

    expect(page.locator(".presences-panel--invite")).to_be_visible(timeout=10000)
    expect(page.locator(".presences-panel-invite")).to_contain_text("Choisissez une journée")


# ── La garde, sur la page comme sur les fragments ─────────────────────────────


@pytest.mark.parametrize("suffixe", ["", "/rounds", "/panel"])
def test_un_membre_simple_est_refuse_sur_l_onglet_et_ses_fragments(
    space_id, competition, suffixe
):
    """Les trois routes, fragments compris.

    `space_scope` garantit qu'une ressource appartient à l'espace de l'URL — pas
    que l'appelant en est administrateur. Sans `require_admin_access` sur chaque
    handler, un membre simple lirait la barre latérale de l'administration.
    """
    reponse = requests.get(
        _url_onglet(space_id, competition) + suffixe,
        headers={**ENTETE_MEMBRE_SIMPLE, "HX-Request": "true"},
        timeout=15,
    )

    assert reponse.status_code == 403, (
        f"la route {suffixe or '/presences'} a répondu {reponse.status_code} "
        "à un membre sans droit d'administration"
    )

"""Tests E2E — entrer dans une fiche d'équipe par un clic (carte 484).

Le défaut : cliquer une équipe depuis « Mes équipes » ne changeait rien à
l'écran, et il fallait rafraîchir pour que la fiche apparaisse. La route rendait
un fragment quand elle voyait `HX-Request` — un en-tête vrai d'une **navigation**
htmx comme d'un échange d'onglet, donc incapable de les distinguer. Les
appelants sélectionnent `#app-content`, le fragment ne le portait pas, htmx
n'échangeait rien.

# Pourquoi aucun test unitaire ne pouvait le voir

Ils appellent la route directement, ou cliquent des onglets **déjà présents dans
la page**. Aucun n'entrait dans la fiche par un clic venu d'ailleurs — et c'est
tout le défaut : la fiche se rendait correctement, seuls ses appelants ne
pouvaient rien en extraire. Deux jours en production sous une suite verte.

# Ce que ces scénarios couvrent

Deux points d'entrée sur les cinq, choisis pour être de natures différentes :
une carte de liste et un lien de retour depuis un autre BC. Les trois autres —
carte archivée, ligne de classement, ligne de classement détaillé — partagent
exactement le même patron `hx-select="#app-content"`.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) et base seedée.
"""

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from htmx_helpers import cliquer_quand_cable


@pytest.fixture(scope="module")
def fiche_ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    equipe = full["team_ids"][0]
    nom = query_db(f"SELECT team_name FROM team_proj WHERE team_id = '{equipe}'")[0]
    joueurs = query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{equipe}' "
        "AND membership = 'Active' ORDER BY player_id"
    )
    assert joueurs, f"aucun joueur dans {equipe}"
    return {"space_id": space_id, "equipe": equipe, "nom": nom, "joueur": joueurs[0]}


def test_cliquer_une_equipe_depuis_mes_equipes_l_affiche(page: Page, fiche_ctx):
    """**Le clic de l'utilisateur, tel qu'il l'a décrit.**

    L'assertion porte sur ce qui a changé à l'écran **sans rechargement** : c'est
    la seule chose que le défaut rendait fausse. La fiche s'affichait très bien
    après un `F5`.
    """
    page.goto(f"{BASE_URL}/app/{fiche_ctx['space_id']}/team/list", wait_until="load")

    carte = page.locator(".team-card, .archived-card").filter(has_text=fiche_ctx["nom"]).first
    expect(carte).to_be_visible(timeout=10000)
    cliquer_quand_cable(page, f".team-card:has-text('{fiche_ctx['nom']}')")

    # L'en-tête de la fiche, que la page « Mes équipes » ne porte pas.
    expect(page.locator(".team-header-name")).to_have_text(fiche_ctx["nom"], timeout=10000)
    expect(page.locator(".team-tabs")).to_be_visible()
    assert page.url.endswith(f"/teams/{fiche_ctx['equipe']}"), (
        f"hx-push-url n'a pas suivi : {page.url}"
    )


def test_le_retour_depuis_une_fiche_joueur_affiche_l_equipe(page: Page, fiche_ctx):
    """Un second point d'entrée, et d'une autre nature : un lien de retour posé
    par le BC `players`, pas une carte de liste."""
    page.goto(
        f"{BASE_URL}/app/{fiche_ctx['space_id']}/players/{fiche_ctx['joueur']}/detail",
        wait_until="load",
    )
    expect(page.locator(".player-page")).to_be_visible(timeout=10000)

    cliquer_quand_cable(page, ".player-detail-back")

    expect(page.locator(".team-header-name")).to_have_text(fiche_ctx["nom"], timeout=10000)
    expect(page.locator(".team-tabs")).to_be_visible()


def test_les_onglets_continuent_de_fonctionner_apres_le_clic(page: Page, fiche_ctx):
    """La contre-épreuve de la correction.

    Rendre toujours la page entière fait marcher l'entrée dans la fiche ; encore
    faut-il que la barre d'onglets sache en extraire ce qu'elle veut. Sans ce
    test, on aurait pu réparer le clic en cassant les onglets.
    """
    page.goto(f"{BASE_URL}/app/{fiche_ctx['space_id']}/team/list", wait_until="load")
    cliquer_quand_cable(page, f".team-card:has-text('{fiche_ctx['nom']}')")
    expect(page.locator(".team-header-name")).to_be_visible(timeout=10000)

    cliquer_quand_cable(page, ".team-tabs a:has-text('Trésorerie')")
    expect(page.locator(".team-treasury")).to_be_visible(timeout=10000)
    expect(page.locator(".team-tabs .tab.active")).to_have_text("Trésorerie")
    # Un seul `#team-tab-zone` dans le DOM : `hx-select` retient l'enveloppe, et
    # un `innerHTML` l'aurait nichée dans elle-même.
    expect(page.locator("#team-tab-zone")).to_have_count(1)

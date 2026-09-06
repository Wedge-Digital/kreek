"""Les mots-clefs du poste s'affichent sur les deux écrans (carte 405).

Deux vérifications qu'aucun test unitaire ne peut faire : le rendu réel des
deux écrans.

La couleur du badge de trait est vérifiée dans `test_haine_journalier`, là où la
Haine est créée — un test qui constate une couleur doit produire la donnée qui
la porte, sinon il se contente de sauter.

Le corpus de démonstration donne à chaque poste son rôle et son espèce
(carte 399) — « Trois-quart, Nain » pour la piétaille des Granitiers.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import pytest
from playwright.sync_api import Page, expect

from db_helpers import query_db

BASE_URL = "http://localhost:3210"


@pytest.fixture(scope="module")
def joueur_de_demo():
    """Un joueur dont le poste porte des mots-clefs, et son équipe."""
    lignes = query_db(
        "SELECT p.player_id, p.team_id, t.space_id, p.roster_line_id "
        "FROM players_proj p JOIN team_proj t ON t.team_id = p.team_id "
        "WHERE p.roster_line_id LIKE 'DEMO\\_%' LIMIT 1"
    )
    assert lignes, "aucun joueur sur un poste de démonstration en base"
    player_id, team_id, space_id, poste = lignes[0].split("|")
    return {"player_id": player_id, "team_id": team_id, "space_id": space_id, "poste": poste}


# Les mots-clefs posés sur les rosters de démonstration (carte 399), en libellés.
LIBELLES_ATTENDUS = {
    "DEMO_GRANIT__PIETAILLE": "Trois-quart, Nain",
    "DEMO_GRANIT__PERCUTEUR": "Blitzer, Nain",
    "DEMO_GRANIT__COLOSSE": "Gros Bras, Nain",
    "DEMO_ZEPHYR__PIETAILLE": "Trois-quart, Elfe",
    "DEMO_ZEPHYR__RECEVEUR": "Receveur, Elfe",
    "DEMO_ZEPHYR__LANCEUR": "Lanceur, Elfe",
    "DEMO_LANTERNE__PIETAILLE": "Trois-quart, Skaven",
    "DEMO_LANTERNE__RODEUR": "Coureur, Skaven",
    "DEMO_LANTERNE__MUTANT": "Spécial, Skaven",
}


def test_le_tableau_d_equipe_affiche_les_mots_clefs(page: Page, joueur_de_demo):
    """La ligne **de ce joueur** porte ses mots-clefs.

    Le test visait `.player-keywords` **`.first`**, c'est-à-dire la première
    ligne du tableau — en affirmant qu'elle portait les mots-clefs du joueur que
    le fixture avait tiré. Deux ordres indépendants, supposés coïncider : celui
    des lignes en base, que le `LIMIT 1` sans `ORDER BY` du fixture suit, et
    celui de l'affichage, qui trie par `display_order` puis par maillot. Rien ne
    les lie, et n'importe quelle carte écrivant dans `players_proj` faisait
    basculer le premier.

    Il passait seul et tombait en suite. La ligne porte son identifiant, on la
    vise donc directement — le tirage du fixture peut rester variable, il
    devient inoffensif, et fait même tourner le test sur des postes différents.
    """
    j = joueur_de_demo
    page.goto(f"{BASE_URL}/app/{j['space_id']}/teams/{j['team_id']}", wait_until="load")
    ligne = page.locator(f'.player-table-row[data-player-detail*="{j["player_id"]}"]')
    ligne.wait_for(timeout=10000)
    expect(ligne.locator(".player-keywords")).to_have_text(LIBELLES_ATTENDUS[j["poste"]])


def test_la_fiche_joueur_affiche_les_mots_clefs(page: Page, joueur_de_demo):
    j = joueur_de_demo
    page.goto(
        f"{BASE_URL}/app/{j['space_id']}/players/{j['player_id']}/detail", wait_until="load"
    )
    mots = page.locator(".player-keywords").first
    mots.wait_for(timeout=10000)
    expect(mots).to_have_text(LIBELLES_ATTENDUS[j["poste"]])

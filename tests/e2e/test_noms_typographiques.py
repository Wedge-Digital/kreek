"""Les noms sortent de la liste blanche — carte 329.

La règle de nommage n'énumère plus les caractères **autorisés** mais ceux qui
sont **refusés** : contrôles, séparateurs de ligne Unicode, overrides
bidirectionnels. Apostrophes, esperluettes, chevrons et emoji passent.

Ce fichier vérifie ce qu'aucun test unitaire ne peut voir : qu'un tel nom
traverse la création, l'enrôlement **et le rendu** sans être altéré. C'est le
rendu qui comptait — le sélecteur d'équipes du calendrier interpolait les noms
dans un `<script>`, où le navigateur ne décode pas les entités HTML, si bien
que « L'Ost » s'y affichait « L&#x27;Ost ».

Prérequis : serveur kreek lancé en dev.
"""

import json

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import (
    BASE_URL,
    build_and_submit_team_http,
    build_full_competition,
)
from db_helpers import query_db

# Une apostrophe, une esperluette, des chevrons et un emoji : chacun était
# refusé par au moins un des onze charsets d'origine.
NOM_TYPOGRAPHIQUE = "L'Ost & Cie <Étoilé> 🏈"


@pytest.fixture(scope="module")
def ctx_typographique(browser, space_id):
    """Une compétition dédiée, avec une équipe au nom éprouvant."""
    ctx = build_full_competition(browser, space_id, num_teams=2)
    page = browser.new_page()
    try:
        coachs = query_db(
            f"SELECT coach_id FROM spaces__user_space WHERE space_id = '{space_id}' LIMIT 3"
        )
        team_id = build_and_submit_team_http(
            page,
            space_id,
            ctx["competition_id"],
            ctx["season_id"],
            coachs[-1],
            roster_index=0,
            team_name=NOM_TYPOGRAPHIQUE,
        )
    finally:
        page.close()
    return {**ctx, "space_id": space_id, "team_id": team_id}


def test_le_nom_traverse_la_creation_sans_alteration(ctx_typographique):
    """Le premier maillon : la base porte le nom tel qu'il a été saisi."""
    noms = query_db(
        "SELECT team_name FROM team_proj "
        f"WHERE team_id = '{ctx_typographique['team_id']}'"
    )
    assert noms == [NOM_TYPOGRAPHIQUE], f"nom altéré à la création : {noms}"


def test_le_selecteur_du_calendrier_rend_le_nom_intact(page: Page, ctx_typographique):
    """Le maillon qui était cassé.

    L'assertion porte sur le nom **après** `JSON.parse`, dans le navigateur :
    c'est le seul endroit où l'on constate que l'attribut a bien été décodé.
    Lire le HTML brut ne prouverait rien — les entités y sont attendues.
    """
    c = ctx_typographique
    page.goto(
        f"{BASE_URL}/app/{c['space_id']}/competitions/{c['competition_id']}"
        f"/{c['season_id']}/admin/schedule",
        wait_until="load",
    )
    detail = requests.get(
        f"{BASE_URL}/app/{c['space_id']}/competitions/{c['competition_id']}"
        f"/{c['season_id']}/admin/schedule/round",
        params={"round_id": c["round_ids"][0]},
        headers={"HX-Request": "true"},
    )
    assert detail.status_code == 200, f"widget round-detail : {detail.status_code}"

    # Le nom ne doit plus être interpolé dans le script : il vit en attribut.
    assert "data-teams=" in detail.text, "les équipes doivent voyager en attribut"

    noms = page.evaluate(
        """(html) => {
             const d = document.createElement('div');
             d.innerHTML = html;
             const el = d.querySelector('.add-match-teams');
             return JSON.parse(el.dataset.teams).map(t => t.text);
           }""",
        detail.text,
    )
    assert NOM_TYPOGRAPHIQUE in noms, (
        f"le nom doit survivre au décodage du navigateur — lu : {noms}"
    )


def test_le_nom_s_affiche_sur_la_fiche_d_equipe(page: Page, ctx_typographique):
    c = ctx_typographique
    page.goto(
        f"{BASE_URL}/app/{c['space_id']}/teams/{c['team_id']}", wait_until="load"
    )
    expect(page.get_by_text(NOM_TYPOGRAPHIQUE).first).to_be_visible(timeout=10000)

"""Les blessures persistantes sur la feuille d'équipe (carte 568).

Une blessure persistante naît d'une blessure sérieuse, et d'elle seule. La
colonne « BP » de la feuille d'équipe en donne le nombre, lu dans
`players_proj.persistent_injuries` — recalculé depuis l'agrégat dans la
transaction de chaque blessure et de chaque compensation de match.

**Ce que ce fichier prouve et qu'aucun test unitaire ne peut voir** : que la
chaîne entière tient — rapport publié, app event, projection, rendu — et que la
dépublication du rapport efface le compteur à l'écran.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import re
import time

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    post_step5,
    publish,
)


def _blesser(space_id: str, mr_id: str, victime: str, injury_type: str) -> None:
    """Une blessure subie par un joueur de l'équipe **domicile** (`step4`)."""
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step4/actions",
        data={
            "turn": "3",
            "player_id": victime,
            "player_type": "regular",
            "action_type": "BLESSE",
            "injury_type": injury_type,
        },
    )
    assert resp.status_code == 200, f"blessure : {resp.status_code}\n{resp.text[:200]}"


def _compteur(player_id: str) -> int:
    rows = query_db(
        f"SELECT persistent_injuries FROM players_proj WHERE player_id = '{player_id}'"
    )
    return int(rows[0])


def _attendre_compteur(player_id: str, attendu: int, timeout_s: int = 25) -> None:
    """Les impacts joueur transitent par l'app event bus : la projection n'est
    pas à jour au retour de la publication."""
    deadline = time.time() + timeout_s
    dernier = None
    while time.time() < deadline:
        dernier = _compteur(player_id)
        if dernier == attendu:
            return
        time.sleep(0.3)
    raise AssertionError(f"BP attendu {attendu} pour {player_id}, obtenu {dernier}")


def _feuille(space_id: str, team_id: str) -> str:
    return f"{BASE_URL}/app/{space_id}/teams/{team_id}"


def _cellule_bp(page: Page, player_id: str):
    ligne = page.locator(f'tr.player-table-row[data-player-detail*="{player_id}"]')
    return ligne.locator("td.player-bp")


@pytest.fixture(scope="module")
def match_avec_blesses(browser, space_id):
    """Un match publié : un joueur domicile subit une blessure sérieuse, un
    autre un « amoché »."""
    ctx = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    domicile, exterieur = ctx["team_ids"][0], ctx["team_ids"][1]
    round_id = ctx["round_ids"][0]

    mr_id = create_draft(space_id, ctx, round_id, domicile, exterieur)
    serieux, amoche = query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{domicile}' "
        "ORDER BY player_id LIMIT 2"
    )
    ensure_pre_match(space_id, mr_id, ctx, round_id, domicile, exterieur)
    ensure_inducements(space_id, mr_id)
    _blesser(space_id, mr_id, serieux, "BLESSURE_SERIEUSE")
    _blesser(space_id, mr_id, amoche, "AMOCHE")
    post_step5(space_id, mr_id)
    publish(space_id, mr_id)
    _attendre_compteur(serieux, 1)

    return {
        "space_id": space_id,
        "team_id": domicile,
        "mr_id": mr_id,
        "serieux": serieux,
        "amoche": amoche,
    }


def test_la_blessure_serieuse_compte_une_blessure_persistante(page: Page, match_avec_blesses):
    ctx = match_avec_blesses
    page.goto(_feuille(ctx["space_id"], ctx["team_id"]), wait_until="load")

    expect(page.locator('.player-table th.player-col-bp')).to_have_text("BP")
    expect(_cellule_bp(page, ctx["serieux"])).to_have_text("1")
    expect(_cellule_bp(page, ctx["serieux"])).to_have_class(re.compile(r"\bplayer-bp--some\b"))
    # L'« amoché » ne laisse aucune trace durable.
    expect(_cellule_bp(page, ctx["amoche"])).to_have_text("—")


def test_le_tiret_du_pied_est_sous_la_colonne_bp(page: Page, match_avec_blesses):
    ctx = match_avec_blesses
    page.set_viewport_size({"width": 1440, "height": 900})
    page.goto(_feuille(ctx["space_id"], ctx["team_id"]), wait_until="load")
    page.locator("tfoot .player-bp.player-foot-dash").wait_for(timeout=15000)

    positions = page.evaluate(
        """() => {
             const x = s => Math.round(document.querySelector(s).getBoundingClientRect().x);
             return { entete: x('.player-table th.player-col-bp'),
                      pied: x('.player-table tfoot .player-bp.player-foot-dash') };
           }"""
    )
    assert positions["pied"] == positions["entete"], f"tiret décalé : {positions}"


def test_depublier_le_rapport_efface_la_blessure_persistante(page: Page, match_avec_blesses):
    """En dernier : il défait ce que les autres tests du module lisent."""
    ctx = match_avec_blesses
    resp = requests.post(
        f"{BASE_URL}/app/{ctx['space_id']}/match-report/{ctx['mr_id']}/recap/unpublish",
        allow_redirects=False,
    )
    assert resp.status_code == 200, f"dépublication : {resp.status_code}"
    _attendre_compteur(ctx["serieux"], 0)

    page.goto(_feuille(ctx["space_id"], ctx["team_id"]), wait_until="load")
    expect(_cellule_bp(page, ctx["serieux"])).to_have_text("—")

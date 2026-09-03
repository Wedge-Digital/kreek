"""Tests E2E — un joueur mort quitte l'effectif visible (carte 488).

La mort est terminale, contrairement à la blessure sérieuse que le listener
post-match relève au match suivant (BR12, carte 225). Un mort ne doit donc plus
figurer là où on choisit, aligne ou compte des joueurs — et il doit rendre sa
place, sans quoi elle resterait bloquée pour toujours.

Le test éprouve les deux moitiés de la carte d'un seul geste :

1. le mort disparaît du tableau de la fiche d'équipe ;
2. sa place est rendue — l'effectif compté par le recrutement baisse d'un.

Et il vérifie surtout la **règle symétrique**, celle qui interdisait de filtrer
sur « alignable au prochain match » : un blessé du même match reste affiché et
garde sa place. C'est cette assertion-là qui échouerait si quelqu'un
reconfondait un jour les deux cas.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) — cf. README.
"""

import time

import pytest
import requests

from competition_lifecycle import BASE_URL
from db_helpers import query_db
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    post_step5,
    publish,
)


def _record_injury(space_id: str, mr_id: str, victim_id: str, injury: str) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step4/actions",
        data={
            "turn": "3",
            "player_id": victim_id,
            "player_type": "regular",
            "action_type": "BLESSE",
            "injury_type": injury,
        },
    )
    assert resp.status_code == 200, f"{injury} : {resp.status_code}\n{resp.text[:200]}"


def _participation_status(player_id: str) -> str | None:
    rows = query_db(
        f"SELECT participation_status FROM players_proj WHERE player_id = '{player_id}'"
    )
    return rows[0] if rows else None


def _wait_status(player_id: str, expected: str, timeout_s: int = 25) -> None:
    """Les impacts joueur transitent par l'app event bus : le statut n'est pas
    à jour au retour de la requête de publication."""
    deadline = time.time() + timeout_s
    last = None
    while time.time() < deadline:
        last = _participation_status(player_id)
        if last == expected:
            return
        time.sleep(0.3)
    raise AssertionError(
        f"statut attendu « {expected} » pour {player_id}, obtenu « {last} » après {timeout_s}s"
    )


def _players_widget(space_id: str, team_id: str) -> str:
    """Le tableau de l'onglet Joueurs — un widget du BC `players`, celui-là
    même que la fiche d'équipe charge en `hx-get`."""
    resp = requests.get(f"{BASE_URL}/app/{space_id}/players/by-team/{team_id}/widget")
    assert resp.status_code == 200, f"widget joueurs : {resp.status_code}"
    return resp.text


def _deux_premiers_joueurs(team_id: str) -> tuple[str, str]:
    rows = query_db(
        "SELECT player_id FROM players_proj "
        f"WHERE team_id = '{team_id}' AND membership = 'Active' "
        "ORDER BY player_id LIMIT 2"
    )
    assert len(rows) == 2, f"il faut deux joueurs dans {team_id}, trouvé {rows}"
    return rows[0], rows[1]


@pytest.fixture(scope="module")
def deces_ctx(browser, space_id):
    from competition_lifecycle import build_full_competition

    full = build_full_competition(browser, space_id, num_teams=12)
    return {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_ids": full["round_ids"],
        "teams": full["team_ids"],
    }


def test_un_mort_quitte_la_fiche_et_rend_sa_place(space_id, deces_ctx):
    ctx = deces_ctx
    # Indices 9/10 : paire réservée à ce module, les autres servent ailleurs.
    equipe = ctx["teams"][9]
    adverse = ctx["teams"][10]
    mort, blesse = _deux_premiers_joueurs(equipe)

    effectif_avant = len(
        query_db(
            f"SELECT player_id FROM players_proj WHERE team_id = '{equipe}' "
            "AND membership = 'Active'"
        )
    )

    mr = create_draft(space_id, ctx, ctx["round_ids"][0], equipe, adverse)
    ensure_pre_match(space_id, mr, ctx, ctx["round_ids"][0], equipe, adverse)
    ensure_inducements(space_id, mr)
    _record_injury(space_id, mr, mort, "MORT")
    _record_injury(space_id, mr, blesse, "BLESSURE_SERIEUSE")
    post_step5(space_id, mr)
    publish(space_id, mr)

    _wait_status(mort, "Dead")
    _wait_status(blesse, "MissingNextGame")

    # ── 1. la fiche d'équipe ──────────────────────────────────────────────────
    tableau = _players_widget(space_id, equipe)
    assert mort not in tableau, "un joueur mort ne s'affiche plus dans l'effectif"
    assert blesse in tableau, (
        "un blessé reste affiché : il revient au match suivant, et c'est la "
        "règle qui interdit de filtrer sur « alignable »"
    )

    # ── 2. l'appartenance n'a pas changé : la mort n'est pas un renvoi ────────
    encore_membre = query_db(
        f"SELECT membership FROM players_proj WHERE player_id = '{mort}'"
    )
    assert encore_membre == ["Active"], (
        "la mort agit sur le statut de participation, pas sur l'appartenance — "
        "l'histoire du joueur reste rattachée à son équipe"
    )

    # ── 3. la place est rendue ────────────────────────────────────────────────
    occupants = len(
        query_db(
            f"SELECT player_id FROM players_proj WHERE team_id = '{equipe}' "
            "AND membership = 'Active' AND participation_status <> 'Dead'"
        )
    )
    assert occupants == effectif_avant - 1, (
        "le mort ne compte plus parmi les occupants : c'est sa place, rendue "
        "au plafond de seize et au quota de son poste"
    )

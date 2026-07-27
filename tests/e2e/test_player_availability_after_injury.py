"""Tests E2E — disponibilité d'un joueur blessé en match (carte 225).

Reproduit le scénario complet en base réelle, à travers le vrai pipeline
d'app events :

1. Un joueur de l'équipe A subit une blessure sérieuse pendant le match N
2. Le rapport est publié
3. Le joueur doit être **absent au prochain match**
4. L'équipe A joue et publie le match N+1
5. Le joueur redevient **disponible** — après ce match-là, pas avant

Le bug corrigé par cette carte faisait échouer l'étape 3 : la conclusion du
match N restaurait la disponibilité du joueur qu'il venait lui-même de
blesser, annulant l'effet « absent au prochain match » pour toutes les
blessures subies en jeu.

Le statut est lu directement dans `players_proj` : c'est la projection que
consomment les écrans, et une désynchronisation entre agrégat et projection
passerait inaperçue si on interrogeait l'event store.

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
    first_player_id,
    post_step5,
    publish,
)


def _record_injury(space_id: str, mr_id: str, victim_id: str) -> None:
    """Enregistre une blessure sérieuse subie par un joueur de l'équipe
    **domicile**, infligée depuis le camp adverse (`step4`)."""
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step4/actions",
        data={
            "turn": "3",
            "player_id": victim_id,
            "player_type": "regular",
            "action_type": "BLESSE",
            "injury_type": "BLESSURE_SERIEUSE",
        },
    )
    assert resp.status_code == 200, f"blessure : {resp.status_code}\n{resp.text[:200]}"


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


@pytest.fixture(scope="module")
def availability_ctx(browser, space_id):
    from competition_lifecycle import build_full_competition

    full = build_full_competition(browser, space_id, num_teams=12)
    return {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_ids": full["round_ids"],
        "teams": full["team_ids"],
    }


def _play_and_publish(space_id, ctx, round_id, home_idx, away_idx, *, injure_home_player=None):
    home_team_id = ctx["teams"][home_idx]
    away_team_id = ctx["teams"][away_idx]
    mr_id = create_draft(space_id, ctx, round_id, home_team_id, away_team_id)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home_team_id, away_team_id)
    ensure_inducements(space_id, mr_id)

    if injure_home_player:
        _record_injury(space_id, mr_id, injure_home_player)

    post_step5(space_id, mr_id)
    publish(space_id, mr_id)
    return mr_id


def test_un_joueur_blesse_en_match_manque_le_match_suivant(space_id, availability_ctx):
    ctx = availability_ctx
    # Indices 6/7 puis 6/8 : l'équipe 6 joue les deux matchs, les autres paires
    # sont réservées aux autres modules de test.
    premier_round = ctx["round_ids"][0]
    second_round = ctx["round_ids"][1]

    mr1 = create_draft(space_id, ctx, premier_round, ctx["teams"][6], ctx["teams"][7])
    victime = first_player_id(mr1, "home")

    # ── Match N : le joueur est blessé ────────────────────────────────────────
    ensure_pre_match(space_id, mr1, ctx, premier_round, ctx["teams"][6], ctx["teams"][7])
    ensure_inducements(space_id, mr1)
    _record_injury(space_id, mr1, victime)
    post_step5(space_id, mr1)
    publish(space_id, mr1)

    # C'est l'assertion qui échouait avant le correctif de la carte 225 : la
    # conclusion du match N remettait le joueur disponible aussitôt blessé.
    _wait_status(victime, "MissingNextGame")

    # ── Match N+1 : le joueur redevient disponible ────────────────────────────
    _play_and_publish(space_id, ctx, second_round, home_idx=6, away_idx=8)

    _wait_status(victime, "Available")

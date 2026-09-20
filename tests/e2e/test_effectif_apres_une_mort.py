"""Tests E2E — les places d'un joueur mort sont libres (carte 559).

Scénarios :
- un joueur meurt en match ; « Modifier l'effectif » accepte ensuite de donner
  son numéro à un coéquipier ;
- la même modification visant le numéro d'un vivant reste refusée.

Tout passe par HTTP : le formulaire d'édition est celui que l'écran envoie,
et un refus métier répond 200 avec le fragment qui porte le message — c'est
ce que HTMX doit pouvoir swapper pour que le coach retrouve sa saisie.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import pytest
import requests

from competition_lifecycle import build_full_competition
from db_helpers import attendre_que, query_db
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    first_player_id,
    post_step5,
    publish,
)

BASE_URL = "http://localhost:3210"
HTMX = {"HX-Request": "true"}


@pytest.fixture(scope="module")
def ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=4)
    return {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_ids": full["round_ids"],
        "teams": full["team_ids"],
    }


def _tuer(space_id, mr_id, side, player_id, turn):
    """Une blessure `MORT` sur ce joueur : l'action que l'écran de saisie envoie."""
    endpoint = "step3" if side == "home" else "step4"
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{endpoint}/actions",
        data={
            "turn": str(turn),
            "player_id": player_id,
            "player_type": "regular",
            "action_type": "BLESSE",
            "injury_type": "MORT",
        },
    )
    assert resp.status_code == 200, f"blessure : {resp.status_code}\n{resp.text[:200]}"


@pytest.fixture(scope="module")
def mort(space_id, ctx):
    """Un match publié entre teams[0] et teams[1], où un joueur de teams[1]
    meurt. Rend l'équipe endeuillée, le mort, et le numéro qu'il portait."""
    home, away = ctx["teams"][0], ctx["teams"][1]
    round_id = ctx["round_ids"][0]
    mr_id = create_draft(space_id, ctx, round_id, home, away)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home, away)
    ensure_inducements(space_id, mr_id)

    victime = first_player_id(mr_id, "away")
    _tuer(space_id, mr_id, "away", victime, turn=1)
    post_step5(space_id, mr_id)
    publish(space_id, mr_id)

    attendre_que(
        lambda: query_db(
            f"SELECT participation_status FROM players_proj WHERE player_id = '{victime}'"
        ) == ["Dead"],
        quoi="la mort du joueur dans la projection",
    )
    numero = query_db(f"SELECT jersey FROM players_proj WHERE player_id = '{victime}'")[0]
    assert numero, "le mort portait un numéro"
    return {"team_id": away, "player_id": victime, "jersey": int(numero)}


def _effectif_vivant(team_id):
    """Les vivants, dans l'ordre de l'écran : `(player_id, jersey)`."""
    lignes = query_db(
        "SELECT player_id || '|' || coalesce(jersey::text, '') FROM players_proj "
        f"WHERE team_id = '{team_id}' AND membership <> 'Dismissed' "
        "AND participation_status <> 'Dead' "
        "ORDER BY display_order NULLS LAST, jersey NULLS LAST, player_id"
    )
    return [tuple(l.split("|")) for l in lignes]


def _poster_l_effectif(space_id, team_id, lignes):
    """Le formulaire tel que l'écran l'envoie : trois tableaux alignés, l'ordre
    des lignes valant rang d'affichage."""
    data = []
    for player_id, jersey in lignes:
        data.append(("player_id", player_id))
        data.append(("personal_name", ""))
        data.append(("jersey", jersey))
    return requests.post(
        f"{BASE_URL}/app/{space_id}/players/by-team/{team_id}/roster",
        data=data,
        headers=HTMX,
    )


def test_le_numero_d_un_mort_peut_etre_repris(space_id, mort):
    lignes = _effectif_vivant(mort["team_id"])
    assert mort["player_id"] not in [p for p, _ in lignes], "le mort n'est plus à l'écran"

    # Le premier vivant prend le numéro du mort.
    heritier = lignes[0][0]
    lignes[0] = (heritier, str(mort["jersey"]))

    resp = _poster_l_effectif(space_id, mort["team_id"], lignes)

    assert resp.status_code == 200, f"{resp.status_code}\n{resp.text[:300]}"
    assert "portent le même numéro" not in resp.text
    assert query_db(f"SELECT jersey FROM players_proj WHERE player_id = '{heritier}'") == [
        str(mort["jersey"])
    ]


def test_le_numero_d_un_vivant_reste_refuse(space_id, mort):
    lignes = _effectif_vivant(mort["team_id"])
    vivants_numerotes = [(p, j) for p, j in lignes if j]
    assert len(vivants_numerotes) >= 2, "il faut deux vivants numérotés"
    (premier, _), (second, numero_du_second) = vivants_numerotes[0], vivants_numerotes[1]

    # Le premier vivant tente de prendre le numéro d'un autre vivant.
    lignes = [(p, numero_du_second if p == premier else j) for p, j in lignes]
    resp = _poster_l_effectif(space_id, mort["team_id"], lignes)

    assert resp.status_code == 200, f"{resp.status_code}\n{resp.text[:300]}"
    assert "portent le même numéro" in resp.text
    assert query_db(f"SELECT jersey FROM players_proj WHERE player_id = '{second}'") == [
        numero_du_second
    ], "un refus n'écrit rien"

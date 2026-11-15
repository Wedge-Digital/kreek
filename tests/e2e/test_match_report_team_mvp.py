"""Le MVP de son camp (carte 657, épic E22).

En « Chacun son équipe », chaque coach désigne ou tire le MVP de son équipe
sans attendre l'autre, et voit où en est l'adversaire.

Scénarios :
- M1 — Le membre simple, coach du visiteur, ouvre son étape MVP avant que le
       domicile ait le sien : pas de renvoi. Il tire son MVP ; la page de
       DevCoach, sur l'étape du domicile, voit l'état du visiteur passer de
       « en cours » au MVP tiré, sans rechargement.
- M2 — Le membre simple qui poste dans `mvp/home` ou `mvp/home/draw` reçoit
       403 : la garde à camp.

Le mode B se démarre par un POST direct (`entry_mode=TeamEntry`) : l'option
reste grisée jusqu'à la carte 659. DevCoach coache le domicile, le membre
simple (`X-Bypass-Auth-Profile: simple`) l'extérieur.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

import pytest
import requests
from playwright.sync_api import expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import (
    PROFIL_SIMPLE,
    create_draft,
    ensure_team_entry_inducements,
    first_player_id,
)
from realtime_helpers import DIRECT_TIMEOUT_MS, goto_declared

TIRAGE = 6
WITNESS = "() => { window.__sansRechargement = true; }"
STILL_THERE = "() => window.__sansRechargement === true"
OPPONENT = ".mvp-opponent .opponent-mvp-row"


@pytest.fixture(scope="module")
def competition(browser, space_id):
    return build_full_competition(browser, space_id, num_teams=2, num_rounds=2)


def _membre():
    session = requests.Session()
    session.headers.update(PROFIL_SIMPLE)
    return session


def _kicked_off(space_id, ctx, round_index):
    """Un match en « Chacun son équipe », coup d'envoi donné."""
    domicile, exterieur = ctx["team_ids"][0], ctx["team_ids"][1]
    mr_id = create_draft(
        space_id, ctx, ctx["round_ids"][round_index], domicile, exterieur, entry_mode="TeamEntry"
    )
    ensure_team_entry_inducements(space_id, mr_id)
    return mr_id


def _url(space_id, mr_id, suffix):
    return f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{suffix}"


def _designated(mr_id, side):
    team = query_db(
        f"SELECT {side}_team_id FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )[0]
    players = query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{team}' "
        f"AND membership = 'Active' ORDER BY player_id LIMIT {TIRAGE}"
    )
    assert len(players) == TIRAGE, f"{TIRAGE} joueurs attendus : {players}"
    return [("player_id", p) for p in players] + [("player_type", "regular")] * TIRAGE


def _draw(session, space_id, mr_id, side):
    return session.post(
        _url(space_id, mr_id, f"mvp/{side}/draw"),
        data=_designated(mr_id, side),
        headers={"HX-Request": "true"},
        allow_redirects=False,
    )


def test_chacun_tire_son_mvp_et_voit_l_autre(browser, space_id, competition):
    mr_id = _kicked_off(space_id, competition, 0)

    membre = browser.new_context(extra_http_headers=PROFIL_SIMPLE).new_page()
    membre.goto(_url(space_id, mr_id, "mvp/away"), wait_until="load")
    assert membre.url.endswith(f"/{mr_id}/mvp/away"), membre.url
    expect(membre.locator(".mr-banner-label")).to_have_text("Votre équipe")
    expect(membre.get_by_role("link", name="← Match")).to_have_attribute(
        "href", f"/app/{space_id}/match-report/{mr_id}/live"
    )

    devcoach = browser.new_context().new_page()
    goto_declared(devcoach, _url(space_id, mr_id, "mvp/home"))
    expect(devcoach.locator(OPPONENT)).to_contain_text("en cours")
    devcoach.evaluate(WITNESS)

    tirage = _draw(_membre(), space_id, mr_id, "away")
    assert tirage.status_code == 200, f"obtenu {tirage.status_code} : {tirage.text[:200]}"

    expect(devcoach.locator(OPPONENT)).to_contain_text("MVP :", timeout=DIRECT_TIMEOUT_MS)
    expect(devcoach.locator(OPPONENT)).to_contain_text("tiré au sort parmi")
    assert devcoach.evaluate(STILL_THERE), "la page de DevCoach s'est rechargée"


def test_le_mvp_adverse_est_refuse(space_id, competition):
    mr_id = _kicked_off(space_id, competition, 1)
    membre = _membre()

    saisie = membre.post(
        _url(space_id, mr_id, "mvp/home"),
        data={"player_id": first_player_id(mr_id, "home"), "player_type": "regular"},
        headers={"HX-Request": "true"},
        allow_redirects=False,
    )
    assert saisie.status_code == 403, f"obtenu {saisie.status_code}"

    tirage = _draw(membre, space_id, mr_id, "home")
    assert tirage.status_code == 403, f"obtenu {tirage.status_code}"

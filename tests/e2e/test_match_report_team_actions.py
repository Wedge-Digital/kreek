"""Les actions de son camp (carte 656, épic E22).

En « Chacun son équipe », chaque coach saisit et efface les actions de son
équipe seule, et voit celles de l'autre en direct. Le match ne commence qu'une
fois les deux fans factors et les coups de pouce enregistrés.

Scénarios :
- C1 — Deux navigateurs sur `/live` : le membre simple n'a qu'un bouton, celui
       de son équipe ; DevCoach, admin de l'espace, garde le choix d'équipe
       (§ 12.6). Chacun voit l'action de l'autre sans rechargement, avec son
       auteur ; le ✕ du membre n'est que sur ses lignes, celui de l'admin sur
       toutes.
- C2 — Le membre simple qui poste dans `step3/actions` ou efface une action du
       domicile reçoit 403 : la garde à camp fixe, puis le domaine.
- C3 — Avant le coup d'envoi, `/live` et `step4` renvoient le membre à son
       avant-match, et une action postée est refusée par le domaine (422).

Le mode B se démarre par un POST direct (`entry_mode=TeamEntry`) : l'option
reste grisée jusqu'à la carte 659. DevCoach coache le domicile, le membre
simple (`X-Bypass-Auth-Profile: simple`) l'extérieur.

**Sans rechargement** : un témoin posé sur `window` survit au remplacement d'un
widget, pas à un chargement de page.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

import re

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
    record_action_api,
)
from realtime_helpers import DIRECT_TIMEOUT_MS, open_live

WITNESS = "() => { window.__sansRechargement = true; }"
STILL_THERE = "() => window.__sansRechargement === true"
HOME_ROW = ".mr-log-entry--home"
AWAY_ROW = ".mr-log-entry--away"
CROSS = ".mr-log-delete"


@pytest.fixture(scope="module")
def competition(browser, space_id):
    return build_full_competition(browser, space_id, num_teams=2, num_rounds=3)


def _membre():
    session = requests.Session()
    session.headers.update(PROFIL_SIMPLE)
    return session


def _match_b(space_id, ctx, round_index):
    domicile, exterieur = ctx["team_ids"][0], ctx["team_ids"][1]
    return create_draft(
        space_id, ctx, ctx["round_ids"][round_index], domicile, exterieur, entry_mode="TeamEntry"
    )


def _kicked_off(space_id, ctx, round_index):
    """Un match en « Chacun son équipe », coup d'envoi donné."""
    mr_id = _match_b(space_id, ctx, round_index)
    ensure_team_entry_inducements(space_id, mr_id)
    return mr_id


def _url(space_id, mr_id, suffix):
    return f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{suffix}"


def _action_of(mr_id, side):
    rows = query_db(
        f"SELECT action_id FROM match_report_actions WHERE match_report_id = '{mr_id}' "
        f"AND team_side = '{side}' AND NOT is_deleted LIMIT 1"
    )
    assert rows, f"aucune action {side} dans {mr_id}"
    return rows[0]


def test_chacun_saisit_son_camp_et_voit_l_autre_en_direct(browser, space_id, competition):
    mr_id = _kicked_off(space_id, competition, 0)
    membre = open_live(browser, space_id, mr_id, PROFIL_SIMPLE)
    devcoach = open_live(browser, space_id, mr_id)

    expect(membre.get_by_role("link", name=re.compile("Action des"))).to_be_visible()
    expect(membre.locator(".live-team-pick")).to_have_count(0)
    expect(membre.locator(".live-presence")).to_have_count(1)
    expect(devcoach.locator(".live-team-pick")).to_have_count(2)
    membre.evaluate(WITNESS)
    devcoach.evaluate(WITNESS)

    record_action_api(space_id, mr_id, "home", first_player_id(mr_id, "home"), 1, "TOUCHDOWN")
    record_action_api(
        space_id, mr_id, "away", first_player_id(mr_id, "away"), 2, "TOUCHDOWN", via=_membre()
    )

    expect(membre.locator(HOME_ROW)).to_have_count(1, timeout=DIRECT_TIMEOUT_MS)
    expect(membre.locator(AWAY_ROW)).to_have_count(1, timeout=DIRECT_TIMEOUT_MS)
    expect(devcoach.locator(AWAY_ROW)).to_have_count(1, timeout=DIRECT_TIMEOUT_MS)
    expect(membre.locator(HOME_ROW)).to_contain_text("saisi par")
    assert membre.evaluate(STILL_THERE), "la page du membre s'est rechargée"
    assert devcoach.evaluate(STILL_THERE), "la page de DevCoach s'est rechargée"

    expect(membre.locator(f"{AWAY_ROW} {CROSS}")).to_have_count(1)
    expect(membre.locator(f"{HOME_ROW} {CROSS}")).to_have_count(0)
    expect(devcoach.locator(CROSS)).to_have_count(2)


def test_le_camp_adverse_est_refuse(space_id, competition):
    mr_id = _kicked_off(space_id, competition, 1)
    record_action_api(space_id, mr_id, "home", first_player_id(mr_id, "home"), 1, "TOUCHDOWN")
    membre = _membre()

    chez_le_domicile = membre.post(
        _url(space_id, mr_id, "step3/actions"),
        data={
            "turn": "2",
            "player_id": first_player_id(mr_id, "home"),
            "player_type": "regular",
            "action_type": "TOUCHDOWN",
        },
        headers={"HX-Request": "true"},
        allow_redirects=False,
    )
    assert chez_le_domicile.status_code == 403, f"obtenu {chez_le_domicile.status_code}"

    effacement = membre.delete(
        _url(space_id, mr_id, f"actions/{_action_of(mr_id, 'home')}"),
        headers={"HX-Request": "true"},
        allow_redirects=False,
    )
    assert effacement.status_code == 403, f"obtenu {effacement.status_code}"
    assert _action_of(mr_id, "home"), "l'action du domicile est toujours là"


def test_avant_le_coup_d_envoi_le_coach_retourne_a_son_avant_match(space_id, competition):
    mr_id = _match_b(space_id, competition, 2)
    membre = _membre()

    for page in ("live", "step4"):
        resp = membre.get(_url(space_id, mr_id, page), allow_redirects=False)
        assert resp.status_code in (302, 303), f"{page} : obtenu {resp.status_code}"
        assert resp.headers["Location"].endswith(f"/{mr_id}/step2"), resp.headers["Location"]

    action = membre.post(
        _url(space_id, mr_id, "step4/actions"),
        data={
            "turn": "1",
            "player_id": first_player_id(mr_id, "away"),
            "player_type": "regular",
            "action_type": "TOUCHDOWN",
        },
        headers={"HX-Request": "true"},
        allow_redirects=False,
    )
    assert action.status_code == 422, f"obtenu {action.status_code}"
    assert "pas commencé" in action.text

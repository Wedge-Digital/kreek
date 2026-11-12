"""Deux coachs écrivent en même temps (carte 653, épic E22).

En « Chacun son équipe », chaque coach saisit les actions de son camp, au même
moment que l'autre. Les deux écritures visent la même version du flux : l'event
store refuse la seconde, et avant cette carte le perdant recevait un 409 avec
rechargement — sa saisie en cours effacée. Désormais une commande d'un camp
perdue contre l'autre est relue et rejouée une fois côté serveur.

**Ce que ce test prouve, et ce qu'il ne prouve pas.** Les deux requêtes partent
ensemble, derrière une barrière, sans `sleep` : la course est probable, pas
garantie à chaque paire. Le test vérifie le résultat visible — aucune réponse
autre que 200, et toutes les actions au journal —, qui doit tenir que la course
ait eu lieu ou non. La preuve que le réessai s'exécute, elle, est déterministe :
`src/app/match_report/io/repository/test_concurrent_side_writes.rs` provoque le
conflit sur la vraie base à chaque exécution.

Le mode B se démarre par un POST direct (`entry_mode=TeamEntry`) : l'option
reste grisée jusqu'à la carte 659. DevCoach coache la première équipe que monte
`build_full_competition`, le membre simple (`X-Bypass-Auth-Profile: simple`) la
deuxième (cf. `test_match_report_entry_mode.py`).
"""

import threading
from concurrent.futures import ThreadPoolExecutor

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import create_draft, ensure_team_entry_inducements, first_player_id

PROFIL_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}
PAIRES = 10


@pytest.fixture(scope="module")
def match_en_mode_b(browser, space_id):
    """Un match en « Chacun son équipe », chaque camp son fan factor (carte
    654) et coups de pouce passés : les deux camps peuvent saisir leurs
    actions."""
    ctx = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    domicile, exterieur = ctx["team_ids"][0], ctx["team_ids"][1]
    mr_id = create_draft(
        space_id, ctx, ctx["round_ids"][0], domicile, exterieur, entry_mode="TeamEntry"
    )
    ensure_team_entry_inducements(space_id, mr_id)
    return {"space_id": space_id, "mr_id": mr_id}


def _poster_action(session, match, camp, joueur, tour, barriere):
    """Attend l'autre coach à la barrière, puis poste : les deux requêtes
    partent au même instant. Rend le code HTTP, que l'appelant vérifie."""
    etape = "step3" if camp == "home" else "step4"
    barriere.wait(timeout=30)
    reponse = session.post(
        f"{BASE_URL}/app/{match['space_id']}/match-report/{match['mr_id']}/{etape}/actions",
        data={
            "turn": str(tour),
            "player_id": joueur,
            "player_type": "regular",
            "action_type": "TOUCHDOWN",
        },
        timeout=30,
    )
    return reponse.status_code


def _sessions():
    devcoach = requests.Session()
    simple = requests.Session()
    simple.headers.update(PROFIL_SIMPLE)
    return devcoach, simple


def _actions_au_journal(mr_id, camp):
    return int(query_db(
        "SELECT count(*) FROM match_report_actions "
        f"WHERE match_report_id = '{mr_id}' AND team_side = '{camp}' AND NOT is_deleted"
    )[0])


def test_deux_actions_postees_au_meme_instant_sont_toutes_deux_au_journal(match_en_mode_b):
    mr_id = match_en_mode_b["mr_id"]
    joueurs = {camp: first_player_id(mr_id, camp) for camp in ("home", "away")}
    devcoach, simple = _sessions()

    codes = []
    with ThreadPoolExecutor(max_workers=2) as pool:
        for paire in range(PAIRES):
            barriere = threading.Barrier(2)
            tour = paire % 16 + 1
            envois = [
                pool.submit(_poster_action, devcoach, match_en_mode_b, "home", joueurs["home"], tour, barriere),
                pool.submit(_poster_action, simple, match_en_mode_b, "away", joueurs["away"], tour, barriere),
            ]
            codes.extend(envoi.result() for envoi in envois)

    assert codes == [200] * (2 * PAIRES), f"codes reçus : {codes}"
    assert _actions_au_journal(mr_id, "home") == PAIRES
    assert _actions_au_journal(mr_id, "away") == PAIRES

"""Les gestes de commissaire sur une équipe s'ouvrent à l'admin de sa
compétition (carte 570).

Ajuster la trésorerie et renvoyer une équipe passaient par
`SpacePermissions::is_admin()` — l'admin d'espace seul. L'admin de la
compétition, qui peut tout le reste sur la fiche, en était exclu sans que ce fût
voulu. Ils passent désormais par `is_admin`, la règle commune à tout kreek.

**Le montage isole le chemin « admin de compétition »** : `E2E Coach 01` est
`SpaceUser` dans l'espace e2e, il n'entre que par `competitions_members` —
comme dans `test_competition_admin_settings.py`. Sans cette isolation, la
branche resterait invisible : `DevCoach`, admin d'espace, passerait toujours
par l'autre porte.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import execute_db, query_db

HX = {"HX-Request": "true"}
#: Le coach que `bypass_auth` connecte sur ce profile : `SpaceUser` de l'espace e2e.
PLAIN_MEMBER = {**HX, "X-Bypass-Auth-Profile": "simple"}
PLAIN_COACH = "E2E Coach 01"


def _coach_id(name: str) -> str:
    return query_db(f"SELECT id FROM auth__users WHERE coach_name = '{name}'")[0]


def _ledger_rows(team_id: str) -> int:
    return int(query_db(f"SELECT count(*) FROM teams__treasury_ledger WHERE team_id = '{team_id}'")[0])


def _adjust(space_id: str, team_id: str) -> requests.Response:
    return requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{team_id}/tresorerie/ajuster",
        data={"direction": "Credit", "amount_kpo": "10", "note": "Compensation de forfait"},
        headers=PLAIN_MEMBER,
        timeout=20,
    )


@pytest.fixture(scope="module")
def team(browser, space_id):
    ctx = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    coach = _coach_id(PLAIN_COACH)
    profile = query_db(
        f"SELECT profile FROM spaces__user_space WHERE space_id = '{space_id}' AND coach_id = '{coach}'"
    )
    assert profile == ["SpaceUser"], f"le montage suppose un membre simple : {profile}"
    return {
        "space_id": space_id,
        "competition_id": ctx["competition_id"],
        "team_id": ctx["team_ids"][0],
        "coach": coach,
    }


@pytest.fixture
def competition_admin(team):
    """`E2E Coach 01` admin de la compétition, le temps d'un test."""
    execute_db(
        "INSERT INTO competitions_members (competition_id, coach_id, competition_profile, created_at) "
        f"VALUES ('{team['competition_id']}', '{team['coach']}', 'CompetitionAdmin', now()) "
        "ON CONFLICT DO NOTHING"
    )
    yield team
    execute_db(
        "DELETE FROM competitions_members "
        f"WHERE competition_id = '{team['competition_id']}' AND coach_id = '{team['coach']}'"
    )


def test_plain_member_cannot_adjust_treasury(team):
    before = _ledger_rows(team["team_id"])
    response = _adjust(team["space_id"], team["team_id"])
    assert response.status_code == 403, f"refus attendu, obtenu {response.status_code}"
    assert _ledger_rows(team["team_id"]) == before, "rien ne doit s'écrire"


def test_competition_admin_sees_adjust_button(competition_admin):
    ctx = competition_admin
    body = requests.get(
        f"{BASE_URL}/app/{ctx['space_id']}/teams/{ctx['team_id']}/tresorerie",
        headers={"X-Bypass-Auth-Profile": "simple"},
        timeout=20,
    ).text
    assert "tr-btn-adjust" in body, "l'admin de compétition doit voir le bouton"


def test_competition_admin_adjusts_treasury(competition_admin):
    ctx = competition_admin
    before = _ledger_rows(ctx["team_id"])
    response = _adjust(ctx["space_id"], ctx["team_id"])
    assert response.status_code == 200, f"{response.status_code} : {response.text[:200]}"
    assert _ledger_rows(ctx["team_id"]) == before + 1, "le mouvement doit s'inscrire au relevé"

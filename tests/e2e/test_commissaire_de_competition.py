"""Les gestes de commissaire sur une équipe s'ouvrent à l'admin de sa
compétition (carte 570).

Ajuster la trésorerie et renvoyer une équipe passaient par
`SpacePermissions::is_admin()` — l'admin d'espace seul. L'admin de la
compétition, qui peut tout le reste sur la fiche, en était exclu sans que ce fût
voulu. Ils passent désormais par `est_admin`, la règle commune à tout kreek.

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
#: Le coach que `bypass_auth` connecte sur ce profil : `SpaceUser` de l'espace e2e.
MEMBRE_SIMPLE = {**HX, "X-Bypass-Auth-Profile": "simple"}
COACH_SIMPLE = "E2E Coach 01"


def _coach_id(nom: str) -> str:
    return query_db(f"SELECT id FROM auth__users WHERE coach_name = '{nom}'")[0]


def _mouvements(team_id: str) -> int:
    return int(query_db(f"SELECT count(*) FROM teams__treasury_ledger WHERE team_id = '{team_id}'")[0])


def _ajuster(space_id: str, team_id: str) -> requests.Response:
    return requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{team_id}/tresorerie/ajuster",
        data={"direction": "Credit", "amount_kpo": "10", "note": "Compensation de forfait"},
        headers=MEMBRE_SIMPLE,
        timeout=20,
    )


@pytest.fixture(scope="module")
def equipe(browser, space_id):
    ctx = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    coach = _coach_id(COACH_SIMPLE)
    profil = query_db(
        f"SELECT profile FROM spaces__user_space WHERE space_id = '{space_id}' AND coach_id = '{coach}'"
    )
    assert profil == ["SpaceUser"], f"le montage suppose un membre simple : {profil}"
    return {
        "space_id": space_id,
        "competition_id": ctx["competition_id"],
        "team_id": ctx["team_ids"][0],
        "coach": coach,
    }


@pytest.fixture
def admin_de_la_competition(equipe):
    """`E2E Coach 01` admin de la compétition, le temps d'un test."""
    execute_db(
        "INSERT INTO competitions_members (competition_id, coach_id, competition_profile, created_at) "
        f"VALUES ('{equipe['competition_id']}', '{equipe['coach']}', 'CompetitionAdmin', now()) "
        "ON CONFLICT DO NOTHING"
    )
    yield equipe
    execute_db(
        "DELETE FROM competitions_members "
        f"WHERE competition_id = '{equipe['competition_id']}' AND coach_id = '{equipe['coach']}'"
    )


def test_un_simple_membre_ne_peut_pas_ajuster_la_tresorerie(equipe):
    avant = _mouvements(equipe["team_id"])
    reponse = _ajuster(equipe["space_id"], equipe["team_id"])
    assert reponse.status_code == 403, f"refus attendu, obtenu {reponse.status_code}"
    assert _mouvements(equipe["team_id"]) == avant, "rien ne doit s'écrire"


def test_l_admin_de_la_competition_voit_le_bouton_d_ajustement(admin_de_la_competition):
    ctx = admin_de_la_competition
    corps = requests.get(
        f"{BASE_URL}/app/{ctx['space_id']}/teams/{ctx['team_id']}/tresorerie",
        headers={"X-Bypass-Auth-Profile": "simple"},
        timeout=20,
    ).text
    assert "tr-btn-adjust" in corps, "l'admin de compétition doit voir le bouton"


def test_l_admin_de_la_competition_ajuste_la_tresorerie(admin_de_la_competition):
    ctx = admin_de_la_competition
    avant = _mouvements(ctx["team_id"])
    reponse = _ajuster(ctx["space_id"], ctx["team_id"])
    assert reponse.status_code == 200, f"{reponse.status_code} : {reponse.text[:200]}"
    assert _mouvements(ctx["team_id"]) == avant + 1, "le mouvement doit s'inscrire au relevé"

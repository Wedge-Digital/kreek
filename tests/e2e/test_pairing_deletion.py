"""Tests E2E — suppression d'un pairing depuis l'administration du calendrier.

Deux règles y sont vérifiées :

- un rapport **engagé** (sélection confirmée) n'empêche pas la suppression, mais
  celle-ci doit libérer les deux équipes. Le point est invisible en test
  unitaire de bout en bout : la confirmation les verrouille en
  `MatchReporting` (BC `teams`), et la seule autre sortie de cette phase est la
  publication du rapport ;
- un rapport **publié** interdit la suppression — le match resterait au
  classement tout en disparaissant du calendrier.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import time

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import create_draft, ensure_pre_match, play_match


# ── Helpers ────────────────────────────────────────────────────────────────────


def _wait(condition, description: str, timeout_s: int = 30):
    deadline = time.time() + timeout_s
    last = None
    while time.time() < deadline:
        last = condition()
        if last:
            return last
        time.sleep(0.3)
    raise AssertionError(f"{description} — non satisfait après {timeout_s}s (dernier : {last})")


def _first_pairing(season_id: str, round_id: str) -> dict:
    rows = query_db(
        "SELECT pairing_id, home_team_id, away_team_id "
        "FROM competition_match_display_proj "
        f"WHERE season_id = '{season_id}' AND round_id = '{round_id}' "
        "ORDER BY pairing_id LIMIT 1"
    )
    assert rows, f"aucun pairing projeté pour la journée {round_id}"
    pairing_id, home_team_id, away_team_id = rows[0].split("|")
    return {"pairing_id": pairing_id, "home": home_team_id, "away": away_team_id}


def _report_id_of_pairing(pairing_id: str) -> str | None:
    rows = query_db(
        f"SELECT match_report_id FROM match_report_proj WHERE pairing_id = '{pairing_id}'"
    )
    return rows[0] if rows else None


def _phase(mr_id: str) -> str | None:
    rows = query_db(f"SELECT phase FROM match_report_proj WHERE match_report_id = '{mr_id}'")
    return rows[0] if rows else None


def _game_phase(team_id: str) -> str | None:
    rows = query_db(f"SELECT game_phase FROM team_proj WHERE team_id = '{team_id}'")
    return rows[0] if rows else None


def _delete_pairing(space_id: str, ctx: dict, pairing_id: str) -> requests.Response:
    url = (
        f"{BASE_URL}/app/{space_id}/competitions/{ctx['competition_id']}"
        f"/{ctx['season_id']}/admin/schedule/delete-match"
    )
    return requests.delete(
        url,
        json={"pairing_id": pairing_id},
        headers={"HX-Request": "true"},
    )


# ── Fixtures ──────────────────────────────────────────────────────────────────


@pytest.fixture(scope="module")
def deletion_ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=4, num_rounds=2)
    return {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_ids": full["round_ids"],
    }


@pytest.fixture(scope="module")
def published_ctx(browser, space_id):
    """Compétition dédiée au scénario « rapport publié ».

    Séparée de `deletion_ctx` : le premier test y verrouille des équipes sur un
    nouveau rapport, et jouer un match avec elles échouerait — les deux
    scénarios doivent rester indépendants de leur ordre d'exécution."""
    full = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    return {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_ids": full["round_ids"],
    }


# ── Test ──────────────────────────────────────────────────────────────────────


def test_suppression_pairing_annule_le_rapport_et_libere_les_equipes(space_id, deletion_ctx):
    round_id = deletion_ctx["round_ids"][0]
    pairing = _first_pairing(deletion_ctx["season_id"], round_id)

    mr_id = _wait(
        lambda: _report_id_of_pairing(pairing["pairing_id"]),
        "un brouillon de rapport doit exister pour le pairing",
    )

    # Confirmer la sélection : c'est ce qui verrouille les deux équipes.
    ensure_pre_match(space_id, mr_id, deletion_ctx, round_id, pairing["home"], pairing["away"])
    for team_id in (pairing["home"], pairing["away"]):
        _wait(
            lambda t=team_id: _game_phase(t) == "MatchReporting",
            f"l'équipe {team_id} doit être verrouillée en saisie",
        )

    resp = _delete_pairing(space_id, deletion_ctx, pairing["pairing_id"])
    assert resp.status_code == 200, f"suppression du pairing : {resp.status_code}"

    _wait(lambda: _phase(mr_id) == "Cancelled", "le rapport doit être annulé")
    for team_id in (pairing["home"], pairing["away"]):
        _wait(
            lambda t=team_id: _game_phase(t) == "ReadyToPlay",
            f"l'équipe {team_id} doit être libérée",
        )

    # Vérification fonctionnelle : les deux équipes peuvent repartir sur un
    # autre rapport. `update_match_selection` répond 409 (TeamNotAvailable) si
    # l'une d'elles est encore verrouillée.
    autre_journee = deletion_ctx["round_ids"][1]
    nouveau_mr = create_draft(
        space_id, deletion_ctx, autre_journee, pairing["home"], pairing["away"]
    )
    ensure_pre_match(
        space_id, nouveau_mr, deletion_ctx, autre_journee, pairing["home"], pairing["away"]
    )


def test_un_rapport_publie_interdit_la_suppression_du_pairing(space_id, published_ctx):
    round_id = published_ctx["round_ids"][0]
    pairing = _first_pairing(published_ctx["season_id"], round_id)

    mr_id = play_match(space_id, published_ctx, round_id, pairing["home"], pairing["away"])
    _wait(lambda: _phase(mr_id) == "Published", "le rapport doit être publié")

    resp = _delete_pairing(space_id, published_ctx, pairing["pairing_id"])
    assert resp.status_code == 422, f"la suppression doit être refusée : {resp.status_code}"
    assert "publié" in resp.json()["error"]

    reste = query_db(
        f"SELECT count(*) FROM competition_match_day_pairings WHERE id = '{pairing['pairing_id']}'"
    )
    assert reste == ["1"], "le pairing doit toujours exister"

    # Et le bouton de suppression n'est pas rendu pour cette rencontre.
    detail = requests.get(
        f"{BASE_URL}/app/{space_id}/competitions/{published_ctx['competition_id']}"
        f"/{published_ctx['season_id']}/admin/schedule/round",
        params={"round_id": round_id},
        headers={"HX-Request": "true"},
    )
    assert detail.status_code == 200, f"widget round-detail : {detail.status_code}"
    assert "match-row-locked" in detail.text
    assert "match-row-delete" not in detail.text

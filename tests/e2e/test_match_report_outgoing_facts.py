"""Tests E2E — les faits de saisie sortent du BC (carte 643).

Une action saisie, une sélection de coups de pouce validée : chacune était
appendée sans être émise sur le bus du BC, et rien hors de `match_report` ne
pouvait la voir. Le canal temps réel n'existant pas encore, la preuve qu'un
fait a quitté l'append est la table `event_log`, que `event_log_feeder` remplit
de **chaque** domain event émis : un fait appendé sans être émis n'y est pas.

Scénarios couverts, dans l'ordre du fichier — ils partagent un seul rapport :
- F1 — Les coups de pouce passés au navigateur : chaque `InducementsRecorded`
       de l'event store du rapport a sa ligne dans `event_log`
- F2 — Un touchdown saisi au navigateur : son `ActionRecorded` a sa ligne dans
       `event_log`

`event_log_feeder` écrit dans une tâche séparée : la ligne arrive après la
réponse, d'où l'attente par condition (`attendre_que`) plutôt qu'une lecture
dans la foulée.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée.
"""

import re

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import attendre_que, query_db
from match_report_helpers import create_draft, ensure_pre_match, pass_inducements_in_browser


def _url(space_id, mr_id, suffixe):
    return f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{suffixe}"


def _appended(mr_id: str, event_type: str) -> int:
    rows = query_db(
        "SELECT count(*) FROM match_report_event_store "
        f"WHERE match_report_id = '{mr_id}' AND event_type = '{event_type}'"
    )
    return int(rows[0])


def _emitted(mr_id: str, event_type: str) -> int:
    rows = query_db(
        "SELECT count(*) FROM event_log "
        f"WHERE emitter = '{mr_id}' AND event_type = '{event_type}'"
    )
    return int(rows[0])


def _wait_every_appended_fact_emitted(mr_id: str, event_type: str) -> None:
    appended = _appended(mr_id, event_type)
    assert appended >= 1, f"aucun {event_type} appendé pour {mr_id} : le scénario n'a rien saisi"
    attendre_que(
        lambda: _emitted(mr_id, event_type) == appended,
        quoi=f"{appended} ligne(s) {event_type} dans event_log pour {mr_id}",
    )


@pytest.fixture(scope="module")
def facts_ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=4)
    ctx = {**full, "teams": full["team_ids"]}
    round_id = full["round_ids"][0]
    home, away = full["team_ids"][0], full["team_ids"][1]
    mr_id = create_draft(space_id, ctx, round_id, home, away)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home, away)
    return {"space_id": space_id, "mr_id": mr_id}


# ── F1 ────────────────────────────────────────────────────────────────────────

def test_validated_inducements_reach_event_log(page: Page, facts_ctx):
    space_id, mr_id = facts_ctx["space_id"], facts_ctx["mr_id"]
    page.goto(_url(space_id, mr_id, "step2"), wait_until="load")
    page.fill("input[name='home_fan_roll']", "2")
    page.fill("input[name='away_fan_roll']", "3")
    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    pass_inducements_in_browser(page)
    page.wait_for_url(re.compile(rf".*/{mr_id}/live$"), timeout=10000)

    _wait_every_appended_fact_emitted(mr_id, "InducementsRecorded")


# ── F2 ────────────────────────────────────────────────────────────────────────

def test_a_recorded_action_reaches_event_log(page: Page, facts_ctx):
    space_id, mr_id = facts_ctx["space_id"], facts_ctx["mr_id"]
    page.goto(_url(space_id, mr_id, "live"), wait_until="load")

    page.locator(".match-report-live .live-new-action-toggle").click()
    equipes = page.locator(".match-report-live .live-team-pick")
    expect(equipes.first).to_be_visible()
    with page.expect_navigation(wait_until="load"):
        equipes.first.click()

    # La saisie du camp, comme L2 de test_match_report_live.
    page.wait_for_selector("#turn-selector .mr-turn-btn", timeout=6000)
    page.wait_for_selector("#player-selector .mr-player-chip", timeout=6000)
    page.locator(".mr-turn-btn[data-turn='4']").first.click()
    page.locator(".mr-player-chip").first.click()
    page.wait_for_selector("#action-panel .mr-action-grid", timeout=6000)
    page.locator(".mr-action-btn").filter(has_text="Touchdown").click()
    expect(page.locator("#action-log .mr-log-entry", has_text="Touchdown")).to_be_visible(timeout=6000)

    _wait_every_appended_fact_emitted(mr_id, "ActionRecorded")

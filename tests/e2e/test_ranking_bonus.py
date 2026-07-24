"""Test E2E — bonus de classement reflétés dans le widget Classement.

Vérifie ce qu'aucun test unitaire ne voit : après publication d'un rapport de
match, le total de points affiché par le widget ranking inclut réellement les
points bonus calculés (chaîne config compétition → publication → app event →
projection → rendu HTML).

Scénario couvert (le bonus agressif, seul bonus entièrement nouveau) :
- une compétition avec le bonus agressif activé (+N points si > 1 sortie infligée) ;
- match A : l'équipe home marque 1 TD (victoire) ET inflige 2 sorties → bonus ;
- match B : une autre équipe home marque 1 TD (victoire) sans aucune sortie → pas de bonus ;
- le delta de points entre les deux vainqueurs vaut exactement les points du bonus,
  ce qui isole le bonus du barème V/N/D par défaut (inconnu ici) ;
- le widget Classement affiche bien les deux totaux distincts.

Le calcul lui-même (seuils, gate d'activation, cumul) est couvert unitairement
dans `ranking_line.rs` ; ce test valide le câblage réel de bout en bout.

Extensions à ajouter une fois ce scénario vert : bonus offensif (≥ TD marqués),
défensif (≤ TD encaissés), et cas désactivé (condition remplie mais 0 point).

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) sur :3210.
"""

import re
import time

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import (
    BASE_URL,
    BYPASS_AUTH_COACH_NAME,
    FAKE_LOGO_URL,
    build_and_submit_team,
    sync_and_generate_schedule,
)
from db_helpers import query_db

_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")

AGG_POINTS = 5  # valeur distinctive pour le bonus agressif
AGG_MIN_CAS = 1  # bonus si strictement > 1 sortie


# ── Création d'une compétition avec le bonus agressif activé ──────────────────
# Reprend le flow de `create_full_competition` (competition_lifecycle) en
# activant en plus le bonus agressif en phase 2 (cf. test_competition_rules_bonus).


def _create_competition_with_aggressive_bonus(page: Page, competition_create_url: str, num_rounds: int = 3) -> dict:
    competition_name = f"Ligue Bonus Ranking E2E {time.time_ns()}"

    # Phase 1 : infos + admin = utilisateur bypass_auth (accès aux pages match-report).
    page.goto(competition_create_url, wait_until="load")
    page.fill("input[name='name']", competition_name)
    page.wait_for_selector(".coach-result-row", timeout=5000)
    page.locator("input[name='q']").press_sequentially(BYPASS_AUTH_COACH_NAME, delay=30)
    page.wait_for_timeout(600)
    page.locator(".coach-result-row", has_text=BYPASS_AUTH_COACH_NAME).first.click()
    page.evaluate(f"document.getElementById('logo_url').value = '{FAKE_LOGO_URL}'")
    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    match = re.search(r"/competitions/create/([0-9A-Za-z]+)/([0-9A-Za-z]+)/rules", page.url)
    assert match, f"competition_id/season_id introuvables dans {page.url}"
    competition_id, season_id = match.group(1), match.group(2)

    # Phase 2 : règles + activation du bonus agressif.
    page.wait_for_selector(".tier-block [data-slot='star'] .roster-chip", timeout=5000)
    xp_input = page.locator(".tier-block").first.locator("input.tier-xp")
    xp_input.fill("6")
    xp_input.dispatch_event("change")
    page.fill("#season_name", f"Saison Bonus Ranking {time.time_ns()}")
    page.check("#agg_activated")
    page.fill("#agg_points", str(AGG_POINTS))
    page.fill("#agg_min_cas", str(AGG_MIN_CAS))
    page.click("button[onclick='submitRules()']")
    page.wait_for_selector("#groups-config", timeout=10000)

    # Phase 3 : structure — num_rounds journées à date fixe.
    for i in range(num_rounds):
        page.click("button[onclick=\"addDate('fixed_date')\"]")
        page.locator("#dates-list .date-input").nth(i).fill(f"2026-0{(i % 9) + 1}-01")
    page.click("button[onclick='submitStructure()']")
    page.wait_for_selector("#access-mode-btns", timeout=10000)

    # Phase 4 : invitations — acceptation automatique.
    page.click("#validation-mode-btns .choice-btn[data-val='false']")
    page.click("button[onclick='submitInvitations()']")
    page.wait_for_selector(".recap-row", timeout=10000)

    # Phase 5 : publication.
    page.click(".btn-cta")
    page.wait_for_timeout(1000)

    return {"competition_id": competition_id, "season_id": season_id, "name": competition_name}


# ── Helpers de publication de rapport (calqués sur test_match_report_recap) ────


def _create_draft(space_id, ctx, round_id, home_idx, away_idx):
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/new",
        data={
            "competition_id": ctx["competition_id"],
            "season_id": ctx["season_id"],
            "round_id": round_id,
            "home_team_id": ctx["teams"][home_idx],
            "away_team_id": ctx["teams"][away_idx],
        },
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"create: {resp.status_code}\n{resp.text[:200]}"
    m = _ULID_RE.search(resp.headers.get("Location", ""))
    assert m, f"match_report_id introuvable dans Location: {resp.headers.get('Location')!r}"
    return m.group(1)


def _ensure_pre_match(space_id, mr_id, ctx, round_id, home_idx, away_idx):
    check = requests.get(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2", allow_redirects=False)
    if check.status_code == 200:
        return
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}",
        data={
            "competition_id": ctx["competition_id"],
            "season_id": ctx["season_id"],
            "round_id": round_id,
            "home_team_id": ctx["teams"][home_idx],
            "away_team_id": ctx["teams"][away_idx],
        },
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"confirm: {resp.status_code}\n{resp.text[:200]}"


def _ensure_inducements(space_id, mr_id):
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "2", "away_fan_roll": "3"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"fan factor: {resp.status_code}"
    location = resp.headers.get("Location", "")
    for _ in range(3):
        if not location or "/inducements/" not in location:
            break
        resp = requests.post(f"{BASE_URL}{location}", data={"selection": ""}, allow_redirects=False)
        if resp.status_code not in (302, 303):
            break
        location = resp.headers.get("Location", "")


def _record_action_api(space_id, mr_id, side, player_id, turn, action_type):
    endpoint = "step3" if side == "home" else "step4"
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{endpoint}/actions",
        data={"turn": str(turn), "player_id": player_id, "player_type": "regular", "action_type": action_type},
    )
    assert resp.status_code == 200, f"record_action {action_type}: {resp.status_code}\n{resp.text[:200]}"


def _first_player_id(mr_id, side):
    rows = _query_side_team(mr_id, side)
    assert rows, f"match report {mr_id} introuvable en DB"
    player_rows = query_db(f"SELECT player_id FROM players_proj WHERE team_id = '{rows[0]}' LIMIT 1")
    assert player_rows, f"aucun joueur trouvé pour l'équipe {rows[0]}"
    return player_rows[0]


def _query_side_team(mr_id, side):
    return query_db(f"SELECT {side}_team_id FROM match_report_proj WHERE match_report_id = '{mr_id}'")


def _post_step5(space_id, mr_id):
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step5",
        data={
            "home_gain": "50000",
            "away_gain": "40000",
            "home_fan_mod": "1",
            "away_fan_mod": "-1",
            "summary_title": "Match E2E bonus ranking",
            "summary_body": "Compte-rendu automatique.",
        },
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"step5: {resp.status_code}\n{resp.text[:200]}"


def _publish(space_id, mr_id):
    resp = requests.post(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap/publish", allow_redirects=False)
    assert resp.status_code in (302, 303), f"publish: {resp.status_code}\n{resp.text[:200]}"


def _play_match(space_id, ctx, round_id, home_idx, away_idx, home_sorties):
    """Joue et publie un match : home marque 1 TD (victoire) + `home_sorties` sorties."""
    mr_id = _create_draft(space_id, ctx, round_id, home_idx, away_idx)
    _ensure_pre_match(space_id, mr_id, ctx, round_id, home_idx, away_idx)
    _ensure_inducements(space_id, mr_id)

    home_player = _first_player_id(mr_id, "home")
    _record_action_api(space_id, mr_id, "home", home_player, turn=1, action_type="TOUCHDOWN")
    for i in range(home_sorties):
        _record_action_api(space_id, mr_id, "home", home_player, turn=2 + i, action_type="SORTIE")

    _post_step5(space_id, mr_id)
    _publish(space_id, mr_id)
    return mr_id


def _wait_ranking_points(season_id, team_id, timeout_s=20):
    """Attend que la projection ranking (asynchrone via app event) soit peuplée."""
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        rows = query_db(
            f"SELECT ranking_points FROM ranking_lines "
            f"WHERE season_id = '{season_id}' AND team_id = '{team_id}' "
            f"ORDER BY sequence DESC LIMIT 1"
        )
        if rows:
            return int(rows[0])
        time.sleep(0.3)
    raise AssertionError(f"aucune ligne de classement pour team={team_id} après {timeout_s}s")


# ── Fixture ───────────────────────────────────────────────────────────────────


@pytest.fixture(scope="module")
def bonus_ctx(browser, space_id):
    """Compétition dédiée avec bonus agressif + 4 équipes inscrites + 3 journées pairées."""
    page = browser.new_page()
    try:
        create_url = f"{BASE_URL}/app/{space_id}/competitions/create"
        comp = _create_competition_with_aggressive_bonus(page, create_url, num_rounds=3)
        team_ids = [
            build_and_submit_team(page, space_id, comp["name"], coach_option_index=i, roster_index=i)
            for i in range(4)
        ]
        sync_and_generate_schedule(page, space_id, comp["competition_id"], comp["season_id"])
        round_ids = query_db(
            f"SELECT id FROM competition_match_days WHERE season_id = '{comp['season_id']}' ORDER BY position;"
        )
        return {
            "space_id": space_id,
            "competition_id": comp["competition_id"],
            "season_id": comp["season_id"],
            "teams": team_ids,
            "round_ids": round_ids,
        }
    finally:
        page.close()


# ── Test ──────────────────────────────────────────────────────────────────────


def test_aggressive_bonus_reflected_in_standings(page: Page, bonus_ctx, console_errors):
    space_id = bonus_ctx["space_id"]
    season_id = bonus_ctx["season_id"]

    # Match A : home (team 0) gagne 1-0 ET inflige 2 sorties (> 1) → bonus agressif.
    _play_match(space_id, bonus_ctx, bonus_ctx["round_ids"][0], home_idx=0, away_idx=1, home_sorties=2)
    # Match B : home (team 2) gagne 1-0 sans aucune sortie → pas de bonus.
    _play_match(space_id, bonus_ctx, bonus_ctx["round_ids"][1], home_idx=2, away_idx=3, home_sorties=0)

    points_with_bonus = _wait_ranking_points(season_id, bonus_ctx["teams"][0])
    points_without_bonus = _wait_ranking_points(season_id, bonus_ctx["teams"][2])

    # Le seul écart entre deux vainqueurs 1-0 est le bonus agressif.
    assert points_with_bonus - points_without_bonus == AGG_POINTS, (
        f"attendu un écart de {AGG_POINTS} (bonus agressif), "
        f"obtenu {points_with_bonus} vs {points_without_bonus}"
    )

    # Le widget Classement affiche les totaux (incluant le bonus) dans une vraie table.
    detail_url = f"{BASE_URL}/app/{space_id}/competitions/{bonus_ctx['competition_id']}/{season_id}"
    page.goto(detail_url, wait_until="load")
    page.wait_for_selector(".ranking-classement-widget .standings-row", timeout=8000)

    pts_cells = [c.inner_text().strip() for c in page.locator(".standings-pts").all()]
    assert str(points_with_bonus) in pts_cells, (
        f"le total avec bonus ({points_with_bonus}) devrait apparaître dans {pts_cells}"
    )
    assert str(points_without_bonus) in pts_cells, (
        f"le total sans bonus ({points_without_bonus}) devrait apparaître dans {pts_cells}"
    )

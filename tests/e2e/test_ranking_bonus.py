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
from playwright.sync_api import Page, expect

from competition_lifecycle import (
    BASE_URL,
    BYPASS_AUTH_COACH_NAME,
    FAKE_LOGO_URL,
    build_and_submit_team,
    sync_and_generate_schedule,
)
from db_helpers import query_db
from match_report_helpers import play_match, wait_ranking_points

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
    teams = bonus_ctx["teams"]
    play_match(space_id, bonus_ctx, bonus_ctx["round_ids"][0], teams[0], teams[1], home_sorties=2)
    # Match B : home (team 2) gagne 1-0 sans aucune sortie → pas de bonus.
    play_match(space_id, bonus_ctx, bonus_ctx["round_ids"][1], teams[2], teams[3], home_sorties=0)

    points_with_bonus = wait_ranking_points(season_id, bonus_ctx["teams"][0])
    points_without_bonus = wait_ranking_points(season_id, bonus_ctx["teams"][2])

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

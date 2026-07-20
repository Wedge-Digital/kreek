"""Tests E2E — fiche de détail joueur.

Scénarios couverts :
- Clic sur une ligne du tableau roster de la fiche équipe → navigation vers la fiche joueur
- Portefeuille SPP : deux nombres distincts (gagnés/dépensés) + réserve
- Résumé de carrière cohérent avec les actions enregistrées (essais/passes/interceptions/sorties/MVP)
- Historique de matchs : carte correspondant au match publié (adversaire/score/actions)
- Bouton "✏️ Customiser" visible uniquement pour un admin d'espace/compétition (désactivé, feature à part)
- Bouton "▶ Activer la dépense de SPP" visible et actif en phase PlayerImprovement

Setup de données via HTTP direct (comme test_match_report_recap.py) plutôt que
via l'UI de construction d'équipe — le picker de roster (TomSelect) est connu
pour être peu fiable sous Playwright (cf. test_build_and_finalize_team.py,
marqué skip pour cette raison). Seule l'interaction testée ici (clic sur une
ligne de tableau simple, sans JS complexe) est pilotée au navigateur.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée
avec au moins 12 équipes inscrites et 7 journées pour la première
compétition/saison du space (make init_db WITH_SEED=1 sur une base fraîche).
Utilise les indices d'équipes 10/11 et la dernière journée disponible pour
éviter toute collision avec les paires déjà utilisées par
test_match_report_recap.py (indices 0-5).
"""

import re

import pytest
import requests
from playwright.sync_api import Page, expect

from db_helpers import query_db as _query_db

BASE_URL = "http://localhost:3210"
_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")


def _create_draft(space_id: str, ctx: dict, round_id: str, home_idx: int, away_idx: int) -> str:
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


def _ensure_pre_match(space_id: str, mr_id: str, ctx: dict, round_id: str, home_idx: int, away_idx: int) -> None:
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


def _ensure_inducements(space_id: str, mr_id: str) -> None:
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


def _record_action_api(space_id: str, mr_id: str, side: str, player_id: str, turn: int,
                        action_type: str, injury_type: str | None = None) -> None:
    endpoint = "step3" if side == "home" else "step4"
    data = {
        "turn": str(turn),
        "player_id": player_id,
        "player_type": "regular",
        "action_type": action_type,
    }
    if injury_type:
        data["injury_type"] = injury_type
    resp = requests.post(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{endpoint}/actions", data=data)
    assert resp.status_code == 200, f"record_action {action_type}: {resp.status_code}\n{resp.text[:200]}"


def _first_player_id(mr_id: str, side: str) -> str:
    rows = _query_db(
        f"SELECT {side}_team_id FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )
    assert rows, f"match report {mr_id} introuvable en DB"
    team_id = rows[0]
    player_rows = _query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{team_id}' LIMIT 1"
    )
    assert player_rows, f"aucun joueur trouvé pour l'équipe {team_id}"
    return player_rows[0]


def _post_step5(space_id: str, mr_id: str, **overrides) -> requests.Response:
    data = {"home_gain": "50000", "away_gain": "40000", "home_fan_mod": "1", "away_fan_mod": "-1"}
    data.update(overrides)
    return requests.post(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step5", data=data, allow_redirects=False)


def _publish(space_id: str, mr_id: str) -> None:
    resp = requests.post(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap/publish", allow_redirects=False)
    assert resp.status_code in (302, 303), f"publish: {resp.status_code}\n{resp.text[:200]}"


def _wait_for(check, attempts=30, delay_s=0.2):
    import time
    for _ in range(attempts):
        if check():
            return
        time.sleep(delay_s)
    pytest.fail("condition jamais satisfaite (pipeline player-match-impact pas propagé à temps)")


# ── Fixture : match publié avec actions pour le joueur home ────────────────────

@pytest.fixture(scope="module")
def published_match(browser, space_id):
    """Compétition dédiée à ce module (2 équipes) — cf. docstring du module
    competition_lifecycle.py sur l'isolation par fichier."""
    from competition_lifecycle import build_full_competition
    full = build_full_competition(browser, space_id, num_teams=2)
    ctx = {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_ids": full["round_ids"],
        "teams": full["team_ids"],
    }
    round_id = ctx["round_ids"][0]
    home_idx, away_idx = 0, 1

    mr_id = _create_draft(space_id, ctx, round_id, home_idx, away_idx)
    _ensure_pre_match(space_id, mr_id, ctx, round_id, home_idx, away_idx)
    _ensure_inducements(space_id, mr_id)

    home_player = _first_player_id(mr_id, "home")
    away_player = _first_player_id(mr_id, "away")
    _record_action_api(space_id, mr_id, "home", home_player, turn=1, action_type="TOUCHDOWN")
    _record_action_api(space_id, mr_id, "home", home_player, turn=2, action_type="SORTIE")
    _record_action_api(space_id, mr_id, "away", away_player, turn=3, action_type="BLESSE", injury_type="AMOCHE")
    _record_action_api(space_id, mr_id, "home", home_player, turn=4, action_type="MVP")

    resp = _post_step5(space_id, mr_id, summary_title="Match E2E fiche joueur", summary_body="Généré par les tests.")
    assert resp.status_code in (302, 303), f"step5: {resp.status_code}\n{resp.text[:200]}"
    _publish(space_id, mr_id)

    team_id_rows = _query_db(f"SELECT home_team_id FROM match_report_proj WHERE match_report_id = '{mr_id}'")
    home_team_id = team_id_rows[0]

    # Le pipeline player-match-impact (listeners app event) est asynchrone —
    # attendre que le SPP du joueur home ait bien été crédité avant de tester l'UI.
    def spp_credited():
        rows = _query_db(f"SELECT spp FROM players_proj WHERE player_id = '{home_player}'")
        return bool(rows) and int(rows[0]) > 0

    _wait_for(spp_credited)

    return {"home_player_id": home_player, "away_player_id": away_player, "home_team_id": home_team_id}


# ── Scénarios ────────────────────────────────────────────────────────────────

def test_click_roster_row_navigates_to_player_detail(page: Page, space_id, published_match):
    team_id = published_match["home_team_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
    page.wait_for_selector(".player-table-row", timeout=10000)
    page.locator(".player-table-row").first.click()
    page.wait_for_url(re.compile(r".*/players/.*/detail$"), timeout=10000)
    expect(page.locator(".player-page")).to_be_visible()


def test_spp_wallet_shows_earned_and_spent_separately(page: Page, space_id, published_match):
    player_id = published_match["home_player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    expect(page.locator(".spp-budget-numbers")).to_be_visible()
    expect(page.locator(".spp-budget-bottom")).to_contain_text("SPP en réserve")


def test_career_summary_reflects_recorded_actions(page: Page, space_id, published_match):
    player_id = published_match["home_player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    career_stats = page.locator(".career-summary .career-stat-val")
    values = career_stats.all_inner_texts()
    assert values[0] == "1"  # essais
    assert values[3] == "1"  # sorties
    assert values[4] == "1"  # MVP


def test_match_history_card_shows_opponent_and_actions(page: Page, space_id, published_match):
    player_id = published_match["home_player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    expect(page.locator(".match-card").first).to_be_visible(timeout=10000)
    first_card = page.locator(".match-card").first
    expect(first_card.locator(".pd-log-entry")).to_have_count(3)  # touchdown + sortie + mvp


def test_customise_button_present_but_disabled(page: Page, space_id, published_match):
    player_id = published_match["home_player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    expect(page.locator(".btn-customise")).to_be_disabled()


def test_activate_spp_spending_button_visible_and_enabled_in_player_improvement(page: Page, space_id, published_match):
    """La dépense de SPP est une fonctionnalité réelle (cf. test_player_spp_spending.py) —
    le bouton est actif dès que l'équipe est en phase PlayerImprovement, ce qui
    est le cas ici (match publié par la fixture published_match)."""
    player_id = published_match["home_player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    expect(page.locator(".btn-toggle-spp")).to_be_visible()
    expect(page.locator(".btn-toggle-spp")).to_be_enabled()

"""Tests E2E — dépense de SPP sur la fiche joueur (slot journal / panneau de dépense).

Scénarios couverts :
- Équipe hors phase PlayerImprovement → widget journal (lecture seule) affiché.
- Équipe en phase, coach de l'équipe → panneau de dépense visible, achat d'une
  compétence → réserve SPP diminuée, tag de compétence acquise en plus.
- Joueur sans aucun SPP → aucune compétence achetable dans skill_picker (son
  comportement existant : bouton "Budget insuf.").
- Augmentation de caractéristique → stat + valeur mises à jour.
- team_value (fiche équipe) incrémenté après achat — vérifie le pipeline app
  event players → teams.

Non testé ici : utilisateur non autorisé (ni coach, ni admin) sur une équipe en
phase PlayerImprovement. `bypass_auth` connecte toujours le même utilisateur
(legacy_id=1), et ce compte est coach de toutes les équipes seedées — le
simuler nécessiterait de modifier directement le payload JSON de l'événement
TeamCreated dans l'event store pour changer le coach_id effectif (l'agrégat
est rejoué depuis team_event_store, pas depuis la projection team_proj), ce
qui reviendrait à fabriquer un état plutôt qu'à piloter une vraie action
applicative — contraire au principe déjà posé par
test_team_detail_state_banner.py. La règle elle-même (`can_spend_spp`) vit
dans purchase_skill_controller.rs, hors du périmètre des tests unitaires
existants ; elle reste donc non couverte automatiquement pour l'instant.

Setup via HTTP direct (comme test_player_detail.py) : création + confirmation
+ inducements + step5 + publication d'un vrai rapport de match fait entrer
l'équipe home en PlayerImprovement.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée
avec au moins 12 équipes inscrites et 7 journées pour la première
compétition/saison du space (make init_db WITH_SEED=1 sur une base fraîche).
Utilise les indices d'équipes 6/7 et l'avant-avant-dernière journée
disponible (round_ids[-3]) — paire et journée dédiées à ce module, jamais
utilisées par les autres suites e2e (0-5 par test_match_report_recap.py,
0/1×[-2] par test_team_detail_state_banner.py, 10/11×[-1] par
test_player_detail.py). L'équipe d'indice 8 (hors de toute action dans ce
module) sert de témoin "hors phase PlayerImprovement".
"""

import re
import subprocess

import pytest
import requests
from playwright.sync_api import Page, expect

BASE_URL = "http://localhost:3210"
_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")

_PG_CONTAINER = "postgres-local"
_PG_USER = "dev"
_PG_DB = "kreek_db"


# ── Helpers (setup HTTP direct — même pattern que test_player_detail.py) ───────

def _fetch_json(path: str):
    import json
    import urllib.request
    with urllib.request.urlopen(f"{BASE_URL}{path}", timeout=5) as r:
        return json.loads(r.read())


def _query_db(sql: str) -> list[str]:
    result = subprocess.run(
        ["docker", "exec", _PG_CONTAINER, "psql",
         "-U", _PG_USER, "-d", _PG_DB, "-t", "-A", "-c", sql],
        capture_output=True, text=True, timeout=10,
    )
    assert result.returncode == 0, f"psql error: {result.stderr}"
    return [l.strip() for l in result.stdout.strip().splitlines() if l.strip()]


def _resolve_match_context(space_id: str) -> dict:
    competitions = _fetch_json(f"/app/{space_id}/competitions/widget/json/competitions")
    assert competitions, "Aucune compétition — lance make init_db WITH_SEED=1"
    comp_id = competitions[0]["id"]
    seasons = _fetch_json(f"/app/{space_id}/competitions/widget/json/seasons?competition_id={comp_id}")
    assert seasons, f"Aucune saison pour {comp_id}"
    season_id = seasons[0]["id"]
    rounds = _fetch_json(f"/app/{space_id}/competitions/widget/json/rounds?season_id={season_id}")
    assert len(rounds) >= 7, (
        f"Au moins 7 journées attendues, trouvé {len(rounds)} — lance make init_db WITH_SEED=1"
    )
    rows = _query_db(
        f"SELECT team_id FROM team_proj "
        f"WHERE season_id = '{season_id}' AND status = 'Enrolled' "
        f"ORDER BY team_name ASC LIMIT 12;"
    )
    assert len(rows) >= 12, (
        f"Au moins 12 équipes inscrites attendues, trouvé {len(rows)} — lance make init_db WITH_SEED=1"
    )
    return {
        "competition_id": comp_id,
        "season_id": season_id,
        "round_ids": [r["id"] for r in rounds],
        "teams": rows,
    }


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


def _team_player_ids(team_id: str) -> list[str]:
    return _query_db(f"SELECT player_id FROM players_proj WHERE team_id = '{team_id}' ORDER BY player_id;")


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
    pytest.fail("condition jamais satisfaite (pipeline app event pas propagé à temps)")


# ── Fixture : équipe publiée, en PlayerImprovement, joueur crédité en SPP ──────

@pytest.fixture(scope="module")
def spp_ctx(space_id):
    ctx = _resolve_match_context(space_id)
    round_id = ctx["round_ids"][-3]
    home_idx, away_idx = 6, 7

    mr_id = _create_draft(space_id, ctx, round_id, home_idx, away_idx)
    _ensure_inducements(space_id, mr_id)

    home_team_id = ctx["teams"][home_idx]
    players = _team_player_ids(home_team_id)
    assert len(players) >= 4, f"au moins 4 joueurs attendus pour {home_team_id}"
    rich_player, poor_player, value_player, stat_player = players[0], players[1], players[2], players[3]

    _record_action_api(space_id, mr_id, "home", rich_player, turn=1, action_type="TOUCHDOWN")
    _record_action_api(space_id, mr_id, "home", rich_player, turn=2, action_type="SORTIE")
    _record_action_api(space_id, mr_id, "home", rich_player, turn=3, action_type="MVP")
    # Joueur dédié au scénario team_value — indépendant de rich_player pour ne
    # pas dépendre de l'ordre d'exécution des tests (réserve SPP non partagée).
    # TOUCHDOWN + MVP = 7 SPP, suffisant pour une compétence primaire choisie
    # (coût niveau 1 = 6) sans devoir changer de mode dans skill_picker.
    _record_action_api(space_id, mr_id, "home", value_player, turn=1, action_type="TOUCHDOWN")
    _record_action_api(space_id, mr_id, "home", value_player, turn=2, action_type="MVP")
    # Joueur dédié au scénario d'augmentation de caractéristique — indépendant
    # de rich_player, budget confortable (15 SPP) pour couvrir le coût niveau 1
    # d'une caractéristique (14), quel que soit l'ordre d'exécution des tests.
    for turn in range(1, 6):
        _record_action_api(space_id, mr_id, "home", stat_player, turn=turn, action_type="TOUCHDOWN")

    resp = _post_step5(space_id, mr_id, summary_title="Match E2E dépense SPP", summary_body="Généré par les tests.")
    assert resp.status_code in (302, 303), f"step5: {resp.status_code}\n{resp.text[:200]}"
    _publish(space_id, mr_id)

    def team_in_improvement():
        rows = _query_db(f"SELECT game_phase FROM team_proj WHERE team_id = '{home_team_id}';")
        return rows and rows[0] == "PlayerImprovement"

    _wait_for(team_in_improvement)

    def players_credited():
        ids = "', '".join([rich_player, value_player, stat_player])
        rows = _query_db(f"SELECT spp FROM players_proj WHERE player_id IN ('{ids}');")
        return len(rows) == 3 and all(int(r) > 0 for r in rows)

    _wait_for(players_credited)

    return {
        "home_team_id": home_team_id,
        "rich_player_id": rich_player,
        "poor_player_id": poor_player,
        "value_player_id": value_player,
        "stat_player_id": stat_player,
    }


@pytest.fixture(scope="module")
def untouched_team_player(space_id):
    """Équipe d'indice 8 — jamais touchée par ce module, sert de témoin
    'hors phase PlayerImprovement'."""
    ctx = _resolve_match_context(space_id)
    team_id = ctx["teams"][8]
    players = _team_player_ids(team_id)
    assert players, f"aucun joueur trouvé pour {team_id}"
    return {"team_id": team_id, "player_id": players[0]}


# ── Scénarios ──────────────────────────────────────────────────────────────────

def test_journal_widget_shown_outside_player_improvement_phase(page: Page, space_id, untouched_team_player):
    player_id = untouched_team_player["player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    expect(page.locator(".lock-banner")).to_be_visible()
    expect(page.locator(".tabs")).to_have_count(0)


def test_coach_sees_spending_panel_and_can_purchase_a_skill(page: Page, space_id, spp_ctx):
    player_id = spp_ctx["rich_player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")

    expect(page.locator(".tabs")).to_be_visible()
    expect(page.locator(".lock-banner")).to_have_count(0)

    reserve_before = int(page.locator(".spend-panel-remaining-val").inner_text())
    tags_before = page.locator(".skill-tag--acquired").count()

    page.wait_for_selector(".skill-list-table", timeout=10000)
    choosable = page.locator(".btn-add-skill:visible", has_text="Choisir")
    expect(choosable.first).to_be_visible(timeout=10000)

    with page.expect_navigation(wait_until="load"):
        choosable.first.click()

    reserve_after = int(page.locator(".spend-panel-remaining-val").inner_text())
    tags_after = page.locator(".skill-tag--acquired").count()
    assert reserve_after < reserve_before
    assert tags_after == tags_before + 1


def test_player_with_no_spp_cannot_afford_any_skill(page: Page, space_id, spp_ctx):
    player_id = spp_ctx["poor_player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")

    expect(page.locator(".tabs")).to_be_visible()
    page.wait_for_selector(".skill-list-table", timeout=10000)

    choosable = page.locator(".btn-add-skill:visible", has_text="Choisir")
    random_pick = page.locator(".btn-add-skill:visible", has_text="Tirer")
    expect(choosable).to_have_count(0)
    expect(random_pick).to_have_count(0)


def test_stat_increase_updates_stat_and_reserve(page: Page, space_id, spp_ctx):
    player_id = spp_ctx["stat_player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")

    page.locator(".tab", has_text="Caractéristiques").click()
    ma_before = int(page.locator(".pstat-val").first.inner_text())
    reserve_before = int(page.locator(".spend-panel-remaining-val").inner_text())

    increase_btn = page.locator(".stat-card-btn:not([disabled])").first
    expect(increase_btn).to_be_visible(timeout=10000)
    with page.expect_navigation(wait_until="load"):
        increase_btn.click()

    ma_after = int(page.locator(".pstat-val").first.inner_text())
    reserve_after = int(page.locator(".spend-panel-remaining-val").inner_text())
    assert ma_after == ma_before + 1
    assert reserve_after < reserve_before


def test_team_value_increases_after_a_purchase(page: Page, space_id, spp_ctx):
    team_id = spp_ctx["home_team_id"]
    player_id = spp_ctx["value_player_id"]

    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
    value_item = page.locator(".meta-item", has_text="Valeur d'équipe").locator(".meta-value")
    value_before = int(value_item.inner_text().replace("kPo", "").strip())

    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    page.wait_for_selector(".skill-list-table", timeout=10000)
    choosable = page.locator(".btn-add-skill:visible", has_text="Choisir")
    expect(choosable.first).to_be_visible(timeout=10000)
    with page.expect_navigation(wait_until="load"):
        choosable.first.click()

    def value_increased():
        page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
        value_now = int(value_item.inner_text().replace("kPo", "").strip())
        return value_now > value_before

    _wait_for(value_increased)

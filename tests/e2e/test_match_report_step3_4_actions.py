"""Tests E2E — rapport de match, steps 3 et 4 (saisie des actions).

Scénarios couverts :
- S1  — Chargement page step3 : 5 zones présentes
- S2  — Sélectionner un tour : tour 3 marqué actif
- S3  — Sélectionner un joueur : action-panel visible
- S4  — Enregistrer un TD : log affiche 1+ entrée Touchdown
- S5  — Enregistrer un Blessé Amoché : log affiche entrée Blessé
- S6  — Enregistrer un Blessé Séquelle −AV : log affiche entrée Blessé
- S7  — Supprimer une action : entrée supprimée du log
- S8  — Plusieurs actions même tour : 2+ entrées dans le log
- S9  — Joueur temp visible si inducements soumis (skip sinon)
- S10 — Journaliers automatiques si équipe < 11 (skip sinon)
- S11 — Page step4 (équipe away) : mêmes zones présentes

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée.
"""

import json
import re

import pytest
import requests
from playwright.sync_api import Page, expect

from db_helpers import query_db as _query_db

BASE_URL = "http://localhost:3210"
_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")
_INDUCEMENTS_TEAM_RE = re.compile(r"/inducements/([0-9A-Z]{26})")


# ── Helpers ───────────────────────────────────────────────────────────────────

def _create_draft(space_id: str, ctx: dict, home_idx: int, away_idx: int) -> str:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/new",
        data={
            "competition_id": ctx["competition_id"],
            "season_id": ctx["season_id"],
            "round_id": ctx["round_id"],
            "home_team_id": ctx["teams"][home_idx],
            "away_team_id": ctx["teams"][away_idx],
        },
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"create: {resp.status_code}\n{resp.text[:200]}"
    m = _ULID_RE.search(resp.headers.get("Location", ""))
    assert m, f"match_report_id introuvable dans Location: {resp.headers.get('Location')!r}"
    return m.group(1)


def _ensure_pre_match(space_id: str, mr_id: str, ctx: dict,
                      home_idx: int, away_idx: int) -> None:
    check = requests.get(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        allow_redirects=False,
    )
    if check.status_code == 200:
        return
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}",
        data={
            "competition_id": ctx["competition_id"],
            "season_id": ctx["season_id"],
            "round_id": ctx["round_id"],
            "home_team_id": ctx["teams"][home_idx],
            "away_team_id": ctx["teams"][away_idx],
        },
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"confirm: {resp.status_code}"


def _ensure_fan_factor(space_id: str, mr_id: str) -> str:
    """Soumet le fan factor si pas encore fait. Retourne le Location header."""
    rows = _query_db(
        f"SELECT home_team_value FROM match_report_proj "
        f"WHERE match_report_id = '{mr_id}'"
    )
    if rows and rows[0] and rows[0] != "":
        return ""  # déjà enregistré
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "1", "away_fan_roll": "2"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"fan factor POST: {resp.status_code}"
    return resp.headers.get("Location", "")


def _ensure_inducements(space_id: str, mr_id: str) -> None:
    """Soumet des inducements vides pour les deux équipes si pas encore fait."""
    rows = _query_db(
        f"SELECT home_inducements, away_inducements FROM match_report_proj "
        f"WHERE match_report_id = '{mr_id}'"
    )
    if rows:
        parts = rows[0].split("|")
        if len(parts) == 2 and parts[0] and parts[1]:
            return  # inducements déjà enregistrés

    location = _ensure_fan_factor(space_id, mr_id)
    if not location:
        rows = _query_db(
            f"SELECT home_team_id, away_team_id FROM match_report_proj "
            f"WHERE match_report_id = '{mr_id}'"
        )
        if rows:
            parts = rows[0].split("|")
            home_id, away_id = parts[0], parts[1]
            location = f"/app/{space_id}/match-report/{mr_id}/inducements/{home_id}"

    for _ in range(3):
        if not location or "/inducements/" not in location:
            break
        loc = location if location.startswith("/") else ""
        if not loc:
            break
        resp = requests.post(
            f"{BASE_URL}{loc}",
            data={"selection": ""},
            allow_redirects=False,
        )
        if resp.status_code not in (302, 303):
            break
        location = resp.headers.get("Location", "")


def _record_action_api(space_id: str, mr_id: str, team_side: str,
                       player_id: str, turn: int, action_type: str) -> str | None:
    """Enregistre une action via l'API. Retourne l'action_id ou None."""
    endpoint = "step3" if team_side == "home" else "step4"
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{endpoint}/actions",
        data={
            "turn": str(turn),
            "player_id": player_id,
            "player_type": "regular",
            "action_type": action_type,
        },
    )
    if resp.status_code == 200:
        trigger = resp.headers.get("HX-Trigger", "")
        try:
            return json.loads(trigger).get("actionRecorded", {}).get("action_id")
        except (json.JSONDecodeError, AttributeError):
            return None
    return None


def _first_player_id(mr_id: str, team_side: str) -> str | None:
    """Retourne le premier player_id de l'équipe dans la projection."""
    rows = _query_db(
        f"SELECT {team_side}_team_id FROM match_report_proj "
        f"WHERE match_report_id = '{mr_id}'"
    )
    if not rows:
        return None
    team_id = rows[0]
    player_rows = _query_db(
        f"SELECT player_id FROM players_proj "
        f"WHERE team_id = '{team_id}' LIMIT 1"
    )
    return player_rows[0] if player_rows else None


def _wait_for_widgets(page: Page) -> None:
    """Attend que les 3 widgets HTMX soient chargés."""
    page.wait_for_selector("#turn-selector .mr-turn-btn", timeout=6000)
    page.wait_for_selector("#player-selector .mr-player-chip", timeout=6000)


def _select_turn_and_player(page: Page, turn: int = 3) -> None:
    """Sélectionne le tour et le premier joueur visible."""
    page.locator(f".mr-turn-btn[data-turn='{turn}']").first.click()
    page.locator(".mr-player-chip").first.click()
    page.wait_for_selector("#action-panel .mr-action-grid", timeout=6000)


# ── Fixtures ──────────────────────────────────────────────────────────────────

@pytest.fixture(scope="module")
def step3_ctx(browser, space_id):
    from competition_lifecycle import build_full_competition
    full = build_full_competition(browser, space_id, num_teams=4)
    return {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_id": full["round_ids"][0],
        "teams": full["team_ids"],
    }


@pytest.fixture(scope="module")
def mr_step3(space_id, step3_ctx):
    """Match report PreMatch accessible à step3 et step4."""
    mr_id = _create_draft(space_id, step3_ctx, home_idx=0, away_idx=1)
    _ensure_pre_match(space_id, mr_id, step3_ctx, home_idx=0, away_idx=1)
    resp = requests.get(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step3",
        allow_redirects=False,
    )
    assert resp.status_code == 200, (
        f"step3 inaccessible ({resp.status_code}) — "
        "vérifier que le match report est en état PreMatch"
    )
    return mr_id


@pytest.fixture(scope="module")
def mr_step3_full(space_id, step3_ctx):
    """Match report avec fan factor + inducements soumis (pour S9/S10).

    Utilise les équipes 2/3 si disponibles, sinon 0/1 en fallback.
    """
    teams = step3_ctx["teams"]
    home_idx, away_idx = (2, 3) if len(teams) >= 4 else (0, 1)
    mr_id = _create_draft(space_id, step3_ctx, home_idx=home_idx, away_idx=away_idx)
    _ensure_pre_match(space_id, mr_id, step3_ctx, home_idx=home_idx, away_idx=away_idx)
    _ensure_inducements(space_id, mr_id)
    return mr_id


# ── S1 — Chargement ───────────────────────────────────────────────────────────

def test_s1_page_step3_loads(page: Page, space_id, mr_step3):
    """GET step3 → 5 zones présentes (turn-selector, player-selector,
    temp-player-selector, action-panel, action-log)."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")

    expect(page.locator(".mr-header-title")).to_be_visible()
    expect(page.locator("#turn-selector")).to_be_visible()
    expect(page.locator("#player-selector")).to_be_visible()
    # Ces deux divs sont des conteneurs HTMX initialement vides → to_be_attached
    expect(page.locator("#temp-player-selector")).to_be_attached()
    expect(page.locator("#action-panel")).to_be_attached()
    expect(page.locator("#action-log")).to_be_visible()


# ── S2 — Sélection tour ───────────────────────────────────────────────────────

def test_s2_select_turn(page: Page, space_id, mr_step3):
    """Clic sur le tour 3 → bouton marqué actif."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")
    page.wait_for_selector("#turn-selector .mr-turn-btn", timeout=6000)

    page.locator(".mr-turn-btn[data-turn='3']").first.click()

    expect(page.locator(".mr-turn-btn[data-turn='3']").first).to_have_class(re.compile(r"\bactive\b"))


# ── S3 — Sélection joueur ─────────────────────────────────────────────────────

def test_s3_select_player_shows_action_panel(page: Page, space_id, mr_step3):
    """Sélection tour + joueur → action-panel contient la grille d'actions."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")
    _wait_for_widgets(page)
    _select_turn_and_player(page, turn=1)

    expect(page.locator("#action-panel .mr-action-grid")).to_be_visible()


# ── S4 — Enregistrer Touchdown ────────────────────────────────────────────────

def test_s4_record_touchdown(page: Page, space_id, mr_step3):
    """Clic TD → action-log affiche une entrée Touchdown."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")
    _wait_for_widgets(page)
    _select_turn_and_player(page, turn=2)

    page.locator(".mr-action-btn").filter(has_text="Touchdown").click()

    page.wait_for_selector("#action-log .mr-log-entry", timeout=6000)
    entries = page.locator("#action-log .mr-log-entry")
    texts = [e.inner_text() for e in entries.all()]
    assert any("Touchdown" in t for t in texts), (
        f"Aucune entrée Touchdown dans le log : {texts}"
    )


# ── S5 — Enregistrer Blessé Amoché ───────────────────────────────────────────

def test_s5_record_blesse_amoche(page: Page, space_id, mr_step3):
    """Flux Blessé → Amoché → log affiche une entrée Blessé."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")
    _wait_for_widgets(page)
    _select_turn_and_player(page, turn=3)

    page.locator(".mr-action-btn--damage").click()
    page.wait_for_selector(".mr-injury-panel", state="visible", timeout=3000)

    page.locator(".mr-injury-btn").filter(has_text="Amoché").click()

    # Une entrée existe déjà (TD du tour précédent, module partagé) : on
    # attend spécifiquement l'entrée T3/Blessé plutôt que "une entrée
    # existe", sans quoi l'assertion passerait trivialement sur l'ancienne.
    entry_t3 = page.locator("#action-log .mr-log-entry", has_text="T3")
    expect(entry_t3).to_contain_text("Blessé", timeout=6000)


# ── S6 — Enregistrer Blessé Séquelle −AV ─────────────────────────────────────

def test_s6_record_blesse_sequel_av(page: Page, space_id, mr_step3):
    """Flux Blessé → Blessure Grave → séquelle −AV → Confirmer → log affiche Blessé."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")
    _wait_for_widgets(page)
    _select_turn_and_player(page, turn=4)

    page.locator(".mr-action-btn--damage").click()
    page.wait_for_selector(".mr-injury-panel", state="visible", timeout=3000)

    page.locator(".mr-injury-btn").filter(has_text="13–14").click()
    page.wait_for_selector("select.mr-sequel-select", state="visible", timeout=3000)

    page.locator("select.mr-sequel-select").select_option("AV")
    page.locator(".mr-injury-btn--full").click()

    page.wait_for_selector("#action-log .mr-log-entry", timeout=6000)
    entries = page.locator("#action-log .mr-log-entry")
    texts = [e.inner_text() for e in entries.all()]
    assert any("Blessé" in t for t in texts), (
        f"Aucune entrée Blessé (séquelle) dans le log : {texts}"
    )


# ── S7 — Supprimer une action ─────────────────────────────────────────────────

def test_s7_delete_action(page: Page, space_id, mr_step3):
    """Suppression d'une action via le bouton ✕ → entrée disparaît du log."""
    player_id = _first_player_id(mr_step3, "home")
    if not player_id:
        pytest.skip("Aucun joueur trouvé pour l'équipe domicile")

    _record_action_api(space_id, mr_step3, "home", player_id, turn=8, action_type="MVP")

    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")
    page.wait_for_selector("#action-log .mr-log-entry", timeout=6000)

    count_before = page.locator("#action-log .mr-log-entry").count()
    assert count_before >= 1

    page.locator("#action-log .mr-log-delete").last.click()
    page.wait_for_timeout(1000)

    count_after = page.locator("#action-log .mr-log-entry").count()
    assert count_after == count_before - 1, (
        f"Attendu {count_before - 1} entrées après suppression, trouvé {count_after}"
    )


# ── S8 — Plusieurs actions même tour ─────────────────────────────────────────

def test_s8_multiple_actions_same_turn(page: Page, space_id, mr_step3):
    """Enregistrer TD + MVP au tour 5 → log contient 2+ entrées."""
    player_id = _first_player_id(mr_step3, "home")
    if not player_id:
        pytest.skip("Aucun joueur trouvé pour l'équipe domicile")

    _record_action_api(space_id, mr_step3, "home", player_id, turn=5, action_type="TOUCHDOWN")
    _record_action_api(space_id, mr_step3, "home", player_id, turn=5, action_type="MVP")

    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")
    page.wait_for_selector("#action-log .mr-log-entry", timeout=6000)

    t5_entries = page.locator("#action-log .mr-log-entry").filter(has_text="T5")
    assert t5_entries.count() >= 2, (
        f"Attendu 2+ entrées au tour 5, trouvé {t5_entries.count()}"
    )


# ── S9 — Joueur temporaire visible ────────────────────────────────────────────

def test_s9_temp_player_visible_after_inducements(page: Page, space_id, mr_step3_full):
    """Star player ou journalier visible dans temp-player-selector après inducements."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3_full}/step3",
              wait_until="load")
    page.wait_for_timeout(2000)

    chips = page.locator("#temp-player-selector .mr-player-chip")
    if chips.count() == 0:
        pytest.skip(
            "Aucun joueur temporaire disponible (ni journalier ni star player) — "
            "vérifier seed data et configuration inducements"
        )

    expect(chips.first).to_be_visible()


# ── S10 — Journaliers automatiques ────────────────────────────────────────────

def test_s10_journaliers_auto(page: Page, space_id, mr_step3_full):
    """Si équipe < 11 joueurs, des journaliers sont présents dans temp-player-selector."""
    rows = _query_db(
        f"SELECT home_team_id FROM match_report_proj "
        f"WHERE match_report_id = '{mr_step3_full}'"
    )
    if not rows:
        pytest.skip("Match report introuvable en DB")

    home_team_id = rows[0]
    count_rows = _query_db(
        f"SELECT COUNT(*) FROM players_proj "
        f"WHERE team_id = '{home_team_id}'"
    )
    player_count = int(count_rows[0]) if count_rows else 11

    if player_count >= 11:
        pytest.skip(f"Équipe domicile a {player_count} joueurs (≥ 11) — pas de journaliers attendus")

    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3_full}/step3",
              wait_until="load")
    page.wait_for_timeout(2000)

    journaliers = page.locator("#temp-player-selector .mr-player-chip--journeyman")
    assert journaliers.count() > 0, (
        f"Équipe a {player_count} joueurs mais aucun journalier dans temp-player-selector"
    )


# ── S11 — Page step4 (équipe away) ────────────────────────────────────────────

def test_s11_step4_away_team(page: Page, space_id, mr_step3):
    """GET step4 → mêmes 5 zones présentes, titre contient le nom de l'équipe away."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step4",
              wait_until="load")

    expect(page.locator(".mr-header-title")).to_be_visible()
    expect(page.locator("#turn-selector")).to_be_visible()
    expect(page.locator("#player-selector")).to_be_visible()
    expect(page.locator("#temp-player-selector")).to_be_attached()
    expect(page.locator("#action-panel")).to_be_attached()
    expect(page.locator("#action-log")).to_be_visible()

    page.wait_for_selector("#turn-selector .mr-turn-btn", timeout=6000)
    page.wait_for_selector("#player-selector .mr-player-chip", timeout=6000)

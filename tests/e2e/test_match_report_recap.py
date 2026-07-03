"""Tests E2E — rapport de match, page récap.

Scénarios couverts :
- TC-01 — Page charge en état ReadyToPublish : CTA "Publier" + "← Modifier étape 5"
- TC-02 — Sortie sans badge de blessure ; Bilan sanitaire ne liste que les Blesse{injury}
- TC-03 — MVP affiché à la fois en sidebar et en fin de chronologie
- TC-04 — Publication (clic bouton) → CTA devient "Retour" + "Voir fiche"
- TC-05 — Double publication (POST direct sur un rapport déjà publié) → 409
- TC-06 — Draft/PreMatch → 404, Cancelled → 410
- TC-07 — Dégradation gracieuse si find_round_context échoue (pas de bandeau contexte)
- TC-08 — Carte Performances (SPP) absente proprement si aucune action enregistrée

Allocation des paires d'équipes : une équipe est verrouillée (TeamNotAvailable) dès la
confirmation (Draft → PreMatch) — verrou permanent en l'état actuel de l'appli, aucune remise
à ReadyToPlay après publication (comportement connu, hors scope ici). Chaque scénario qui
confirme un match report (mr_ready, mr_for_click_publish, mr_published, TC-06 PreMatch, TC-07,
TC-08) utilise donc sa propre paire d'équipes jamais réutilisée. Les scénarios Draft/Cancelled
de TC-06 ne confirment jamais — ils réutilisent librement la première paire.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée avec au moins
12 équipes inscrites et 7 journées pour la première compétition/saison du space (make init_db
WITH_SEED=1 sur une base fraîche).
"""

import json
import re
import subprocess
import time

import pytest
import requests
from playwright.sync_api import Page, expect

BASE_URL = "http://localhost:3210"
_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")

_PG_CONTAINER = "postgres-local"
_PG_USER = "dev"
_PG_DB = "kreek_db"

_INJURY_LABELS = ("Commotion", "Amoché", "Blessure Sérieuse", "Séquelle", "Mort")
_FAKE_ROUND_ID = "01ARZ3NDEKTSV4RRFFQ69G5FAV"


# ── Helpers ───────────────────────────────────────────────────────────────────

def _fetch_json(path: str):
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
        f"Au moins 7 journées attendues pour isoler les scénarios, trouvé {len(rounds)} — "
        "lance make init_db WITH_SEED=1"
    )
    rows = _query_db(
        f"SELECT team_id FROM team_proj "
        f"WHERE season_id = '{season_id}' AND status = 'Enrolled' "
        f"ORDER BY team_name ASC LIMIT 12;"
    )
    assert len(rows) >= 12, (
        f"Au moins 12 équipes inscrites attendues (6 paires jamais réutilisées — une équipe "
        f"reste verrouillée après confirmation d'un match, cf. docstring), trouvé {len(rows)} — "
        "lance make init_db WITH_SEED=1"
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


def _ensure_pre_match(space_id: str, mr_id: str, ctx: dict, round_id: str, home_idx: int, away_idx: int) -> None:
    """Confirme la sélection si nécessaire — no-op si déjà PreMatch (dédup via /new)."""
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
    """Fan factor puis inducements vides pour les deux équipes."""
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
    data = {
        "home_gain": "50000",
        "away_gain": "40000",
        "home_fan_mod": "1",
        "away_fan_mod": "-1",
    }
    data.update(overrides)
    return requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step5",
        data=data,
        allow_redirects=False,
    )


def _publish(space_id: str, mr_id: str) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap/publish",
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"publish: {resp.status_code}\n{resp.text[:200]}"


def _advance_to_ready_to_publish(space_id: str, ctx: dict, round_id: str,
                                  home_idx: int, away_idx: int,
                                  with_actions: bool = True,
                                  publish_after: bool = False) -> str:
    """Crée un match report et l'avance jusqu'à ReadyToPublish (option : publie ensuite,
    ce qui libère immédiatement la paire d'équipes pour un scénario suivant)."""
    mr_id = _create_draft(space_id, ctx, round_id, home_idx, away_idx)
    _ensure_pre_match(space_id, mr_id, ctx, round_id, home_idx, away_idx)
    _ensure_inducements(space_id, mr_id)

    if with_actions:
        home_player = _first_player_id(mr_id, "home")
        away_player = _first_player_id(mr_id, "away")
        _record_action_api(space_id, mr_id, "home", home_player, turn=1, action_type="TOUCHDOWN")
        _record_action_api(space_id, mr_id, "home", home_player, turn=2, action_type="SORTIE")
        _record_action_api(space_id, mr_id, "away", away_player, turn=3,
                            action_type="BLESSE", injury_type="AMOCHE")
        _record_action_api(space_id, mr_id, "home", home_player, turn=4, action_type="MVP")
        _record_action_api(space_id, mr_id, "away", away_player, turn=5, action_type="MVP")

    resp = _post_step5(
        space_id, mr_id,
        summary_title="Match capté par les tests E2E",
        summary_body="Compte-rendu généré automatiquement.",
    )
    assert resp.status_code in (302, 303), f"step5: {resp.status_code}\n{resp.text[:200]}"

    if publish_after:
        _publish(space_id, mr_id)

    return mr_id


# ── Fixtures ──────────────────────────────────────────────────────────────────

@pytest.fixture(scope="module")
def recap_ctx(space_id):
    return _resolve_match_context(space_id)


@pytest.fixture(scope="module")
def mr_ready(space_id, recap_ctx):
    """Paire teams[0..1] — reste ReadyToPublish tout le module. TD/Sortie/Blessé/MVP des deux côtés."""
    round_id = recap_ctx["round_ids"][0]
    return _advance_to_ready_to_publish(space_id, recap_ctx, round_id, home_idx=0, away_idx=1)


@pytest.fixture(scope="module")
def mr_for_click_publish(space_id, recap_ctx):
    """Paire teams[2..3] — ReadyToPublish, publiée par TC-04 via clic."""
    round_id = recap_ctx["round_ids"][1]
    return _advance_to_ready_to_publish(
        space_id, recap_ctx, round_id, home_idx=2, away_idx=3, with_actions=False,
    )


@pytest.fixture(scope="module")
def mr_published(space_id, recap_ctx):
    """Paire teams[4..5] — publiée via API directe pour TC-05."""
    round_id = recap_ctx["round_ids"][2]
    return _advance_to_ready_to_publish(
        space_id, recap_ctx, round_id, home_idx=4, away_idx=5,
        with_actions=False, publish_after=True,
    )


# ── TC-01 — Chargement en état ReadyToPublish ─────────────────────────────────

def test_tc01_recap_loads_ready_to_publish(page: Page, space_id, mr_ready):
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_ready}/recap", wait_until="load")

    expect(page.locator(".ms-hero")).to_be_visible()
    expect(page.locator(".ms-btn-primary", has_text="Publier")).to_be_visible()
    expect(page.locator(".ms-btn-outline", has_text="Modifier étape 5")).to_be_visible()
    expect(page.locator(".ms-btn-outline", has_text="Retour aux résultats")).to_have_count(0)


# ── TC-02 — Sortie sans badge de blessure / Bilan sanitaire ──────────────────

def test_tc02_sortie_has_no_injury_label(page: Page, space_id, mr_ready):
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_ready}/recap", wait_until="load")

    sortie_event = page.locator(".ms-event", has_text="Sortie").first
    expect(sortie_event).to_be_visible()
    sortie_text = sortie_event.inner_text()
    for label in _INJURY_LABELS:
        assert label not in sortie_text, (
            f"L'événement Sortie ne devrait porter aucun libellé de blessure, trouvé {label!r}"
        )


def test_tc02_bilan_sanitaire_only_lists_blesse(page: Page, space_id, mr_ready):
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_ready}/recap", wait_until="load")

    injury_rows = page.locator(".ms-injury-row")
    expect(injury_rows).to_have_count(1)
    expect(injury_rows.first.locator(".ms-injury-badge")).to_have_text("Amoché")


# ── TC-03 — MVP affiché deux fois ─────────────────────────────────────────────

def test_tc03_mvp_shown_in_sidebar_and_timeline(page: Page, space_id, mr_ready):
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_ready}/recap", wait_until="load")

    expect(page.locator(".ms-mvp-row")).to_have_count(2)
    mvp_timeline_entries = page.locator(".ms-event-turn--mvp")
    expect(mvp_timeline_entries).to_have_count(2)


# ── TC-04 — Publication via clic bouton ───────────────────────────────────────

def test_tc04_publish_button_updates_cta(page: Page, space_id, mr_for_click_publish):
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_for_click_publish}/recap", wait_until="load")
    expect(page.locator(".ms-btn-primary", has_text="Publier")).to_be_visible()

    page.locator(".ms-btn-primary", has_text="Publier").click()
    page.wait_for_load_state("load")

    assert page.url.endswith(f"/match-report/{mr_for_click_publish}/recap")
    expect(page.locator(".ms-btn-outline", has_text="Retour aux résultats")).to_be_visible()
    expect(page.locator(".ms-btn-primary", has_text="Voir la fiche")).to_be_visible()
    expect(page.locator(".ms-btn-primary", has_text="Publier")).to_have_count(0)


# ── TC-05 — Double publication refusée ────────────────────────────────────────

def test_tc05_republish_returns_conflict(space_id, mr_published):
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_published}/recap/publish",
        allow_redirects=False,
    )
    assert resp.status_code == 409, f"attendu 409, obtenu {resp.status_code}"


# ── TC-06 — États non accessibles ─────────────────────────────────────────────

def test_tc06_draft_returns_404(space_id, recap_ctx):
    """teams[0..1] réutilisée sans confirmation — aucun verrou posé sur un Draft."""
    round_id = recap_ctx["round_ids"][3]
    mr_id = _create_draft(space_id, recap_ctx, round_id, home_idx=0, away_idx=1)
    resp = requests.get(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap", allow_redirects=False)
    assert resp.status_code == 404


def test_tc06_pre_match_returns_404(space_id, recap_ctx):
    """Paire teams[6..7] dédiée — reste verrouillée en PreMatch, jamais réutilisée ensuite."""
    round_id = recap_ctx["round_ids"][4]
    mr_id = _create_draft(space_id, recap_ctx, round_id, home_idx=6, away_idx=7)
    _ensure_pre_match(space_id, mr_id, recap_ctx, round_id, home_idx=6, away_idx=7)
    resp = requests.get(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap", allow_redirects=False)
    assert resp.status_code == 404


def test_tc06_cancelled_returns_410(space_id, recap_ctx):
    """teams[0..1] réutilisée — le Draft est annulé, jamais confirmé, aucun verrou posé."""
    round_id = recap_ctx["round_ids"][5]
    mr_id = _create_draft(space_id, recap_ctx, round_id, home_idx=1, away_idx=0)

    rows = _query_db(f"SELECT pairing_id FROM match_report_proj WHERE match_report_id = '{mr_id}'")
    if not rows or not rows[0]:
        pytest.skip("Ce match report n'est rattaché à aucun pairing planifié — annulation impossible")
    pairing_id = rows[0]

    resp = requests.delete(
        f"{BASE_URL}/app/{space_id}/competitions/{recap_ctx['competition_id']}"
        f"/{recap_ctx['season_id']}/admin/schedule/delete-match",
        json={"pairing_id": pairing_id},
    )
    assert resp.status_code == 200, f"delete_match: {resp.status_code}\n{resp.text[:200]}"

    for _ in range(10):
        phase_rows = _query_db(
            f"SELECT phase FROM match_report_proj WHERE match_report_id = '{mr_id}'"
        )
        if phase_rows and phase_rows[0] == "Cancelled":
            break
        time.sleep(0.2)

    resp = requests.get(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap", allow_redirects=False)
    assert resp.status_code == 410


# ── TC-07 — Dégradation gracieuse round_context ───────────────────────────────

def test_tc07_graceful_degradation_missing_round_context(page: Page, space_id, recap_ctx):
    """round_id syntaxiquement valide mais inconnu → pas de bandeau contexte, page fonctionnelle.
    Paire teams[8..9] dédiée, jamais réutilisée ensuite."""
    mr_id = _advance_to_ready_to_publish(
        space_id, recap_ctx, _FAKE_ROUND_ID, home_idx=8, away_idx=9, with_actions=False,
    )

    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap", wait_until="load")

    expect(page.locator(".ms-hero-header")).to_have_count(0)
    expect(page.locator(".ms-hero")).to_be_visible()
    expect(page.locator(".ms-score-num").first).to_be_visible()


# ── TC-08 — Performances (SPP) absente sans action ────────────────────────────

def test_tc08_performances_card_absent_without_actions(page: Page, space_id, recap_ctx):
    """Paire teams[10..11] dédiée."""
    round_id = recap_ctx["round_ids"][6]
    mr_id = _advance_to_ready_to_publish(
        space_id, recap_ctx, round_id, home_idx=10, away_idx=11, with_actions=False,
    )

    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap", wait_until="load")

    expect(page.locator(".ms-card-header-title", has_text="Performances")).to_have_count(0)
    expect(page.locator(".ms-hero")).to_be_visible()

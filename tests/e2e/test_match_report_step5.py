"""Tests E2E — rapport de match, step 5 (séquence après-match).

Scénarios couverts :
- S1 — Accès step5 depuis état PreMatch : page affichée, score et CAS visibles
- S2 — Suggestion de gain pré-remplie dans les inputs (après fan factor enregistré)
- S3 — Soumission valide (gains + fan mods + résumé) → redirect, état ReadyToPublish
- S4 — Soumission minimale sans résumé → acceptée
- S5 — Re-soumission → formulaire pré-rempli avec valeurs existantes, modification acceptée
- S6 — Accès depuis état Draft → redirect vers edit match report

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée.
"""

import json
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
    seasons = _fetch_json(
        f"/app/{space_id}/competitions/widget/json/seasons?competition_id={comp_id}"
    )
    assert seasons, f"Aucune saison pour {comp_id}"
    season_id = seasons[0]["id"]
    rounds = _fetch_json(
        f"/app/{space_id}/competitions/widget/json/rounds?season_id={season_id}"
    )
    assert rounds, f"Aucun round pour {season_id}"
    round_id = rounds[0]["id"]
    rows = _query_db(
        f"SELECT team_id FROM team_projection "
        f"WHERE season_id = '{season_id}' AND status = 'Enrolled' "
        f"ORDER BY team_name ASC LIMIT 4;"
    )
    assert len(rows) >= 2, "Pas assez d'équipes inscrites — make init_db WITH_SEED=1"
    return {
        "competition_id": comp_id,
        "season_id": season_id,
        "round_id": round_id,
        "teams": rows,
    }


def _create_draft(space_id: str, ctx: dict, home_idx: int = 0, away_idx: int = 1) -> str:
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
    assert m, f"match_report_id introuvable dans Location"
    return m.group(1)


def _ensure_pre_match(space_id: str, mr_id: str, ctx: dict,
                      home_idx: int = 0, away_idx: int = 1) -> None:
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
    """Soumet le fan factor D3. Retourne Location header."""
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "2", "away_fan_roll": "3"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"fan factor POST: {resp.status_code}"
    return resp.headers.get("Location", "")


def _ensure_inducements(space_id: str, mr_id: str) -> None:
    """Passe les inducements (soumission vide) pour les deux équipes."""
    location = _ensure_fan_factor(space_id, mr_id)
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


def _advance_to_step5_ready(space_id: str, ctx: dict,
                             home_idx: int = 0, away_idx: int = 1) -> str:
    """Crée et avance un match report jusqu'à step5 accessible (PreMatch + fan factor)."""
    mr_id = _create_draft(space_id, ctx, home_idx, away_idx)
    _ensure_pre_match(space_id, mr_id, ctx, home_idx, away_idx)
    _ensure_inducements(space_id, mr_id)
    return mr_id


def _post_step5(space_id: str, mr_id: str, **form_overrides) -> requests.Response:
    """Soumet le formulaire step5 avec des valeurs par défaut."""
    data = {
        "home_gain": "50000",
        "away_gain": "40000",
        "home_fan_mod": "0",
        "away_fan_mod": "0",
    }
    data.update(form_overrides)
    return requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step5",
        data=data,
        allow_redirects=False,
    )


def _phase_in_db(mr_id: str) -> str:
    rows = _query_db(
        f"SELECT phase FROM match_report_projection "
        f"WHERE match_report_id = '{mr_id}'"
    )
    return rows[0] if rows else ""


# ── Fixtures ──────────────────────────────────────────────────────────────────

@pytest.fixture(scope="module")
def step5_ctx(space_id):
    return _resolve_match_context(space_id)


@pytest.fixture(scope="module")
def mr_step5(space_id, step5_ctx):
    """Match report PreMatch avec fan factor enregistré, step5 accessible."""
    return _advance_to_step5_ready(space_id, step5_ctx, home_idx=0, away_idx=1)


# ── S1 — Chargement de la page ────────────────────────────────────────────────

def test_s1_step5_page_loads(page: Page, space_id, mr_step5):
    """GET step5 → page affichée avec score banner, gains, fan factor, résumé."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step5}/step5",
              wait_until="load")

    expect(page.locator(".mr-header-title")).to_be_visible()
    expect(page.locator(".mr-score-banner")).to_be_visible()
    expect(page.locator(".mr-score-value").first).to_be_visible()
    expect(page.locator(".mr-gains-input").first).to_be_visible()
    expect(page.locator(".mr-fan-mod-btns").first).to_be_visible()
    expect(page.locator(".mr-steps")).to_be_visible()


# ── S2 — Suggestion de gain pré-remplie ──────────────────────────────────────

def test_s2_gain_suggestion_prefilled(page: Page, space_id, mr_step5):
    """Les inputs de gain sont pré-remplis avec une valeur > 0 (fan factor enregistré)."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step5}/step5",
              wait_until="load")

    home_input = page.locator("input[name='home_gain']")
    away_input = page.locator("input[name='away_gain']")
    expect(home_input).to_be_visible()
    expect(away_input).to_be_visible()

    home_val = int(home_input.input_value())
    away_val = int(away_input.input_value())
    assert home_val > 0, f"Suggestion home gain attendue > 0, got {home_val}"
    assert away_val > 0, f"Suggestion away gain attendue > 0, got {away_val}"


# ── S3 — Soumission valide avec résumé ───────────────────────────────────────

def test_s3_valid_submission_with_summary(space_id, step5_ctx):
    """POST step5 valide → redirect vers step5, rapport en ReadyToPublish."""
    mr_id = _advance_to_step5_ready(space_id, step5_ctx, home_idx=0, away_idx=1)

    resp = _post_step5(
        space_id, mr_id,
        home_gain="60000",
        away_gain="50000",
        home_fan_mod="1",
        away_fan_mod="-1",
        summary_title="Les Berserkers s'imposent",
        summary_body="Victoire au bout du suspense.",
    )
    assert resp.status_code in (302, 303), f"POST step5: {resp.status_code}\n{resp.text[:200]}"
    assert f"/step5" in resp.headers.get("Location", ""), \
        f"Redirect attendu vers step5, got: {resp.headers.get('Location')!r}"

    phase = _phase_in_db(mr_id)
    assert phase == "ReadyToPublish", f"Phase attendue ReadyToPublish, got {phase!r}"


# ── S4 — Soumission minimale (sans résumé) ───────────────────────────────────

def test_s4_minimal_submission_without_summary(space_id, step5_ctx):
    """POST step5 sans résumé → accepté, état ReadyToPublish."""
    mr_id = _advance_to_step5_ready(space_id, step5_ctx, home_idx=0, away_idx=1)

    resp = _post_step5(space_id, mr_id, home_gain="30000", away_gain="30000")
    assert resp.status_code in (302, 303), f"POST step5 minimal: {resp.status_code}"

    phase = _phase_in_db(mr_id)
    assert phase == "ReadyToPublish", f"Phase attendue ReadyToPublish, got {phase!r}"


# ── S5 — Re-soumission ────────────────────────────────────────────────────────

def test_s5_resubmission_prefills_existing_values(page: Page, space_id, step5_ctx):
    """Après une première soumission, step5 est pré-rempli avec les valeurs existantes."""
    mr_id = _advance_to_step5_ready(space_id, step5_ctx, home_idx=0, away_idx=1)

    # Première soumission
    resp = _post_step5(
        space_id, mr_id,
        home_gain="70000",
        away_gain="40000",
        home_fan_mod="2",
        summary_title="Premier résumé",
    )
    assert resp.status_code in (302, 303)

    # Vérifier que le formulaire est pré-rempli avec les valeurs existantes
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step5", wait_until="load")

    home_val = int(page.locator("input[name='home_gain']").input_value())
    away_val = int(page.locator("input[name='away_gain']").input_value())
    assert home_val == 70000, f"home_gain attendu 70000, got {home_val}"
    assert away_val == 40000, f"away_gain attendu 40000, got {away_val}"

    title_val = page.locator("input[name='summary_title']").input_value()
    assert title_val == "Premier résumé", f"summary_title attendu 'Premier résumé', got {title_val!r}"

    expect(page.locator(".mr-info-banner")).to_be_visible()

    # Re-soumission avec nouvelles valeurs
    resp2 = _post_step5(space_id, mr_id, home_gain="80000", away_gain="50000")
    assert resp2.status_code in (302, 303)

    phase = _phase_in_db(mr_id)
    assert phase == "ReadyToPublish"


# ── S6 — Accès depuis Draft → redirect ────────────────────────────────────────

def test_s6_draft_redirects_to_edit(space_id, step5_ctx):
    """GET step5 depuis état Draft → redirect vers edit match report."""
    mr_id = _create_draft(space_id, step5_ctx, home_idx=0, away_idx=1)

    resp = requests.get(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step5",
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"Redirect attendu, got {resp.status_code}"
    location = resp.headers.get("Location", "")
    assert "/match-report/" in location, f"Redirect inattendu: {location!r}"
    assert "/step5" not in location, f"Ne doit pas rediriger vers step5: {location!r}"

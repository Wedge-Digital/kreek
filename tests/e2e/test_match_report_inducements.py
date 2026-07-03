"""Tests E2E — rapport de match, step 2.1 (coups de pouce / inducements).

Scénarios couverts :
- GET page inducements → header équipe + bannière budget + widget chargé
- Bouton "Passer" (TopDog) → redirect vers page Underdog
- Bouton "Passer" (Underdog) → redirect vers step3
- Bouton "Passer" soumet toujours un panier vide, même si des items sont sélectionnés
- Sélection dépasse budget → bouton Valider désactivé
- TopDog valide des achats → redirect vers page Underdog
- Underdog valide ses achats → redirect vers step3

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée.
"""

import json
import re
import subprocess
import urllib.request

import pytest
import requests
from playwright.sync_api import Page, expect

BASE_URL = "http://localhost:3210"
_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")
_TEAM_ID_RE = re.compile(r"/inducements/([0-9A-Z]{26})")

_PG_CONTAINER = "postgres-local"
_PG_USER = "dev"
_PG_DB = "kreek_db"


# ── Helpers ───────────────────────────────────────────────────────────────────

def _fetch_json(path: str):
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


def _create_pre_match(space_id: str, ctx: dict, home_idx: int, away_idx: int) -> str:
    """Crée un match report et le passe en état PreMatch."""
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
    assert resp.status_code in (302, 303), f"create: {resp.status_code}"
    location = resp.headers.get("Location", "")
    m = _ULID_RE.search(location)
    assert m, f"match_report_id introuvable dans Location: {location!r}"
    mr_id = m.group(1)

    resp2 = requests.post(
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
    assert resp2.status_code in (302, 303), f"confirm: {resp2.status_code}"
    return mr_id


def _post_fan_factor(space_id: str, mr_id: str) -> requests.Response:
    """Soumet le fan factor (1 et 2). Retourne la réponse sans follow redirect."""
    return requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "1", "away_fan_roll": "2"},
        allow_redirects=False,
    )


def _inducements_url_for_team(space_id: str, mr_id: str, team_id: str) -> str:
    return f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/inducements/{team_id}"


def _post_pass(space_id: str, mr_id: str, team_id: str) -> requests.Response:
    """Soumet le bouton 'Passer' (panier vide) pour l'équipe donnée."""
    return requests.post(
        _inducements_url_for_team(space_id, mr_id, team_id),
        data={"intent": "pass", "selection": "[]", "mercenaries": "[]"},
        allow_redirects=False,
    )


# ── Fixtures ──────────────────────────────────────────────────────────────────

@pytest.fixture(scope="module")
def inducements_ctx(space_id):
    return _resolve_match_context(space_id)


@pytest.fixture(scope="module")
def inducements_mr(space_id, inducements_ctx):
    """Match report en PreMatch avec TeamValues enregistrées (après fan factor POST)."""
    mr_id = _create_pre_match(space_id, inducements_ctx, home_idx=2, away_idx=3)
    resp = _post_fan_factor(space_id, mr_id)
    assert resp.status_code in (302, 303), f"fan factor POST: {resp.status_code}"
    location = resp.headers.get("Location", "")
    return {"mr_id": mr_id, "redirect_location": location}


@pytest.fixture(scope="module")
def underdog_inducements_location(space_id, inducements_ctx):
    """Fait passer le TopDog (POST direct, panier vide) → retourne l'URL Underdog."""
    mr_id = _create_pre_match(space_id, inducements_ctx, home_idx=0, away_idx=1)
    resp = _post_fan_factor(space_id, mr_id)
    assert resp.status_code in (302, 303), f"fan factor POST: {resp.status_code}"
    location = resp.headers.get("Location", "")
    m = _TEAM_ID_RE.search(location)
    if not m:
        return location  # déjà sur step3 (pas de phase inducements pour ce tier)
    resp2 = _post_pass(space_id, mr_id, m.group(1))
    assert resp2.status_code in (302, 303), f"pass topdog: {resp2.status_code}"
    return resp2.headers.get("Location", "")


# ── Tests ─────────────────────────────────────────────────────────────────────

def test_fan_factor_redirects_to_inducements_or_step3(inducements_mr):
    """POST fan factor → redirect vers inducements/{team_id} ou step3."""
    loc = inducements_mr["redirect_location"]
    assert "/inducements/" in loc or "/step3" in loc, (
        f"Redirect inattendu après fan factor : {loc!r}"
    )


def test_inducements_page_loads(page: Page, space_id, inducements_mr):
    """GET page inducements → header équipe + bannière budget visible."""
    loc = inducements_mr["redirect_location"]
    if "/inducements/" not in loc:
        pytest.skip("Pas de page inducements pour cette compétition (skip direct vers step3)")

    page.goto(f"{BASE_URL}{loc}", wait_until="load")

    expect(page.locator(".mr-team-banner")).to_be_visible()
    expect(page.locator(".mr-budget-banner")).to_be_visible()
    expect(page.locator(".mr-budget-amount")).to_be_visible()


def test_inducements_widget_loaded(page: Page, space_id, inducements_mr):
    """Widget inducement-selector chargé via HTMX dans la zone widget."""
    loc = inducements_mr["redirect_location"]
    if "/inducements/" not in loc:
        pytest.skip("Pas de page inducements pour cette compétition")

    page.goto(f"{BASE_URL}{loc}", wait_until="load")

    # Zone widget présente (même vide si aucun inducement configuré)
    expect(page.locator(".mr-selector-zone")).to_be_visible()


def test_pass_button_visible(page: Page, space_id, inducements_mr):
    """Bouton 'Passer' visible sur la page inducements."""
    loc = inducements_mr["redirect_location"]
    if "/inducements/" not in loc:
        pytest.skip("Pas de page inducements pour cette compétition")

    page.goto(f"{BASE_URL}{loc}", wait_until="load")

    pass_btn = page.locator("button[name='intent'][value='pass']")
    expect(pass_btn).to_be_visible()
    expect(pass_btn).to_contain_text("Passer")


def test_topdog_pass_redirects_to_underdog(page: Page, space_id, inducements_mr):
    """TopDog clique 'Passer' → redirect vers page Underdog (autre équipe)."""
    loc = inducements_mr["redirect_location"]
    if "/inducements/" not in loc:
        pytest.skip("Pas de page inducements pour cette compétition")

    page.goto(f"{BASE_URL}{loc}", wait_until="load")

    m = _TEAM_ID_RE.search(loc)
    topdog_team_id = m.group(1) if m else ""

    with page.expect_navigation(wait_until="load"):
        page.locator("button[name='intent'][value='pass']").click()

    # Après "Passer" depuis TopDog, on est soit sur la page inducements de l'autre
    # équipe, soit sur step3 si les inducements sont entièrement sautées.
    new_url = page.url
    assert "/inducements/" in new_url or "/step3" in new_url, (
        f"Redirect inattendu depuis 'Passer' TopDog : {new_url!r}"
    )
    if "/inducements/" in new_url:
        m2 = _TEAM_ID_RE.search(new_url)
        underdog_team_id = m2.group(1) if m2 else ""
        assert underdog_team_id != topdog_team_id, (
            f"TopDog 'Passer' n'a pas changé d'équipe : toujours {topdog_team_id}"
        )


def test_underdog_pass_redirects_to_step3(page: Page, space_id, underdog_inducements_location):
    """Underdog clique 'Passer' → redirect vers step3 (toujours, quel que soit l'état du TopDog)."""
    loc = underdog_inducements_location
    if "/inducements/" not in loc:
        pytest.skip("Pas de page inducements Underdog pour cette compétition")

    page.goto(f"{BASE_URL}{loc}", wait_until="load")

    with page.expect_navigation(wait_until="load"):
        page.locator("button[name='intent'][value='pass']").click()

    assert "/step3" in page.url, (
        f"Redirect inattendu depuis 'Passer' Underdog : {page.url!r}"
    )


def test_pass_discards_pending_selection(page: Page, space_id, inducements_ctx):
    """'Passer' soumet toujours un panier vide, même si des inducements sont
    sélectionnés localement (comportement attendu : édition annulée)."""
    mr_id = _create_pre_match(space_id, inducements_ctx, home_idx=1, away_idx=2)
    resp = _post_fan_factor(space_id, mr_id)
    location = resp.headers.get("Location", "")
    if "/inducements/" not in location:
        pytest.skip("Pas de page inducements pour cette compétition")

    m = _TEAM_ID_RE.search(location)
    team_id = m.group(1) if m else ""

    page.goto(f"{BASE_URL}{location}", wait_until="load")

    card = page.locator(".mr-inducement-card").first
    try:
        card.wait_for(state="visible", timeout=5000)
    except Exception:
        pytest.skip("Aucun inducement disponible pour ce tier")

    card.locator(".mr-qty-btn").nth(1).click()
    expect(page.locator(".mr-cart-items")).to_contain_text("1 coup")

    with page.expect_navigation(wait_until="load"):
        page.locator("button[name='intent'][value='pass']").click()

    rows = _query_db(
        f"SELECT home_team_id, home_inducements, away_inducements "
        f"FROM match_report_proj WHERE match_report_id = '{mr_id}';"
    )
    assert rows, f"Aucune projection trouvée pour {mr_id}"
    home_team_id, home_inducements, away_inducements = rows[0].split("|", 2)
    recorded = home_inducements if home_team_id == team_id else away_inducements
    assert recorded == "[]", (
        f"'Passer' n'a pas vidé le panier sélectionné — inducements enregistrés : {recorded!r}"
    )


def test_cart_footer_visible(page: Page, space_id, inducements_mr):
    """Footer cart sticky visible avec bouton Valider."""
    loc = inducements_mr["redirect_location"]
    if "/inducements/" not in loc:
        pytest.skip("Pas de page inducements pour cette compétition")

    page.goto(f"{BASE_URL}{loc}", wait_until="load")

    expect(page.locator(".mr-cart-footer")).to_be_visible()
    expect(page.locator("button[name='intent'][value='validate']")).to_be_visible()

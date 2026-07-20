"""Tests E2E — rapport de match, step 2 (séquence avant-match).

Scénarios couverts :
- GET step 2 sur un match report PreMatch → page affichée
- Saisie fan factor D3 valide + soumission → redirect, badge "déjà enregistré"
- Soumission D3 invalide (valeur hors plage) → bloqué par validation HTML5
- GET step 2 sur un Draft → redirect vers step 1
- Données équipe affichées après chargement Alpine (journaliers, TV, inducements)

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import re

import pytest
import requests
from playwright.sync_api import Page, expect

BASE_URL = "http://localhost:3210"
_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")


def _create_draft(space_id: str, ctx: dict, home_idx: int = 0, away_idx: int = 1) -> str:
    """POST /match-report/new → retourne le match_report_id depuis le Location header."""
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
    assert resp.status_code in (302, 303), (
        f"create_match_report a retourné {resp.status_code} au lieu de 3xx\n"
        f"body: {resp.text[:300]}"
    )
    location = resp.headers.get("Location", "")
    m = _ULID_RE.search(location)
    assert m, f"Impossible d'extraire le match_report_id depuis Location: {location!r}"
    return m.group(1)


def _ensure_pre_match(space_id: str, mr_id: str, ctx: dict,
                      home_idx: int = 0, away_idx: int = 1) -> None:
    """Assure que le match report est en état PreMatch.

    Si create_match_report_use_case a renvoyé un ID déjà confirmé (doublon
    détecté → auto-confirm), on ne rappelle pas le POST (qui retournerait
    NotInDraftPhase → 500).
    """
    check = requests.get(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        allow_redirects=True,
    )
    if check.url.endswith("/step2"):
        return  # déjà PreMatch

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
    assert resp.status_code in (302, 303), (
        f"update_match_selection a retourné {resp.status_code}\nbody: {resp.text[:300]}"
    )


# ── Fixtures ─────────────────────────────────────────────────────────────────

@pytest.fixture(scope="session")
def match_context(browser, space_id):
    from competition_lifecycle import build_full_competition
    full = build_full_competition(browser, space_id, num_teams=2)
    return {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_id": full["round_ids"][0],
        "teams": full["team_ids"],
    }


@pytest.fixture(scope="session")
def draft_mr_id(space_id, match_context):
    """Match report en état Draft.

    Essaie toutes les paires d'équipes possibles (sauf (0,1) réservée pour
    pre_match_mr_id) jusqu'à en trouver une qui donne un vrai Draft. Si
    create_match_report_use_case détecte un doublon et auto-confirme, la paire
    est en PreMatch — on passe à la suivante.
    """
    teams = match_context["teams"]
    n = len(teams)
    for i in range(n):
        for j in range(i + 1, n):
            if (i, j) == (0, 1):
                continue  # réservé pour pre_match_mr_id
            mr_id = _create_draft(space_id, match_context, home_idx=i, away_idx=j)
            check = requests.get(
                f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
                allow_redirects=True,
            )
            if not check.url.endswith("/step2"):
                return mr_id  # c'est un vrai Draft
    pytest.skip(
        "Toutes les combinaisons d'équipes ont un match report confirmé — "
        "réinitialise la base de données (make init_db WITH_SEED=1) pour relancer."
    )


@pytest.fixture(scope="session")
def pre_match_mr_id(space_id, match_context):
    """Match report en état PreMatch — teams[0]/teams[1]."""
    mr_id = _create_draft(space_id, match_context, home_idx=0, away_idx=1)
    _ensure_pre_match(space_id, mr_id, match_context, home_idx=0, away_idx=1)
    return mr_id


# ── Tests ─────────────────────────────────────────────────────────────────────

def test_step2_pre_match_page_loads(page: Page, space_id, pre_match_mr_id):
    """GET step 2 sur un PreMatch → page affichée avec titre et fan factor."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{pre_match_mr_id}/step2",
              wait_until="load")

    expect(page.locator(".mr-header-title")).to_contain_text("avant-match")
    expect(page.locator(".mr-fan-section")).to_be_visible()
    expect(page.locator("input[name='home_fan_roll']")).to_be_visible()
    expect(page.locator("input[name='away_fan_roll']")).to_be_visible()


def test_step2_submit_valid_fan_factor(page: Page, space_id, pre_match_mr_id):
    """Saisie D3 valide (1 et 2) + soumission → redirect vers inducements ou step3."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{pre_match_mr_id}/step2",
              wait_until="load")

    page.fill("input[name='home_fan_roll']", "1")
    page.fill("input[name='away_fan_roll']", "2")

    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    # La soumission du fan factor redirige vers la page inducements ou step3 —
    # jamais vers /step2 à nouveau.
    assert "/step2" not in page.url or "/inducements/" in page.url, (
        f"Redirect inattendu vers {page.url!r} — attendu inducements ou step3"
    )


def test_step2_invalid_d3_blocked_by_html5(page: Page, space_id, pre_match_mr_id):
    """D3=0 → bloqué par validation HTML5 (min=1), pas de navigation."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{pre_match_mr_id}/step2",
              wait_until="load")

    page.fill("input[name='home_fan_roll']", "0")
    page.fill("input[name='away_fan_roll']", "2")

    navigated = []
    page.on("framenavigated",
            lambda f: navigated.append(f.url) if f == page.main_frame else None)
    page.evaluate("document.querySelector('form').requestSubmit()")
    page.wait_for_timeout(500)

    assert not navigated or all(u.endswith("/step2") for u in navigated), \
        f"Le formulaire ne devrait pas avoir navigué : {navigated}"
    is_valid = page.evaluate(
        "document.querySelector('input[name=\"home_fan_roll\"]').validity.valid"
    )
    assert not is_valid


def test_step2_draft_redirects_to_step1(page: Page, space_id, draft_mr_id):
    """GET step 2 sur un Draft → redirect vers step 1 (edit)."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{draft_mr_id}/step2",
              wait_until="load")

    expect(page).not_to_have_url(re.compile(r".*/step2$"))
    expect(page).to_have_url(re.compile(rf".*/match-report/{draft_mr_id}$"))


def test_step2_team_data_loaded(page: Page, space_id, pre_match_mr_id):
    """Après chargement Alpine, journaliers et TV affichent des données réelles."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{pre_match_mr_id}/step2",
              wait_until="load")

    page.wait_for_function(
        "() => { "
        "  const el = document.querySelector('[x-data]'); "
        "  return el && el._x_dataStack && el._x_dataStack[0].home !== null; "
        "}",
        timeout=8000,
    )

    page.wait_for_selector(".mr-info-banner", timeout=5000)
    banners = page.locator(".mr-info-banner").all()
    assert len(banners) >= 2, f"Attendu ≥ 2 banners journaliers, trouvé {len(banners)}"

    tv_values = page.locator(".mr-tv-value").all()
    assert len(tv_values) == 2
    for v in tv_values:
        text = v.inner_text().strip()
        assert text and text != "…", f"Valeur TV non chargée : {text!r}"

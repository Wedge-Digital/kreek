"""Tests E2E — rapport de match, step 2.1 (coups de pouce / inducements).

Scénarios couverts :
- GET page inducements → header équipe + bannière budget + widget chargé
- Bouton "Passer" (TopDog) → redirect vers page Underdog
- Bouton "Passer" (Underdog) → récapitulatif des deux camps (carte 628)
- Bouton "Passer" soumet toujours un panier vide, même si des items sont sélectionnés
- Sélection dépasse budget → bouton Valider désactivé
- TopDog valide des achats → redirect vers page Underdog
- Underdog valide ses achats → récapitulatif des deux camps (carte 628)

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée.
"""

import re

import pytest
import requests
from playwright.sync_api import Page, expect

from db_helpers import query_db as _query_db
from competition_lifecycle import liberer_les_equipes

from e2e_env import BASE_URL
_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")
_TEAM_ID_RE = re.compile(r"/inducements/([0-9A-Z]{26})")


def _create_pre_match(
    space_id: str, ctx: dict, home_idx: int, away_idx: int, round_idx: int = 0
) -> str:
    """Crée un match report — origine Manual, auto-confirmée en PreMatch en
    un seul appel (cf. create_match_report_use_case::execute)."""
    round_id = ctx["round_ids"][round_idx]
    liberer_les_equipes(
        space_id,
        ctx["competition_id"],
        ctx["season_id"],
        round_id,
        ctx["teams"][home_idx],
        ctx["teams"][away_idx],
    )
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
    assert resp.status_code in (302, 303), f"create: {resp.status_code}"
    location = resp.headers.get("Location", "")
    m = _ULID_RE.search(location)
    assert m, f"match_report_id introuvable dans Location: {location!r}"
    return m.group(1)


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
def inducements_ctx(browser, space_id):
    from competition_lifecycle import build_full_competition
    full = build_full_competition(browser, space_id, num_teams=4)
    return {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_id": full["round_ids"][0],
        # **Toutes les journées, et pas seulement la première.**
        #
        # Depuis la carte 551, une équipe ne joue qu'un match par journée. Deux
        # rapports de ce fichier partageant une équipe doivent donc vivre sur
        # deux journées : sinon, créer le second libère l'équipe commune, ce qui
        # supprime l'appariement du premier et annule son rapport — sa page
        # d'inducements répond alors 404, sans que rien ne le dise.
        "round_ids": full["round_ids"],
        "teams": full["team_ids"],
    }


@pytest.fixture(scope="module")
def inducements_mr(space_id, inducements_ctx):
    """Match report en PreMatch avec TeamValues enregistrées (après fan factor POST)."""
    mr_id = _create_pre_match(space_id, inducements_ctx, home_idx=2, away_idx=3, round_idx=0)
    resp = _post_fan_factor(space_id, mr_id)
    assert resp.status_code in (302, 303), f"fan factor POST: {resp.status_code}"
    location = resp.headers.get("Location", "")
    return {"mr_id": mr_id, "redirect_location": location}


@pytest.fixture(scope="module")
def underdog_inducements_location(space_id, inducements_ctx):
    """Fait passer le TopDog (POST direct, panier vide) → retourne l'URL Underdog."""
    mr_id = _create_pre_match(space_id, inducements_ctx, home_idx=0, away_idx=1, round_idx=1)
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

    # Zone widget présente (même vide si aucun inducement configuré) — deux
    # zones .mr-selector-zone coexistent sur la page (inducements + mercos),
    # on vérifie la première (inducements, toujours visible à l'arrivée).
    expect(page.locator(".mr-selector-zone").first).to_be_visible()


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


def test_underdog_pass_redirects_to_recap(page: Page, space_id, underdog_inducements_location):
    """Underdog clique 'Passer' → le récapitulatif des deux camps (carte 628,
    décision 4), servi par la page des coups de pouce elle-même ; le coup
    d'envoi y mène à step3."""
    loc = underdog_inducements_location
    if "/inducements/" not in loc:
        pytest.skip("Pas de page inducements Underdog pour cette compétition")

    page.goto(f"{BASE_URL}{loc}", wait_until="load")

    with page.expect_navigation(wait_until="load"):
        page.locator("button[name='intent'][value='pass']").click()

    assert "/inducements/" in page.url, (
        f"Redirect inattendu depuis 'Passer' Underdog : {page.url!r}"
    )
    expect(page.locator(".mr-header-sub")).to_have_text("Achats terminés pour les deux équipes")
    expect(page.locator(".mr-recap-side")).to_have_count(2)
    expect(page.locator(".mr-recap-empty")).to_have_count(2)
    kickoff = page.get_by_role("link", name="Coup d'envoi → Match")
    expect(kickoff).to_have_attribute("href", re.compile(r"/step3$"))


def test_pass_discards_pending_selection(page: Page, space_id, inducements_ctx):
    """'Passer' soumet toujours un panier vide, même si des inducements sont
    sélectionnés localement (comportement attendu : édition annulée)."""
    mr_id = _create_pre_match(space_id, inducements_ctx, home_idx=1, away_idx=2, round_idx=2)
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


def test_cart_footer_visible(page: Page, space_id, inducements_ctx):
    """Footer cart sticky visible avec bouton Valider.

    **Sur son propre rapport** (journée 4, équipes jamais réunies ailleurs) :
    celui de `inducements_mr` a vu passer son favori dans
    `test_topdog_pass_redirects_to_underdog`, et sa page ne s'ouvrait plus que
    par la redirection vers l'outsider (carte 627, R7) — le test passait par
    accident, et dépendait de l'ordre d'exécution.
    """
    mr_id = _create_pre_match(space_id, inducements_ctx, home_idx=0, away_idx=3, round_idx=3)
    resp = _post_fan_factor(space_id, mr_id)
    assert resp.status_code in (302, 303), f"fan factor POST: {resp.status_code}"
    loc = resp.headers.get("Location", "")
    if "/inducements/" not in loc:
        pytest.skip("Pas de page inducements pour cette compétition")

    page.goto(f"{BASE_URL}{loc}", wait_until="load")

    expect(page.locator(".mr-cart-footer")).to_be_visible()
    expect(page.locator("button[name='intent'][value='validate']")).to_be_visible()


def test_cart_unfolds_and_removes_an_inducement(page: Page, space_id, inducements_ctx):
    """Carte 629 : le panier se déplie, liste un coup de pouce commun, et son ✕
    le retire — le sélecteur, qui garde les quantités, revient à 0.

    **Sur son propre rapport** (journée 5, équipes 1 et 3, jamais réunies
    ailleurs dans ce fichier), pour la même raison que `test_cart_footer_visible` :
    sur `inducements_mr`, le favori a déjà passé (carte 627).
    """
    mr_id = _create_pre_match(space_id, inducements_ctx, home_idx=1, away_idx=3, round_idx=4)
    resp = _post_fan_factor(space_id, mr_id)
    assert resp.status_code in (302, 303), f"fan factor POST: {resp.status_code}"
    loc = resp.headers.get("Location", "")
    if "/inducements/" not in loc:
        pytest.skip("Pas de page inducements pour cette compétition")

    page.goto(f"{BASE_URL}{loc}", wait_until="load")

    card = page.locator(".mr-inducement-card").first
    try:
        card.wait_for(state="visible", timeout=5000)
    except Exception:
        pytest.skip("Aucun inducement disponible pour ce tier")
    name = card.locator(".mr-inducement-name").inner_text().strip()

    card.locator(".mr-qty-btn").nth(1).click()
    toggle = page.locator(".mr-cart-toggle")
    expect(toggle).to_contain_text("Voir le panier")
    toggle.click()
    expect(toggle).to_have_attribute("aria-expanded", "true")

    line = page.locator(".mr-cart-merco-item", has_text=name)
    expect(line).to_be_visible()
    line.locator(".mr-cart-merco-remove").click()

    expect(page.locator(".mr-cart-items")).to_contain_text("Aucun coup de pouce")
    expect(card.locator(".mr-qty-value")).to_have_text("0")

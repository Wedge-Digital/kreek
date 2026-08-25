"""Tests E2E — rapport de match, step 2 : sélection de mercenaires.

Scénarios couverts :
- TC-MERC-01 : tab Mercenaires visible, clic → widget chargé
- TC-MERC-02 : positions journalier absentes de la grille
- TC-MERC-03 : clic carte → hire panel visible + prix corrects
- TC-MERC-04 : clic Recruter → compteur dots + résumé panier
- TC-MERC-05 : max 3 mercenaires côté frontend
- TC-MERC-06 : suppression via ✕ → compteur réduit
- TC-MERC-07 : position à max_qty → carte disabled
- TC-MERC-08 : soumission avec mercenaire → redirect valide
- TC-MERC-09 : soumission sans mercenaire (non-régression)

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée.
"""

import re

import pytest
import requests
from playwright.sync_api import Page, expect

BASE_URL = "http://localhost:3210"
_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")
_TEAM_ID_RE = re.compile(r"/inducements/([0-9A-Z]{26})")


# ── Helpers ───────────────────────────────────────────────────────────────────

def _create_pre_match(space_id: str, ctx: dict, home_idx: int, away_idx: int) -> str:
    """Crée un match report — origine Manual, auto-confirmée en PreMatch en
    un seul appel (cf. create_match_report_use_case::execute)."""
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
    return m.group(1)


def _advance_to_inducements(space_id: str, mr_id: str) -> str:
    """Soumet fan factor → retourne l'URL Location de redirect."""
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "1", "away_fan_roll": "2"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"fan factor POST: {resp.status_code}"
    return resp.headers.get("Location", "")


# ── Fixtures ──────────────────────────────────────────────────────────────────

@pytest.fixture(scope="module")
def merco_ctx(browser, space_id):
    from competition_lifecycle import build_full_competition
    full = build_full_competition(browser, space_id, num_teams=4)
    return {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_id": full["round_ids"][0],
        "teams": full["team_ids"],
    }


@pytest.fixture(scope="module")
def merco_mr(space_id, merco_ctx):
    """Match report avec fan factors saisis, en phase inducements."""
    mr_id = _create_pre_match(space_id, merco_ctx, home_idx=0, away_idx=1)
    location = _advance_to_inducements(space_id, mr_id)
    return {"mr_id": mr_id, "redirect_location": location}


def _get_inducements_url(page: Page, space_id: str, merco_mr: dict) -> str | None:
    loc = merco_mr["redirect_location"]
    if "/inducements/" not in loc:
        return None
    return f"{BASE_URL}{loc}"


def _open_mercenaires_tab(page: Page) -> None:
    """Clique sur l'onglet Mercenaires et attend le chargement du widget."""
    merco_tab = page.locator(".mr-tab", has_text="Mercenaires")
    merco_tab.click()
    page.locator(".merco-selector").wait_for(state="attached", timeout=5000)


# ── Tests ─────────────────────────────────────────────────────────────────────

def test_mercenaires_tab_visible_and_widget_loads(page: Page, space_id, merco_mr):
    """TC-MERC-01 : onglet Mercenaires présent, clic → widget chargé."""
    url = _get_inducements_url(page, space_id, merco_mr)
    if url is None:
        pytest.skip("Pas de page inducements pour cette compétition")

    page.goto(url, wait_until="load")

    tab = page.locator(".mr-tab", has_text="Mercenaires")
    expect(tab).to_be_visible()

    tab.click()
    page.locator(".merco-selector").wait_for(state="attached", timeout=5000)
    expect(page.locator(".merco-position-grid")).to_be_visible()


def test_journeymen_excluded_from_grid(page: Page, space_id, merco_mr):
    """TC-MERC-02 : aucune position marquée 'journalier' dans la grille."""
    url = _get_inducements_url(page, space_id, merco_mr)
    if url is None:
        pytest.skip("Pas de page inducements pour cette compétition")

    page.goto(url, wait_until="load")
    _open_mercenaires_tab(page)

    # Le serveur filtre is_journeyman côté handler — les cards présentes ne doivent
    # pas contenir le texte "journalier" (nom de position issu du roster reference).
    cards = page.locator(".merco-position-card")
    for i in range(cards.count()):
        text = cards.nth(i).inner_text().lower()
        assert "journalier" not in text, (
            f"Position journalier trouvée dans la grille : {text!r}"
        )


def test_position_click_shows_hire_panel(page: Page, space_id, merco_mr):
    """TC-MERC-03 : clic carte non-disabled → hire panel visible + prix corrects."""
    url = _get_inducements_url(page, space_id, merco_mr)
    if url is None:
        pytest.skip("Pas de page inducements pour cette compétition")

    page.goto(url, wait_until="load")
    _open_mercenaires_tab(page)

    available = page.locator(".merco-position-card:not(.disabled)").first
    if not available.is_visible():
        pytest.skip("Aucune position disponible dans la grille")

    name_text = available.locator(".merco-position-name").inner_text()
    base_cost_text = available.locator(".merco-position-price").inner_text()
    base_cost = int("".join(filter(str.isdigit, base_cost_text)))

    available.click()

    hire_panel = page.locator(".merco-hire-panel")
    expect(hire_panel).to_be_visible()
    expect(page.locator(".merco-hire-panel-pos")).to_contain_text(name_text)

    price_base = page.locator(".merco-hire-option-price").first.inner_text()
    price_lvl1 = page.locator(".merco-hire-option-price").nth(1).inner_text()

    assert str(base_cost + 30) in price_base, (
        f"Prix Base incorrect: attendu {base_cost + 30} kPo, obtenu {price_base!r}"
    )
    assert str(base_cost + 80) in price_lvl1, (
        f"Prix Niv.1 incorrect: attendu {base_cost + 80} kPo, obtenu {price_lvl1!r}"
    )


def test_recruit_base_adds_to_cart(page: Page, space_id, merco_mr):
    """TC-MERC-04 : clic Recruter → dots 1/3, résumé panier mis à jour."""
    url = _get_inducements_url(page, space_id, merco_mr)
    if url is None:
        pytest.skip("Pas de page inducements pour cette compétition")

    page.goto(url, wait_until="load")
    _open_mercenaires_tab(page)

    available = page.locator(".merco-position-card:not(.disabled)").first
    if not available.is_visible():
        pytest.skip("Aucune position disponible dans la grille")

    available.click()
    page.locator(".merco-hire-btn--base").click()

    dot_label = page.locator(".merco-dot-label")
    expect(dot_label).to_contain_text("1 / 3")
    expect(page.locator(".mr-cart-merco-list")).to_be_visible()
    expect(page.locator(".mr-cart-merco-item")).to_have_count(1)


def test_max_3_mercenaires_frontend(page: Page, space_id, merco_mr):
    """TC-MERC-05 : après 3 recrutements, le 4e est bloqué côté frontend."""
    url = _get_inducements_url(page, space_id, merco_mr)
    if url is None:
        pytest.skip("Pas de page inducements pour cette compétition")

    # Le panier (.mr-cart-footer, position fixed) grandit à chaque ajout et
    # finit par chevaucher la grille de positions sur un petit viewport.
    page.set_viewport_size({"width": 1280, "height": 2000})
    page.goto(url, wait_until="load")
    _open_mercenaires_tab(page)

    cards = page.locator(".merco-position-card:not(.disabled)")
    if cards.count() == 0:
        pytest.skip("Aucune position disponible")

    # Recruter 3 mercenaires (même position si une seule dispo)
    for i in range(3):
        cards.first.click()
        page.locator(".merco-hire-btn--base").click()

    expect(page.locator(".merco-dot-label")).to_contain_text("3 / 3")

    # Un 4e clic ne doit pas changer le compteur (guard `hiredList.length >= 3`)
    cards.first.click()
    page.locator(".merco-hire-btn--base").click()

    expect(page.locator(".merco-dot-label")).to_contain_text("3 / 3")
    expect(page.locator(".mr-cart-merco-item")).to_have_count(3)


def test_remove_mercenaire_from_cart(page: Page, space_id, merco_mr):
    """TC-MERC-06 : clic ✕ → compteur réduit, liste panier mise à jour."""
    url = _get_inducements_url(page, space_id, merco_mr)
    if url is None:
        pytest.skip("Pas de page inducements pour cette compétition")

    page.goto(url, wait_until="load")
    _open_mercenaires_tab(page)

    available = page.locator(".merco-position-card:not(.disabled)").first
    if not available.is_visible():
        pytest.skip("Aucune position disponible")

    available.click()
    page.locator(".merco-hire-btn--base").click()
    expect(page.locator(".merco-dot-label")).to_contain_text("1 / 3")

    page.locator(".mr-cart-merco-remove").first.click()

    expect(page.locator(".merco-dot-label")).to_contain_text("0 / 3")
    expect(page.locator(".mr-cart-merco-list")).to_be_hidden()


def test_full_position_card_disabled(page: Page, space_id, merco_mr):
    """TC-MERC-07 : carte à max_qty → classe disabled, hire panel inaccessible."""
    url = _get_inducements_url(page, space_id, merco_mr)
    if url is None:
        pytest.skip("Pas de page inducements pour cette compétition")

    page.goto(url, wait_until="load")
    _open_mercenaires_tab(page)

    disabled_cards = page.locator(".merco-position-card.disabled")
    if disabled_cards.count() == 0:
        pytest.skip("Aucune position complète dans les données de test")

    # La carte disabled ne doit pas avoir de @click (pointer-events: none en CSS)
    expect(disabled_cards.first.locator(".merco-position-full")).to_be_visible()
    # Vérifier que le hire panel ne s'ouvre pas (pas d'attribut @click)
    assert disabled_cards.first.get_attribute("@click") is None


def test_submit_with_mercenaire_creates_temp_player(page: Page, space_id, merco_mr):
    """TC-MERC-08 : formulaire soumis avec mercenaire → redirect valide."""
    url = _get_inducements_url(page, space_id, merco_mr)
    if url is None:
        pytest.skip("Pas de page inducements pour cette compétition")

    page.goto(url, wait_until="load")
    _open_mercenaires_tab(page)

    available = page.locator(".merco-position-card:not(.disabled)").first
    if not available.is_visible():
        pytest.skip("Aucune position disponible")

    available.click()
    page.locator(".merco-hire-btn--base").click()

    expect(page.locator(".mr-cart-merco-item")).to_have_count(1)

    with page.expect_navigation(wait_until="load"):
        page.locator("button[name='intent'][value='validate']").click()

    new_url = page.url
    assert "/inducements/" in new_url or "/step3" in new_url, (
        f"Redirect inattendu après soumission avec mercenaire : {new_url!r}"
    )


def test_submit_without_mercenaires_regression(page: Page, space_id, merco_ctx, merco_mr):
    """TC-MERC-09 : soumission sans mercenaire + inducements classiques → OK."""
    url = _get_inducements_url(page, space_id, merco_mr)
    if url is None:
        pytest.skip("Pas de page inducements pour cette compétition")

    # Crée un nouveau match report pour éviter l'interférence avec TC-MERC-08
    mr_id = _create_pre_match(space_id, merco_ctx, home_idx=2, away_idx=3)
    location = _advance_to_inducements(space_id, mr_id)
    if "/inducements/" not in location:
        pytest.skip("Pas de page inducements pour cette compétition")

    page.goto(f"{BASE_URL}{location}", wait_until="load")

    # Soumet sans toucher aux mercenaires (champ mercenaries vide = [])
    with page.expect_navigation(wait_until="load"):
        page.locator("button[name='intent'][value='validate']").click()

    new_url = page.url
    assert "/inducements/" in new_url or "/step3" in new_url, (
        f"Non-régression échouée — redirect inattendu : {new_url!r}"
    )

# ── Carte 406 — le mercenaire d'un poste à seize exemplaires ──────────────────


def test_un_mercenaire_sur_un_poste_a_seize_exemplaires_survit(page: Page, space_id, merco_ctx):
    """Le défaut de la carte 406, reproduit de bout en bout.

    Un poste dont `max_quantity` dépasse dix — la piétaille s'aligne à seize —
    faisait échouer la construction de sa spécification, sans empêcher l'achat
    d'être enregistré. Le domaine jetait ensuite l'orphelin par un `filter_map`,
    et le mercenaire, pourtant facturé, n'existait plus nulle part.

    **La soumission passe par HTTP et non par la grille** : celle-ci écarte les
    postes journalier, et c'est justement le seul type de poste que le corpus de
    démonstration place au-dessus de dix. Le serveur, lui, ne les filtre pas —
    la production le prouve. Poster directement reproduit donc le cas réel sans
    déplacer les plafonds du corpus, dont plusieurs tests dépendent.
    """
    mr_id = _create_pre_match(space_id, merco_ctx, home_idx=2, away_idx=3)
    location = _advance_to_inducements(space_id, mr_id)
    if "/inducements/" not in location:
        pytest.skip("Pas de page inducements pour cette compétition")
    team_id = _TEAM_ID_RE.search(location).group(1)

    reponse = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/inducements/{team_id}",
        headers={"HX-Request": "true", "Content-Type": "application/x-www-form-urlencoded"},
        data={
            "intent": "validate",
            "selection": "[]",
            "mercenaries": '[{"position_uid": "DEMO_GRANIT__PIETAILLE", "level": "base"}]',
        },
        allow_redirects=False,
    )
    assert reponse.status_code in (302, 303), (
        f"l'engagement doit être accepté : {reponse.status_code}\n{reponse.text[:300]}"
    )

    from db_helpers import query_db

    achats = query_db(
        "SELECT payload::text FROM match_report_event_store "
        f"WHERE match_report_id = '{mr_id}' AND event_type = 'InducementsRecorded'"
    )
    assert achats, "l'événement doit exister"
    assert any("MERCO:DEMO_GRANIT__PIETAILLE" in ligne for ligne in achats), (
        "le mercenaire doit figurer dans l'événement — c'est là qu'il "
        f"disparaissait : {achats}"
    )


"""Tests E2E — le match en direct (carte 636).

Le match se saisit depuis un écran de journal : le score, le journal des deux
camps, « ＋ Nouvelle action » et « Fin du match → MVP ». La nouvelle action
demande l'équipe dans la page, puis ouvre l'étape de saisie de ce camp, dont le
seul retour est « ← Retour au journal ». L'autre coach ouvre le même écran,
sans aucun bouton d'écriture.

Scénarios couverts, dans l'ordre du fichier — ils partagent un seul rapport :
- L1 — De l'avant-match au journal : le fan factor, les coups de pouce passés,
       le coup d'envoi mènent au match en direct, indicateur à 2
- L2 — « Nouvelle action » → équipe → saisie → « Retour au journal » : l'action
       est au journal et le score la compte
- L3 — L'autre coach suit : « DevCoach saisit tout le match », aucun bouton
       d'écriture, aucun ✕ ; une page de saisie le ramène au match en direct

Le membre simple (`X-Bypass-Auth-Profile: simple`) coache la **deuxième**
équipe que monte `build_full_competition` ; DevCoach, identité de la suite,
coache la première et démarre le match.

**Aucun des clics de ce fichier n'attend un câblage htmx** : le choix de
l'équipe est un `<details>` natif, les équipes et les retours des liens
ordinaires — `cliquer_quand_cable` n'aboutirait jamais sur eux.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée.
"""

import re

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from match_report_helpers import create_draft, ensure_pre_match

ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}


def _url(space_id, mr_id, suffixe):
    return f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{suffixe}"


@pytest.fixture(scope="module")
def live_ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=4)
    ctx = {**full, "teams": full["team_ids"]}
    round_id = full["round_ids"][0]
    home, away = full["team_ids"][0], full["team_ids"][1]
    mr_id = create_draft(space_id, ctx, round_id, home, away)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home, away)
    return {"space_id": space_id, "mr_id": mr_id}


def _passer_les_coups_de_pouce(page: Page) -> None:
    """Le favori puis l'outsider passent ; le récapitulatif offre le coup
    d'envoi. Rien à faire si le tier n'a pas de coups de pouce."""
    passer = page.locator("button[name='intent'][value='pass']")
    for _ in range(2):
        if "/inducements/" not in page.url or passer.count() == 0:
            return
        with page.expect_navigation(wait_until="load"):
            passer.click()
    coup_d_envoi = page.get_by_role("link", name="Coup d'envoi → Match")
    if coup_d_envoi.count():
        with page.expect_navigation(wait_until="load"):
            coup_d_envoi.click()


# ── L1 ────────────────────────────────────────────────────────────────────────

def test_l1_de_l_avant_match_au_journal(page: Page, live_ctx):
    space_id, mr_id = live_ctx["space_id"], live_ctx["mr_id"]
    page.goto(_url(space_id, mr_id, "step2"), wait_until="load")
    page.fill("input[name='home_fan_roll']", "2")
    page.fill("input[name='away_fan_roll']", "3")
    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    _passer_les_coups_de_pouce(page)

    page.wait_for_url(re.compile(rf".*/{mr_id}/live$"), timeout=10000)
    expect(page.locator(".mr-step--active")).to_have_text("2")
    expect(page.locator(".match-report-live .live-score-banner")).to_be_visible()
    expect(page.locator(".match-report-live .live-log .mr-log-title")).to_be_visible(timeout=10000)
    expect(page.locator(".match-report-live .live-end-match")).to_be_visible()


# ── L2 ────────────────────────────────────────────────────────────────────────

def test_l2_nouvelle_action_equipe_saisie_retour(page: Page, live_ctx):
    space_id, mr_id = live_ctx["space_id"], live_ctx["mr_id"]
    page.goto(_url(space_id, mr_id, "live"), wait_until="load")

    equipes = page.locator(".match-report-live .live-team-pick")
    expect(equipes.first).to_be_hidden()
    page.locator(".match-report-live .live-new-action-toggle").click()
    expect(equipes).to_have_count(2)
    expect(equipes.first).to_be_visible()

    with page.expect_navigation(wait_until="load"):
        equipes.first.click()
    assert page.url.endswith(f"/{mr_id}/step3"), page.url

    # La saisie du camp, comme S4 de test_match_report_step3_4_actions.
    page.wait_for_selector("#turn-selector .mr-turn-btn", timeout=6000)
    page.wait_for_selector("#player-selector .mr-player-chip", timeout=6000)
    page.locator(".mr-turn-btn[data-turn='4']").first.click()
    page.locator(".mr-player-chip").first.click()
    page.wait_for_selector("#action-panel .mr-action-grid", timeout=6000)
    page.locator(".mr-action-btn").filter(has_text="Touchdown").click()
    expect(page.locator("#action-log .mr-log-entry", has_text="Touchdown")).to_be_visible(timeout=6000)

    retour = page.locator(".mr-nav-row a", has_text="Retour au journal")
    with page.expect_navigation(wait_until="load"):
        retour.click()
    assert page.url.endswith(f"/{mr_id}/live"), page.url

    entree = page.locator(".match-report-live .mr-log-entry", has_text="Touchdown")
    expect(entree).to_be_visible(timeout=10000)
    expect(page.locator(".match-report-live .live-score-value").first).to_have_text("1")
    # Qui saisit garde le ✕.
    expect(entree.locator(".mr-log-delete")).to_have_count(1)


# ── L3 ────────────────────────────────────────────────────────────────────────

def test_l3_l_autre_coach_suit_sans_bouton(browser, live_ctx):
    space_id, mr_id = live_ctx["space_id"], live_ctx["mr_id"]
    contexte = browser.new_context(extra_http_headers=ENTETE_MEMBRE_SIMPLE)
    try:
        vue = contexte.new_page()
        vue.goto(_url(space_id, mr_id, "live"), wait_until="load")

        expect(vue.locator(".match-report-live .live-notice")).to_contain_text(
            "DevCoach saisit tout le match"
        )
        entree = vue.locator(".match-report-live .mr-log-entry", has_text="Touchdown")
        expect(entree).to_be_visible(timeout=10000)
        expect(vue.locator(".match-report-live .mr-log-delete")).to_have_count(0)
        expect(vue.locator(".match-report-live .live-new-action")).to_have_count(0)
        expect(vue.locator(".match-report-live .live-end-match")).to_have_count(0)

        # Une page de saisie le ramène au match en direct.
        vue.goto(_url(space_id, mr_id, "step3"), wait_until="load")
        assert vue.url.endswith(f"/{mr_id}/live"), vue.url
    finally:
        contexte.close()

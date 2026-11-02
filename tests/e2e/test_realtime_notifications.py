"""Tests E2E : les notifications de match (carte 645, épic E23).

DevCoach saisit seul (« Je saisis tout ») le match qui l'oppose au membre
simple. Le membre simple, coach de l'autre équipe, reste **sur une autre page
de l'application** : il voit un toast à chaque fait — le démarrage, la
sélection des coups de pouce, le touchdown —, et le lien du toast mène au
direct. Un troisième coach, étranger au match, ne voit rien ; DevCoach ne voit
pas ses propres faits.

**Trois identités, sans contournement nouveau** (décision 17) : DevCoach par
défaut, le membre simple par l'en-tête `X-Bypass-Auth-Profile: simple`, et
`E2E Coach 02` **réellement connecté** par `/auth/login` et `SEED_PASSWORD` —
`bypass_auth` ne touche pas une session déjà ouverte.

**Le constat négatif ne vaut qu'après un témoin positif** : le routeur livre
les destinataires d'un même fait dans la même boucle ; quand le membre simple a
reçu le toast du touchdown, un toast mal adressé serait arrivé aussi.

**Ce qui est compté, et pourquoi pas à l'écran** : DevCoach navigue d'un écran
de saisie à l'autre par chargements complets, qui effacent les toasts. Un
script posé avant la page note dans `sessionStorage` — qui survit aux
navigations de l'onglet — chaque toast à lien inséré. Le négatif lit ce
journal, pas l'écran.

**Pas de `networkidle`** : le flux SSE reste ouvert, l'état n'arrive jamais.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

import re

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from match_report_helpers import pass_inducements_in_browser

SIMPLE_MEMBER_HEADER = {"X-Bypass-Auth-Profile": "simple"}
STRANGER_NAME = "E2E Coach 02"
# `src/cli/seed_e2e.rs`, `SEED_PASSWORD`.
SEED_PASSWORD = "changeme-dev-only"
DESKTOP = {"width": 1280, "height": 900}
# Un toast à lien dure 6 s : l'attente doit tenir dans sa vie.
TOAST_TIMEOUT_MS = 5000

_ULID = r"[0-9A-Z]{26}"

HELLO_RECEIVED = "() => window.kreekRealtime && window.kreekRealtime.connectionId"

# Note chaque toast à lien inséré, dans un journal qui survit aux navigations.
RECORD_LINKED_TOASTS = """
new MutationObserver(function (mutations) {
  mutations.forEach(function (m) {
    m.addedNodes.forEach(function (node) {
      if (!node.classList || !node.classList.contains('toast--linked')) return;
      var seen = JSON.parse(sessionStorage.getItem('linkedToasts') || '[]');
      seen.push(node.textContent);
      sessionStorage.setItem('linkedToasts', JSON.stringify(seen));
    });
  });
}).observe(document, { childList: true, subtree: true });
"""


@pytest.fixture
def match_of_two(browser, space_id):
    """Deux équipes, une journée : un seul appariement, DevCoach contre le
    membre simple."""
    full = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    return {**full, "space_id": space_id}


def _context(browser, headers=None):
    context = browser.new_context(viewport=DESKTOP, extra_http_headers=headers or {})
    context.add_init_script(RECORD_LINKED_TOASTS)
    return context


def _wait_for_stream(page: Page) -> None:
    page.wait_for_function(HELLO_RECEIVED, timeout=15000)


def _log_in_as_stranger(page: Page) -> None:
    page.goto(f"{BASE_URL}/auth/login", wait_until="load")
    page.fill("input[name='coach_name']", STRANGER_NAME)
    page.fill("input[name='password']", SEED_PASSWORD)
    page.click("button[type='submit']")
    page.wait_for_url("**/auth/login/success", timeout=5000)


def _park_on_competitions(page: Page, space_id: str) -> None:
    """Une page de l'application qui n'est pas celle du match, flux ouvert."""
    page.goto(f"{BASE_URL}/app/{space_id}/competitions", wait_until="load")
    _wait_for_stream(page)


def _linked_toasts_seen(page: Page) -> list[str]:
    return page.evaluate("() => JSON.parse(sessionStorage.getItem('linkedToasts') || '[]')")


def _toast(page: Page, text: str):
    return page.locator(".toast-zone .toast--linked", has_text=text)


def _start_from_the_schedule(page: Page, ctx: dict) -> str:
    """Le calendrier, la ligne du match, « Je saisis tout », « Commencer »."""
    page.goto(
        f"{BASE_URL}/app/{ctx['space_id']}/competitions/{ctx['competition_id']}"
        f"/{ctx['season_id']}/calendrier",
        wait_until="load",
    )
    with page.expect_navigation(wait_until="load"):
        page.locator(".cal-row--clickable .cal-row-link").click()
    page.wait_for_url(re.compile(rf".*/match-report/{_ULID}$"), timeout=10000)
    mr_id = page.url.rstrip("/").rsplit("/", 1)[-1]
    with page.expect_navigation(wait_until="load"):
        page.get_by_role("button", name="Commencer").click()
    page.wait_for_url(re.compile(rf".*/{mr_id}/step2$"), timeout=10000)
    return mr_id


def _through_pre_match(page: Page) -> bool:
    """Avant-match jusqu'au direct ; rend vrai si l'écran des coups de pouce a
    été traversé — chaque camp y valide alors une sélection vide."""
    page.fill("input[name='home_fan_roll']", "2")
    page.fill("input[name='away_fan_roll']", "3")
    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")
    crossed = "/inducements/" in page.url
    pass_inducements_in_browser(page)
    return crossed


def _record_a_home_touchdown(page: Page, mr_id: str) -> None:
    page.locator(".match-report-live .live-new-action-toggle").click()
    with page.expect_navigation(wait_until="load"):
        page.locator(".match-report-live .live-team-pick").nth(0).click()
    assert page.url.endswith(f"/{mr_id}/step3"), page.url
    page.wait_for_selector("#turn-selector .mr-turn-btn", timeout=10000)
    page.wait_for_selector("#player-selector .mr-player-chip", timeout=10000)
    page.locator(".mr-turn-btn[data-turn='1']").first.click()
    page.locator("#player-selector .mr-player-chip").first.click()
    page.wait_for_selector("#action-panel .mr-action-grid", timeout=10000)
    page.locator(".mr-action-btn").filter(has_text="Touchdown").click()
    expect(
        page.locator("#action-log .mr-log-entry--home", has_text="Touchdown")
    ).to_have_count(1, timeout=10000)


def test_the_other_coach_is_notified_and_a_stranger_is_not(browser, match_of_two):
    ctx = match_of_two
    space_id = ctx["space_id"]
    entry = _context(browser).new_page()
    follower = _context(browser, SIMPLE_MEMBER_HEADER).new_page()
    stranger = _context(browser).new_page()
    _log_in_as_stranger(stranger)
    _park_on_competitions(follower, space_id)
    _park_on_competitions(stranger, space_id)

    mr_id = _start_from_the_schedule(entry, ctx)
    expect(_toast(follower, "Saisie du match démarrée par DevCoach")).to_be_visible(
        timeout=TOAST_TIMEOUT_MS
    )

    if _through_pre_match(entry):
        expect(_toast(follower, "aucun coup de pouce — validé par DevCoach").first).to_be_visible(
            timeout=TOAST_TIMEOUT_MS
        )
    entry.wait_for_url(re.compile(rf".*/{mr_id}/live$"), timeout=10000)

    _record_a_home_touchdown(entry, mr_id)
    touchdown = _toast(follower, "Touchdown : ")
    expect(touchdown).to_contain_text("— par DevCoach", timeout=TOAST_TIMEOUT_MS)
    expect(touchdown.locator("a.toast-link")).to_have_text("Voir le match")

    # Le témoin positif est là : un toast mal adressé serait arrivé aussi.
    assert _linked_toasts_seen(stranger) == [], _linked_toasts_seen(stranger)
    assert _linked_toasts_seen(entry) == [], _linked_toasts_seen(entry)

    with follower.expect_navigation(wait_until="load"):
        touchdown.locator("a.toast-link").click()
    assert follower.url.endswith(f"/app/{space_id}/match-report/{mr_id}/live"), follower.url


def test_a_toast_without_options_keeps_its_former_shape(page: Page, space_id):
    """`kreekToast(message, type)` sans troisième argument : même texte nu,
    pas de lien, et le toast s'efface comme avant."""
    page.goto(f"{BASE_URL}/app/{space_id}/competitions", wait_until="load")
    page.evaluate("() => window.kreekToast('Sauvegardé', 'success')")

    toast = page.locator(".toast-zone .toast--success", has_text="Sauvegardé")
    expect(toast).to_be_visible()
    expect(toast).not_to_have_class(re.compile("toast--linked"))
    expect(toast.locator(".toast-title, .toast-link")).to_have_count(0)
    expect(toast).to_have_count(0, timeout=4000)

"""Tests E2E : le direct sur la page d'un match (carte 646, épic E23).

Le canal temps réel n'envoie qu'un signal — « le match X a changé » — aux
onglets présents sur ce match (carte 644) ; `/live` recharge alors son journal
et son score par leurs propres routes. Plus d'interrogation toutes les 15 s.

Scénarios :
- D1 — DevCoach saisit un touchdown dans son navigateur ; chez le membre
       simple, sur `/live`, le journal et le score changent sans rechargement,
       en moins de 15 s
- D2 — Une action saisie sur un autre match ne recharge rien : le témoin, un
       onglet sur l'autre match, voit son score bouger ; l'onglet du premier
       match n'a émis aucune requête de widget
- D3a — Le serveur termine le flux : le navigateur se reconnecte de lui-même
        après son `retry`, et les widgets se rechargent sur `realtimeResync`,
        montrant l'action saisie pendant la coupure
- D3b — Le serveur termine le flux et la reconnexion reçoit un 502, comme
        derrière nginx pendant un redéploiement : le flux passe à `CLOSED`, la
        page le rouvre elle-même, et les widgets se rechargent

**La coupure vient du serveur** (`POST /app/realtime/test/close-my-streams`,
réservée aux e2e) : `context.set_offline(True)` ne coupe pas un flux déjà
ouvert sous Chromium — mesuré, il reçoit encore tous ses événements vingt
secondes après. Fermer le flux côté serveur produit ce que le navigateur voit
quand le serveur s'arrête.

Les identités (décision 17 de la fiche E23) : DevCoach, admin de l'espace,
coache la première équipe et démarre les deux matchs ; le membre simple
(`X-Bypass-Auth-Profile: simple`, « E2E Coach 01 ») coache la deuxième et
suit. Le second match oppose les équipes 3 et 4 : seul DevCoach, admin, y est
présent.

**La présence avant le geste** : un onglet ne reçoit le signal qu'une fois sa
présence inscrite, après le `hello` du flux. Chaque onglet attend donc la
réponse de son `POST /app/realtime/presence` avant que l'autre saisisse. Le
signal émis entre le chargement des widgets et cette réponse est perdu — trou
accepté par la carte, de quelques dizaines de millisecondes.

**Pas de `networkidle`** : le flux SSE reste ouvert, l'état n'arrive jamais.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

import pytest
from playwright.sync_api import Browser, Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    first_player_id,
    record_action_api,
)

SIMPLE_MEMBER = {"X-Bypass-Auth-Profile": "simple"}
LOG_ENTRIES = ".match-report-live .mr-log-entry"
SCORES = ".match-report-live .live-score-value"
# Le direct doit battre l'ancienne interrogation : moins de 15 s.
DIRECT_TIMEOUT_MS = 14000
# Une reconnexion attend le `retry` de 5 s ; une réouverture après `CLOSED`,
# de 2,5 à 7,5 s de plus. Puis les widgets se rechargent.
RECONNECT_TIMEOUT_MS = 30000
STREAM_PATTERN = "**/app/realtime/stream"
STREAM_DOWN = "() => window.kreekRealtime.source.readyState !== EventSource.OPEN"
STREAM_CLOSED = "() => window.kreekRealtime.source.readyState === EventSource.CLOSED"
CONNECTION_ID = "() => window.kreekRealtime.connectionId"
NEW_CONNECTION = "(old) => window.kreekRealtime.connectionId && window.kreekRealtime.connectionId !== old"
# Les flux de l'appelant, fermés par le serveur (route réservée aux e2e).
CLOSE_MY_STREAMS = """() => fetch('/app/realtime/test/close-my-streams', {
    method: 'POST', credentials: 'same-origin', headers: { 'HX-Request': 'true' }
}).then(r => r.status)"""


def _url(space_id, mr_id, suffix):
    return f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{suffix}"


def _started(space_id, ctx, round_id, home, away) -> str:
    mr_id = create_draft(space_id, ctx, round_id, home, away)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home, away)
    ensure_inducements(space_id, mr_id)
    return mr_id


@pytest.fixture(scope="module")
def direct_ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=4, num_rounds=2)
    t, rounds = full["team_ids"], full["round_ids"]
    followed = _started(space_id, full, rounds[1], t[0], t[1])
    other = _started(space_id, full, rounds[0], t[2], t[3])
    return {"space_id": space_id, "followed": followed, "other": other}


def _presence_declared(response) -> bool:
    return response.url.endswith("/app/realtime/presence") and response.request.method == "POST"


def _open_live(browser: Browser, space_id, mr_id, headers=None) -> Page:
    """`/live`, ses deux widgets chargés et sa présence inscrite."""
    page = browser.new_context(extra_http_headers=headers or {}).new_page()
    with page.expect_response(_presence_declared):
        with page.expect_response(lambda r: r.url.endswith(f"/{mr_id}/live/score")):
            with page.expect_response(lambda r: r.url.endswith(f"/{mr_id}/action-log")):
                page.goto(_url(space_id, mr_id, "live"), wait_until="load")
    assert page.url.endswith(f"/{mr_id}/live"), page.url
    return page


def _home_score(page: Page) -> int:
    first = page.locator(SCORES).first
    expect(first).to_be_visible(timeout=10000)
    return int(first.inner_text())


def _count_navigations(page: Page) -> list:
    navigations: list = []
    page.on("framenavigated", lambda f: f.parent_frame is None and navigations.append(f.url))
    return navigations


def _count_widget_requests(page: Page, mr_id: str) -> list:
    requests_seen: list = []
    page.on(
        "request",
        lambda r: (f"/{mr_id}/action-log" in r.url or f"/{mr_id}/live/score" in r.url)
        and requests_seen.append(r.url),
    )
    return requests_seen


def _record_touchdown_in_browser(writer: Page, space_id, mr_id) -> None:
    """DevCoach, sur la saisie du domicile : un touchdown au tour 4."""
    writer.goto(_url(space_id, mr_id, "step3"), wait_until="load")
    writer.wait_for_selector("#turn-selector .mr-turn-btn", timeout=10000)
    writer.wait_for_selector("#player-selector .mr-player-chip", timeout=10000)
    writer.locator(".mr-turn-btn[data-turn='4']").first.click()
    writer.locator("#player-selector .mr-player-chip").first.click()
    writer.wait_for_selector("#action-panel .mr-action-grid", timeout=10000)
    before = writer.locator("#action-log .mr-log-entry", has_text="Touchdown").count()
    writer.locator(".mr-action-btn").filter(has_text="Touchdown").click()
    expect(writer.locator("#action-log .mr-log-entry", has_text="Touchdown")).to_have_count(
        before + 1, timeout=10000
    )


def _record_touchdown_by_api(space_id, mr_id) -> None:
    player = first_player_id(mr_id, "home")
    record_action_api(space_id, mr_id, "home", player, turn=5, action_type="TOUCHDOWN")


# ── D1 ────────────────────────────────────────────────────────────────────────

def test_d1_the_follower_sees_the_log_and_score_change_without_reloading(browser, direct_ctx):
    space_id, mr_id = direct_ctx["space_id"], direct_ctx["followed"]
    follower = _open_live(browser, space_id, mr_id, SIMPLE_MEMBER)
    writer = browser.new_context().new_page()
    try:
        entries, score = follower.locator(LOG_ENTRIES).count(), _home_score(follower)
        navigations = _count_navigations(follower)

        _record_touchdown_in_browser(writer, space_id, mr_id)

        expect(follower.locator(LOG_ENTRIES)).to_have_count(entries + 1, timeout=DIRECT_TIMEOUT_MS)
        expect(follower.locator(LOG_ENTRIES).first).to_contain_text("Touchdown")
        expect(follower.locator(SCORES).first).to_have_text(str(score + 1), timeout=DIRECT_TIMEOUT_MS)
        assert navigations == [], f"la page de qui suit a été rechargée : {navigations}"
    finally:
        writer.context.close()
        follower.context.close()


# ── D2 ────────────────────────────────────────────────────────────────────────

def test_d2_a_signal_of_another_match_reloads_nothing(browser, direct_ctx):
    space_id = direct_ctx["space_id"]
    followed, other = direct_ctx["followed"], direct_ctx["other"]
    follower = _open_live(browser, space_id, followed, SIMPLE_MEMBER)
    witness = _open_live(browser, space_id, other)
    try:
        widget_requests = _count_widget_requests(follower, followed)
        witness_score = _home_score(witness)

        _record_touchdown_by_api(space_id, other)

        # Le témoin d'abord : le signal de l'autre match est bien parti.
        expect(witness.locator(SCORES).first).to_have_text(
            str(witness_score + 1), timeout=DIRECT_TIMEOUT_MS
        )
        assert widget_requests == [], f"widgets rechargés sans raison : {widget_requests}"
    finally:
        witness.context.close()
        follower.context.close()


# ── D3a, D3b ──────────────────────────────────────────────────────────────────

def _close_my_streams(page: Page) -> None:
    status = page.evaluate(CLOSE_MY_STREAMS)
    assert status == 200, f"coupure des flux : {status}"


def _expect_the_outage_action(follower: Page, entries: int, score: int) -> None:
    expect(follower.locator(LOG_ENTRIES)).to_have_count(entries + 1, timeout=RECONNECT_TIMEOUT_MS)
    expect(follower.locator(SCORES).first).to_have_text(str(score + 1), timeout=RECONNECT_TIMEOUT_MS)


def test_d3a_the_widgets_reload_after_the_browser_reconnects(browser, direct_ctx):
    space_id, mr_id = direct_ctx["space_id"], direct_ctx["followed"]
    follower = _open_live(browser, space_id, mr_id, SIMPLE_MEMBER)
    try:
        entries, score = follower.locator(LOG_ENTRIES).count(), _home_score(follower)
        old_connection = follower.evaluate(CONNECTION_ID)

        _close_my_streams(follower)
        # Le témoin de la coupure : sans lui, un flux resté ouvert livrerait le
        # signal normalement, et le test passerait pour une mauvaise raison.
        follower.wait_for_function(STREAM_DOWN, timeout=10000)
        _record_touchdown_by_api(space_id, mr_id)
        # L'action est saisie avant la reconnexion : seul `realtimeResync`
        # peut la faire paraître, aucun signal ne la portera.
        assert follower.evaluate(STREAM_DOWN), "le flux s'est rouvert avant la saisie"
        assert follower.evaluate(CONNECTION_ID) == old_connection

        follower.wait_for_function(NEW_CONNECTION, arg=old_connection, timeout=RECONNECT_TIMEOUT_MS)
        _expect_the_outage_action(follower, entries, score)
    finally:
        follower.context.close()


def test_d3b_the_page_reopens_a_stream_refused_by_the_server(browser, direct_ctx):
    space_id, mr_id = direct_ctx["space_id"], direct_ctx["followed"]
    follower = _open_live(browser, space_id, mr_id, SIMPLE_MEMBER)
    refused: list = []
    try:
        entries, score = follower.locator(LOG_ENTRIES).count(), _home_score(follower)
        old_connection = follower.evaluate(CONNECTION_ID)
        # Le redéploiement derrière nginx : toute reconnexion reçoit un 502.
        follower.route(
            STREAM_PATTERN,
            lambda route: (refused.append(route.request.url), route.fulfill(status=502)),
        )

        _close_my_streams(follower)
        # Le témoin : le navigateur a abandonné le flux, il ne retentera plus.
        follower.wait_for_function(STREAM_CLOSED, timeout=RECONNECT_TIMEOUT_MS)
        assert refused, "aucune reconnexion n'a été refusée"
        _record_touchdown_by_api(space_id, mr_id)
        follower.unroute(STREAM_PATTERN)

        follower.wait_for_function(NEW_CONNECTION, arg=old_connection, timeout=RECONNECT_TIMEOUT_MS)
        _expect_the_outage_action(follower, entries, score)
    finally:
        follower.context.close()

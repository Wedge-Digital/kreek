"""Tests E2E : la présence sur un match (carte 644, épic E23).

Une page de match pose un marqueur `.realtime-presence` (match, activité) ; le
script du layout le déclare au serveur, qui calcule lui-même le rôle — le
minimum du droit et de l'activité de la page. Sur `/live`, celui qui suit lit
alors, au-dessus du bandeau, qui d'autre est sur le match : « DevCoach
saisit ». La mention se retire sans rechargement quand l'onglet se ferme ou
quitte le match.

Scénarios :
- P1 — DevCoach saisit sur `/step3`, le membre simple lit « DevCoach saisit »
       sur `/live`
- P2 — DevCoach ferme son onglet : la mention disparaît chez le membre simple
- P3 — DevCoach quitte le match par le menu (navigation htmx, plus de
       marqueur) : la mention disparaît aussi
- P4 — Un tiers ne peut ni déclarer sa présence sur le match, ni lire le widget

Les identités (décision 17 de la fiche E23) :
- DevCoach, admin de l'espace et identité de la suite, coache la première
  équipe et démarre le match ;
- le membre simple (`X-Bypass-Auth-Profile: simple`, « E2E Coach 01 ») coache
  la deuxième : il suit ;
- « E2E Coach 02 » coache la troisième équipe, hors du match : c'est le tiers.
  Il se connecte **pour de vrai** par `/auth/login` : `bypass_auth` ne
  s'applique qu'à une session vide, l'identité ouverte est donc conservée.

**Le témoin avant l'absence** : chaque disparition n'est constatée qu'après
avoir vu la mention, sans quoi une page qui n'afficherait jamais rien
passerait.

**Pas de `networkidle`** : le flux SSE reste ouvert, l'état n'arrive jamais.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

import re

import pytest
from playwright.sync_api import Browser, Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from match_report_helpers import create_draft, ensure_inducements, ensure_pre_match
from realtime_helpers import HELLO_RECEIVED, login, match_url

SIMPLE_MEMBER = {"X-Bypass-Auth-Profile": "simple"}
THIRD_PARTY = "E2E Coach 02"
PRESENCE = ".match-report-live .live-presence"
WRITER_MENTION = "DevCoach saisit"


def _widget_url(space_id, mr_id):
    return f"{BASE_URL}/app/{space_id}/realtime/matches/{mr_id}/presence"


@pytest.fixture(scope="module")
def presence_ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=4)
    ctx = {**full, "teams": full["team_ids"]}
    round_id = full["round_ids"][0]
    home, away = full["team_ids"][0], full["team_ids"][1]
    mr_id = create_draft(space_id, ctx, round_id, home, away)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home, away)
    ensure_inducements(space_id, mr_id)
    return {"space_id": space_id, "mr_id": mr_id}


def _open_writer(browser: Browser, space_id, mr_id) -> Page:
    """DevCoach sur la saisie du premier camp, présence déclarée."""
    writer = browser.new_context().new_page()
    writer.goto(match_url(space_id, mr_id, "step3"), wait_until="load")
    assert writer.url.endswith(f"/{mr_id}/step3"), writer.url
    writer.wait_for_function(HELLO_RECEIVED, timeout=15000)
    return writer


def _open_follower(browser: Browser, space_id, mr_id) -> Page:
    follower = browser.new_context(extra_http_headers=SIMPLE_MEMBER).new_page()
    follower.goto(match_url(space_id, mr_id, "live"), wait_until="load")
    assert follower.url.endswith(f"/{mr_id}/live"), follower.url
    return follower


# ── P1 et P2 ──────────────────────────────────────────────────────────────────

def test_p1_p2_the_follower_sees_the_writer_until_his_tab_closes(browser, presence_ctx):
    space_id, mr_id = presence_ctx["space_id"], presence_ctx["mr_id"]
    writer = _open_writer(browser, space_id, mr_id)
    follower = _open_follower(browser, space_id, mr_id)
    try:
        expect(follower.locator(PRESENCE)).to_contain_text(WRITER_MENTION, timeout=15000)
        # La ligne du mode reste en dessous, vraie même sans l'onglet du saisisseur.
        expect(follower.locator(".match-report-live .live-notice")).to_contain_text(
            "DevCoach saisit tout le match"
        )

        writer.context.close()

        expect(follower.locator(PRESENCE)).not_to_contain_text(WRITER_MENTION, timeout=20000)
        expect(follower.locator(".match-report-live .live-notice")).to_contain_text(
            "DevCoach saisit tout le match"
        )
    finally:
        follower.context.close()


# ── P3 ────────────────────────────────────────────────────────────────────────

def test_p3_leaving_the_match_by_the_menu_withdraws_the_presence(browser, presence_ctx):
    space_id, mr_id = presence_ctx["space_id"], presence_ctx["mr_id"]
    writer = _open_writer(browser, space_id, mr_id)
    follower = _open_follower(browser, space_id, mr_id)
    try:
        expect(follower.locator(PRESENCE)).to_contain_text(WRITER_MENTION, timeout=15000)

        entry = writer.locator(".sub-menu-btn", has_text="Mes équipes")
        expect(entry).to_be_visible(timeout=15000)
        entry.click()
        expect(writer).to_have_url(re.compile(r"/team/list$"), timeout=15000)

        expect(follower.locator(PRESENCE)).not_to_contain_text(WRITER_MENTION, timeout=20000)
    finally:
        writer.context.close()
        follower.context.close()


# ── P4 ────────────────────────────────────────────────────────────────────────

DECLARE = """async ([url, space, match]) => {
    const body = new URLSearchParams({
        connection_id: window.kreekRealtime.connectionId,
        space_id: space, match_report_id: match, activity: 'follow'
    });
    const r = await fetch(url, {
        method: 'POST', credentials: 'same-origin', body,
        headers: { 'HX-Request': 'true' }
    });
    return r.status;
}"""


def test_p4_a_third_party_can_neither_declare_nor_read(browser, presence_ctx):
    space_id, mr_id = presence_ctx["space_id"], presence_ctx["mr_id"]
    third = login(browser, THIRD_PARTY)
    try:
        third.goto(f"{BASE_URL}/app/{space_id}/competitions", wait_until="load")
        third.wait_for_function(HELLO_RECEIVED, timeout=15000)
        presence_url = third.locator(".realtime-presence-client").get_attribute("data-url")

        status = third.evaluate(DECLARE, [presence_url, space_id, mr_id])
        assert status == 403, status

        widget = third.request.get(_widget_url(space_id, mr_id))
        assert widget.status == 403, widget.status
    finally:
        third.context.close()

"""Tests E2E : trois matchs en parallèle dans le même espace (carte 647, E23).

Le « Terminé quand » de l'épic, rejoué en navigateur. Les cartes 642 à 646
éprouvent chacune un match ; aucune ne voit plusieurs matchs à la fois, ni
l'étanchéité entre eux.

Le décor : six équipes, une journée, trois rencontres sans coach commun.

| Match | domicile — saisit et démarre | extérieur — reçoit |
|-------|------------------------------|--------------------|
| M1    | DevCoach                     | E2E Coach 01       |
| M2    | E2E Coach 02                 | E2E Coach 03       |
| M3    | E2E Coach 04                 | E2E Coach 05       |

**Chaque match est démarré par son coach domicile.** En saisie solo, le
démarreur qui coache une équipe saisit seul ; l'autre coach suit, et c'est à
lui seul que `/live` montre la présence (`write.is_none()`). Un admin qui ne
coache pas, démarrant M2, en aurait fait deux saisisseurs.

Scénarios :
- T1 — les six coachs sont connectés en même temps ; on saisit deux touchdowns
       par match, en alternant les trois. Chaque destinataire reçoit le toast
       de son match, et nul autre ne le reçoit ; le journal de M1, suivi sur
       `/live` par E2E Coach 01, bouge sur les saisies de M1 et sur elles seules
- T2 — E2E Coach 02 ferme son onglet : E2E Coach 03 perd sa mention ;
       E2E Coach 01, sur l'autre match, garde celle de DevCoach

Les identités (décision 17) : DevCoach par défaut, E2E Coach 01 par l'en-tête
`X-Bypass-Auth-Profile: simple`, les quatre autres **réellement connectés** par
`/auth/login`. Les saisies partent des cookies du contexte du saisisseur
(`http_session_of`) : l'auteur est la vraie identité, que le toast doit nommer.

**Le témoin avant l'absence** : un toast n'est déclaré absent d'un onglet
qu'après que son destinataire l'a reçu (`assert_none_after`).

**Le match d'un toast se lit à son lien**, qui porte `match_report_id` : les
toasts des trois matchs ont le même texte au nom de l'auteur près.

**Six contextes, six pools.** Chacun a sa propre limite de six connexions
HTTP/1.1. Le plus chargé, `/live`, tient le flux temps réel, celui de
`tower-livereload` en debug, et ses trois widgets : cinq, sous la limite.

**Pas de `networkidle`** : le flux SSE reste ouvert, l'état n'arrive jamais.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

import pytest
from playwright.sync_api import Browser, Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import entry_mode_of, first_player_id, record_action_api, start_match
from realtime_helpers import (
    DIRECT_TIMEOUT_MS,
    LOG_ENTRIES,
    SCORES,
    assert_none_after,
    count_navigations,
    count_widget_requests,
    goto_declared,
    goto_live,
    home_score,
    http_session_of,
    linked_toasts_seen,
    log_in,
    login,
    match_url,
    open_live,
    recording_context,
    toast,
    wait_for_match_toasts,
    wait_for_stream,
)

SIMPLE_MEMBER = {"X-Bypass-Auth-Profile": "simple"}
COACHES = [
    "DevCoach", "E2E Coach 01", "E2E Coach 02",
    "E2E Coach 03", "E2E Coach 04", "E2E Coach 05",
]
PRESENCE = ".match-report-live .live-presence"
# Un toast à lien dure 6 s : l'attente doit tenir dans sa vie.
TOAST_TIMEOUT_MS = 5000
TOUCHDOWNS_PER_MATCH = 2
# Qui saisit quoi, et qui reçoit : (indice du match, saisisseur, destinataire).
PLAYS = [(0, "dev", "c01"), (1, "c02", "c03"), (2, "c04", "c05")]
AUTHORS = {"dev": COACHES[0], "c02": COACHES[2], "c04": COACHES[4]}


# ── Le décor ──────────────────────────────────────────────────────────────────

def _coach_of(team_id: str) -> str | None:
    rows = query_db(
        "SELECT u.coach_name FROM team_proj t JOIN auth__users u ON u.id = t.coach_id "
        f"WHERE t.team_id = '{team_id}'"
    )
    return rows[0] if rows else None


def _assert_started_by_home_coach(mr_id: str, home_team_id: str) -> None:
    """Le témoin du décor : un match démarré par un admin qui ne coache pas
    ouvre l'écriture aux deux coachs, et `/live` ne montre plus la présence à
    personne — T2 échouerait alors sur un widget absent, loin de sa cause."""
    _, _, starter = entry_mode_of(mr_id)
    home_coach = query_db(f"SELECT coach_id FROM team_proj WHERE team_id = '{home_team_id}'")
    assert home_coach and starter == home_coach[0], (
        f"le match {mr_id} n'a pas été démarré par le coach domicile : {starter!r}"
    )


def _session_of_coach(browser: Browser, coach_name: str):
    """Une session serveur ouverte pour de vrai ; elle survit au contexte."""
    page = login(browser, coach_name)
    session = http_session_of(page.context)
    page.context.close()
    return session


@pytest.fixture(scope="module")
def trio(browser, space_id):
    """Trois matchs démarrés avant que quiconque soit connecté : les toasts du
    démarrage ne trouvent aucun onglet, et n'entrent dans aucun journal."""
    full = build_full_competition(browser, space_id, num_teams=6, num_rounds=1)
    t, round_id = full["team_ids"], full["round_ids"][0]
    coaches = [_coach_of(team) for team in t]
    assert coaches == COACHES, f"distribution des équipes inattendue : {coaches}"
    m1 = start_match(space_id, full, round_id, t[0], t[1])
    m2 = start_match(space_id, full, round_id, t[2], t[3], via=_session_of_coach(browser, COACHES[2]))
    m3 = start_match(space_id, full, round_id, t[4], t[5], via=_session_of_coach(browser, COACHES[4]))
    for mr_id, home in ((m1, t[0]), (m2, t[2]), (m3, t[4])):
        _assert_started_by_home_coach(mr_id, home)
    return {"space_id": space_id, "matches": [m1, m2, m3]}


# ── Les onglets ───────────────────────────────────────────────────────────────

def _writer_tab(browser: Browser, space_id, mr_id, coach_name=None) -> Page:
    """La saisie du domicile, présence inscrite."""
    page = recording_context(browser).new_page()
    if coach_name:
        log_in(page, coach_name)
    goto_declared(page, match_url(space_id, mr_id, "step3"))
    assert page.url.endswith(f"/{mr_id}/step3"), page.url
    return page


def _parked_tab(browser: Browser, coach_name, url) -> Page:
    """Une page de l'application qui n'est pas celle du match, flux ouvert."""
    page = recording_context(browser).new_page()
    log_in(page, coach_name)
    page.goto(url, wait_until="load")
    wait_for_stream(page)
    return page


def _follower_tab(browser: Browser, space_id, mr_id) -> Page:
    page = recording_context(browser, SIMPLE_MEMBER).new_page()
    goto_live(page, space_id, mr_id)
    return page


@pytest.fixture
def six_tabs(browser, trio):
    space_id, (m1, m2, m3) = trio["space_id"], trio["matches"]
    app = f"{BASE_URL}/app/{space_id}"
    tabs = {
        "dev": _writer_tab(browser, space_id, m1),
        "c01": _follower_tab(browser, space_id, m1),
        "c02": _writer_tab(browser, space_id, m2, COACHES[2]),
        "c03": _parked_tab(browser, COACHES[3], f"{app}/competitions"),
        "c04": _writer_tab(browser, space_id, m3, COACHES[4]),
        "c05": _parked_tab(browser, COACHES[5], f"{app}/team/list"),
    }
    yield tabs
    for page in tabs.values():
        page.context.close()


# ── T1 ────────────────────────────────────────────────────────────────────────

def _touchdown(space_id, mr_id, writer: Page, turn: int) -> None:
    player = first_player_id(mr_id, "home")
    record_action_api(
        space_id, mr_id, "home", player, turn, "TOUCHDOWN", via=http_session_of(writer.context)
    )


def _recipient_got_it(recipient: Page, mr_id, writer: str, turn: int):
    """Le témoin positif : à l'écran la première fois, au journal toujours."""
    def witness():
        if turn == 1:
            expect(toast(recipient, f"— par {AUTHORS[writer]}")).to_be_visible(
                timeout=TOAST_TIMEOUT_MS
            )
        wait_for_match_toasts(recipient, mr_id, turn)
    return witness


def _the_log_moved(follower: Page, entries: int, score: int) -> None:
    expect(follower.locator(LOG_ENTRIES)).to_have_count(entries + 1, timeout=DIRECT_TIMEOUT_MS)
    expect(follower.locator(LOG_ENTRIES).first).to_contain_text("Touchdown")
    expect(follower.locator(SCORES).first).to_have_text(str(score + 1), timeout=DIRECT_TIMEOUT_MS)


def _the_log_stood_still(follower: Page, entries: int, requests_seen: list, before: int) -> None:
    """Lu après le témoin : le fait de l'autre match a déjà été routé."""
    assert requests_seen[before:] == [], f"widgets rechargés sans raison : {requests_seen[before:]}"
    assert follower.locator(LOG_ENTRIES).count() == entries


def _play(tabs, trio, play, turn, requests_seen) -> None:
    index, writer, recipient = play
    mr_id, follower = trio["matches"][index], tabs["c01"]
    entries, score = follower.locator(LOG_ENTRIES).count(), home_score(follower)
    before = len(requests_seen)
    _touchdown(trio["space_id"], mr_id, tabs[writer], turn)
    others = [page for name, page in tabs.items() if name != recipient]
    assert_none_after(_recipient_got_it(tabs[recipient], mr_id, writer, turn), others, mr_id)
    if recipient == "c01":
        _the_log_moved(follower, entries, score)
    else:
        _the_log_stood_still(follower, entries, requests_seen, before)


def _assert_whole_logs(tabs, trio) -> None:
    """Chaque journal entier : les destinataires n'ont que leur match, les
    saisisseurs rien — ni leurs propres faits, ni ceux des autres."""
    for _, writer, _ in PLAYS:
        assert linked_toasts_seen(tabs[writer]) == [], (writer, linked_toasts_seen(tabs[writer]))
    for index, _, recipient in PLAYS:
        seen, mr_id = linked_toasts_seen(tabs[recipient]), trio["matches"][index]
        assert len(seen) == TOUCHDOWNS_PER_MATCH, (recipient, seen)
        assert all(mr_id in (t["href"] or "") for t in seen), (recipient, seen)


def test_t1_each_coach_gets_only_his_match_and_the_log_moves_only_on_it(six_tabs, trio):
    follower = six_tabs["c01"]
    requests_seen = count_widget_requests(follower, trio["matches"][0])
    navigations = count_navigations(follower)

    for turn in range(1, TOUCHDOWNS_PER_MATCH + 1):
        for play in PLAYS:
            _play(six_tabs, trio, play, turn, requests_seen)

    assert navigations == [], f"la page de qui suit a été rechargée : {navigations}"
    _assert_whole_logs(six_tabs, trio)


# ── T2 ────────────────────────────────────────────────────────────────────────

def _presence_tabs(browser: Browser, space_id, m1, m2) -> dict:
    """Un saisisseur et un suiveur sur chacun des deux premiers matchs."""
    dev = browser.new_context().new_page()
    goto_declared(dev, match_url(space_id, m1, "step3"))
    c02 = login(browser, COACHES[2])
    goto_declared(c02, match_url(space_id, m2, "step3"))
    c03 = login(browser, COACHES[3])
    goto_live(c03, space_id, m2)
    c01 = open_live(browser, space_id, m1, SIMPLE_MEMBER)
    return {"dev": dev, "c01": c01, "c02": c02, "c03": c03}


def test_t2_a_closed_tab_leaves_its_match_and_only_its_match(browser, trio):
    space_id, (m1, m2, _) = trio["space_id"], trio["matches"]
    tabs = _presence_tabs(browser, space_id, m1, m2)
    on_m1, on_m2 = tabs["c01"].locator(PRESENCE), tabs["c03"].locator(PRESENCE)
    try:
        expect(on_m2).to_contain_text("E2E Coach 02 saisit", timeout=15000)
        expect(on_m1).to_contain_text("DevCoach saisit", timeout=15000)
        expect(on_m1).not_to_contain_text("E2E Coach 02")

        tabs.pop("c02").context.close()

        expect(on_m2).not_to_contain_text("E2E Coach 02 saisit", timeout=20000)
        # Lu après la disparition : la fermeture est déjà propagée.
        expect(on_m1).to_contain_text("DevCoach saisit")
    finally:
        for page in tabs.values():
            page.context.close()

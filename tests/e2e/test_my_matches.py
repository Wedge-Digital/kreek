"""Tests E2E : la page « Mes matchs » (carte 678, épic I7-E1-KR).

Les scénarios S1 à S6 de l'épic projet I7-E1, rejoués en navigateur contre le
serveur de la suite. Les cartes 675 à 677 éprouvent chacune leur règle par des
tests unitaires ; aucune ne voit la page, son widget, ses boutons et l'onglet
mobile ensemble.

Scénarios :
- S1 — Un coach à deux équipes inscrites : le prochain match de chacune porte
       « Commencer », même programmé des semaines à l'avance ; ses autres
       matchs à jouer n'ont pas de bouton.
- S2 — Il commence l'un d'eux depuis la page et va jusqu'au coup d'envoi ; de
       retour, le bouton dit « Continuer » et le ramène à l'étape où il s'était
       arrêté. Ce second temps est marqué `xfail` strict : « Continuer » le
       renvoie à l'avant-match (défaut produit, cf. le test).
- S3 — L'adversaire a commencé en « Je saisis tout » : le bouton dit
       « Suivre », en contour, et ouvre la lecture seule (`/live`).
- S4 — L'adversaire a commencé en « Chacun son équipe » : « Continuer » mène
       à l'avant-match du camp du coach.
- S5 — Une compétition de l'espace interdit le hors calendrier : « Nouveau
       match » n'apparaît pas ; autorisé, il ouvre la saisie d'un match hors
       calendrier.
- S6 — Sur mobile, l'onglet « Matchs » ouvre la page, que le hors calendrier
       soit permis ou non.

**Des coachs réellement connectés, et pas DevCoach.** Deux raisons :

- DevCoach est admin de l'espace et de chaque compétition de la suite :
  `can_write_report` l'autorise toujours, il ne verrait jamais « Suivre ».
- DevCoach et E2E Coach 01 tiennent les deux premières équipes de chaque
  `build_full_competition` : plus d'une centaine d'équipes inscrites chacun
  dans la base de la suite. Le widget consulte l'accès de **tous** leurs
  prochains matchs à la fois (`join_all`), ce qui porte le pool du serveur e2e
  à son maximum — autant de connexions que Postgres en accepte — et refuse
  ensuite toute autre connexion pendant dix minutes, `psql` des tests compris
  (cf. compte rendu de la carte 678).

Les scénarios regardent donc par les yeux d'E2E Coach 08 et 09, qui n'ont que
quelques équipes. DevCoach ne fait que monter les compétitions, dont il est
l'admin par le magicien.

**Une compétition par scénario.** Un match en cours entre les mêmes équipes
fait refuser un démarrage (409), et une équipe restée dans sa séquence
d'après-match bloque la suite : aucun scénario ne partage ses équipes.

**Une carte se retrouve par le nom de sa compétition.** Un coach a des matchs
dans d'autres compétitions de la suite ; le nom, unique par exécution, ne
désigne que celles du scénario.

**Les clics** : les boutons d'une carte sont des liens ordinaires, sans
câblage htmx ; « Nouveau match » et l'onglet mobile sont des `hx-get`, cliqués
par `cliquer_quand_cable_locator`.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make seed_e2e`.
"""

import datetime
import re

import pytest
import requests
from playwright.sync_api import Browser, Page, expect

from competition_lifecycle import (
    BASE_URL,
    _wait_teams_enrolled,
    build_and_submit_team_http,
    create_full_competition,
)
from db_helpers import execute_db, query_db
from htmx_helpers import cliquer_quand_cable_locator
from match_report_helpers import draft_of_round, entry_mode_of, pass_inducements_in_browser
from realtime_helpers import log_in, login

COACH = "E2E Coach 08"
OPPONENT = "E2E Coach 09"
THIRD = "E2E Coach 10"
HX = {"HX-Request": "true"}
FORM = {**HX, "Content-Type": "application/x-www-form-urlencoded"}
MOBILE = {"width": 390, "height": 844}
OUTLINE = re.compile(r"\bbtn-outline\b")
ACTIVE = re.compile(r"\bactive\b")


# ── Le décor ──────────────────────────────────────────────────────────────────

def _coach_id(name: str) -> str:
    rows = query_db(f"SELECT id FROM auth__users WHERE coach_name = '{name}'")
    assert rows, f"coach {name!r} absent de la base (make seed_e2e)"
    return rows[0].strip()


def _admin(space_id, ctx, suffix) -> str:
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/admin/{suffix}"
    )


def _in_weeks(weeks: int) -> str:
    return (datetime.date.today() + datetime.timedelta(weeks=weeks)).isoformat()


def _date_the_rounds(space_id, ctx, weeks) -> None:
    """Repousse chaque journée de `weeks` semaines, par la route d'édition de
    l'admin. **Avant** la génération des appariements : la date de journée
    est recopiée dans la projection d'affichage à leur création."""
    for round_id, n in zip(ctx["round_ids"], weeks):
        day = _in_weeks(n)
        resp = requests.put(
            _admin(space_id, ctx, f"schedule/rounds/{round_id}"),
            json={"date_start": day, "date_end": day},
            headers=HX,
            timeout=30,
        )
        assert resp.ok, f"date de journée : {resp.status_code} {resp.text[:200]}"


def _build(browser, space_id, coaches: list[str], weeks: tuple[int, ...] = ()) -> dict:
    """Une compétition publiée, une équipe par coach nommé — un coach peut en
    tenir deux —, autant de journées que `weeks` (une par défaut), repoussées
    d'autant de semaines s'il y en a, puis appariées.

    `build_full_competition` ne convient pas : elle donne les équipes aux
    premiers coachs de l'espace, DevCoach et E2E Coach 01."""
    page = browser.new_page()
    try:
        ctx = create_full_competition(
            page, f"{BASE_URL}/app/{space_id}/competitions/create",
            num_rounds=max(len(weeks), 1),
        )
        ctx["team_ids"] = [
            build_and_submit_team_http(
                page, space_id, ctx["competition_id"], ctx["season_id"],
                _coach_id(coach), roster_index=i,
            )
            for i, coach in enumerate(coaches)
        ]
        _wait_teams_enrolled(ctx["team_ids"])
        assert page.request.get(_admin(space_id, ctx, "schedule/rounds")).ok
        ctx["round_ids"] = [r.strip() for r in query_db(
            f"SELECT id FROM competition_match_days WHERE season_id = '{ctx['season_id']}' "
            "ORDER BY position"
        )]
        _date_the_rounds(space_id, ctx, weeks)
        resp = page.request.post(_admin(space_id, ctx, "schedule/generate-all"), headers=HX)
        assert resp.ok, f"generate-all : {resp.status} {resp.text()[:300]}"
        return ctx
    finally:
        page.close()


def _clear_the_schedule(space_id, ctx) -> None:
    """Retire les rencontres du scénario, par la route de l'admin : ses
    équipes n'ont plus de match à jouer.

    **Pourquoi ce ménage, que les autres fichiers ne font pas.** Le widget
    consulte l'accès de chaque prochain match du coach en même temps ; chaque
    exécution laissant au coach de nouveaux matchs à jouer, la page finirait
    par vider le pool du serveur, comme elle le fait pour DevCoach."""
    rounds = "','".join(ctx["round_ids"])
    for pairing_id in query_db(
        f"SELECT id FROM competition_match_day_pairings WHERE match_day_id IN ('{rounds}')"
    ):
        resp = requests.delete(
            _admin(space_id, ctx, "schedule/delete-match"),
            json={"pairing_id": pairing_id.strip()},
            headers=HX,
            timeout=30,
        )
        assert resp.ok, f"retrait de la rencontre : {resp.status_code} {resp.text[:200]}"


@pytest.fixture
def scenario(browser, space_id):
    """Monte la compétition d'un scénario, et la vide de ses rencontres après."""
    built = []

    def build(coaches, weeks=()):
        built.append(_build(browser, space_id, coaches, weeks))
        return built[-1]

    yield build
    for ctx in built:
        _clear_the_schedule(space_id, ctx)


def _match_of_two(scenario) -> dict:
    """Deux équipes, une journée : un seul appariement, un seul brouillon."""
    return scenario([COACH, OPPONENT])


@pytest.fixture
def competition_deux_equipes(scenario):
    """Quatre équipes dont deux au coach, deux journées à 8 et 12 semaines."""
    return scenario([COACH, COACH, OPPONENT, THIRD], weeks=(8, 12))


@pytest.fixture
def competition_commencer(scenario):
    return _match_of_two(scenario)


@pytest.fixture
def competition_suivre(scenario):
    return _match_of_two(scenario)


@pytest.fixture
def competition_chacun_son_equipe(scenario):
    return _match_of_two(scenario)


@pytest.fixture
def competition_hors_calendrier(scenario):
    return _match_of_two(scenario)


@pytest.fixture
def competition_mobile(scenario):
    return _match_of_two(scenario)


@pytest.fixture
def coach(page: Page) -> Page:
    """Le coach du scénario, sur la page du test — dont les erreurs de console
    sont vérifiées."""
    log_in(page, COACH)
    return page


@pytest.fixture
def opponent(browser: Browser):
    """Son adversaire, connecté dans son propre navigateur."""
    page = login(browser, OPPONENT)
    yield page
    page.context.close()


# ── La page ───────────────────────────────────────────────────────────────────

def _my_matches_url(space_id) -> str:
    return f"{BASE_URL}/app/{space_id}/my-matches"


def _open_my_matches(page: Page, space_id) -> None:
    page.goto(_my_matches_url(space_id), wait_until="load")
    expect(page.locator(".my-matches-list")).to_be_visible(timeout=15000)


def _cards(page: Page, ctx):
    """Les cartes du scénario, et elles seules."""
    return page.locator(".my-matches-match", has_text=ctx["name"])


def _button(card):
    return card.locator(".my-matches-actions a")


def _the_card(page: Page, ctx):
    cards = _cards(page, ctx)
    expect(cards).to_have_count(1)
    return cards


def _follow(page: Page, card) -> None:
    """Un bouton de carte est un lien ordinaire : une navigation complète."""
    with page.expect_navigation(wait_until="load"):
        _button(card).click()


def _start_from_my_matches(page: Page, space_id, ctx, team_entry: bool) -> str:
    """« Commencer » sur la page, puis « Qui saisit ? » → « Commencer ».
    Rend l'identifiant du rapport."""
    mr_id = draft_of_round(ctx["round_ids"][0])
    _open_my_matches(page, space_id)
    card = _the_card(page, ctx)
    expect(_button(card)).to_have_text("Commencer")
    _follow(page, card)
    page.wait_for_url(f"**/match-report/{mr_id}", timeout=10000)

    entry = page.locator(".entry-mode-card")
    if team_entry:
        entry.locator(".entry-mode-option--team").click()
        expect(entry.locator("input[name='entry_mode'][value='TeamEntry']")).to_be_checked()
    with page.expect_navigation(wait_until="load"):
        page.get_by_role("button", name="Commencer").click()
    return mr_id


def _side_of(mr_id, team_id) -> str:
    home = query_db(
        f"SELECT home_team_id FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )[0].strip()
    return "home" if home == team_id else "away"


# ── S1 — le prochain match de chaque équipe, même lointain ────────────────────

def _pairings_of(ctx) -> list[tuple[str, str, str]]:
    """`(journée, domicile, extérieur)` des rencontres de la compétition."""
    rounds = "','".join(ctx["round_ids"])
    rows = query_db(
        "SELECT match_day_id || '|' || home_team_id || '|' || away_team_id "
        f"FROM competition_match_day_pairings WHERE match_day_id IN ('{rounds}')"
    )
    return [tuple(r.strip().split("|")) for r in rows if r.strip()]


def _round_name(round_id) -> str:
    return query_db(f"SELECT name FROM competition_match_days WHERE id = '{round_id}'")[0].strip()


def test_s1_le_prochain_match_de_chaque_equipe_porte_commencer(
    coach: Page, space_id, competition_deux_equipes
):
    ctx = competition_deux_equipes
    mine = set(ctx["team_ids"][:2])
    first, second = ctx["round_ids"]
    mine_by_round = {
        rid: [p for p in _pairings_of(ctx) if p[0] == rid and mine & set(p[1:])]
        for rid in (first, second)
    }
    assert all(mine_by_round.values()), f"chaque journée doit engager le coach : {mine_by_round}"

    _open_my_matches(coach, space_id)
    cards = _cards(coach, ctx)
    # Un derby entre ses deux équipes ne compte qu'une fois.
    expect(cards).to_have_count(len(mine_by_round[first]) + len(mine_by_round[second]))

    nexts = cards.filter(has_text=f"{ctx['name']} · {_round_name(first)}")
    expect(nexts).to_have_count(len(mine_by_round[first]))
    due = datetime.date.fromisoformat(_in_weeks(8)).strftime("%d/%m/%Y")
    for i in range(nexts.count()):
        card = nexts.nth(i)
        expect(card).to_contain_text("Prochain match")
        expect(card.locator(".my-matches-due")).to_have_text(f"avant le {due}")
        expect(_button(card)).to_have_text("Commencer")

    later = cards.filter(has_text=f"{ctx['name']} · {_round_name(second)}")
    expect(later).to_have_count(len(mine_by_round[second]))
    expect(later.locator(".my-matches-actions a")).to_have_count(0)
    expect(later.filter(has_text="Prochain match")).to_have_count(0)


# ── S2 — « Continuer » ramène à l'étape en cours ──────────────────────────────

def _start_and_kick_off(coach: Page, space_id, ctx) -> tuple[str, str]:
    """Commencé depuis la page en « Je saisis tout », fan factor lancé, coups
    de pouce passés : le coach est au match. Rend le rapport et l'URL où il
    s'est arrêté."""
    mr_id = _start_from_my_matches(coach, space_id, ctx, team_entry=False)
    coach.wait_for_url(f"**/{mr_id}/step2", timeout=10000)
    assert entry_mode_of(mr_id)[:2] == ("PreMatch", "SoloEntry"), entry_mode_of(mr_id)

    coach.fill("input[name='home_fan_roll']", "2")
    coach.fill("input[name='away_fan_roll']", "3")
    with coach.expect_navigation(wait_until="load"):
        coach.click("button[type='submit']")
    pass_inducements_in_browser(coach)
    assert coach.url.endswith(f"/{mr_id}/live"), coach.url
    return mr_id, coach.url


def _continue(coach: Page, space_id, ctx) -> None:
    _open_my_matches(coach, space_id)
    card = _the_card(coach, ctx)
    expect(_button(card)).to_have_text("Continuer")
    expect(_button(card)).not_to_have_class(OUTLINE)
    _follow(coach, card)


@pytest.fixture
def competition_continuer(scenario):
    return _match_of_two(scenario)


def test_s2_le_match_commence_porte_continuer(coach: Page, space_id, competition_commencer):
    """Le bouton change, et mène au rapport ; où exactement, c'est le test
    suivant."""
    mr_id, _ = _start_and_kick_off(coach, space_id, competition_commencer)
    _continue(coach, space_id, competition_commencer)
    assert f"/match-report/{mr_id}/" in coach.url, coach.url


@pytest.mark.xfail(
    strict=True,
    reason=(
        "Défaut produit relevé par la carte 678 : `from_pairing` mène à "
        "`edit_match_report`, dont l'aiguillage (`match_selection_controller::"
        "edit_page`) renvoie tout rapport en phase `PreMatch` sur `/step2` — "
        "l'avant-match, même une fois le coup d'envoi donné. Strict : ce test "
        "passera quand l'aiguillage sera corrigé, et le marqueur devra partir."
    ),
)
def test_s2_continuer_ramene_a_l_etape_en_cours(coach: Page, space_id, competition_continuer):
    mr_id, current_step = _start_and_kick_off(coach, space_id, competition_continuer)
    _continue(coach, space_id, competition_continuer)
    assert coach.url == current_step, f"attendu {current_step}, obtenu {coach.url}"


# ── S3 — « Suivre » ouvre la lecture seule ────────────────────────────────────

def test_s3_suivre_ouvre_la_lecture_seule(coach: Page, space_id, competition_suivre, opponent):
    ctx = competition_suivre
    _open_my_matches(opponent, space_id)
    expect(_button(_the_card(opponent, ctx))).to_have_text("Commencer")

    mr_id = _start_from_my_matches(coach, space_id, ctx, team_entry=False)
    coach.wait_for_url(f"**/{mr_id}/step2", timeout=10000)
    assert entry_mode_of(mr_id)[1] == "SoloEntry"

    _open_my_matches(opponent, space_id)
    card = _the_card(opponent, ctx)
    expect(_button(card)).to_have_text("Suivre")
    expect(_button(card)).to_have_class(OUTLINE)
    _follow(opponent, card)
    assert opponent.url.endswith(f"/match-report/{mr_id}/live"), opponent.url
    expect(opponent.locator(".match-report-live")).to_be_visible(timeout=10000)


# ── S4 — « Continuer » mène à l'avant-match de son camp ───────────────────────

def test_s4_continuer_mene_a_l_avant_match_de_son_camp(
    coach: Page, space_id, competition_chacun_son_equipe, opponent
):
    ctx = competition_chacun_son_equipe
    mr_id = _start_from_my_matches(coach, space_id, ctx, team_entry=True)
    coach.wait_for_url(f"**/{mr_id}/step2/{_side_of(mr_id, ctx['team_ids'][0])}", timeout=10000)
    assert entry_mode_of(mr_id)[:2] == ("PreMatch", "TeamEntry"), entry_mode_of(mr_id)

    _open_my_matches(opponent, space_id)
    card = _the_card(opponent, ctx)
    expect(_button(card)).to_have_text("Continuer")
    expect(_button(card)).not_to_have_class(OUTLINE)
    _follow(opponent, card)
    opponent.wait_for_url(
        f"**/{mr_id}/step2/{_side_of(mr_id, ctx['team_ids'][1])}", timeout=10000
    )
    expect(opponent.locator("input[name='fan_roll']")).to_be_visible()


# ── Le hors calendrier, étanche ───────────────────────────────────────────────

def _season_options(space_id: str) -> dict[str, str | None]:
    rows = query_db(
        "SELECT s.id || '|' || coalesce(s.options::text, 'NULL') "
        "FROM competition_seasons s JOIN competitions c ON c.id = s.competition_id "
        f"WHERE c.space_id = '{space_id}'"
    )
    state = {}
    for row in filter(str.strip, rows):
        season_id, raw = row.strip().split("|", 1)
        state[season_id] = None if raw == "NULL" else raw
    return state


def _set_options(where: str, value: str | None) -> None:
    what = "NULL" if value is None else f"'{value}'::jsonb"
    execute_db(f"UPDATE competition_seasons SET options = {what} WHERE {where}")


@pytest.fixture
def hors_calendrier_etanche(space_id):
    """Un espace où rien n'interdit, rendu tel qu'il était — la règle est
    « une seule suffit » : une compétition laissée en interdiction par un
    autre test masquerait « Nouveau match » à tout l'espace. Même geste que
    `options_etanches` de `test_competition_hors_calendrier`."""
    before = _season_options(space_id)
    _set_options(
        f"competition_id IN (SELECT id FROM competitions WHERE space_id = '{space_id}')", None
    )
    yield
    for season_id, value in before.items():
        _set_options(f"id = '{season_id}'", value)


def _allow_off_schedule(space_id, ctx, allowed: bool) -> None:
    """Coche ou décoche l'option, par la route du panneau : une case décochée
    n'est pas envoyée, le corps vide porte l'interdiction."""
    resp = requests.post(
        _admin(space_id, ctx, "settings/general-options"),
        data={"autorise_hors_calendrier": "on"} if allowed else {},
        headers=FORM,
        timeout=15,
    )
    assert resp.status_code == 200, resp.text[:300]


def _new_match(page: Page):
    return page.locator(".my-matches-new a", has_text="Nouveau match")


# ── S5 — « Nouveau match » suit la règle du hors calendrier ───────────────────

def test_s5_nouveau_match_suit_le_hors_calendrier(
    coach: Page, space_id, competition_hors_calendrier, hors_calendrier_etanche
):
    ctx = competition_hors_calendrier
    _allow_off_schedule(space_id, ctx, False)
    _open_my_matches(coach, space_id)
    expect(_the_card(coach, ctx)).to_be_visible()
    expect(_new_match(coach)).to_have_count(0)

    _allow_off_schedule(space_id, ctx, True)
    _open_my_matches(coach, space_id)
    expect(_new_match(coach)).to_have_count(1)
    cliquer_quand_cable_locator(coach, _new_match(coach))
    coach.wait_for_url(f"**/app/{space_id}/match-report/new", timeout=10000)
    expect(coach.locator(".match-report-step1 form[action$='/match-report/new']")).to_be_visible()


# ── S6 — l'onglet mobile « Matchs » ouvre la page ─────────────────────────────

def _matches_tab(page: Page):
    """Correspondance exacte : `has_text="Match"` attraperait « Matchs »."""
    return page.locator(".mobile-tab").filter(has=page.get_by_text("Matchs", exact=True))


def test_s6_l_onglet_mobile_matchs_ouvre_la_page(
    coach: Page, space_id, competition_mobile, hors_calendrier_etanche
):
    coach.set_viewport_size(MOBILE)
    for allowed in (True, False):
        _allow_off_schedule(space_id, competition_mobile, allowed)
        coach.goto(f"{BASE_URL}/app/{space_id}/home", wait_until="load")
        tab = _matches_tab(coach)
        expect(tab).to_be_visible(timeout=15000)
        expect(tab).not_to_have_class(ACTIVE)

        cliquer_quand_cable_locator(coach, tab)
        coach.wait_for_url(_my_matches_url(space_id), timeout=10000)
        expect(coach.locator(".my-matches .h3-gras")).to_have_text("Mes matchs")
        expect(coach.locator(".my-matches-list")).to_be_visible(timeout=15000)
        expect(_the_card(coach, competition_mobile)).to_be_visible()
        expect(_new_match(coach)).to_have_count(1 if allowed else 0)
        expect(_matches_tab(coach)).to_have_class(ACTIVE, timeout=10000)

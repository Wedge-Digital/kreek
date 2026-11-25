"""Tests E2E : deux coachs saisissent le même match en « Chacun son équipe »
(carte 661, épic E22).

Le « Terminé quand » de l'épic, rejoué en navigateur. Les cartes 652 à 660
éprouvent chacune leur morceau ; aucune ne voit le parcours d'un bout à
l'autre, ni n'éprouve d'un seul geste toutes les familles de routes contre une
écriture dans le camp adverse.

Les identités : **DevCoach**, admin de l'espace, par défaut (`bypass_auth` sur
une session vide) ; **E2E Coach 01**, membre simple, **réellement connecté**
par `/auth/login` (`realtime_helpers.login`). Ses saisies HTTP partent des
cookies de son contexte (`http_session_of`) : l'auteur est sa vraie identité,
que le journal nomme. `build_full_competition` donne la première équipe à
DevCoach, la deuxième à E2E Coach 01.

Scénarios :
- E1 — Le parcours de bout en bout. DevCoach démarre en « Chacun son équipe »
       depuis la carte « Qui saisit ? » ; chacun lance son fan factor ;
       l'outsider attend sur sa page, qui s'ouvre sur ses achats sans
       rechargement quand le favori déclare « J'ai fini mes achats » ; il
       termine les siens. Chacun saisit son action — E2E Coach 01 voit celle
       de DevCoach arriver au direct, la sienne « saisi par vous » —, son MVP,
       ses gains et son paragraphe. E2E Coach 01 valide depuis l'écran
       d'attente ; la validation de DevCoach publie. Le récapitulatif, chez
       l'un comme chez l'autre, montre les deux paragraphes et « Publié par ».
- E2 — Les refus : E2E Coach 01 poste dans le camp de DevCoach, sur chaque
       famille de routes, au moment où elle est active — `step2/{side}`,
       coups de pouce, mercenaires, actions, suppression d'action, MVP,
       tirage, `step5/{side}`, `step5/{side}/validate`,
       `step5/{side}/withdraw-validation`, `recap/withdraw-validation` avec le
       `team_side` adverse. Chacun rend 403, et le camp de DevCoach n'en garde
       aucune trace.
- E3 — Le gel : E2E Coach 01 valide son camp ; son après-match reposté rend
       409. Après « Retirer ma validation », le même POST passe.
- E4 — Le démarrage concurrent : les deux coachs ont le brouillon ouvert ;
       DevCoach commence en « Chacun son équipe », puis E2E Coach 01 clique
       « Commencer » sur sa page restée ouverte, en « Je saisis tout ». Un
       seul mode est retenu ; le second reçoit 409, et retrouve le match dans
       son avant-match.

**Le refus vient de la garde, pas du formulaire.** Chaque POST porte le corps
qu'emploient les e2e de sa carte : un 400 ou un 422 ne peut pas se faire passer
pour le 403 attendu. **Un seul sens s'éprouve** : DevCoach est admin, il écrit
les deux camps (§ 12.6) ; c'est donc E2E Coach 01 qui frappe chez lui.

**Une compétition par scénario.** Un match en cours entre les mêmes équipes
fait refuser un démarrage (409), et une équipe restée dans sa séquence
d'après-match ne reçoit pas le match publié suivant : aucun scénario ne
partage ses équipes.

**Le camp de chacun se lit en base** pour E1 et E4, démarrés depuis le
brouillon de la journée, dont le tirage fixe le domicile ; E2 et E3 passent par
`create_draft`, qui met DevCoach au domicile.

**Les clics sans câblage htmx** : « Commencer », « J'ai fini mes achats »,
« Enregistrer », « Valider les … », « Retirer ma validation » et « … et
publier » sont des formulaires ordinaires ; `cliquer_quand_cable` n'aboutirait
jamais sur eux. Le bouton de l'outsider, inséré sans rechargement, est attendu
visible avant d'être cliqué.

**Pas de `networkidle`** : le flux SSE reste ouvert, l'état n'arrive jamais.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

import re

import pytest
import requests
from playwright.sync_api import Browser, Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import (
    draft_of_round,
    ensure_mvps,
    ensure_team_entry_inducements,
    entry_mode_of,
    first_player_id,
    pass_team_entry_inducements,
    record_action_api,
    roll_team_fan_factors,
    start_team_entry,
)
from realtime_helpers import (
    DIRECT_TIMEOUT_MS,
    goto_declared,
    goto_live,
    http_session_of,
    login,
    presence_declared,
    wait_for_stream,
)

COACH = "E2E Coach 01"
FINISHED = "J'ai fini mes achats"
WAIT = ".match-report-inducements .inducements-wait"
VALIDATE = re.compile(r"^Valider les ")
PUBLISHES = re.compile(r"et publier")
WITNESS = "() => { window.__sansRechargement = true; }"
STILL_THERE = "() => window.__sansRechargement === true"
TIRAGE = 6


# ── Le décor ──────────────────────────────────────────────────────────────────

def _competition(browser, space_id):
    """Deux équipes, une journée : un seul appariement, un seul brouillon."""
    return build_full_competition(browser, space_id, num_teams=2, num_rounds=1)


@pytest.fixture
def competition_parcours(browser, space_id):
    return _competition(browser, space_id)


@pytest.fixture
def competition_refus(browser, space_id):
    return _competition(browser, space_id)


@pytest.fixture
def competition_gel(browser, space_id):
    return _competition(browser, space_id)


@pytest.fixture
def competition_concurrente(browser, space_id):
    return _competition(browser, space_id)


@pytest.fixture
def coach(browser: Browser):
    """E2E Coach 01, connecté pour de vrai, dans son propre navigateur."""
    page = login(browser, COACH)
    yield page
    page.context.close()


@pytest.fixture
def devcoach(browser: Browser):
    page = browser.new_context().new_page()
    yield page
    page.context.close()


def _url(space_id, mr_id, suffix=""):
    base = f"{BASE_URL}/app/{space_id}/match-report/{mr_id}"
    return f"{base}/{suffix}" if suffix else base


def _sides(mr_id, ctx) -> dict:
    """Le camp de chacun, lu sur la projection : `{"dev": …, "coach": …}`."""
    home = query_db(
        f"SELECT home_team_id FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )[0]
    dev = "home" if home == ctx["team_ids"][0] else "away"
    return {"dev": dev, "coach": "away" if dev == "home" else "home"}


def _coach_id(team_id) -> str:
    return query_db(f"SELECT coach_id FROM team_proj WHERE team_id = '{team_id}'")[0]


def _appended(mr_id, event_type) -> int:
    return int(query_db(
        "SELECT count(*) FROM match_report_event_store "
        f"WHERE match_report_id = '{mr_id}' AND event_type = '{event_type}'"
    )[0])


# ── E1 — le parcours de bout en bout ──────────────────────────────────────────

def _start_from_the_card(page: Page, space_id, mr_id, side) -> None:
    """« Qui saisit ? » → « Chacun son équipe » → « Commencer »."""
    page.goto(_url(space_id, mr_id), wait_until="load")
    carte = page.locator(".entry-mode-card")
    carte.locator(".entry-mode-option--team").click()
    expect(carte.locator("input[name='entry_mode'][value='TeamEntry']")).to_be_checked()
    with page.expect_navigation(wait_until="load"):
        page.get_by_role("button", name="Commencer").click()
    page.wait_for_url(f"**/{mr_id}/step2/{side}", timeout=10000)


def _assert_started_by_devcoach(mr_id, ctx) -> None:
    phase, mode, starter = entry_mode_of(mr_id)
    assert (phase, mode) == ("PreMatch", "TeamEntry"), (phase, mode)
    assert starter == _coach_id(ctx["team_ids"][0]), f"démarreur inattendu : {starter!r}"


def _roll(page: Page, space_id, mr_id, side, roll) -> None:
    """Son fan factor, sur l'avant-match de son camp."""
    page.goto(_url(space_id, mr_id, "step2"), wait_until="load")
    assert page.url.endswith(f"/{mr_id}/step2/{side}"), page.url
    page.fill("input[name='fan_roll']", str(roll))
    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")
    assert "/step2" not in page.url, page.url


def _topdog_and_underdog(mr_id) -> tuple[str, str]:
    """Le favori puis l'outsider, par équipe : la plus forte valeur figée au
    premier jet, le domicile à égalité (R10)."""
    values = query_db(
        "SELECT (payload->>'home_team_value') || '|' || (payload->>'away_team_value') "
        "FROM match_report_event_store "
        f"WHERE match_report_id = '{mr_id}' AND event_type = 'TeamValuesRecorded'"
    )[0]
    teams = query_db(
        "SELECT home_team_id || '|' || away_team_id FROM match_report_proj "
        f"WHERE match_report_id = '{mr_id}'"
    )[0]
    home_tv, away_tv = (int(v) for v in values.split("|"))
    home, away = teams.split("|")
    return (away, home) if away_tv > home_tv else (home, away)


def _click_finished(page: Page) -> None:
    with page.expect_navigation(wait_until="load"):
        page.get_by_role("button", name=FINISHED).click()


def _outsider_waits(page: Page, space_id, mr_id, team_id) -> None:
    """L'outsider sur sa page, présence inscrite et flux ouvert : l'attente."""
    goto_declared(page, _url(space_id, mr_id, f"inducements/{team_id}"))
    wait_for_stream(page)
    expect(page.locator(WAIT)).to_contain_text("achète ses coups de pouce")
    expect(page.get_by_role("button", name=FINISHED)).to_have_count(0)
    page.evaluate(WITNESS)


def _favorite_finishes(page: Page, space_id, mr_id, team_id) -> None:
    """Le favori déclare avoir fini, panier vide ; son attente se déclare."""
    page.goto(_url(space_id, mr_id, f"inducements/{team_id}"), wait_until="load")
    with page.expect_response(presence_declared):
        _click_finished(page)
    expect(page.locator(WAIT)).to_contain_text("Vos coups de pouce sont validés")


def _inducements_in_turn(pages: dict, space_id, mr_id) -> None:
    topdog, underdog = _topdog_and_underdog(mr_id)
    favori, outsider = pages[topdog], pages[underdog]
    _outsider_waits(outsider, space_id, mr_id, underdog)
    _favorite_finishes(favori, space_id, mr_id, topdog)

    expect(outsider.get_by_role("button", name=FINISHED)).to_be_visible(timeout=DIRECT_TIMEOUT_MS)
    assert outsider.evaluate(STILL_THERE), "la page de l'outsider s'est rechargée"
    _click_finished(outsider)
    expect(favori.locator(".match-report-inducements .mr-header-sub")).to_contain_text(
        "Achats terminés pour les deux équipes", timeout=DIRECT_TIMEOUT_MS
    )
    assert _appended(mr_id, "InducementsRecorded") == 2


def _each_his_action(coach: Page, space_id, mr_id, sides) -> None:
    """Chacun son touchdown ; E2E Coach 01, sur `/live`, voit arriver celui de
    DevCoach au direct, et le sien à son nom."""
    goto_live(coach, space_id, mr_id)
    wait_for_stream(coach)
    dev, mine = sides["dev"], sides["coach"]
    record_action_api(space_id, mr_id, dev, first_player_id(mr_id, dev), 1, "TOUCHDOWN")
    dev_row = coach.locator(f".mr-log-entry--{dev}")
    expect(dev_row).to_have_count(1, timeout=DIRECT_TIMEOUT_MS)
    expect(dev_row).to_contain_text("saisi par DevCoach")

    via = http_session_of(coach.context)
    record_action_api(space_id, mr_id, mine, first_player_id(mr_id, mine), 2, "TOUCHDOWN", via=via)
    own_row = coach.locator(f".mr-log-entry--{mine}")
    expect(own_row).to_have_count(1, timeout=DIRECT_TIMEOUT_MS)
    expect(own_row).to_contain_text("saisi par vous")


def _record_post_match(page: Page, space_id, mr_id, side, gain, paragraph) -> None:
    """Gains et paragraphe de son camp ; le titre, s'il lui revient (R8)."""
    page.goto(_url(space_id, mr_id, f"step5/{side}"), wait_until="load")
    title = page.locator("input[name=summary_title]")
    if title.count():
        title.fill("Le match à deux")
    page.locator("input[name=gain]").fill(str(gain))
    page.locator("textarea[name=summary_paragraph]").fill(paragraph)
    with page.expect_navigation(wait_until="load"):
        page.get_by_role("button", name="Enregistrer").click()


def _coach_validates_first(coach: Page, space_id, mr_id, side, paragraph) -> None:
    _record_post_match(coach, space_id, mr_id, side, 45, paragraph)
    assert coach.url.endswith(f"/{mr_id}/step5/{side}"), coach.url
    with coach.expect_navigation(wait_until="load"):
        coach.get_by_role("button", name=VALIDATE).click()
    expect(coach.locator(".team-step5-validated")).to_contain_text("Validé ✓")


def _devcoach_publishes(devcoach: Page, space_id, mr_id, side, paragraph) -> None:
    """Le second après-match mène au récapitulatif ; sa validation publie."""
    _record_post_match(devcoach, space_id, mr_id, side, 60, paragraph)
    assert devcoach.url.endswith(f"/{mr_id}/recap"), devcoach.url
    publier = devcoach.locator(".ms-btn-primary", has_text=PUBLISHES)
    expect(publier).to_have_count(1)
    with devcoach.expect_navigation(wait_until="load"):
        publier.click()
    assert devcoach.url.endswith(f"/{mr_id}/recap"), devcoach.url


def _assert_published_with_both(page: Page, paragraphs) -> None:
    byline = page.locator(".ms-card-header-byline")
    expect(byline).to_contain_text("Publié par")
    expect(byline.locator("strong")).to_have_text("DevCoach")
    expect(page.locator(".ms-validation-row")).to_have_count(0)
    article = page.locator(".ms-card", has_text="Compte-rendu")
    for paragraph in paragraphs:
        expect(article).to_contain_text(paragraph)


def test_e1_deux_coachs_saisissent_le_match_de_bout_en_bout(
    space_id, competition_parcours, devcoach, coach
):
    ctx = competition_parcours
    mr_id = draft_of_round(ctx["round_ids"][0])
    sides = _sides(mr_id, ctx)
    paragraphs = ("DevCoach raconte sa victoire.", "E2E Coach 01 raconte sa défaite.")

    _start_from_the_card(devcoach, space_id, mr_id, sides["dev"])
    _assert_started_by_devcoach(mr_id, ctx)
    _roll(devcoach, space_id, mr_id, sides["dev"], 2)
    _roll(coach, space_id, mr_id, sides["coach"], 3)
    _inducements_in_turn(dict(zip(ctx["team_ids"], (devcoach, coach))), space_id, mr_id)

    _each_his_action(coach, space_id, mr_id, sides)
    ensure_mvps(space_id, mr_id, vias={sides["coach"]: http_session_of(coach.context)})
    _coach_validates_first(coach, space_id, mr_id, sides["coach"], paragraphs[1])
    _devcoach_publishes(devcoach, space_id, mr_id, sides["dev"], paragraphs[0])

    _assert_published_with_both(devcoach, paragraphs)
    coach.goto(_url(space_id, mr_id, "recap"), wait_until="load")
    _assert_published_with_both(coach, paragraphs)


# ── E2 — toute écriture dans le camp adverse est refusée ──────────────────────

HX = {"HX-Request": "true"}


def _home_team(mr_id) -> str:
    return query_db(
        f"SELECT home_team_id FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )[0]


def _home_action(mr_id) -> str:
    rows = query_db(
        f"SELECT action_id FROM match_report_actions WHERE match_report_id = '{mr_id}' "
        "AND team_side = 'home' AND NOT is_deleted LIMIT 1"
    )
    assert rows, f"aucune action du domicile dans {mr_id}"
    return rows[0]


def _home_designated(mr_id) -> list:
    """Six joueurs du domicile désignés pour le tirage — le corps de
    `test_match_report_team_mvp`."""
    players = query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{_home_team(mr_id)}' "
        f"AND membership = 'Active' ORDER BY player_id LIMIT {TIRAGE}"
    )
    assert len(players) == TIRAGE, f"{TIRAGE} joueurs attendus : {players}"
    return [("player_id", p) for p in players] + [("player_type", "regular")] * TIRAGE


def _attempt(session, space_id, mr_id, attempt) -> tuple[str, int]:
    """Une écriture directe : `(libellé, méthode, suffixe, corps, en-têtes)`."""
    label, method, suffix, data, headers = attempt
    resp = session.request(
        method, _url(space_id, mr_id, suffix),
        data=data, headers=headers, allow_redirects=False, timeout=30,
    )
    return label, resp.status_code


def _not_refused(session, space_id, mr_id, attempts) -> list:
    """Les tentatives qui n'ont pas rendu 403 — toutes, pour un seul constat."""
    results = [_attempt(session, space_id, mr_id, attempt) for attempt in attempts]
    return [(label, status) for label, status in results if status != 403]


def _pre_match_attempts():
    return [("step2/home", "POST", "step2/home", {"fan_roll": "2"}, {})]


def _inducements_attempts(mr_id):
    home_team = _home_team(mr_id)
    return [
        ("coups de pouce", "POST", f"inducements/{home_team}",
         {"intent": "validate", "selection": "[]", "mercenaries": "[]"}, {}),
        ("mercenaires", "GET", f"step2/{home_team}/mercenaires", None, HX),
    ]


def _match_attempts(mr_id):
    player = first_player_id(mr_id, "home")
    return [
        ("step3/actions", "POST", "step3/actions",
         {"turn": "2", "player_id": player, "player_type": "regular", "action_type": "TOUCHDOWN"},
         HX),
        ("suppression d'action", "DELETE", f"actions/{_home_action(mr_id)}", None, HX),
        ("mvp/home", "POST", "mvp/home", {"player_id": player, "player_type": "regular"}, HX),
        ("mvp/home/draw", "POST", "mvp/home/draw", _home_designated(mr_id), HX),
    ]


def _post_match_attempts():
    return [
        ("step5/home", "POST", "step5/home",
         {"gain": "50", "fan_mod": "0", "summary_paragraph": "Intrusion"}, {}),
        ("step5/home/validate", "POST", "step5/home/validate", None, {}),
        ("step5/home/withdraw-validation", "POST", "step5/home/withdraw-validation", None, {}),
    ]


def _recap_attempts():
    return [
        ("recap/withdraw-validation", "POST", "recap/withdraw-validation",
         {"team_side": "home"}, {}),
    ]


def _post_match(session, space_id, mr_id, side, gain):
    return session.post(
        _url(space_id, mr_id, f"step5/{side}"),
        data={"gain": str(gain), "fan_mod": "0", "summary_paragraph": f"Le camp {side}."},
        allow_redirects=False,
        timeout=30,
    )


def _ready_to_publish(space_id, mr_id, intruder) -> None:
    """Chacun son après-match : le rapport attend les validations."""
    assert _post_match(intruder, space_id, mr_id, "away", 40).status_code in (302, 303)
    assert _post_match(requests.Session(), space_id, mr_id, "home", 50).status_code in (302, 303)
    phase = entry_mode_of(mr_id)[0]
    assert phase == "ReadyToPublish", f"le rapport doit attendre ses validations : {phase}"


def _home_untouched(mr_id) -> None:
    """Rien d'E2E Coach 01 dans le camp de DevCoach : son action et son MVP,
    et aucune validation."""
    rows = query_db(
        f"SELECT count(*) FROM match_report_actions WHERE match_report_id = '{mr_id}' "
        "AND team_side = 'home' AND NOT is_deleted"
    )
    assert rows == ["2"], f"le domicile doit garder son action et son MVP : {rows}"
    assert _appended(mr_id, "TeamSideValidated") == 0


def test_e2_toute_ecriture_dans_le_camp_adverse_est_refusee(
    space_id, competition_refus, coach
):
    ctx = competition_refus
    mr_id = start_team_entry(space_id, ctx, ctx["round_ids"][0], *ctx["team_ids"][:2])
    intruder = http_session_of(coach.context)
    refused = _not_refused(intruder, space_id, mr_id, _pre_match_attempts())

    pages = roll_team_fan_factors(space_id, mr_id, away_via=intruder)
    refused += _not_refused(intruder, space_id, mr_id, _inducements_attempts(mr_id))
    pass_team_entry_inducements(pages)

    record_action_api(space_id, mr_id, "home", first_player_id(mr_id, "home"), 1, "TOUCHDOWN")
    refused += _not_refused(intruder, space_id, mr_id, _match_attempts(mr_id))
    ensure_mvps(space_id, mr_id, vias={"away": intruder})
    refused += _not_refused(intruder, space_id, mr_id, _post_match_attempts())

    _ready_to_publish(space_id, mr_id, intruder)
    refused += _not_refused(intruder, space_id, mr_id, _recap_attempts())

    assert refused == [], f"écritures du camp adverse non refusées en 403 : {refused}"
    _home_untouched(mr_id)


# ── E3 — un camp validé est gelé, jusqu'au retrait ────────────────────────────

def test_e3_un_camp_valide_est_gele_jusqu_au_retrait(space_id, competition_gel, coach):
    ctx = competition_gel
    mr_id = start_team_entry(space_id, ctx, ctx["round_ids"][0], *ctx["team_ids"][:2])
    session = http_session_of(coach.context)
    ensure_team_entry_inducements(space_id, mr_id, away_via=session)
    ensure_mvps(space_id, mr_id, vias={"away": session})
    assert _post_match(session, space_id, mr_id, "away", 40).status_code in (302, 303)

    coach.goto(_url(space_id, mr_id, "step5/away"), wait_until="load")
    with coach.expect_navigation(wait_until="load"):
        coach.get_by_role("button", name=VALIDATE).click()
    expect(coach.locator(".team-step5-validated")).to_contain_text("Validé ✓")

    gele = _post_match(session, space_id, mr_id, "away", 80)
    assert gele.status_code == 409, f"obtenu {gele.status_code}"

    with coach.expect_navigation(wait_until="load"):
        coach.get_by_role("button", name="Retirer ma validation").click()
    expect(coach.locator(".team-step5-validated")).to_have_count(0)

    degele = _post_match(session, space_id, mr_id, "away", 80)
    assert degele.status_code in (302, 303), f"obtenu {degele.status_code}"
    coach.reload(wait_until="load")
    expect(coach.locator("input[name=gain]")).to_have_value("80")


# ── E4 — le démarrage concurrent ──────────────────────────────────────────────

def _is_start_post(mr_id):
    return lambda r: r.request.method == "POST" and r.url.endswith(f"/match-report/{mr_id}")


def test_e4_deux_commencer_un_seul_mode_retenu(
    space_id, competition_concurrente, devcoach, coach
):
    ctx = competition_concurrente
    mr_id = draft_of_round(ctx["round_ids"][0])
    sides = _sides(mr_id, ctx)
    coach.goto(_url(space_id, mr_id), wait_until="load")
    expect(coach.get_by_role("button", name="Commencer")).to_be_visible()

    _start_from_the_card(devcoach, space_id, mr_id, sides["dev"])

    # Sa page est restée sur le brouillon, cochée sur « Je saisis tout ».
    with coach.expect_response(_is_start_post(mr_id)) as second:
        coach.get_by_role("button", name="Commencer").click()
    assert second.value.status == 409, f"obtenu {second.value.status}"
    assert second.value.headers.get("hx-refresh") == "true", second.value.headers

    _assert_started_by_devcoach(mr_id, ctx)
    assert _appended(mr_id, "EntryModeChosen") == 1, "un seul mode choisi"

    # L'état du match : son avant-match, en « Chacun son équipe ».
    coach.goto(_url(space_id, mr_id), wait_until="load")
    coach.wait_for_url(f"**/{mr_id}/step2/{sides['coach']}", timeout=10000)

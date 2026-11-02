"""Tests E2E — le parcours « Je saisis tout » de bout en bout (carte 639, épic E21).

Le « Terminé quand » de l'épic, rejoué en navigateur. Les cartes 631 à 638 ont
chacune testé leur morceau ; aucune ne voit le parcours d'un bout à l'autre :

    calendrier → « Qui saisit ? » → Commencer → avant-match → coups de pouce
    → match en direct → action du domicile → action du visiteur
    → MVP du domicile tiré au sort parmi six → MVP du visiteur à la main
    → gains et fans → validation → publication

Scénarios couverts — chacun construit sa propre compétition et son propre
rapport, aucun état n'est partagé entre eux :

- P1 (ordinateur) et P2 (viewport mobile) — le parcours complet, par les mêmes
  routes : DevCoach démarre depuis le calendrier et saisit tout ; l'autre
  coach suit dans un second contexte de navigateur — aucun bouton d'écriture,
  le journal à jour au rechargement, la publication refusée.
- A1 — Un compte rendu commencé avant E21 : un flux sans `EntryModeChosen`,
  deux MVP dans le même camp à des tours quelconques, écrits comme le faisait
  l'ancien code. Il se termine et se publie par les écrans d'aujourd'hui.

**Pourquoi une compétition par test.** Démarrer un match verrouille ses deux
équipes jusqu'à la publication. Un parcours qui échoue en route laisserait les
équipes prises, et le test suivant échouerait au « Commencer » sans rien dire
de ce qu'il teste. Deux équipes suffisent : leur unique appariement est le
match du calendrier.

Le membre simple (`X-Bypass-Auth-Profile: simple`) coache la **deuxième**
équipe que monte `build_full_competition` ; DevCoach, identité de la suite,
coache la première — l'autre coach du match est donc le membre simple.

**Les clics qui n'attendent pas de câblage htmx** : le choix de l'équipe est un
`<details>` natif, les puces de joueur et les boutons d'action sont câblés par
Alpine ou par le script du widget, les liens de navigation sont ordinaires —
`cliquer_quand_cable` n'aboutirait jamais sur eux. Seuls les deux formulaires
de l'étape MVP (`hx-post`) l'attendent.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

import random
import re

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import execute_db, query_db
from htmx_helpers import attendre_cablage
from match_report_helpers import (
    draft_of_round,
    ensure_inducements,
    ensure_pre_match,
    entry_mode_of,
    pass_inducements_in_browser,
    record_action_api,
)

ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}
ORDINATEUR = {"width": 1280, "height": 900}
MOBILE = {"width": 390, "height": 844}
TIRAGE = 6

_ULID = r"[0-9A-Z]{26}"

# Toutes les pages que le parcours a le droit de traverser. Aucune n'est propre
# au mobile : c'est la décision du 2026-11-01, « pas de routes propres au
# mobile ». Les identifiants sont remplacés par `{id}`.
ROUTES_DU_PARCOURS = {
    "/app/{id}/competitions/{id}/{id}/calendrier",
    "/app/{id}/match-report/pairing/{id}",
    "/app/{id}/match-report/{id}",
    "/app/{id}/match-report/{id}/step2",
    "/app/{id}/match-report/{id}/inducements/{id}",
    "/app/{id}/match-report/{id}/live",
    "/app/{id}/match-report/{id}/step3",
    "/app/{id}/match-report/{id}/step4",
    "/app/{id}/match-report/{id}/mvp/home",
    "/app/{id}/match-report/{id}/mvp/away",
    "/app/{id}/match-report/{id}/step5",
    "/app/{id}/match-report/{id}/recap",
}


def _url(space_id, mr_id, suffixe=""):
    base = f"{BASE_URL}/app/{space_id}/match-report/{mr_id}"
    return f"{base}/{suffixe}" if suffixe else base


def _route(url: str) -> str:
    """Le chemin d'une URL, sans requête ni fragment, chaque identifiant
    effacé — tout segment d'au moins vingt caractères alphanumériques, quelle
    que soit la forme d'identifiant de son BC."""
    chemin = re.sub(r"^https?://[^/]+", "", url).split("?")[0].split("#")[0]
    return "/".join(
        "{id}" if re.fullmatch(r"[0-9A-Za-z-]{20,}", segment) else segment
        for segment in chemin.split("/")
    )


@pytest.fixture
def match_du_calendrier(browser, space_id):
    """Une compétition à deux équipes, une journée : un seul appariement, donc
    un seul brouillon, celui que le calendrier ouvre."""
    full = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    return {**full, "space_id": space_id}


# ══ Les gestes du coach qui saisit ═══════════════════════════════════════════


def _demarrer_depuis_le_calendrier(page: Page, ctx: dict) -> str:
    """Depuis l'onglet Calendrier : la ligne du match, « Qui saisit ? » coché
    sur « Je saisis tout », sans sélection d'équipes, puis « Commencer »."""
    page.goto(
        f"{BASE_URL}/app/{ctx['space_id']}/competitions/{ctx['competition_id']}"
        f"/{ctx['season_id']}/calendrier",
        wait_until="load",
    )
    ligne = page.locator(".cal-row--clickable .cal-row-link")
    expect(ligne).to_have_count(1)
    with page.expect_navigation(wait_until="load"):
        ligne.click()

    page.wait_for_url(re.compile(rf".*/match-report/{_ULID}$"), timeout=10000)
    mr_id = page.url.rstrip("/").rsplit("/", 1)[-1]
    carte = page.locator(".entry-mode-card")
    expect(carte).to_contain_text("Qui saisit ce match ?")
    expect(carte.locator("input[name='entry_mode'][value='SoloEntry']")).to_be_checked()
    expect(page.locator("#teams-container")).to_have_count(0)

    with page.expect_navigation(wait_until="load"):
        page.get_by_role("button", name="Commencer").click()
    page.wait_for_url(re.compile(rf".*/{mr_id}/step2$"), timeout=10000)

    phase, mode, demarreur = entry_mode_of(mr_id)
    assert phase == "PreMatch", phase
    assert mode == "SoloEntry", mode
    assert demarreur, "le démarreur doit être enregistré"
    return mr_id


def _de_l_avant_match_au_direct(page: Page, mr_id: str) -> None:
    page.fill("input[name='home_fan_roll']", "2")
    page.fill("input[name='away_fan_roll']", "3")
    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")
    pass_inducements_in_browser(page)
    page.wait_for_url(re.compile(rf".*/{mr_id}/live$"), timeout=10000)
    expect(page.locator(".mr-step--active")).to_have_text("2")


def _saisir_un_touchdown(page: Page, mr_id: str, rang: int, tour: int) -> None:
    """« Nouvelle action » → l'équipe (`rang` 0 domicile, 1 visiteur) → un
    Touchdown au tour donné → « Retour au journal »."""
    page.locator(".match-report-live .live-new-action-toggle").click()
    equipes = page.locator(".match-report-live .live-team-pick")
    expect(equipes).to_have_count(2)
    with page.expect_navigation(wait_until="load"):
        equipes.nth(rang).click()
    etape, camp = ("step3", "home") if rang == 0 else ("step4", "away")
    assert page.url.endswith(f"/{mr_id}/{etape}"), page.url

    page.wait_for_selector("#turn-selector .mr-turn-btn", timeout=10000)
    page.wait_for_selector("#player-selector .mr-player-chip", timeout=10000)
    page.locator(f".mr-turn-btn[data-turn='{tour}']").first.click()
    page.locator("#player-selector .mr-player-chip").first.click()
    page.wait_for_selector("#action-panel .mr-action-grid", timeout=10000)
    page.locator(".mr-action-btn").filter(has_text="Touchdown").click()
    # Le journal montre les deux camps : c'est l'entrée de ce camp qu'on attend.
    expect(
        page.locator(f"#action-log .mr-log-entry--{camp}", has_text="Touchdown")
    ).to_have_count(1, timeout=10000)

    with page.expect_navigation(wait_until="load"):
        page.locator(".mr-nav-row a", has_text="Retour au journal").click()
    assert page.url.endswith(f"/{mr_id}/live"), page.url


def _designer(page: Page, puce, attendu: int) -> None:
    """Désigne un joueur pour le tirage et attend que le compteur l'ait vu.

    Même geste que `test_match_report_mvp_draw` : la puce n'est pas un élément
    htmx, son clic émet `playerSelected`. Un clic tombé avant que le script du
    widget ait câblé la puce est rejoué."""
    compteur = page.locator(".match-report-mvp .mvp-draw-count")
    for _ in range(3):
        puce.click()
        try:
            expect(compteur).to_have_text(f"{attendu} / {TIRAGE}", timeout=2000)
            return
        except AssertionError:
            continue
    raise AssertionError(f"le compteur n'atteint pas {attendu} / {TIRAGE}")


def _mvp_du_domicile_tire_au_sort(page: Page, mr_id: str) -> None:
    with page.expect_navigation(wait_until="load"):
        page.locator(".match-report-live .live-end-match").click()
    assert page.url.endswith(f"/{mr_id}/mvp/home"), page.url
    expect(page.locator(".mr-step--active")).to_have_text("3")
    expect(page.locator(".mr-banner-label")).to_contain_text("MVP 1 / 2")
    expect(page.locator(".match-report-mvp .mvp-mode-draw")).to_have_class(re.compile(r"\bselected\b"))

    puces = page.locator(".match-report-mvp .mvp-players .mr-player-chip")
    puces.first.wait_for(state="visible", timeout=10000)
    assert puces.count() >= TIRAGE, f"au moins {TIRAGE} joueurs attendus : {puces.count()}"
    for i in range(TIRAGE):
        _designer(page, puces.nth(i), i + 1)

    attendre_cablage(page, ".match-report-mvp form.mvp-draw-form")
    page.locator(".match-report-mvp form.mvp-draw-form button[type=submit]").click()
    expect(page.locator(".match-report-mvp .mvp-drawn-among")).to_contain_text(
        "Tiré au sort parmi", timeout=10000
    )
    expect(page.locator(".match-report-mvp .mvp-row-delete")).to_have_count(0)

    with page.expect_navigation(wait_until="load"):
        page.locator(".match-report-mvp .mr-nav-row a.btn-primary-sm").click()
    assert page.url.endswith(f"/{mr_id}/mvp/away"), page.url


def _mvp_du_visiteur_a_la_main(page: Page, mr_id: str) -> None:
    expect(page.locator(".mr-banner-label")).to_contain_text("MVP 2 / 2")
    onglet = page.locator(".match-report-mvp .mvp-mode-manual")
    onglet.click()
    expect(onglet).to_have_class(re.compile(r"\bselected\b"))

    puce = page.locator(".match-report-mvp .mr-player-chip").first
    puce.wait_for(state="visible", timeout=10000)
    puce.click()
    suite = page.locator(".match-report-mvp form.mvp-manual-form button[type=submit]")
    expect(suite).to_be_enabled()
    attendre_cablage(page, ".match-report-mvp form.mvp-manual-form")
    suite.click()
    page.wait_for_url(re.compile(rf".*/{mr_id}/step5$"), timeout=10000)


def _gains_et_validation(page: Page, mr_id: str) -> None:
    expect(page.locator(".mr-step--active")).to_have_text("4")
    page.fill("input[name='home_gain']", "50")
    page.fill("input[name='away_gain']", "40")
    with page.expect_navigation(wait_until="load"):
        page.locator("form button[type='submit']", has_text="Enregistrer").click()
    page.wait_for_url(re.compile(rf".*/{mr_id}/recap$"), timeout=10000)
    expect(page.locator(".mr-step--active")).to_have_text("5")


def _publier(page: Page, mr_id: str) -> None:
    with page.expect_navigation(wait_until="load"):
        page.locator(".ms-btn-primary", has_text="Publier").click()
    assert page.url.endswith(f"/{mr_id}/recap"), page.url
    expect(page.locator(".ms-btn-outline", has_text="Retour aux résultats")).to_be_visible()
    assert entry_mode_of(mr_id)[0] == "Published"


# ══ Ce que voit l'autre coach ════════════════════════════════════════════════


def _l_autre_coach_suit(vue: Page, space_id: str, mr_id: str, entrees: int) -> None:
    """Le match en direct, sans aucun bouton d'écriture ; `entrees` actions au
    journal, chacune « saisie par DevCoach »."""
    expect(vue.locator(".match-report-live .live-notice")).to_contain_text(
        "DevCoach saisit tout le match"
    )
    journal = vue.locator(".match-report-live .mr-log-entry")
    expect(journal).to_have_count(entrees, timeout=10000)
    for i in range(entrees):
        expect(journal.nth(i)).to_contain_text("saisi par DevCoach")
    expect(vue.locator(".match-report-live .mr-log-delete")).to_have_count(0)
    expect(vue.locator(".match-report-live .live-new-action")).to_have_count(0)
    expect(vue.locator(".match-report-live .live-end-match")).to_have_count(0)


def _l_autre_coach_ne_publie_pas(vue: Page, space_id: str, mr_id: str) -> None:
    """Le récapitulatif lui est lisible, sans bouton de publication ; le POST
    de publication lui est refusé, et le rapport n'a pas bougé."""
    vue.goto(_url(space_id, mr_id, "recap"), wait_until="load")
    expect(vue.locator(".ms-hero")).to_be_visible()
    expect(vue.locator(".ms-btn-primary", has_text="Publier")).to_have_count(0)

    refus = vue.request.post(
        _url(space_id, mr_id, "recap/publish"),
        headers={**ENTETE_MEMBRE_SIMPLE, "HX-Request": "true"},
        max_redirects=0,
    )
    assert refus.status == 403, f"publication par l'autre coach : {refus.status}"
    assert entry_mode_of(mr_id)[0] == "ReadyToPublish"


# ══ P1, P2 — le parcours complet ═════════════════════════════════════════════


def _mvps(mr_id: str) -> list[str]:
    """Les MVP vivants du rapport : « camp:tour:draw_size », triés."""
    return sorted(query_db(
        "SELECT team_side || ':' || turn_number || ':' || COALESCE(draw_size::text, '') "
        f"FROM match_report_actions WHERE match_report_id = '{mr_id}' AND NOT is_deleted "
        "AND action_json = '\"Mvp\"'::jsonb"
    ))


@pytest.mark.parametrize("taille", ["ordinateur", "mobile"])
def test_le_parcours_je_saisis_tout(browser, match_du_calendrier, taille):
    viewport = ORDINATEUR if taille == "ordinateur" else MOBILE
    space_id = match_du_calendrier["space_id"]
    saisie = browser.new_context(viewport=viewport)
    suivi = browser.new_context(viewport=viewport, extra_http_headers=ENTETE_MEMBRE_SIMPLE)
    try:
        page = saisie.new_page()
        visitees: list[str] = []
        page.on(
            "framenavigated",
            lambda f: f.parent_frame is None and f.url.startswith(BASE_URL) and visitees.append(f.url),
        )
        vue = suivi.new_page()

        mr_id = _demarrer_depuis_le_calendrier(page, match_du_calendrier)
        _de_l_avant_match_au_direct(page, mr_id)

        _saisir_un_touchdown(page, mr_id, rang=0, tour=4)
        vue.goto(_url(space_id, mr_id, "live"), wait_until="load")
        _l_autre_coach_suit(vue, space_id, mr_id, entrees=1)

        _saisir_un_touchdown(page, mr_id, rang=1, tour=5)
        # Le journal commun : une action par camp, saisies par le même coach.
        scores = page.locator(".match-report-live .live-score-value")
        expect(scores.nth(0)).to_have_text("1")
        expect(scores.nth(1)).to_have_text("1")
        for camp in ("home", "away"):
            entree = page.locator(f".match-report-live .mr-log-entry--{camp}")
            expect(entree).to_have_count(1, timeout=10000)
            expect(entree).to_contain_text("saisi par vous")
        # Sans mise à jour en direct (E23), l'autre coach voit l'action du
        # visiteur au rechargement.
        vue.reload(wait_until="load")
        _l_autre_coach_suit(vue, space_id, mr_id, entrees=2)

        _mvp_du_domicile_tire_au_sort(page, mr_id)
        _mvp_du_visiteur_a_la_main(page, mr_id)
        assert _mvps(mr_id) == ["away:16:", f"home:16:{TIRAGE}"], _mvps(mr_id)

        _gains_et_validation(page, mr_id)
        _l_autre_coach_ne_publie_pas(vue, space_id, mr_id)
        _publier(page, mr_id)

        hors_parcours = sorted({_route(u) for u in visitees} - ROUTES_DU_PARCOURS)
        assert not hors_parcours, f"routes hors du parcours commun : {hors_parcours}"
    finally:
        suivi.close()
        saisie.close()


# ══ A1 — un compte rendu commencé avant E21 ══════════════════════════════════

_CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"


def _nouvel_action_id() -> str:
    """Un identifiant à la forme d'un ULID, comme ceux que l'ancien code
    engendrait pour chaque action."""
    return "0" + "".join(random.choice(_CROCKFORD) for _ in range(25))


def _retirer_le_mode_de_saisie(mr_id: str) -> None:
    """Rend au flux la forme qu'il avait avant la carte 631 : pas
    d'`EntryModeChosen`. Les événements suivants descendent d'un rang, pour que
    le flux reste continu — c'est son nombre d'événements que le dépôt prend
    pour version attendue. Le passage par des versions négatives évite de
    heurter l'index unique en cours de renumérotation.

    La projection n'a rien à défaire : la migration de la carte 631 donne à un
    rapport d'avant « Je saisis tout » et, pour démarreur, le `confirmed_by` de
    `SelectionConfirmed` — ce que le démarrage d'aujourd'hui y a écrit."""
    execute_db(
        "WITH retire AS ("
        f"  DELETE FROM match_report_event_store WHERE match_report_id = '{mr_id}' "
        "   AND event_type = 'EntryModeChosen' RETURNING version) "
        "UPDATE match_report_event_store SET version = -version "
        f"WHERE match_report_id = '{mr_id}' AND version > (SELECT version FROM retire); "
        "UPDATE match_report_event_store SET version = -version - 1 "
        f"WHERE match_report_id = '{mr_id}' AND version < 0; "
        "UPDATE match_report_proj SET version = (SELECT max(version) "
        f"FROM match_report_event_store WHERE match_report_id = '{mr_id}') "
        f"WHERE match_report_id = '{mr_id}';"
    )


def _mvp_a_l_ancienne(mr_id: str, action_source: str) -> None:
    """Un MVP tel que l'ancien code l'écrivait : une action `Mvp` comme une
    autre, au tour de l'action dont il est la copie, appendée au flux et
    projetée dans la même écriture."""
    nouvel_id = _nouvel_action_id()
    execute_db(
        "INSERT INTO match_report_event_store "
        "(match_report_id, event_type, event_version, payload, version) "
        "SELECT match_report_id, event_type, event_version, "
        f"jsonb_set(jsonb_set(payload, '{{action_id}}', to_jsonb('{nouvel_id}'::text)), "
        "'{action}', '\"Mvp\"'::jsonb), "
        f"(SELECT max(version) + 1 FROM match_report_event_store WHERE match_report_id = '{mr_id}') "
        f"FROM match_report_event_store WHERE match_report_id = '{mr_id}' "
        f"AND event_type = 'ActionRecorded' AND payload->>'action_id' = '{action_source}'; "
        "INSERT INTO match_report_actions (action_id, match_report_id, team_side, turn_number, "
        "player_id, player_type, action_json, player_display_name, player_position, recorded_by) "
        f"SELECT '{nouvel_id}', match_report_id, team_side, turn_number, player_id, player_type, "
        "'\"Mvp\"'::jsonb, player_display_name, player_position, recorded_by "
        f"FROM match_report_actions WHERE action_id = '{action_source}'; "
        "UPDATE match_report_proj SET version = (SELECT max(version) "
        f"FROM match_report_event_store WHERE match_report_id = '{mr_id}') "
        f"WHERE match_report_id = '{mr_id}';"
    )


def _deux_joueurs(mr_id: str, side: str) -> list[str]:
    equipe = query_db(
        f"SELECT {side}_team_id FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )[0]
    joueurs = query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{equipe}' "
        "AND membership = 'Active' ORDER BY player_id LIMIT 2"
    )
    assert len(joueurs) == 2, f"deux joueurs attendus dans l'équipe {equipe} : {joueurs}"
    return joueurs


def _action_de(mr_id: str, player_id: str) -> str:
    return query_db(
        "SELECT action_id FROM match_report_actions "
        f"WHERE match_report_id = '{mr_id}' AND player_id = '{player_id}' AND NOT is_deleted "
        "ORDER BY turn_number LIMIT 1"
    )[0]


def _rapport_d_avant_e21(ctx: dict) -> str:
    """Démarré, ses actions saisies, puis réécrit à la forme d'avant : aucun
    `EntryModeChosen`, deux MVP au domicile et un au visiteur, aux tours 1, 2
    et 3 — l'ancien code ne connaissait ni l'étape MVP ni le tour 16."""
    space_id, round_id = ctx["space_id"], ctx["round_ids"][0]
    mr_id = draft_of_round(round_id)
    home, away = query_db(
        "SELECT home_team_id || '|' || away_team_id FROM match_report_proj "
        f"WHERE match_report_id = '{mr_id}'"
    )[0].split("|")
    ensure_pre_match(space_id, mr_id, ctx, round_id, home, away)
    _retirer_le_mode_de_saisie(mr_id)
    ensure_inducements(space_id, mr_id)

    buteur, sorteur = _deux_joueurs(mr_id, "home")
    adverse = _deux_joueurs(mr_id, "away")[0]
    record_action_api(space_id, mr_id, "home", buteur, turn=1, action_type="TOUCHDOWN")
    record_action_api(space_id, mr_id, "home", sorteur, turn=2, action_type="SORTIE")
    record_action_api(space_id, mr_id, "away", adverse, turn=3, action_type="TOUCHDOWN")
    for joueur in (buteur, sorteur, adverse):
        _mvp_a_l_ancienne(mr_id, _action_de(mr_id, joueur))
    return mr_id


def test_un_rapport_d_avant_e21_se_termine_et_se_publie(browser, match_du_calendrier):
    ctx = match_du_calendrier
    space_id = ctx["space_id"]
    mr_id = _rapport_d_avant_e21(ctx)

    # Le décor est bien celui d'avant : aucun mode au flux, et la projection
    # donne ce que la migration de la carte 631 donne à l'existant.
    assert not query_db(
        "SELECT 1 FROM match_report_event_store "
        f"WHERE match_report_id = '{mr_id}' AND event_type = 'EntryModeChosen'"
    )
    confirme_par = query_db(
        "SELECT payload->>'confirmed_by' FROM match_report_event_store "
        f"WHERE match_report_id = '{mr_id}' AND event_type = 'SelectionConfirmed'"
    )
    assert entry_mode_of(mr_id) == ("PreMatch", "SoloEntry", confirme_par[0])
    assert _mvps(mr_id) == ["away:3:", "home:1:", "home:2:"], _mvps(mr_id)

    contexte = browser.new_context(viewport=ORDINATEUR)
    try:
        page = contexte.new_page()
        page.goto(_url(space_id, mr_id, "live"), wait_until="load")
        # Les trois MVP d'avant sont au journal, comme des actions.
        expect(
            page.locator(".match-report-live .mr-log-entry", has_text="MVP")
        ).to_have_count(3, timeout=10000)

        # Le domicile a ses deux MVP : l'étape les montre, sans rien redemander.
        with page.expect_navigation(wait_until="load"):
            page.locator(".match-report-live .live-end-match").click()
        assert page.url.endswith(f"/{mr_id}/mvp/home"), page.url
        expect(page.locator(".match-report-mvp .mvp-row")).to_have_count(2)
        expect(page.locator(".match-report-mvp .mr-player-chip")).to_have_count(0)
        with page.expect_navigation(wait_until="load"):
            page.locator(".match-report-mvp .mr-nav-row a.btn-primary-sm").click()

        assert page.url.endswith(f"/{mr_id}/mvp/away"), page.url
        expect(page.locator(".match-report-mvp .mvp-row")).to_have_count(1)
        with page.expect_navigation(wait_until="load"):
            page.locator(".match-report-mvp .mr-nav-row a.btn-primary-sm").click()
        page.wait_for_url(re.compile(rf".*/{mr_id}/step5$"), timeout=10000)

        _gains_et_validation(page, mr_id)
        _publier(page, mr_id)
    finally:
        contexte.close()

    # Rien n'a été réécrit : les trois MVP d'avant sont publiés tels quels.
    assert _mvps(mr_id) == ["away:3:", "home:1:", "home:2:"], _mvps(mr_id)

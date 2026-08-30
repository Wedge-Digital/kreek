"""Tests E2E — l'onglet Matchs d'une équipe (carte 478).

Ce que ces scénarios prouvent, et qu'aucun test unitaire ne voit : le rendu du
composant partagé sur sa nouvelle page, l'ordre chronologique centré sur
maintenant, et le lien de rapport présent ou absent **selon qui regarde**.

# Le montage, et pourquoi ces identités-là

`build_full_competition` distribue les équipes aux coachs de l'espace triés par
nom : `DevCoach` d'abord — administrateur — puis `E2E Coach 01`, `02`, `03`.
Or `E2E Coach 01` est justement le coach que `bypass_auth` connecte sur
`X-Bypass-Auth-Profile: simple`, et il est `SpaceUser`, pas administrateur.

C'est ce qui rend les deux moitiés du contrôle d'accès observables sur la même
base :

- sur la fiche de **son** équipe, il voit les liens de rapport ;
- sur celle d'une équipe de l'autre appariement — qu'il ne coache pas et contre
  laquelle il n'a pas joué — il ne les voit pas, là où l'administrateur les voit.

La contre-épreuve n'est pas une politesse : sans elle, le test passerait aussi
bien si le lien avait disparu pour tout le monde.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) et base seedée.
"""

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import (
    BASE_URL,
    build_and_submit_team_http,
    build_full_competition,
)
from db_helpers import query_db
from htmx_helpers import cliquer_quand_cable
from match_report_helpers import play_match

ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}
COACH_SIMPLE = "E2E Coach 01"


# ── Montage ───────────────────────────────────────────────────────────────────


def _coach_de(team_id: str) -> str:
    """Le nom du coach, **résolu par jointure sur `coach_id`**.

    `team_proj.coach_name` est vide pour les équipes construites en HTTP : le
    constructeur poste un nom vide et laisse l'agrégat résoudre par
    l'identifiant. S'y fier ici rendait une liste vide, et le montage échouait
    sur un `IndexError` qui ne disait rien de sa cause.
    """
    lignes = query_db(
        "SELECT u.coach_name FROM team_proj p JOIN auth__users u ON u.id = p.coach_id "
        f"WHERE p.team_id = '{team_id}'"
    )
    return lignes[0] if lignes else ""


def _paires(round_id: str) -> list[tuple[str, str]]:
    lignes = query_db(
        "SELECT home_team_id || '|' || away_team_id FROM competition_match_day_pairings "
        f"WHERE match_day_id = '{round_id}' ORDER BY id"
    )
    return [tuple(l.split("|")) for l in lignes]


@pytest.fixture(scope="module")
def matchs_ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=4, num_rounds=2)
    # **On joue la SECONDE journée et on laisse la première à venir.**
    #
    # L'inverse serait plus naturel et ne prouverait rien : le match à venir
    # aurait alors la position la plus haute, et un `round_position DESC` nu —
    # le tri de l'onglet compétition, celui que cette carte refuse — produirait
    # exactement l'ordre attendu. Vérifié : le test passait sans le `CASE` sur
    # le statut.
    round_1, round_2 = full["round_ids"][0], full["round_ids"][1]
    paires = _paires(round_2)
    assert len(paires) == 2, f"deux appariements attendus, {len(paires)} obtenus"

    # L'équipe du coach simple, et l'appariement qui ne la concerne pas.
    equipes_simple = [t for t in full["team_ids"] if _coach_de(t) == COACH_SIMPLE]
    assert equipes_simple, (
        f"aucune équipe de « {COACH_SIMPLE} » : le contrôle d'accès ne serait pas "
        "observable, et les deux tests passeraient sans rien prouver"
    )
    sienne = equipes_simple[0]
    paire_sienne = next(p for p in paires if sienne in p)
    paire_etrangere = next(p for p in paires if sienne not in p)

    # **Les deux appariements sont joués.** Sans le second, le test du visiteur
    # passerait faute de lien à cacher, et non faute de droit.
    for home, away in (paire_sienne, paire_etrangere):
        play_match(space_id, full, round_2, home, away, home_gain=50, away_gain=40)

    # Une équipe inscrite **après** la génération du calendrier : elle n'a aucun
    # appariement, ce qui est l'un des deux cas de l'état vide.
    coachs = [l.split("|")[0] for l in query_db(
        "SELECT DISTINCT u.id, u.coach_name FROM spaces__user_space us "
        "JOIN auth__users u ON u.id = us.coach_id "
        f"WHERE us.space_id = '{space_id}' ORDER BY u.coach_name"
    )]
    page = browser.new_page()
    try:
        sans_match = build_and_submit_team_http(
            page, space_id, full["competition_id"], full["season_id"], coachs[4], 0
        )
    finally:
        page.close()

    return {
        "space_id": space_id,
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "sienne": sienne,
        "etrangere": paire_etrangere[0],
        "sans_match": sans_match,
        "round_a_venir": round_1,
    }


def _url(ctx: dict, team_id: str) -> str:
    return f"{BASE_URL}/app/{ctx['space_id']}/teams/{team_id}"


# ── Scénarios ─────────────────────────────────────────────────────────────────


def test_l_onglet_matchs_liste_les_matchs_de_l_equipe(page: Page, matchs_ctx):
    """Le chemin heureux, en partant du clic — pas de l'URL.

    L'onglet arrive par un échange htmx : il est peint avant d'être câblé, et
    un clic tombé dans cette fenêtre ne produirait rien.
    """
    page.goto(_url(matchs_ctx, matchs_ctx["sienne"]), wait_until="load")
    cliquer_quand_cable(page, ".team-tabs a:has-text('Matchs')")

    expect(page.locator(".team-matches")).to_be_visible(timeout=10000)
    expect(page.locator(".team-tabs .tab.active")).to_have_text("Matchs")
    # Un match joué et un à venir : le compte dit que les trois statuts sont
    # servis, et non le seul que l'onglet compétition montrerait.
    expect(page.locator(".match-widget")).to_have_count(2)
    expect(page.locator(".match-score-tds")).to_have_count(1)
    expect(page.locator(".match-outcome")).to_have_count(1)
    assert page.url.endswith("/matchs"), f"hx-push-url n'a pas suivi : {page.url}"


def test_le_prochain_match_apparait_en_tete(page: Page, matchs_ctx):
    """**L'ordre n'est pas celui de la compétition.**

    Celle-ci trie par journée décroissante ; le reprendre en incluant les matchs
    à venir mettrait le plus lointain en tête et enterrerait le prochain. Sur une
    fiche d'équipe, « mon prochain match » est ce qu'un coach vient chercher.
    """
    page.goto(_url(matchs_ctx, matchs_ctx["sienne"]) + "/matchs", wait_until="load")
    expect(page.locator(".team-matches")).to_be_visible(timeout=10000)

    blocs = page.locator(".match-widget")
    # Le premier est à venir : ni score, ni pastille. Le second est joué.
    expect(blocs.nth(0).locator(".match-score-tds")).to_have_count(0)
    expect(blocs.nth(0).locator(".match-outcome")).to_have_count(0)
    expect(blocs.nth(1).locator(".match-score-tds")).to_have_count(1)
    expect(blocs.nth(1).locator(".match-outcome")).to_have_count(1)


def test_le_coach_de_l_equipe_peut_ouvrir_le_rapport(browser, matchs_ctx):
    """Le contrôle d'accès, première moitié : sur la fiche de son équipe, le
    coach atteint le rapport du match qu'il a joué."""
    contexte = browser.new_context(extra_http_headers=ENTETE_MEMBRE_SIMPLE)
    try:
        vue = contexte.new_page()
        vue.goto(_url(matchs_ctx, matchs_ctx["sienne"]) + "/matchs", wait_until="load")
        expect(vue.locator(".team-matches")).to_be_visible(timeout=10000)

        expect(vue.locator(".match-widget-link")).to_have_count(1)
        expect(vue.locator(".match-widget--clickable")).to_have_count(1)
    finally:
        contexte.close()


def test_un_visiteur_ne_voit_pas_le_lien_du_rapport(browser, matchs_ctx):
    """Son autre moitié, avec la contre-épreuve qui la rend concluante.

    Sur la fiche d'une équipe qu'il ne coache pas et contre laquelle il n'a pas
    joué, le membre simple ne voit aucun lien. Sans la vue administrateur, le
    test passerait aussi bien si le lien avait disparu pour tout le monde.
    """
    url = _url(matchs_ctx, matchs_ctx["etrangere"]) + "/matchs"

    contexte = browser.new_context(extra_http_headers=ENTETE_MEMBRE_SIMPLE)
    try:
        vue_tiers = contexte.new_page()
        vue_tiers.goto(url, wait_until="load")
        expect(vue_tiers.locator(".team-matches")).to_be_visible(timeout=10000)
        expect(vue_tiers.locator(".match-widget")).not_to_have_count(0)
        expect(vue_tiers.locator(".match-widget-link")).to_have_count(0)
    finally:
        contexte.close()

    vue_admin = browser.new_page()
    try:
        vue_admin.goto(url, wait_until="load")
        expect(vue_admin.locator(".team-matches")).to_be_visible(timeout=10000)
        expect(vue_admin.locator(".match-widget-link")).not_to_have_count(0)
    finally:
        vue_admin.close()


def test_une_equipe_sans_match_affiche_l_etat_vide(page: Page, matchs_ctx):
    """Une équipe inscrite après la génération du calendrier n'a aucun match —
    et c'est vrai, donc une liste vide et non une erreur."""
    page.goto(_url(matchs_ctx, matchs_ctx["sans_match"]) + "/matchs", wait_until="load")

    expect(page.locator(".tr-empty-title, .tab-empty-state")).to_be_visible(timeout=10000)
    expect(page.get_by_text("Aucun match pour le moment.")).to_be_visible()
    expect(page.locator(".match-widget")).to_have_count(0)


def test_le_bloc_de_match_reste_correct_sur_la_page_competition(page: Page, matchs_ctx):
    """**La non-régression, et c'est ce test qui porte le poids de la carte 476.**

    Elle a sorti 86 règles de `pages/competition-detail.css` vers un composant :
    c'est le seul geste du chantier qui touche une page qui marchait déjà. Un
    composant extrait qui casse sa page d'origine est le mode de panne classique
    de cette manœuvre, et il ne se voit qu'à l'écran.
    """
    page.goto(
        f"{BASE_URL}/app/{matchs_ctx['space_id']}/competitions/"
        f"{matchs_ctx['competition_id']}/{matchs_ctx['season_id']}/resultats",
        wait_until="load",
    )
    expect(page.locator(".match-widget").first).to_be_visible(timeout=10000)

    # Le groupement par journée, que la fiche d'équipe n'a pas.
    expect(page.locator(".matches-list-header").first).to_be_visible()
    # Le bloc, complet : logos ou initiales, noms, score, blessures, lien.
    bloc = page.locator(".match-widget").first
    expect(bloc.locator(".match-team-name")).to_have_count(2)
    expect(bloc.locator(".match-score-num")).to_have_count(2)
    expect(bloc.locator(".match-cas-num")).to_have_count(2)
    expect(bloc.locator(".match-widget-link")).to_have_count(1)
    # Et **rien** de ce que la fiche d'équipe ajoute : la page de compétition
    # n'a pas d'équipe de référence, et son en-tête de groupe dit déjà la journée.
    expect(page.locator(".match-outcome")).to_have_count(0)
    expect(page.locator(".match-round")).to_have_count(0)

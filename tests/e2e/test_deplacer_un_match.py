"""Tests E2E — déplacer un match sur une autre journée (carte 557).

Scénarios :
- un administrateur déplace un match publié depuis la barre d'administration
  du récapitulatif : toast, puis le cartouche dit la nouvelle journée, et les
  sept écritures suivent en base — appariement, ligne d'affichage, projection
  du rapport, lignes de classement, événements des joueurs ;
- le déplacement vers une journée où l'une des deux équipes joue déjà est
  refusé, en nommant le match qui bloque ;
- un coach voit « Corriger le rapport » mais pas le déplacement, et le widget
  lui est refusé.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import build_full_competition, liberer_les_equipes
from db_helpers import attendre_que, query_db
from htmx_helpers import attendre_cablage
from match_report_helpers import play_match

BASE_URL = "http://localhost:3210"
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}
COACH_SIMPLE = "E2E Coach 01"


# ── Fixtures ──────────────────────────────────────────────────────────────────

@pytest.fixture(scope="module")
def ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=8)
    return {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_ids": full["round_ids"],
        "teams": full["team_ids"],
    }


def _publier_un_match(space_id, ctx, round_id, home, away) -> dict:
    mr_id = play_match(space_id, ctx, round_id, home, away, home_td=2, away_td=1)
    pairing_id = query_db(
        f"SELECT pairing_id FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )[0]
    assert pairing_id, "un rapport né d'un appariement le porte (carte 552)"
    return {"mr_id": mr_id, "pairing_id": pairing_id, "home": home, "away": away}


@pytest.fixture(scope="module")
def match_publie(space_id, ctx):
    """Paire teams[0..1], publiée sur la première journée."""
    return _publier_un_match(space_id, ctx, ctx["round_ids"][0], ctx["teams"][0], ctx["teams"][1])


def _face_a_face(round_id: str, home: str, away: str) -> bool:
    return bool(query_db(
        "SELECT id FROM competition_match_day_pairings "
        f"WHERE match_day_id = '{round_id}' "
        f"AND ((home_team_id = '{home}' AND away_team_id = '{away}') "
        f"  OR (home_team_id = '{away}' AND away_team_id = '{home}'))"
    ))


def _journee_sans_le_couple(ctx, home: str, away: str, exclues: set[str]) -> str:
    """Une journée où les deux équipes ne sont **pas** déjà face à face.

    **Carte 580.** Le tirage ne les fait se rencontrer qu'une fois, sur une
    journée au hasard. Si c'est la cible, `liberer_les_equipes` y laisse leur
    rencontre — elle croit que c'est le match qu'on veut ouvrir —, la journée
    n'est plus une cible possible, et les deux équipes sont prises partout
    ailleurs : le sélecteur reste vide. Une fois sur sept, avec `round_ids[1]`
    en dur.
    """
    for round_id in ctx["round_ids"]:
        if round_id not in exclues and not _face_a_face(round_id, home, away):
            return round_id
    raise AssertionError("aucune journée sans le couple — le tirage a changé de forme")


def _journee_ou_le_couple_est_engage(ctx, home: str, away: str, exclues: set[str]) -> str:
    """Une journée où l'une des deux équipes a déjà un match — la cible d'un
    refus."""
    for round_id in ctx["round_ids"]:
        if round_id in exclues:
            continue
        engages = query_db(
            "SELECT count(*) FROM competition_match_day_pairings "
            f"WHERE match_day_id = '{round_id}' "
            f"AND (home_team_id IN ('{home}', '{away}') OR away_team_id IN ('{home}', '{away}'))"
        )
        if int(engages[0]) > 0:
            return round_id
    raise AssertionError("aucune journée où le couple est engagé")


def _nom_de_la_journee(round_id: str) -> str:
    return query_db(f"SELECT name FROM competition_match_days WHERE id = '{round_id}'")[0]


def _url_recap(space_id, mr_id) -> str:
    return f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap"


def _url_move(space_id, ctx) -> str:
    return (
        f"{BASE_URL}/app/{space_id}/competitions/{ctx['competition_id']}"
        f"/{ctx['season_id']}/admin/schedule/move-match"
    )


# ── Le déplacement, par l'écran ───────────────────────────────────────────────

def _attendre_le_widget(page: Page) -> None:
    """Le widget est inséré par htmx puis pris en main par Alpine, et le
    bouton qui déplie le panneau est un `@click` Alpine sans attribut `hx-*` :
    `cliquer_quand_cable` ne le verrait jamais câblé, htmx ne le câble pas.

    Ce qui rend le widget opérant, c'est le câblage du formulaire — lui porte
    le `hx-post` — et l'initialisation d'Alpine sur la racine, sans laquelle
    le `@click` est inerte. On attend les deux, dans cet ordre."""
    attendre_cablage(page, ".move-pairing-panel")
    racine = page.locator(".move-pairing").first.element_handle(timeout=10_000)
    page.wait_for_function("e => !!e._x_dataStack", arg=racine, timeout=10_000)


def test_un_administrateur_deplace_un_match_publie(page: Page, space_id, ctx, match_publie):
    cible = _journee_sans_le_couple(
        ctx, match_publie["home"], match_publie["away"], exclues={ctx["round_ids"][0]}
    )
    # Le tirage a déjà donné un adversaire à chaque équipe sur chaque journée :
    # on libère les deux nôtres sur la cible, sinon le refus de la carte 551
    # est la seule réponse possible.
    liberer_les_equipes(
        space_id, ctx["competition_id"], ctx["season_id"], cible,
        match_publie["home"], match_publie["away"],
    )
    nom_cible = _nom_de_la_journee(cible)

    page.goto(_url_recap(space_id, match_publie["mr_id"]))
    expect(page.locator(".ms-admin-bar")).to_be_visible()

    _attendre_le_widget(page)
    page.locator(".move-pairing-btn").click()
    select = page.locator("kreek-select[name='round_id']")
    select.locator(".ks-control").click()
    select.locator(".ks-option:not(.ks-empty)").first.wait_for(timeout=5000)
    select.locator(".ks-option", has_text=nom_cible).first.click()
    expect(page.locator(".move-pairing-submit")).to_be_enabled()
    page.locator(".move-pairing-submit").click()

    toast = page.locator(".toast--success").first
    toast.wait_for(state="visible", timeout=10_000)
    assert f"Match déplacé en {nom_cible}" in toast.inner_text()

    # `competitions` a écrit avant de répondre ; les trois autres BCs suivent
    # par app events, en quelques millisecondes — d'où les attentes.
    pairing = match_publie["pairing_id"]
    mr_id = match_publie["mr_id"]
    assert query_db(
        f"SELECT match_day_id FROM competition_match_day_pairings WHERE id = '{pairing}'"
    ) == [cible]
    assert query_db(
        f"SELECT round_id || '|' || round_name FROM competition_match_display_proj "
        f"WHERE pairing_id = '{pairing}'"
    ) == [f"{cible}|{nom_cible}"]
    attendre_que(
        lambda: query_db(f"SELECT round_id FROM match_report_proj WHERE match_report_id = '{mr_id}'") == [cible],
        quoi="la journée du rapport",
    )
    attendre_que(
        lambda: query_db(
            f"SELECT DISTINCT round_id FROM ranking_lines WHERE match_report_id = '{mr_id}'"
        ) == [cible],
        quoi="la journée des lignes de classement",
    )
    attendre_que(
        lambda: int(query_db(
            "SELECT count(DISTINCT team_id) FROM players_events "
            f"WHERE event_type = 'MatchRelocated' "
            f"AND payload -> 'MatchRelocated' ->> 'match_report_id' = '{mr_id}'"
        )[0]) == 2,
        quoi="la relocalisation des joueurs des deux équipes",
    )

    page.reload()
    expect(page.locator(".ms-hero-context")).to_contain_text(nom_cible)


# ── Le refus ──────────────────────────────────────────────────────────────────

def test_une_journee_ou_une_equipe_joue_deja_est_refusee(space_id, ctx, match_publie):
    """Sur une journée où le tirage a laissé aux deux équipes un adversaire, le
    déplacement est refusé, et le refus nomme le match.

    La journée d'origine et la journée actuelle du match sont exclues : le
    déplacement du test précédent y a libéré nos équipes, et la cible y serait
    soit vide, soit le match lui-même (carte 580)."""
    actuelle = query_db(
        f"SELECT match_day_id FROM competition_match_day_pairings WHERE id = '{match_publie['pairing_id']}'"
    )[0]
    cible = _journee_ou_le_couple_est_engage(
        ctx, match_publie["home"], match_publie["away"],
        exclues={ctx["round_ids"][0], actuelle},
    )

    resp = requests.post(
        _url_move(space_id, ctx),
        data={"pairing_id": match_publie["pairing_id"], "round_id": cible},
        headers={"HX-Request": "true"},
    )

    assert resp.status_code == 422, f"{resp.status_code}\n{resp.text[:300]}"
    assert "affronte déjà" in resp.json()["error"]
    assert query_db(
        f"SELECT match_day_id FROM competition_match_day_pairings WHERE id = '{match_publie['pairing_id']}'"
    ) != [cible], "un refus n'écrit rien"


# ── Le droit ──────────────────────────────────────────────────────────────────

def test_un_coach_voit_la_correction_mais_pas_le_deplacement(space_id, ctx):
    equipes = query_db(
        f"SELECT team_id FROM team_proj WHERE season_id = '{ctx['season_id']}' "
        f"AND coach_name = '{COACH_SIMPLE}'"
    )
    if not equipes:
        pytest.skip(f"{COACH_SIMPLE} n'a pas d'équipe dans cette saison")
    autre = next(t for t in ctx["teams"] if t != equipes[0])
    match = _publier_un_match(space_id, ctx, ctx["round_ids"][3], equipes[0], autre)

    page = requests.get(_url_recap(space_id, match["mr_id"]), headers=ENTETE_MEMBRE_SIMPLE)
    assert page.status_code == 200, page.status_code
    assert "ms-correct-btn" in page.text, "la correction reste ouverte au coach du match"
    assert "move-match/widget" not in page.text, "le déplacement est réservé aux administrateurs"

    widget = requests.get(
        f"{BASE_URL}/app/{space_id}/competitions/{ctx['competition_id']}"
        f"/{ctx['season_id']}/admin/schedule/move-match/widget?pairing_id={match['pairing_id']}",
        headers=ENTETE_MEMBRE_SIMPLE,
    )
    assert widget.status_code in (401, 403), widget.status_code

"""Tests E2E — l'étape MVP, saisie manuelle (carte 637).

Le MVP est la première étape de l'après-match : domicile, puis visiteur, un
joueur choisi directement. Il s'inscrit au journal comme une action `Mvp` au
tour 16, un par camp.

Scénarios couverts, dans l'ordre du fichier — ils partagent un seul rapport :
- M1 — La fin du match (match en direct) mène au MVP du domicile, qui y ramène
- M2 — MVP domicile au clic : indicateur à 3, « MVP 1 / 2 », suite vers le visiteur
- M3 — MVP visiteur : le domicile rappelé en tête, « MVP 2 / 2 », suite vers les gains
- M4 — Les deux MVP sont au journal, au tour 16, un par camp
- M5 — Un second MVP pour un camp est refusé (422), sans écriture
- M6 — Les gains et fans reviennent au MVP du visiteur
- M7 — Le MVP saisi se supprime (✕) et l'étape repropose le choix

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée.
"""

import re

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    first_player_id,
)


def _mvp_url(space_id, mr_id, side):
    return f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/mvp/{side}"


def _mvp_rows(mr_id):
    """Les MVP vivants du rapport : (camp, tour)."""
    rows = query_db(
        f"SELECT team_side || ':' || turn_number FROM match_report_actions "
        f"WHERE match_report_id = '{mr_id}' AND NOT is_deleted "
        f"AND action_json = '\"Mvp\"'::jsonb ORDER BY team_side"
    )
    return sorted(rows)


@pytest.fixture(scope="module")
def mvp_ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=4)
    ctx = {**full, "teams": full["team_ids"]}
    round_id = full["round_ids"][0]
    home, away = full["team_ids"][0], full["team_ids"][1]
    mr_id = create_draft(space_id, ctx, round_id, home, away)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home, away)
    ensure_inducements(space_id, mr_id)
    return {"mr_id": mr_id}


def _home_mvp_name(mr_id):
    """Le nom figé dans l'action, tel que le journal et l'étape l'affichent."""
    rows = query_db(
        f"SELECT player_display_name FROM match_report_actions "
        f"WHERE match_report_id = '{mr_id}' AND NOT is_deleted AND team_side = 'home' "
        f"AND action_json = '\"Mvp\"'::jsonb"
    )
    assert len(rows) == 1, f"un MVP domicile attendu : {rows}"
    return rows[0]


def _choisir_un_joueur_et_valider(page: Page):
    """Le premier joueur de l'équipe, puis le bouton de suite qui l'enregistre.

    La puce arrive par un widget htmx, mais elle-même n'est pas un élément
    htmx — son clic émet `playerSelected` : attendre son câblage n'aboutirait
    jamais. On attend donc que le bouton de suite, lui, soit activé par la
    sélection."""
    puce = page.locator(".match-report-mvp .mr-player-chip").first
    puce.wait_for(state="visible", timeout=10000)
    puce.click()
    suite = page.locator(".match-report-mvp form.mr-nav-row button[type=submit]")
    expect(suite).to_be_enabled()
    suite.click()


# ── M1 ────────────────────────────────────────────────────────────────────────

def test_m1_la_fin_du_match_mene_au_mvp_du_domicile(page: Page, space_id, mvp_ctx):
    """Depuis le match en direct (carte 636) ; le retour du MVP domicile y
    ramène."""
    mr_id = mvp_ctx["mr_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/live", wait_until="load")
    fin = page.locator(".match-report-live .live-end-match")
    expect(fin).to_contain_text("Fin du match")
    assert fin.get_attribute("href").endswith(f"/{mr_id}/mvp/home")

    page.goto(_mvp_url(space_id, mr_id, "home"), wait_until="load")
    retour = page.locator(".match-report-mvp a.btn-outline-sm")
    assert retour.get_attribute("href").endswith(f"/{mr_id}/live")


# ── M2 ────────────────────────────────────────────────────────────────────────

def test_m2_mvp_du_domicile(page: Page, space_id, mvp_ctx):
    mr_id = mvp_ctx["mr_id"]
    page.goto(_mvp_url(space_id, mr_id, "home"), wait_until="load")

    expect(page.locator(".mr-step--active")).to_have_text("3")
    expect(page.locator(".mr-banner-label")).to_contain_text("MVP 1 / 2")
    # Rien n'est choisi : le bouton de suite attend un joueur.
    expect(page.locator(".match-report-mvp form.mr-nav-row button[type=submit]")).to_be_disabled()

    _choisir_un_joueur_et_valider(page)
    page.wait_for_url(re.compile(rf".*/{mr_id}/mvp/away$"), timeout=10000)
    mvp_ctx["home_mvp"] = _home_mvp_name(mr_id)


# ── M3 ────────────────────────────────────────────────────────────────────────

def test_m3_mvp_du_visiteur(page: Page, space_id, mvp_ctx):
    mr_id = mvp_ctx["mr_id"]
    page.goto(_mvp_url(space_id, mr_id, "away"), wait_until="load")

    expect(page.locator(".mr-banner-label")).to_contain_text("MVP 2 / 2")
    # Le MVP du domicile, replié en tête : on ne revient pas en arrière.
    expect(page.locator(".match-report-mvp .mvp-done-row")).to_contain_text(mvp_ctx["home_mvp"])

    _choisir_un_joueur_et_valider(page)
    page.wait_for_url(re.compile(rf".*/{mr_id}/step5$"), timeout=10000)


# ── M4 ────────────────────────────────────────────────────────────────────────

def test_m4_un_mvp_par_camp_au_tour_16(mvp_ctx):
    assert _mvp_rows(mvp_ctx["mr_id"]) == ["away:16", "home:16"]


# ── M5 ────────────────────────────────────────────────────────────────────────

def test_m5_un_second_mvp_est_refuse(space_id, mvp_ctx):
    mr_id = mvp_ctx["mr_id"]
    resp = requests.post(
        _mvp_url(space_id, mr_id, "home"),
        data={"player_id": first_player_id(mr_id, "home"), "player_type": "regular"},
        allow_redirects=False,
    )
    assert resp.status_code == 422, f"attendu 422, obtenu {resp.status_code}"
    assert _mvp_rows(mr_id) == ["away:16", "home:16"]


# ── M6 ────────────────────────────────────────────────────────────────────────

def test_m6_les_gains_reviennent_au_mvp_du_visiteur(page: Page, space_id, mvp_ctx):
    mr_id = mvp_ctx["mr_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step5", wait_until="load")
    expect(page.locator(".mr-step--active")).to_have_text("4")
    retour = page.locator("a.btn-outline-sm", has_text="Retour")
    assert retour.get_attribute("href").endswith(f"/{mr_id}/mvp/away")


# ── M7 ────────────────────────────────────────────────────────────────────────

def test_m7_supprimer_le_mvp_repropose_le_choix(page: Page, space_id, mvp_ctx):
    mr_id = mvp_ctx["mr_id"]
    page.goto(_mvp_url(space_id, mr_id, "home"), wait_until="load")

    # Le MVP saisi est montré, avec son ✕, et sans sélecteur de joueurs.
    expect(page.locator(".match-report-mvp .mvp-row")).to_contain_text(mvp_ctx["home_mvp"])
    expect(page.locator(".match-report-mvp .mr-player-chip")).to_have_count(0)

    with page.expect_navigation(wait_until="load"):
        page.locator(".match-report-mvp .mvp-row-delete").click()

    expect(page.locator(".match-report-mvp .mvp-row")).to_have_count(0)
    expect(page.locator(".match-report-mvp .mr-player-chip").first).to_be_visible(timeout=10000)
    assert _mvp_rows(mr_id) == ["away:16"]

"""Tests E2E — le tirage au sort du MVP, et le MVP obligatoire (carte 638).

Le coach désigne six joueurs qui ont joué ; le serveur en tire un. « Tirer »
reste fermé tant qu'il n'y a pas six désignés. Le résultat est définitif : ni
✕ sur la page ni au journal, et la route de suppression le refuse.

Le MVP est obligatoire : sans lui, ni l'étape du visiteur ni les gains ne
s'ouvrent, et la clôture de l'après-match est refusée.

Scénarios couverts, dans l'ordre du fichier — ils partagent un seul rapport :
- D1 — Sans MVP domicile, l'étape du visiteur renvoie au domicile
- D2 — Sans MVP, l'étape 5 renvoie au MVP et sa clôture est refusée (422)
- D3 — Le tirage est l'onglet par défaut ; « Tirer » fermé à cinq, ouvert à six
- D4 — Tirer : la page montre le tiré et ses désignés, sans ✕
- D5 — L'action tirée est au tour 16, parmi les six, `draw_size` à 6
- D6 — Au journal : « tiré parmi 6 désignés », sans ✕
- D7 — Le MVP tiré ne se supprime pas (422)

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée.
"""

import re

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import create_draft, ensure_inducements, ensure_pre_match

TIRAGE = 6


def _mr_url(space_id, mr_id, suffix):
    return f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{suffix}"


def _home_mvp(mr_id):
    """Le MVP domicile vivant : (action_id, player_id, tour, draw_size)."""
    rows = query_db(
        f"SELECT action_id || '|' || player_id || '|' || turn_number || '|' "
        f"|| COALESCE(draw_size::text, '') FROM match_report_actions "
        f"WHERE match_report_id = '{mr_id}' AND NOT is_deleted AND team_side = 'home' "
        f"AND action_json = '\"Mvp\"'::jsonb"
    )
    assert len(rows) == 1, f"un MVP domicile attendu : {rows}"
    return rows[0].split("|")


@pytest.fixture(scope="module")
def draw_ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=4)
    ctx = {**full, "teams": full["team_ids"]}
    round_id = full["round_ids"][0]
    home, away = full["team_ids"][2], full["team_ids"][3]
    mr_id = create_draft(space_id, ctx, round_id, home, away)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home, away)
    ensure_inducements(space_id, mr_id)
    return {"mr_id": mr_id}


def _designer(page: Page, puce, attendu: int):
    """Désigne un joueur et attend que le compteur l'ait vu.

    La puce vient d'un widget htmx, mais n'est pas elle-même un élément htmx :
    son clic émet `playerSelected`, et `cliquer_quand_cable` attendrait un
    câblage qui n'arrive jamais. Le compteur, lui, dit si le clic a porté ; un
    clic tombé avant que le script du widget ait câblé la puce est rejoué."""
    compteur = page.locator(".match-report-mvp .mvp-draw-count")
    for _ in range(3):
        puce.click()
        try:
            expect(compteur).to_have_text(f"{attendu} / {TIRAGE}", timeout=2000)
            return
        except AssertionError:
            continue
    raise AssertionError(f"le compteur n'atteint pas {attendu} / {TIRAGE}")


# ── D1 ────────────────────────────────────────────────────────────────────────

def test_d1_le_visiteur_attend_le_mvp_du_domicile(space_id, draw_ctx):
    mr_id = draw_ctx["mr_id"]
    resp = requests.get(_mr_url(space_id, mr_id, "mvp/away"), allow_redirects=False)
    assert resp.status_code in (302, 303), f"redirection attendue : {resp.status_code}"
    assert resp.headers["Location"].endswith(f"/{mr_id}/mvp/home")


# ── D2 ────────────────────────────────────────────────────────────────────────

def test_d2_sans_mvp_pas_de_gains(space_id, draw_ctx):
    mr_id = draw_ctx["mr_id"]
    page_5 = requests.get(_mr_url(space_id, mr_id, "step5"), allow_redirects=False)
    assert page_5.status_code in (302, 303), f"redirection attendue : {page_5.status_code}"
    assert page_5.headers["Location"].endswith(f"/{mr_id}/mvp/home")

    cloture = requests.post(
        _mr_url(space_id, mr_id, "step5"),
        data={"home_gain": "50", "away_gain": "40", "home_fan_mod": "0", "away_fan_mod": "0"},
        allow_redirects=False,
    )
    assert cloture.status_code == 422, f"clôture sans MVP : {cloture.status_code}"
    phase = query_db(f"SELECT phase FROM match_report_proj WHERE match_report_id = '{mr_id}'")
    assert phase == ["PreMatch"], f"le rapport ne doit pas avancer : {phase}"


# ── D3 ────────────────────────────────────────────────────────────────────────

def test_d3_tirer_ferme_a_cinq_ouvert_a_six(page: Page, space_id, draw_ctx):
    mr_id = draw_ctx["mr_id"]
    page.goto(_mr_url(space_id, mr_id, "mvp/home"), wait_until="load")

    expect(page.locator(".match-report-mvp .mvp-mode-draw")).to_have_class(re.compile(r"\bselected\b"))
    tirer = page.locator(".match-report-mvp form.mvp-draw-form button[type=submit]")
    expect(tirer).to_be_disabled()

    puces = page.locator(".match-report-mvp .mvp-players .mr-player-chip")
    puces.first.wait_for(state="visible", timeout=10000)
    assert puces.count() >= TIRAGE, f"au moins {TIRAGE} joueurs attendus : {puces.count()}"

    for i in range(TIRAGE - 1):
        _designer(page, puces.nth(i), i + 1)
    expect(tirer).to_be_disabled()

    _designer(page, puces.nth(TIRAGE - 1), TIRAGE)
    expect(tirer).to_be_enabled()
    draw_ctx["designes"] = [puces.nth(i).get_attribute("data-player-id") for i in range(TIRAGE)]


# ── D4 ────────────────────────────────────────────────────────────────────────

def test_d4_tirer_montre_le_tire_et_ses_designes(page: Page, space_id, draw_ctx):
    mr_id = draw_ctx["mr_id"]
    page.goto(_mr_url(space_id, mr_id, "mvp/home"), wait_until="load")
    puces = page.locator(".match-report-mvp .mvp-players .mr-player-chip")
    puces.first.wait_for(state="visible", timeout=10000)
    for i in range(TIRAGE):
        _designer(page, puces.nth(i), i + 1)
    draw_ctx["designes"] = [puces.nth(i).get_attribute("data-player-id") for i in range(TIRAGE)]

    # Le serveur tire, puis recharge la page du camp sur le résultat.
    page.locator(".match-report-mvp form.mvp-draw-form button[type=submit]").click()
    parmi = page.locator(".match-report-mvp .mvp-drawn-among")
    expect(parmi).to_be_visible(timeout=10000)
    expect(parmi).to_contain_text("Tiré au sort parmi")
    expect(parmi).to_contain_text("Le tirage est définitif")
    expect(page.locator(".match-report-mvp .mvp-row")).to_have_count(1)
    expect(page.locator(".match-report-mvp .mvp-row-delete")).to_have_count(0)
    # La suite mène au visiteur.
    suite = page.locator(".match-report-mvp .mr-nav-row a.btn-primary-sm")
    assert suite.get_attribute("href").endswith(f"/{mr_id}/mvp/away")


# ── D5 ────────────────────────────────────────────────────────────────────────

def test_d5_le_tire_est_l_un_des_six_au_tour_16(draw_ctx):
    _, player_id, tour, draw_size = _home_mvp(draw_ctx["mr_id"])
    assert tour == "16"
    assert draw_size == str(TIRAGE)
    assert player_id in draw_ctx["designes"], f"{player_id} n'est pas parmi {draw_ctx['designes']}"


# ── D6 ────────────────────────────────────────────────────────────────────────

def test_d6_au_journal_tire_parmi_six_designes(space_id, draw_ctx):
    journal = requests.get(_mr_url(space_id, draw_ctx["mr_id"], "action-log"))
    assert journal.status_code == 200, journal.status_code
    assert "tiré parmi 6 désignés" in journal.text
    # Le seul MVP du journal est tiré : aucun ✕.
    assert "mr-log-delete" not in journal.text


# ── D7 ────────────────────────────────────────────────────────────────────────

def test_d7_le_mvp_tire_ne_se_supprime_pas(space_id, draw_ctx):
    mr_id = draw_ctx["mr_id"]
    action_id, _, _, _ = _home_mvp(mr_id)
    resp = requests.delete(
        _mr_url(space_id, mr_id, f"actions/{action_id}"),
        headers={"HX-Request": "true"},
    )
    assert resp.status_code == 422, f"attendu 422, obtenu {resp.status_code}"
    # Toujours là, toujours tiré.
    assert _home_mvp(mr_id)[0] == action_id
    # Le visiteur s'ouvre désormais.
    visiteur = requests.get(_mr_url(space_id, mr_id, "mvp/away"), allow_redirects=False)
    assert visiteur.status_code == 200, visiteur.status_code

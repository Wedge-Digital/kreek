"""Tests E2E — la page « Imprimer » d'une équipe (épic E16, carte 601).

Ce que ces scénarios prouvent, et qu'aucun test unitaire ne voit : l'aperçu
qui s'assemble dans le navigateur à partir de fragments de quatre BCs, ses
réactions aux options, la seconde équipe contrôlée dans le chemin, et surtout
**ce qui sort de l'imprimante** — les feuilles seules, une par page A4.

# Le montage

`build_full_competition` (4 équipes, 2 journées) inscrit les équipes et apparie
les deux journées. On joue un match de la **seconde** journée : ses deux
équipes entrent en phase d'après-match, donc ne s'impriment plus (R1). Une
équipe de l'autre appariement reste prête à jouer, et son prochain adversaire
est celui de la **première** journée, encore à venir (R12).

Une cinquième équipe, inscrite après la génération du calendrier, n'a aucun
match : son option « prochain adversaire » est grisée (R2).

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), jeu de
démonstration, base seedée (`make seed_e2e`).
"""

import re

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import (
    BASE_URL,
    _wait_teams_enrolled,
    build_and_submit_team_http,
    build_full_competition,
)
from db_helpers import query_db
from htmx_helpers import cliquer_quand_cable
from match_report_helpers import play_match

INCONNUE = "01JZZZZZZZZZZZZZZZZZZZZZZZ"


# ── Montage ───────────────────────────────────────────────────────────────────


def _paires(round_id: str) -> list[tuple[str, str]]:
    lignes = query_db(
        "SELECT home_team_id || '|' || away_team_id FROM competition_match_day_pairings "
        f"WHERE match_day_id = '{round_id}' ORDER BY id"
    )
    return [tuple(l.split("|")) for l in lignes]


def _nom(team_id: str) -> str:
    return query_db(f"SELECT team_name FROM team_proj WHERE team_id = '{team_id}'")[0]


def _adversaire(paires: list[tuple[str, str]], team_id: str) -> str:
    home, away = next(p for p in paires if team_id in p)
    return away if home == team_id else home


@pytest.fixture(scope="module")
def print_ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=4, num_rounds=2)
    round_1, round_2 = full["round_ids"][0], full["round_ids"][1]
    jouee, a_venir = _paires(round_2)
    play_match(space_id, full, round_2, jouee[0], jouee[1], home_gain=50, away_gain=40)

    imprimable = a_venir[0]
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
    _wait_teams_enrolled([sans_match])

    return {
        "space_id": space_id,
        "imprimable": imprimable,
        "adversaire": _adversaire(_paires(round_1), imprimable),
        # Prête à jouer : la cascade ne propose que celles-là (R3). L'équipe qui
        # vient de jouer en est exclue, à juste titre.
        "autre": a_venir[1],
        "apres_match": jouee[0],
        "sans_match": sans_match,
    }


def _page_url(ctx: dict, team_id: str) -> str:
    return f"{BASE_URL}/app/{ctx['space_id']}/teams/{team_id}/print"


def _ouvrir(page: Page, ctx: dict, team_id: str) -> None:
    """Ouvre la page et attend l'aperçu complet : « Imprimer » ne s'active
    qu'une fois tous les fragments chargés (`printPreviewReady`)."""
    page.goto(_page_url(ctx, team_id), wait_until="load")
    expect(page.locator("[data-print]")).to_be_enabled(timeout=15000)


def _feuilles(page: Page):
    return page.locator(".team-print-pages .print-sheet")


def _fiches(page: Page):
    """Les fiches d'équipe seules : les pages en plus cochées par défaut
    (compétences, feuille de match) ajoutent leurs propres feuilles."""
    return page.locator(".team-print-pages .team-print-sheet-head")


def _sans_pages_en_plus(page: Page) -> None:
    for nom in ("skills_annex", "match_sheet"):
        page.locator(f"[name='{nom}']").uncheck()
    page.wait_for_timeout(400)
    expect(page.locator("[data-print]")).to_be_enabled(timeout=15000)


# ── Scénarios ─────────────────────────────────────────────────────────────────


def test_le_bouton_de_la_fiche_mene_a_la_page(page: Page, print_ctx):
    """E1 — depuis la fiche, par le clic : la page remplace `window.print()`."""
    page.goto(f"{BASE_URL}/app/{print_ctx['space_id']}/teams/{print_ctx['imprimable']}", wait_until="load")
    cliquer_quand_cable(page, ".state-banner-cta:has-text('Imprimer')")

    expect(page.locator(".team-print")).to_be_visible(timeout=10000)
    expect(page.locator("[data-print]")).to_be_enabled(timeout=15000)
    assert page.url.endswith("/print"), f"hx-push-url n'a pas suivi : {page.url}"


def test_une_equipe_non_prete_a_jouer_a_une_page_d_explication(page: Page, print_ctx):
    """E1 — R1 : après son match, l'équipe est en phase d'après-match. Son URL
    directe explique au lieu de répondre 404, et la fiche n'a plus de bouton."""
    page.goto(_page_url(print_ctx, print_ctx["apres_match"]), wait_until="load")
    expect(page.locator(".team-print--unavailable")).to_be_visible()
    expect(page.locator("[data-print]")).to_have_count(0)


def test_un_coach_qui_ne_possede_pas_l_equipe_imprime(browser, print_ctx):
    """E1b — R16 : tout utilisateur connecté imprime, comme il lit la fiche."""
    contexte = browser.new_context(extra_http_headers={"X-Bypass-Auth-Profile": "simple"})
    try:
        vue = contexte.new_page()
        _ouvrir(vue, print_ctx, print_ctx["imprimable"])
        expect(_fiches(vue)).to_have_count(1)
    finally:
        contexte.close()


def test_l_apercu_suit_les_options(page: Page, print_ctx):
    """E2 — une option rechargée, et l'aperçu la reflète."""
    _ouvrir(page, print_ctx, print_ctx["imprimable"])
    _sans_pages_en_plus(page)
    expect(_feuilles(page)).to_have_count(1)
    expect(page.locator(".players-print-table")).to_be_visible()
    expect(page.locator(".team-print-sheet-head")).to_contain_text("Trésorerie")

    page.locator(".team-print-segmented label:has-text('Paysage')").click()
    expect(page.locator(".print-preview.orientation-landscape")).to_have_count(1, timeout=10000)
    expect(page.locator("[data-print]")).to_be_enabled(timeout=15000)



def test_le_prochain_adversaire_ajoute_sa_fiche(page: Page, print_ctx):
    """E3 — R2, R12 : l'adversaire de la journée à venir, trésorerie comprise (R8)."""
    _ouvrir(page, print_ctx, print_ctx["imprimable"])
    option = page.locator(".print-next-opponent")
    nom = _nom(print_ctx["adversaire"])
    expect(option).to_contain_text(nom)

    option.locator("input[type=radio]").check()
    expect(_fiches(page)).to_have_count(2, timeout=15000)
    expect(page.locator("[data-print]")).to_be_enabled(timeout=15000)
    seconde = _feuilles(page).nth(1)
    expect(seconde.locator(".team-print-sheet-head-name")).to_have_text(nom)
    expect(seconde).to_contain_text("Trésorerie")


def test_une_autre_journee_change_l_adversaire(page: Page, print_ctx):
    """Carte 610 — l'équipe imprimable joue les deux journées : la première
    est présélectionnée (R12), la seconde se choisit et change la fiche
    adverse."""
    _ouvrir(page, print_ctx, print_ctx["imprimable"])
    _sans_pages_en_plus(page)
    nom = _nom(print_ctx["autre"])

    journee = page.locator(".print-next-opponent kreek-select[name='print_round_id']")
    journee.locator(".ks-control").click()
    journee.locator(".ks-option", has_text=nom).first.click()

    expect(page.locator(".print-next-opponent input[type=radio]")).to_be_checked()
    expect(page.locator(".print-next-opponent .team-print-choice-hint")).to_contain_text(nom)
    expect(_fiches(page)).to_have_count(2, timeout=15000)
    expect(page.locator("[data-print]")).to_be_enabled(timeout=15000)
    expect(_fiches(page).nth(1).locator(".team-print-sheet-head-name")).to_have_text(nom)


def test_sans_match_a_venir_l_option_est_grisee(page: Page, print_ctx):
    """E3 — R2 : l'option dit pourquoi elle n'est pas disponible."""
    _ouvrir(page, print_ctx, print_ctx["sans_match"])
    option = page.locator(".print-next-opponent")
    expect(option).to_contain_text("Aucun match à venir")
    expect(option.locator("input[type=radio]")).to_be_disabled()


def test_une_autre_equipe_par_la_cascade(page: Page, print_ctx):
    """E4 — R3 : compétition et saison présélectionnées, on choisit l'équipe."""
    _ouvrir(page, print_ctx, print_ctx["imprimable"])
    nom = _nom(print_ctx["autre"])

    equipe = page.locator("kreek-select[name='other_team_id']")
    equipe.locator(".ks-control").click()
    equipe.locator(".ks-option", has_text=nom).first.click()

    expect(page.locator("[name='content'][value='other']")).to_be_checked()
    expect(_fiches(page)).to_have_count(2, timeout=15000)
    expect(_fiches(page).nth(1).locator(".team-print-sheet-head-name")).to_have_text(nom)


def test_une_seconde_equipe_inconnue_de_l_espace_est_refusee(page: Page, print_ctx):
    """E5 — carte 594 : `{other_team_id}` est contrôlé par `space_scope`. Une
    équipe d'un autre espace, ou inconnue, n'atteint jamais le contrôleur."""
    base = f"{BASE_URL}/app/{print_ctx['space_id']}/teams/{print_ctx['imprimable']}/print/preview/with"
    assert page.request.get(f"{base}/{INCONNUE}").status == 404

    etrangeres = query_db(
        "SELECT team_id FROM team_proj "
        f"WHERE space_id <> '{print_ctx['space_id']}' LIMIT 1"
    )
    if etrangeres:
        assert page.request.get(f"{base}/{etrangeres[0]}").status == 404


def test_l_impression_ne_sort_que_les_feuilles(page: Page, print_ctx):
    """E6 — le PDF compte exactement les feuilles de l'aperçu, et le chrome de
    l'application comme les options sont masqués à l'impression."""
    _ouvrir(page, print_ctx, print_ctx["imprimable"])
    _sans_pages_en_plus(page)
    page.locator(".print-next-opponent input[type=radio]").check()
    expect(_feuilles(page)).to_have_count(2, timeout=15000)
    expect(page.locator("[data-print]")).to_be_enabled(timeout=15000)

    page.emulate_media(media="print")
    expect(page.locator(".team-print-options")).to_be_hidden()
    expect(page.locator(".sidebar")).to_be_hidden()
    pdf = page.pdf(prefer_css_page_size=True, print_background=True)
    pages = pdf.count(b"/Type /Page") - pdf.count(b"/Type /Pages")
    assert pages == 2, f"2 feuilles dans l'aperçu, {pages} pages dans le PDF"
    # Chaque page au format A4 portrait (595 × 842 points) : une feuille plus
    # haute que le papier la poussait sur une page blanche, et Firefox gardait
    # l'orientation de sa dernière impression faute d'un `@page` général.
    formats = re.findall(rb"/MediaBox\s*\[\s*0 0 ([\d.]+) ([\d.]+)\s*\]", pdf)
    assert {(round(float(w)), round(float(h))) for w, h in formats} == {(595, 842)}, formats


def test_imprimer_attend_l_apercu_puis_ouvre_l_impression(page: Page, print_ctx):
    """E7 — « Imprimer » n'imprime pas un aperçu à moitié chargé."""
    page.add_init_script("window.__impressions = 0; window.print = () => { window.__impressions += 1; };")
    _ouvrir(page, print_ctx, print_ctx["imprimable"])
    page.locator("[data-print]").click()
    assert page.evaluate("window.__impressions") == 1


def test_rien_n_est_coupe_en_silence(page: Page, print_ctx):
    """E8 — la feuille la plus chargée (paysage, deux équipes, lignes vides)
    tient sans avertissement : aucune feuille marquée en débordement."""
    _ouvrir(page, print_ctx, print_ctx["imprimable"])
    page.locator(".team-print-segmented label:has-text('Paysage')").click()
    page.locator(".print-next-opponent input[type=radio]").check()
    expect(_fiches(page)).to_have_count(2, timeout=15000)
    expect(page.locator("[data-print]")).to_be_enabled(timeout=15000)

    debordees = page.evaluate(
        """() => [...document.querySelectorAll('.print-sheet--overflow')].map(s => {
            const slot = s.closest('.print-slot');
            const label = slot ? slot.querySelector('.print-slot-label').textContent : '?';
            return label + ' : ' + s.scrollHeight + ' > ' + s.clientHeight;
        })"""
    )
    assert not debordees, f"feuilles qui débordent : {debordees}"
    expect(page.locator(".team-print-warning")).to_be_hidden()

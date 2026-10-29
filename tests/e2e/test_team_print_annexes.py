"""Tests E2E — les pages en plus de l'impression (épic E16, carte 609).

L'annexe des compétences, les coups de pouce et star players disponibles, la
feuille de match : des fragments de `players` et de `match_report` qui rendent
leurs propres feuilles dans la page « Imprimer » de `teams`.

# Le montage

Deux équipes, une journée : la seule rencontre les oppose, donc chacune est le
prochain adversaire de l'autre. Les rosters sont imposés — Granitiers contre
Zéphyriens — parce que les assertions en dépendent : le Masseur Douteux est
réservé aux Granitiers (« — » pour l'autre colonne), Grumo le Terne ne joue que
pour eux (R13), et leur règle `BRAWLIN_BRUTES` change le barème SPP.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), jeu de
démonstration, base seedée (`make seed_e2e`).
"""

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db


@pytest.fixture(scope="module")
def annexes_ctx(browser, space_id):
    full = build_full_competition(
        browser, space_id, num_teams=2, num_rounds=1,
        roster_uids=["DEMO_GRANIT", "DEMO_ZEPHYR"],
    )
    granit, zephyr = full["team_ids"][0], full["team_ids"][1]
    return {"space_id": space_id, "own": granit, "granit": granit, "zephyr": zephyr}


@pytest.fixture(scope="module")
def semi_hommes_ctx(browser, space_id):
    """Deux équipes de Semi-hommes. Le harnais recrute le premier poste du
    roster : Tourte-Pied, qui porte « Pas de Côté ». Granitiers et Zéphyriens
    recrutent leur Piétaille, sans aucune compétence — leur annexe est vide, à
    juste titre. Ici, les deux équipes partagent leur compétence : c'est ce qui
    éprouve le dédoublonnage."""
    full = build_full_competition(
        browser, space_id, num_teams=2, num_rounds=1,
        roster_uids=["HALFLING", "HALFLING"],
    )
    return {"space_id": space_id, "own": full["team_ids"][0]}


def _nom(team_id: str) -> str:
    return query_db(f"SELECT team_name FROM team_proj WHERE team_id = '{team_id}'")[0]


def _ve(team_id: str) -> int:
    return int(query_db(f"SELECT team_value FROM team_proj WHERE team_id = '{team_id}'")[0])


def _ouvrir_avec_l_adversaire(page: Page, ctx: dict) -> None:
    page.goto(f"{BASE_URL}/app/{ctx['space_id']}/teams/{ctx['own']}/print", wait_until="load")
    expect(page.locator("[data-print]")).to_be_enabled(timeout=15000)
    page.locator(".print-next-opponent input[type=radio]").check()
    _attendre_l_apercu(page)


def _attendre_l_apercu(page: Page) -> None:
    """Une option rechargée désactive « Imprimer » jusqu'au nouvel aperçu."""
    page.wait_for_timeout(400)
    expect(page.locator("[data-print]")).to_be_enabled(timeout=15000)


# ── Scénarios ─────────────────────────────────────────────────────────────────


def test_l_annexe_liste_chaque_competence_une_fois(page: Page, semi_hommes_ctx):
    """E9 — les deux équipes partagent « Pas de Côté » ; elle n'apparaît
    qu'une fois, avec sa description."""
    _ouvrir_avec_l_adversaire(page, semi_hommes_ctx)
    noms = page.locator(".players-print-skills-name").all_inner_texts()
    assert noms, "l'annexe des compétences est vide"
    assert len(noms) == len(set(noms)), f"compétence en double : {noms}"
    assert "Pas de Côté" in noms, noms
    for description in page.locator(".players-print-skills-desc").all_inner_texts():
        assert description.strip(), "une compétence sans description"


def test_les_coups_de_pouce_ont_le_prix_de_chaque_equipe(page: Page, annexes_ctx):
    """E10 — R7 : une colonne par équipe ; le Masseur Douteux, réservé aux
    Granitiers, est « — » pour les Zéphyriens."""
    _ouvrir_avec_l_adversaire(page, annexes_ctx)
    page.locator("[name='inducements']").check()
    _attendre_l_apercu(page)

    # L'en-tête est en capitales par le CSS : `inner_text` les rend telles.
    entetes = page.locator(".match-report-print-inducements-grid thead").inner_text().upper()
    assert _nom(annexes_ctx["granit"]).upper() in entetes
    assert _nom(annexes_ctx["zephyr"]).upper() in entetes
    masseur = page.locator(".match-report-print-inducements-grid tr:has-text('Masseur Douteux')")
    expect(masseur.locator(".match-report-print-inducements-price").nth(1)).to_have_text("—")


def test_les_star_players_suivent_leur_roster(page: Page, annexes_ctx):
    """E10 — R13 : Grumo le Terne ne joue que pour les Granitiers."""
    _ouvrir_avec_l_adversaire(page, annexes_ctx)
    page.locator("[name='star_players']").check()
    _attendre_l_apercu(page)

    grumo = page.locator(".match-report-print-inducements-star:has-text('Grumo le Terne')")
    expect(grumo).to_have_count(1)
    expect(grumo).to_contain_text(_nom(annexes_ctx["granit"]))
    expect(grumo).not_to_contain_text(_nom(annexes_ctx["zephyr"]))


def test_la_feuille_de_match_dit_le_favori_et_la_petite_monnaie(page: Page, annexes_ctx):
    """E11 — R10, R11 : favori et petite monnaie conformes aux VE ; barème du
    roster ; rappels et météos du corpus de démonstration."""
    _ouvrir_avec_l_adversaire(page, annexes_ctx)
    feuille = page.locator(".match-report-print-sheet")
    expect(feuille).to_have_count(1)
    expect(feuille).to_contain_text("Favori")
    ecart = abs(_ve(annexes_ctx["granit"]) - _ve(annexes_ctx["zephyr"]))
    expect(feuille.locator(".match-report-print-sheet-money:has-text('Outsider')")).to_contain_text(f"{ecart} kPo")

    # Granitiers : BRAWLIN_BRUTES, le touchdown vaut 2.
    expect(feuille.locator(".match-report-print-sheet-legend").first).to_contain_text("TD 2")
    expect(feuille.locator(".match-report-print-sheet-rules")).to_contain_text("MVP.")
    expect(feuille.locator(".match-report-print-sheet-meta .match-report-print-sheet-box")).to_have_count(5)


def test_la_feuille_de_match_en_deux_exemplaires(page: Page, annexes_ctx):
    """Un exemplaire par coach : deux feuilles, chacune numérotée."""
    _ouvrir_avec_l_adversaire(page, annexes_ctx)
    page.locator("[name='two_copies']").check()
    _attendre_l_apercu(page)
    expect(page.locator(".match-report-print-sheet")).to_have_count(2)
    expect(page.locator(".match-report-print-sheet-meta").nth(1)).to_contain_text("exemplaire 2 / 2")

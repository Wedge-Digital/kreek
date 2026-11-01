"""Le tirage des journées du Calendrier (épic E18).

Scénario n°9 — « Générer une journée complète » ne laisse aucune équipe sans
match hors l'exemptée. C'est la régression du défaut de l'ancien tirage
(`generate_round_pairings`), qui préférait renvoyer une équipe chez elle
plutôt que de programmer une revanche.

Scénarios 1 à 8 — l'appariement des présents (carte 619) : décocher les
absentes, voir l'aperçu sans rien écrire, puis écrire après revérification.
Leur saison a **deux poules impaires** et **un coach à deux équipes**, dans la
même poule : de quoi exercer l'exemptée (R8) et la paire interdite (R4).

**Ce qui n'est pas testé : l'aléa.** On vérifie que chaque présente joue, pas
qui joue contre qui : un test qui dépendrait du tirage échouerait une fois sur
N (cf. cartes 532 et 580). « Retirer au sort » se vérifie par un aperçu valide
en retour, jamais par un résultat différent ; l'exemptée, par son appartenance
aux plus servies, jamais par son nom — à égalité, le tirage départage.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import time

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import (
    BASE_URL,
    _space_coach_ids,
    _wait_teams_enrolled,
    build_and_submit_team_http,
    build_full_competition,
)
from db_helpers import query_db
from htmx_helpers import attendre_cablage, cliquer_quand_cable

JSON = {"Content-Type": "application/json", "HX-Request": "true"}


@pytest.fixture(scope="module")
def competition(browser, space_id):
    """Cinq équipes, trois journées déjà générées.

    Cinq et non quatre : un nombre impair force une exemptée, et c'est la seule
    équipe qui a le droit de ne pas jouer. Trois journées : la dernière tirée
    porte un historique, donc des revanches à éviter.
    """
    ctx = build_full_competition(browser, space_id, num_teams=5, num_rounds=3)
    return {"space_id": space_id, **ctx}


def _admin_url(ctx: dict, suffixe: str) -> str:
    return (
        f"{BASE_URL}/app/{ctx['space_id']}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/admin/{suffixe}"
    )


def _equipes_engagees(round_id: str) -> list[str]:
    lignes = query_db(
        "SELECT home_team_id || '|' || away_team_id "
        f"FROM competition_match_day_pairings WHERE match_day_id = '{round_id}'"
    )
    return [equipe for ligne in lignes for equipe in ligne.split("|")]


def _attendre(condition, description: str, timeout_s: int = 15):
    deadline = time.time() + timeout_s
    dernier = None
    while time.time() < deadline:
        dernier = condition()
        if dernier:
            return dernier
        time.sleep(0.3)
    raise AssertionError(f"{description} — non satisfait après {timeout_s}s ({dernier})")


def test_generer_une_journee_complete_fait_jouer_toutes_les_equipes_sauf_l_exemptee(
    page: Page, competition
):
    round_id = competition["round_ids"][-1]
    vidage = requests.post(
        _admin_url(competition, "schedule/clear-round"),
        json={"round_id": round_id},
        headers=JSON,
        timeout=15,
    )
    assert vidage.ok, f"clear-round : {vidage.status_code} {vidage.text[:200]}"
    assert _equipes_engagees(round_id) == []

    page.goto(_admin_url(competition, "schedule"), wait_until="load")
    page.locator(".round-item").nth(len(competition["round_ids"]) - 1).click()
    page.wait_for_selector(".match-list-empty")
    # Le bouton appelle `fetch` en `onclick` : htmx ne le câble jamais. On
    # attend donc son voisin `hx-delete`, câblé avec le même fragment.
    attendre_cablage(page, ".round-detail-footer .btn-danger-sm")
    page.locator(".btn-primary-sm:has-text('Générer une journée complète')").click()

    engagees = _attendre(lambda: _equipes_engagees(round_id), "la journée doit être tirée")
    assert len(engagees) == 4, f"cinq équipes : deux rencontres attendues, obtenu {engagees}"
    assert len(set(engagees)) == 4, f"une équipe joue deux fois : {engagees}"
    assert set(engagees) <= set(competition["team_ids"])

    # R9 — la journée pourvue n'offre plus le bouton.
    page.wait_for_selector(".match-row")
    assert page.locator(".btn-primary-sm:has-text('Générer une journée complète')").count() == 0


# ══ L'appariement des présents (carte 619) ════════════════════════════════════

HX = {"HX-Request": "true"}
PRESENCE = ".schedule-draw input[name=team_ids]"
TIRER = ".schedule-draw button:has-text('Tirer au sort')"
ECRIRE = ".schedule-draw button:has-text('Écrire au calendrier')"


@pytest.fixture(scope="module")
def presents(browser, space_id):
    """Huit équipes en deux poules impaires — A à cinq, B à trois.

    Sept équipes de coachs distincts, puis une huitième pour le coach de la
    première (R4), rangées dans la poule A avec elle. Deux journées tirées
    d'abord, sans poule : un historique, donc des matchs joués inégaux (R8).
    """
    ctx = build_full_competition(browser, space_id, num_teams=7, num_rounds=2)
    ctx = {"space_id": space_id, **ctx}
    ctx["jumelle"] = _seconde_equipe_du_premier_coach(browser, ctx)
    t = ctx["team_ids"]
    ctx["poule_a"] = [t[0], ctx["jumelle"], t[1], t[2], t[3]]
    ctx["poule_b"] = [t[4], t[5], t[6]]
    _deux_poules(ctx)
    return ctx


def _seconde_equipe_du_premier_coach(browser, ctx: dict) -> str:
    coach = _space_coach_ids(ctx["space_id"])[0].split("|")[0]
    page = browser.new_page()
    try:
        equipe = build_and_submit_team_http(
            page, ctx["space_id"], ctx["competition_id"], ctx["season_id"],
            coach, roster_index=7,
        )
    finally:
        page.close()
    _wait_teams_enrolled([equipe])
    return equipe


def _deux_poules(ctx: dict) -> None:
    """Déclare les poules, les matérialise (le projecteur est paresseux), puis
    range chaque équipe — à la main : un tirage des poules séparerait peut-être
    les deux équipes du même coach."""
    declaration = requests.post(
        _admin_url(ctx, "settings/pools"),
        data=[("use_pools", "true"), ("pool_id", ""), ("pool_name", "Poule A"),
              ("pool_id", ""), ("pool_name", "Poule B")],
        headers=HX, timeout=15,
    )
    assert declaration.ok, f"settings/pools : {declaration.status_code}"
    requests.get(_admin_url(ctx, "groups/cards"), headers=HX, timeout=15)
    poules = query_db(
        f"SELECT id FROM competition_groups WHERE season_id = '{ctx['season_id']}' ORDER BY position"
    )
    assert len(poules) == 2, f"deux poules attendues, obtenu {poules}"
    for poule, equipes in zip(poules, [ctx["poule_a"], ctx["poule_b"]]):
        for equipe in equipes:
            rangee = requests.post(
                _admin_url(ctx, "groups/assign"),
                json={"team_id": equipe, "group_id": poule}, headers=JSON, timeout=15,
            )
            assert rangee.ok, f"groups/assign : {rangee.status_code} {rangee.text[:200]}"


def _journee_vide(ctx: dict) -> str:
    """Une journée neuve, ajoutée en fin de calendrier — l'historique des
    journées tirées reste intact."""
    ajout = requests.post(_admin_url(ctx, "schedule/add-round"), json={}, headers=JSON, timeout=15)
    assert ajout.ok, f"add-round : {ajout.status_code} {ajout.text[:200]}"
    return query_db(
        f"SELECT id FROM competition_match_days WHERE season_id = '{ctx['season_id']}' "
        "ORDER BY position DESC LIMIT 1"
    )[0]


def _couples(round_id: str) -> list[tuple[str, str]]:
    lignes = query_db(
        "SELECT home_team_id || '|' || away_team_id "
        f"FROM competition_match_day_pairings WHERE match_day_id = '{round_id}'"
    )
    return [tuple(ligne.split("|")) for ligne in lignes]


def _ouvrir_journee(page: Page, ctx: dict, round_id: str) -> None:
    page.goto(_admin_url(ctx, "schedule"), wait_until="load")
    page.locator(f".round-item[data-round-id='{round_id}']").click()
    page.wait_for_selector(".round-detail-content")


def _ouvrir_presents(page: Page, ctx: dict, round_id: str) -> None:
    _ouvrir_journee(page, ctx, round_id)
    cliquer_quand_cable(page, "button:has-text('Faire les appariements')")
    page.wait_for_selector(".schedule-draw .draw-group")


def _decocher(page: Page, team_id: str) -> None:
    page.locator(f"{PRESENCE}[value='{team_id}']").uncheck()


def _tirer(page: Page) -> None:
    cliquer_quand_cable(page, TIRER)
    page.wait_for_selector(".schedule-draw .draw-match")


def _ecrire(page: Page) -> None:
    cliquer_quand_cable(page, ECRIRE)


def _poule(page: Page, rang: int):
    return page.locator(".schedule-draw .draw-group").nth(rang)


def _matchs_joues(ctx: dict, round_id: str) -> dict[str, int]:
    """R8 — les appariements de la saison, journée tirée exclue."""
    lignes = query_db(
        "SELECT p.home_team_id || '|' || p.away_team_id "
        "FROM competition_match_day_pairings p "
        "JOIN competition_match_days d ON d.id = p.match_day_id "
        f"WHERE d.season_id = '{ctx['season_id']}' AND d.id <> '{round_id}'"
    )
    comptes: dict[str, int] = {}
    for ligne in lignes:
        for equipe in ligne.split("|"):
            comptes[equipe] = comptes.get(equipe, 0) + 1
    return comptes


# ── 1 · R9 ───────────────────────────────────────────────────────────────────


def test_seule_une_journee_vide_offre_les_appariements(page: Page, presents):
    _ouvrir_journee(page, presents, _journee_vide(presents))
    assert page.locator("button:has-text('Faire les appariements')").count() == 1
    assert page.locator("button:has-text('Générer une journée complète')").count() == 1

    _ouvrir_journee(page, presents, presents["round_ids"][0])
    page.wait_for_selector(".match-row")
    assert page.locator("button:has-text('Faire les appariements')").count() == 0
    assert page.locator("button:has-text('Générer une journée complète')").count() == 0


# ── 2 · R2, R10 ──────────────────────────────────────────────────────────────


def test_les_compteurs_suivent_les_cases(page: Page, presents):
    _ouvrir_presents(page, presents, _journee_vide(presents))
    poule_b = _poule(page, 1)
    expect(page.locator(f"{PRESENCE}:checked")).to_have_count(8)  # R2 : toutes cochées
    expect(poule_b.locator(".draw-note--info")).to_be_visible()  # trois présentes : impair

    _decocher(page, presents["poule_b"][0])
    expect(poule_b.locator(".draw-group-count strong")).to_have_text("2")
    expect(poule_b.locator(".draw-note--info")).to_be_hidden()

    for rang in (0, 1):
        _poule(page, rang).locator("button:has-text('Toutes absentes')").click()
    expect(page.locator(TIRER)).to_be_disabled()  # R10 : aucune poule à deux présentes


# ── 3 · L'aperçu n'écrit rien ────────────────────────────────────────────────


def test_l_apercu_n_ecrit_rien(page: Page, presents):
    round_id = _journee_vide(presents)
    _ouvrir_presents(page, presents, round_id)
    _tirer(page)
    cliquer_quand_cable(page, ".schedule-draw button:has-text('Retirer au sort')")
    page.wait_for_selector(".schedule-draw .draw-match")

    cliquer_quand_cable(page, ".schedule-draw button:has-text('Modifier les présents')")
    page.wait_for_selector(PRESENCE)
    page.locator(".draw-footer button:has-text('Annuler')").click()
    page.wait_for_selector("button:has-text('Faire les appariements')")
    assert _couples(round_id) == []


# ── 4 · Modifier les présents ────────────────────────────────────────────────


def test_modifier_les_presents_garde_les_decochages(page: Page, presents):
    absente = presents["poule_a"][2]
    _ouvrir_presents(page, presents, _journee_vide(presents))
    _decocher(page, absente)
    _tirer(page)

    cliquer_quand_cable(page, ".schedule-draw button:has-text('Modifier les présents')")
    page.wait_for_selector(PRESENCE)
    assert not page.locator(f"{PRESENCE}[value='{absente}']").is_checked()
    assert page.locator(f"{PRESENCE}:checked").count() == 7


# ── 5 · Écrire — R8 ──────────────────────────────────────────────────────────


def test_ecrire_le_tirage(page: Page, presents):
    """Poule A : cinq présentes, une exemptée — parmi les plus servies (R8).
    Poule B : une absente, les deux autres se rencontrent."""
    round_id = _journee_vide(presents)
    absente = presents["poule_b"][2]
    joues = _matchs_joues(presents, round_id)
    _ouvrir_presents(page, presents, round_id)
    _decocher(page, absente)
    _tirer(page)
    _ecrire(page)

    page.wait_for_selector(".toast--success")
    page.wait_for_selector(".match-row")
    engagees = [e for couple in _attendre(lambda: _couples(round_id), "écriture") for e in couple]
    assert len(engagees) == 6, f"trois rencontres attendues : {engagees}"
    assert absente not in engagees
    exemptee = set(presents["poule_a"]) - set(engagees)
    plus_servies = _plus_servies(presents["poule_a"], joues)
    assert len(exemptee) == 1 and exemptee <= plus_servies, (
        f"exemptée {exemptee}, attendue parmi {plus_servies} ({joues})"
    )


def _plus_servies(equipes: list[str], joues: dict[str, int]) -> set[str]:
    maximum = max(joues.get(e, 0) for e in equipes)
    return {e for e in equipes if joues.get(e, 0) == maximum}


# ── 6 · R13 ──────────────────────────────────────────────────────────────────


def test_une_rencontre_ecrite_est_ordinaire(page: Page, presents):
    round_id = _journee_vide(presents)
    _ouvrir_presents(page, presents, round_id)
    _tirer(page)
    _ecrire(page)
    couples = _attendre(lambda: _couples(round_id), "écriture")

    pairing_id, domicile, exterieur = query_db(
        "SELECT id || '|' || home_team_id || '|' || away_team_id "
        f"FROM competition_match_day_pairings WHERE match_day_id = '{round_id}' LIMIT 1"
    )[0].split("|")
    retrait = requests.delete(
        _admin_url(presents, "schedule/delete-match"),
        json={"pairing_id": pairing_id}, headers=JSON, timeout=15,
    )
    assert retrait.ok, f"delete-match : {retrait.status_code} {retrait.text[:200]}"
    assert len(_couples(round_id)) == len(couples) - 1

    # Les deux équipes libérées se retrouvent par un ajout à la main.
    ajout = requests.post(
        _admin_url(presents, "schedule/add-match"),
        json={"round_id": round_id, "home_team_id": domicile, "away_team_id": exterieur},
        headers=JSON, timeout=15,
    )
    assert ajout.ok, f"add-match : {ajout.status_code} {ajout.text[:200]}"
    assert len(_couples(round_id)) == len(couples)


# ── 7 · R11 ──────────────────────────────────────────────────────────────────


def test_une_journee_remplie_entre_temps_n_est_pas_ecrasee(page: Page, presents):
    round_id = _journee_vide(presents)
    _ouvrir_presents(page, presents, round_id)
    _tirer(page)
    manuel = (presents["poule_b"][0], presents["poule_b"][1])
    ajout = requests.post(
        _admin_url(presents, "schedule/add-match"),
        json={"round_id": round_id, "home_team_id": manuel[0], "away_team_id": manuel[1]},
        headers=JSON, timeout=15,
    )
    assert ajout.ok, f"add-match : {ajout.status_code} {ajout.text[:200]}"

    _ecrire(page)
    page.wait_for_selector(".toast--error")
    assert _couples(round_id) == [manuel], "seul le match ajouté à la main"


# ── 8 · R4 ───────────────────────────────────────────────────────────────────


def test_deux_equipes_d_un_meme_coach_ne_se_rencontrent_jamais(page: Page, presents):
    """Poule A réduite à trois présentes, dont les deux du même coach : la paire
    est interdite avant le tirage, donc le résultat ne dépend pas de l'aléa."""
    round_id = _journee_vide(presents)
    premiere, jumelle = presents["poule_a"][0], presents["jumelle"]
    _ouvrir_presents(page, presents, round_id)
    for equipe in presents["poule_a"][3:]:
        _decocher(page, equipe)
    _tirer(page)
    _ecrire(page)

    couples = _attendre(lambda: _couples(round_id), "écriture")
    assert {premiere, jumelle} not in [set(c) for c in couples], couples

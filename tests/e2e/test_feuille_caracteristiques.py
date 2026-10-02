"""Les caractéristiques et le solde SPP d'une ligne de la feuille d'équipe
(carte 569).

**Un témoin, écrit avant le changement qu'il surveille.** La carte 569 fait lire
au tableau des joueurs ses caractéristiques et son solde SPP dans `players_proj`
au lieu des agrégats rejoués. Elle change **d'où** vient chaque chiffre, pas
**ce qui** s'affiche. Ce fichier a été écrit et vu vert sur le code d'avant,
puis commité seul : s'il faut le retoucher pour qu'il passe après, c'est que la
projection ne donne pas les mêmes chiffres — un écart, pas un test à mettre à
jour.

Aucun autre test ne lisait les caractéristiques **sur la feuille d'équipe** :
`test_stat_increase_updates_stat_and_reserve` les lit sur la fiche joueur.

Les valeurs sont toujours lues **avant puis après** sur la feuille elle-même,
jamais codées en dur : elles dépendent du roster de la fixture.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import re
import time

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from htmx_helpers import cliquer_quand_cable
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    post_step5,
    publish,
    record_action_api,
)

HX = {"HX-Request": "true"}
STATS = ("ma", "st", "ag", "pa", "av")


# ── Lecture de la feuille ─────────────────────────────────────────────────────


def _ligne(page: Page, space_id: str, team_id: str, player_id: str) -> dict:
    """Les cinq caractéristiques et le solde SPP, tels que la ligne les affiche.

    En texte : « 3+ » et « — » se comparent tels quels, et un tiret qui
    remplacerait un chiffre se verrait.
    """
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
    ligne = page.locator(f'tr.player-table-row[data-player-detail*="{player_id}"]')
    ligne.first.wait_for(state="attached", timeout=15000)
    stats = [t.strip() for t in ligne.first.locator("td.player-stat").all_inner_texts()]
    assert len(stats) == 5, f"cinq caractéristiques attendues : {stats}"
    return {
        **dict(zip(STATS, stats)),
        "spp": ligne.first.locator("td.player-spp").inner_text().strip(),
    }


def _nombre(valeur: str) -> int:
    return int(valeur.rstrip("+"))


def _ecarts(avant: dict, apres: dict) -> dict:
    return {s: (avant[s], apres[s]) for s in STATS if avant[s] != apres[s]}


def _attendre(predicat, quoi: str, timeout_s: int = 25) -> None:
    """Les impacts de match transitent par l'app event bus."""
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if predicat():
            return
        time.sleep(0.3)
    raise AssertionError(f"jamais vrai : {quoi}")


def _phase(team_id: str) -> str:
    return query_db(f"SELECT game_phase FROM team_proj WHERE team_id = '{team_id}'")[0]


def _joueurs(team_id: str) -> list[str]:
    return query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{team_id}' "
        "AND membership = 'Active' ORDER BY player_id"
    )


# ── La séquelle, puis sa dépublication ────────────────────────────────────────


@pytest.fixture(scope="module")
def match_a_sequelle(browser, space_id):
    """Un match publié où un joueur domicile subit une séquelle de MV.

    Une compétition à part : dépublier un rapport n'est possible qu'à chaud, et
    une dépense de SPP faite après lui le figerait.
    """
    ctx = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    domicile, exterieur = ctx["team_ids"]
    round_id = ctx["round_ids"][0]
    victime = _joueurs(domicile)[0]

    page = browser.new_page()
    avant = _ligne(page, space_id, domicile, victime)
    page.close()

    mr_id = create_draft(space_id, ctx, round_id, domicile, exterieur)
    ensure_pre_match(space_id, mr_id, ctx, round_id, domicile, exterieur)
    ensure_inducements(space_id, mr_id)
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step4/actions",
        data={
            "turn": "3",
            "player_id": victime,
            "player_type": "regular",
            "action_type": "BLESSE",
            "injury_type": "SEQUEL",
            "sequel_stat": "MA",
        },
    )
    assert resp.status_code == 200, f"séquelle : {resp.status_code}\n{resp.text[:200]}"
    post_step5(space_id, mr_id)
    publish(space_id, mr_id)
    _attendre(
        lambda: query_db(
            f"SELECT participation_status FROM players_proj WHERE player_id = '{victime}'"
        )[0] == "MissingNextGame",
        "la séquelle est appliquée",
    )
    return {"space_id": space_id, "team_id": domicile, "mr_id": mr_id,
            "victime": victime, "avant": avant}


def test_une_sequelle_fait_perdre_un_cran_a_la_ligne(page: Page, match_a_sequelle):
    ctx = match_a_sequelle
    apres = _ligne(page, ctx["space_id"], ctx["team_id"], ctx["victime"])
    assert _ecarts(ctx["avant"], apres) == {
        "ma": (ctx["avant"]["ma"], str(_nombre(ctx["avant"]["ma"]) - 1))
    }, f"seul MV doit perdre un point : {ctx['avant']} → {apres}"


def test_depublier_le_rapport_rend_le_cran_a_la_ligne(page: Page, match_a_sequelle):
    """En dernier du groupe : il défait ce que le test précédent lit."""
    ctx = match_a_sequelle
    resp = requests.post(
        f"{BASE_URL}/app/{ctx['space_id']}/match-report/{ctx['mr_id']}/recap/unpublish",
        allow_redirects=False,
    )
    assert resp.status_code == 200, f"dépublication : {resp.status_code}"
    _attendre(
        lambda: query_db(
            f"SELECT participation_status FROM players_proj WHERE player_id = '{ctx['victime']}'"
        )[0] == "Available",
        "la compensation est appliquée",
    )
    apres = _ligne(page, ctx["space_id"], ctx["team_id"], ctx["victime"])
    assert _ecarts(ctx["avant"], apres) == {}, f"la séquelle reste : {ctx['avant']} → {apres}"


# ── L'augmentation achetée, et les customisations d'un commissaire ────────────


@pytest.fixture(scope="module")
def equipe_en_amelioration(browser, space_id):
    """Un match publié : un joueur domicile marque huit essais et entre en phase
    d'amélioration avec de quoi acheter une caractéristique.

    Huit essais : l'équipe domicile peut porter `BRAWLIN_BRUTES`, qui ne compte
    l'essai que 2 SPP — 16 couvrent le coût niveau 1 d'une caractéristique
    (14), comme dans `test_player_spp_spending.py`.
    """
    ctx = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    domicile, exterieur = ctx["team_ids"]
    round_id = ctx["round_ids"][0]
    marqueur = _joueurs(domicile)[0]

    mr_id = create_draft(space_id, ctx, round_id, domicile, exterieur)
    ensure_pre_match(space_id, mr_id, ctx, round_id, domicile, exterieur)
    ensure_inducements(space_id, mr_id)
    for turn in range(1, 9):
        record_action_api(space_id, mr_id, "home", marqueur, turn, "TOUCHDOWN")
    post_step5(space_id, mr_id)
    publish(space_id, mr_id)
    _attendre(lambda: _phase(domicile) == "PlayerImprovement", "phase d'amélioration")
    _attendre(
        lambda: int(query_db(f"SELECT spp FROM players_proj WHERE player_id = '{marqueur}'")[0]) > 0,
        "le marqueur est crédité",
    )
    return {"space_id": space_id, "domicile": domicile, "exterieur": exterieur,
            "marqueur": marqueur}


def test_une_augmentation_achetee_monte_d_un_cran_et_baisse_le_solde(
    page: Page, equipe_en_amelioration
):
    ctx = equipe_en_amelioration
    avant = _ligne(page, ctx["space_id"], ctx["domicile"], ctx["marqueur"])

    page.goto(
        f"{BASE_URL}/app/{ctx['space_id']}/players/{ctx['marqueur']}/detail", wait_until="load"
    )
    cliquer_quand_cable(page, ".btn-toggle-spp")
    expect(page.locator(".tabs")).to_be_visible(timeout=10000)
    page.locator(".tab", has_text="Caractéristiques").click()
    bouton = page.locator(".stat-card-btn:not([disabled])").first
    expect(bouton).to_be_visible(timeout=10000)
    with page.expect_navigation(wait_until="load"):
        bouton.click()
    reserve = page.locator(".spend-panel-remaining-val").inner_text().strip()

    apres = _ligne(page, ctx["space_id"], ctx["domicile"], ctx["marqueur"])
    ecarts = _ecarts(avant, apres)
    assert len(ecarts) == 1, f"une seule caractéristique doit bouger : {avant} → {apres}"
    (stat, (v_avant, v_apres)), = ecarts.items()
    assert abs(_nombre(v_apres) - _nombre(v_avant)) == 1, f"{stat} : {v_avant} → {v_apres}"
    assert int(apres["spp"]) < int(avant["spp"]), f"le solde ne baisse pas : {avant} → {apres}"
    assert apres["spp"] == reserve, f"ligne {apres['spp']}, fiche {reserve}"


# ── Customisations d'un commissaire (joueurs de l'équipe extérieure) ──────────


def _custo_url(ctx: dict, player_id: str, suffixe: str) -> str:
    return f"{BASE_URL}/app/{ctx['space_id']}/players/{player_id}/{suffixe}"


def _version(ctx: dict, player_id: str) -> int:
    html = requests.get(_custo_url(ctx, player_id, "widgets/customisation"), timeout=10).text
    m = re.search(r'name="expected_version" value="(\d+)"', html)
    assert m, "le panneau de customisation ne porte pas de version"
    return int(m.group(1))


def _customiser(ctx: dict, player_id: str, route: str, data: dict) -> None:
    """Ajoute au panier, puis valide — ce que fait le commissaire."""
    ajout = requests.post(
        _custo_url(ctx, player_id, f"customisation/{route}"),
        data={**data, "expected_version": _version(ctx, player_id)},
        headers=HX, timeout=10,
    )
    assert ajout.status_code == 200, ajout.text[:200]
    validation = requests.post(
        _custo_url(ctx, player_id, "customisation/validate"),
        data={"expected_version": _version(ctx, player_id)},
        headers=HX, timeout=10,
    )
    assert validation.status_code == 200, validation.text[:200]


def _retirer_la_customisation(ctx: dict, player_id: str) -> None:
    html = requests.get(_custo_url(ctx, player_id, "widgets/customisation"), timeout=10).text
    section = re.search(r"Customisations appliquées(.*?)Modifications en attente", html, re.S)
    assert section, "section « Customisations appliquées » absente"
    ids = re.findall(r'"customisation_id": "([^"]+)"', section.group(1))
    assert len(ids) == 1, f"une seule customisation attendue : {ids}"
    retrait = requests.post(
        _custo_url(ctx, player_id, "customisation/applied/remove"),
        data={"customisation_id": ids[0]},
        headers=HX, timeout=10,
    )
    assert retrait.status_code == 200, retrait.text[:200]


def test_une_caracteristique_customisee_puis_retiree(page: Page, equipe_en_amelioration):
    ctx = equipe_en_amelioration
    joueur = _joueurs(ctx["exterieur"])[0]
    avant = _ligne(page, ctx["space_id"], ctx["exterieur"], joueur)

    _customiser(ctx, joueur, "stats/add", {"stat": "ma", "crans": 1})
    pendant = _ligne(page, ctx["space_id"], ctx["exterieur"], joueur)
    assert _ecarts(avant, pendant) == {"ma": (avant["ma"], str(_nombre(avant["ma"]) + 1))}, (
        f"MV doit gagner un point : {avant} → {pendant}"
    )

    _retirer_la_customisation(ctx, joueur)
    apres = _ligne(page, ctx["space_id"], ctx["exterieur"], joueur)
    assert _ecarts(avant, apres) == {}, f"le retrait ne rend pas la ligne : {avant} → {apres}"


def test_des_spp_customises_puis_retires(page: Page, equipe_en_amelioration):
    ctx = equipe_en_amelioration
    joueur = _joueurs(ctx["exterieur"])[1]
    avant = _ligne(page, ctx["space_id"], ctx["exterieur"], joueur)

    _customiser(ctx, joueur, "spp/add", {"amount": 3})
    pendant = _ligne(page, ctx["space_id"], ctx["exterieur"], joueur)
    assert int(pendant["spp"]) == int(avant["spp"]) + 3, f"{avant} → {pendant}"

    _retirer_la_customisation(ctx, joueur)
    apres = _ligne(page, ctx["space_id"], ctx["exterieur"], joueur)
    assert apres["spp"] == avant["spp"], f"le retrait ne rend pas le solde : {avant} → {apres}"

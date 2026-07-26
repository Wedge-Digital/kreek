"""Test E2E — départages appliqués au classement affiché.

Vérifie ce qu'aucun test unitaire ne voit : la chaîne complète configuration de
la compétition → publication de matchs → app event → projection → widget rendu
en navigateur. La comparaison elle-même est couverte unitairement
(`domain/standings.rs`), son câblage aussi (`builders.rs`) ; ce qui reste
invérifiable ailleurs, c'est que l'ordre et les rangs **affichés** soient ceux
que le domaine a calculés.

Aucun critère n'est configuré à la main : le formulaire de phase 2 active les
sept critères par défaut, dans l'ordre canonique — `diff_td` est donc déjà le
critère n°1.

**Les bonus, eux, doivent être explicitement désactivés** : le formulaire coche
les bonus offensif et défensif par défaut. Avec eux, un vainqueur 3-0 marque un
point de plus qu'un vainqueur 1-0, et un perdant 0-1 un de plus qu'un perdant
0-3 : les équipes ne sont jamais à égalité, le classement est décidé par les
seuls points, et un test de départage passerait au vert sans rien départager.
C'est exactement ce qui s'est produit à la première écriture de ce fichier.

Les égalités de points sont ensuite obtenues par **construction** : deux équipes
comptant chacune exactement une victoire ont le même total quel que soit le
barème V/N/D configuré.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) sur :3210, seed
`make seed_e2e`, et surtout **base réinitialisée** — les lignes de classement
antérieures à la migration des compteurs de départage les ont tous à 0 et
rendraient toutes les équipes ex æquo.
"""

import re

import pytest
from playwright.sync_api import Page

from competition_lifecycle import BASE_URL, build_full_competition
from match_report_helpers import play_match, wait_ranking_lines

_TEAM_ID_RE = re.compile(r"[0-9A-Z]{26}")


# ── Lecture du widget ─────────────────────────────────────────────────────────


def _open_standings(page: Page, ctx) -> None:
    url = f"{BASE_URL}/app/{ctx['space_id']}/competitions/{ctx['competition_id']}/{ctx['season_id']}"
    page.goto(url, wait_until="load")
    page.wait_for_selector(".ranking-classement-widget .standings-row", timeout=10000)


def _standings(page: Page) -> list[tuple[int, str]]:
    """(rang, team_id) pour chaque ligne, dans l'ordre d'affichage.

    Le rang est extrait des seuls chiffres : la cellule du rang 1 est préfixée
    d'un 🏆 par le template. L'équipe est lue dans le `hx-get` de la ligne
    plutôt que par son nom — l'id est ce que les fixtures connaissent, et un
    nom d'équipe tronqué à l'affichage ne fausserait pas l'assertion.
    """
    rows = page.locator(".ranking-classement-widget .standings-row")
    result = []
    for i in range(rows.count()):
        row = rows.nth(i)
        rank = int(re.sub(r"\D", "", row.locator(".standings-rank").inner_text()))
        team_id = _TEAM_ID_RE.findall(row.get_attribute("hx-get"))[-1]
        result.append((rank, team_id))
    return result


# ── Fixtures — une compétition par scénario ───────────────────────────────────
# Les lignes de classement s'accumulent par saison : deux scénarios dans la même
# compétition se marcheraient dessus.


@pytest.fixture(scope="module")
def order_ctx(browser, space_id):
    """4 équipes, 2 journées — pour le scénario « le critère ordonne »."""
    ctx = build_full_competition(browser, space_id, num_teams=4, num_rounds=2, with_default_bonuses=False)
    ctx["space_id"] = space_id
    return ctx


@pytest.fixture(scope="module")
def tie_ctx(browser, space_id):
    """4 équipes, 4 journées — pour le scénario « ex æquo et numérotation »."""
    ctx = build_full_competition(browser, space_id, num_teams=4, num_rounds=4, with_default_bonuses=False)
    ctx["space_id"] = space_id
    return ctx


# ── Scénario 1 — le premier critère ordonne des équipes à égalité de points ───


def test_first_criterion_orders_teams_tied_on_points(page: Page, order_ctx, console_errors):
    """Deux victoires d'ampleurs différentes, deux défaites d'ampleurs
    différentes : les points ne séparent personne au sein de chaque paire, seule
    la différence de touchdowns le fait.

    Le couple des perdants est le plus révélateur : t3 (−1) doit passer devant
    t1 (−3). Un sens de comparaison inversé les permuterait sans rien casser
    d'autre — et le classement resterait d'apparence plausible.
    """
    teams = order_ctx["team_ids"]
    rounds = order_ctx["round_ids"]

    play_match(order_ctx["space_id"], order_ctx, rounds[0], teams[0], teams[1], home_td=3, away_td=0)
    play_match(order_ctx["space_id"], order_ctx, rounds[1], teams[2], teams[3], home_td=1, away_td=0)
    wait_ranking_lines(order_ctx["season_id"], expected_lines=4)

    _open_standings(page, order_ctx)

    # t0 (+3) puis t2 (+1) : deux vainqueurs à égalité de points.
    # t3 (−1) puis t1 (−3) : deux perdants à égalité de points.
    assert _standings(page) == [(1, teams[0]), (2, teams[2]), (3, teams[3]), (4, teams[1])]


# ── Scénario 2 — ex æquo et numérotation standard (règles 19 et 20) ───────────


def test_teams_equal_on_every_criterion_share_a_rank_and_the_next_one_skips(
    page: Page, tie_ctx, console_errors
):
    """Quatre matchs 2-0 produisant un classement 1, 2, 2, 4.

    t1 et t2 finissent chacune sur une victoire et une défaite, 2 touchdowns
    marqués et 2 encaissés. Aucune sortie, agression ni passe n'étant
    enregistrée, les cinq autres compteurs sont à 0 des deux côtés : elles sont
    strictement égales sur les **sept** critères actifs, donc ex æquo (règle 19).

    L'équipe suivante affiche alors 4 et non 3 (règle 20) — c'est le seul
    endroit où la numérotation standard se voit vraiment, un ex æquo en fin de
    classement ne prouverait rien du saut.
    """
    teams = tie_ctx["team_ids"]
    rounds = tie_ctx["round_ids"]
    space_id = tie_ctx["space_id"]

    play_match(space_id, tie_ctx, rounds[0], teams[0], teams[1], home_td=2, away_td=0)
    play_match(space_id, tie_ctx, rounds[1], teams[0], teams[2], home_td=2, away_td=0)
    play_match(space_id, tie_ctx, rounds[2], teams[1], teams[3], home_td=2, away_td=0)
    play_match(space_id, tie_ctx, rounds[3], teams[2], teams[3], home_td=2, away_td=0)
    wait_ranking_lines(tie_ctx["season_id"], expected_lines=8)

    _open_standings(page, tie_ctx)
    standings = _standings(page)

    assert [rank for rank, _ in standings] == [1, 2, 2, 4], (
        f"numérotation standard attendue après un ex æquo, obtenu {standings}"
    )
    assert standings[0][1] == teams[0], "t0 (2 victoires) doit mener le classement"
    # L'ordre entre les deux ex æquo n'est pas garanti — le tri est stable, donc
    # il reflète l'ordre d'entrée, un détail d'implémentation qu'on n'assert pas.
    assert {standings[1][1], standings[2][1]} == {teams[1], teams[2]}
    assert standings[3][1] == teams[3], "t3 (2 défaites) doit fermer le classement"


# ── Le trophée suit le rang 1, ex æquo compris ────────────────────────────────


def test_every_team_ranked_first_gets_the_trophy(page: Page, tie_ctx, console_errors):
    """Le template préfixe d'un 🏆 toute ligne de rang 1. Ici une seule équipe
    est première ; le test fixe la correspondance rang 1 ⇄ trophée, qui devient
    visible autrement le jour où deux équipes se partagent la tête.
    """
    _open_standings(page, tie_ctx)

    rows = page.locator(".ranking-classement-widget .standings-row")
    for i in range(rows.count()):
        cell = rows.nth(i).locator(".standings-rank").inner_text()
        rank = int(re.sub(r"\D", "", cell))
        assert ("🏆" in cell) == (rank == 1), f"trophée incohérent avec le rang {rank} : {cell!r}"

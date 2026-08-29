"""Test E2E — onglet « Classement détaillé ».

Vérifie ce qu'aucun test unitaire ne peut voir : que la mise en évidence du
critère décisif atterrit sur la **bonne cellule du bon tableau**. La résolution
elle-même (`tiebreak_outcomes`) est couverte dans le domaine, et sa traduction en
états de cellules dans `builders.rs` ; ce qui reste invérifiable ailleurs, c'est
le rendu HTML réel.

Piège hérité de la carte 219 : le formulaire de phase 2 coche les bonus offensif
et défensif par défaut. Avec eux, un vainqueur 3-0 totalise un point de plus
qu'un vainqueur 1-0 — aucune équipe n'est jamais à égalité de points, le
classement est décidé par les seuls points, et un test de départage passe au vert
sans rien départager. D'où `with_default_bonuses=False` partout où une égalité
est nécessaire.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) sur :3210, `make
seed_e2e`. Aucun `reset_db` : chaque test crée sa propre saison, et le classement
est filtré par `season_id` — les lignes antérieures ne peuvent pas fuiter.
"""

import pytest
from playwright.sync_api import Page

from competition_lifecycle import BASE_URL, build_full_competition
from match_report_helpers import play_match, wait_ranking_lines

# Nombre de colonnes fixes avant le bloc des départages :
# # / Équipe / MJ / G / N / D / Bonus / Manuel / Total.
#
# **À mettre à jour dès qu'une colonne s'ajoute avant les départages.** Sans
# cela, `_tiebreak_headers` décale sa fenêtre et rend une colonne fixe comme si
# c'était un critère — ce qui s'est produit à l'ajout de « Manuel » (carte 451).
FIXED_COLUMNS = 9

DECISIVE = "sd-decisive"
TIED = "sd-tied"


# ── Lecture du tableau ────────────────────────────────────────────────────────


def _open_detailed_tab(page: Page, ctx) -> None:
    url = (
        f"{BASE_URL}/app/{ctx['space_id']}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/detailed-standings"
    )
    page.goto(url, wait_until="load")
    page.wait_for_selector(".ranking-detailed-standings-widget .sd-table tbody tr", timeout=10000)


def _rows(page: Page):
    return page.locator(".ranking-detailed-standings-widget .sd-table tbody tr")


def _tiebreak_states(row) -> list[str]:
    """État de chaque cellule de départage d'une ligne, dans l'ordre des colonnes.

    `sd-decisive`, `sd-tied`, ou `""` pour une cellule neutre.
    """
    cells = row.locator("td.sd-tb")
    states = []
    for i in range(cells.count()):
        classes = (cells.nth(i).get_attribute("class") or "").split()
        states.append(next((c for c in classes if c in (DECISIVE, TIED)), ""))
    return states


def _tiebreak_headers(page: Page) -> list[str]:
    headers = page.locator(".ranking-detailed-standings-widget .sd-col-head th")
    return [headers.nth(i).inner_text().strip() for i in range(FIXED_COLUMNS, headers.count())]


def _ranks(page: Page) -> list[int]:
    rows = _rows(page)
    return [int(rows.nth(i).locator("td.sd-rank").inner_text().strip()) for i in range(rows.count())]


# ── Fixtures — une compétition par scénario ───────────────────────────────────


@pytest.fixture(scope="module")
def decisive_ctx(browser, space_id):
    """4 équipes, 2 journées, sans bonus — pour l'égalité départagée."""
    ctx = build_full_competition(browser, space_id, num_teams=4, num_rounds=2, with_default_bonuses=False)
    ctx["space_id"] = space_id
    return ctx


@pytest.fixture(scope="module")
def tie_ctx(browser, space_id):
    """4 équipes, 4 journées, sans bonus — pour l'ex æquo total."""
    ctx = build_full_competition(browser, space_id, num_teams=4, num_rounds=4, with_default_bonuses=False)
    ctx["space_id"] = space_id
    return ctx


@pytest.fixture(scope="module")
def partial_criteria_ctx(browser, space_id):
    """2 équipes, 1 journée, avec deux critères de départage décochés."""
    ctx = build_full_competition(
        browser,
        space_id,
        num_teams=2,
        num_rounds=1,
        deactivated_tiebreaks=["Nombre de fautes commises", "Nombre de réussites"],
    )
    ctx["space_id"] = space_id
    return ctx


# ── Scénario 1 — le critère décisif est mis en évidence ───────────────────────


def test_the_decisive_criterion_is_highlighted(page: Page, decisive_ctx, console_errors):
    """Deux vainqueurs et deux perdants, chaque paire à égalité de points et
    séparée par la seule différence de touchdowns.

    Le premier critère du catalogue étant `diff_td`, c'est la première colonne de
    départage qui doit porter la mise en évidence — sur les quatre lignes, chacune
    appartenant à une paire départagée. Aucune autre colonne ne doit la porter.
    """
    teams = decisive_ctx["team_ids"]
    rounds = decisive_ctx["round_ids"]
    space_id = decisive_ctx["space_id"]

    play_match(space_id, decisive_ctx, rounds[0], teams[0], teams[1], home_td=3, away_td=0)
    play_match(space_id, decisive_ctx, rounds[1], teams[2], teams[3], home_td=1, away_td=0)
    wait_ranking_lines(decisive_ctx["season_id"], expected_lines=4)

    _open_detailed_tab(page, decisive_ctx)

    rows = _rows(page)
    assert rows.count() == 4
    for i in range(rows.count()):
        states = _tiebreak_states(rows.nth(i))
        assert states[0] == DECISIVE, f"ligne {i} : la différence de TD doit départager, obtenu {states}"
        assert DECISIVE not in states[1:], f"ligne {i} : une seule colonne décisive, obtenu {states}"


# ── Scénario 2 — ex æquo total ────────────────────────────────────────────────


def test_a_full_tie_highlights_nothing_and_shares_a_rank(page: Page, tie_ctx, console_errors):
    """Quatre matchs 2-0 : les deux équipes du milieu finissent sur une victoire
    et une défaite, 2 touchdowns marqués et 2 encaissés, aucune sortie, agression
    ni passe — donc strictement égales sur les sept critères.

    Elles partagent le rang 2, et aucune de leurs cellules n'est mise en
    évidence : toutes sont marquées égales (règle 22).
    """
    teams = tie_ctx["team_ids"]
    rounds = tie_ctx["round_ids"]
    space_id = tie_ctx["space_id"]

    play_match(space_id, tie_ctx, rounds[0], teams[0], teams[1], home_td=2, away_td=0)
    play_match(space_id, tie_ctx, rounds[1], teams[0], teams[2], home_td=2, away_td=0)
    play_match(space_id, tie_ctx, rounds[2], teams[1], teams[3], home_td=2, away_td=0)
    play_match(space_id, tie_ctx, rounds[3], teams[2], teams[3], home_td=2, away_td=0)
    wait_ranking_lines(tie_ctx["season_id"], expected_lines=8)

    _open_detailed_tab(page, tie_ctx)

    assert _ranks(page) == [1, 2, 2, 4]

    rows = _rows(page)
    for i in (1, 2):
        states = _tiebreak_states(rows.nth(i))
        assert DECISIVE not in states, f"ex æquo : aucune colonne décisive attendue, obtenu {states}"
        assert set(states) == {TIED}, f"ex æquo : toutes les colonnes égales attendues, obtenu {states}"


# ── Scénario 3 — seules les colonnes des critères actifs sont affichées ───────


def test_only_activated_criteria_get_a_column(page: Page, partial_criteria_ctx, console_errors):
    """Deux critères décochés en phase 2 : le tableau n'affiche que les cinq
    restants, numérotés de 1 à 5 dans l'ordre canonique.

    C'est le seul test qui couvre la propagation de la configuration jusqu'aux
    en-têtes : sans elle, le tableau afficherait sept colonnes quelle que soit la
    compétition.
    """
    teams = partial_criteria_ctx["team_ids"]
    space_id = partial_criteria_ctx["space_id"]

    play_match(
        space_id,
        partial_criteria_ctx,
        partial_criteria_ctx["round_ids"][0],
        teams[0],
        teams[1],
        home_td=1,
        away_td=0,
    )
    wait_ranking_lines(partial_criteria_ctx["season_id"], expected_lines=2)

    _open_detailed_tab(page, partial_criteria_ctx)

    headers = _tiebreak_headers(page)
    assert len(headers) == 5, f"cinq critères restants attendus, obtenu {headers}"
    assert [h.split(" · ")[0] for h in headers] == ["1", "2", "3", "4", "5"]
    # Les libellés courts des deux critères décochés ne doivent plus apparaître.
    joined = " ".join(headers)
    assert "Ftes" not in joined and "Réu" not in joined, headers

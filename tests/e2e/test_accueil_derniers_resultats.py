"""Test E2E — widget "Derniers résultats" sur la page d'accueil d'un espace.

Le widget (BC competitions, chargé par la page d'accueil du BC news) doit
afficher les derniers matchs terminés d'un espace, toutes compétitions
confondues, triés par date réelle de publication, avec le highlight du
vainqueur neutre en cas d'égalité, et un lien vers le rapport visible
seulement pour un utilisateur autorisé. C'est ce qu'aucun test unitaire ne
peut voir : que le fragment HTMX réel, une fois rendu dans un navigateur,
combine correctement des données de plusieurs compétitions.

Un seul scénario construit deux compétitions distinctes dans l'espace
partagé de la suite et y publie un match chacune (un match décisif, un match
nul) : ça suffit à exercer en une fois le multi-compétitions, le tri
chronologique, le highlight vainqueur/nul et l'autorisation du lien —
inutile de payer le coût de deux compétitions par scénario séparé.

Pas de scénario "état vide" dédié : l'espace de la suite (« Espace E2E »)
est partagé entre tous les fichiers e2e et jamais réinitialisé entre deux
exécutions (cf. docstring de `competition_lifecycle.py`) — un espace garanti
sans aucun résultat n'existe pas dans ce modèle. Le rendu de l'état vide
(condition triviale côté template) reste couvert par la relecture du code.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) sur :3210, `make
seed_e2e`.
"""

import time

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import play_match


def _wait_round_completed(round_id: str, timeout_s: int = 20) -> None:
    """Attend que la projection competitions (asynchrone via l'app event
    MatchReportPublished) marque le pairing de cette journée comme terminé."""
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        rows = query_db(
            f"SELECT match_status FROM competition_match_display_proj "
            f"WHERE round_id = '{round_id}'"
        )
        if rows and rows[0] == "completed":
            return
        time.sleep(0.3)
    raise AssertionError(f"round {round_id} jamais passé à completed après {timeout_s}s")


@pytest.fixture(scope="module")
def two_competitions_with_results(browser, space_id):
    """Deux compétitions dédiées du même espace, chacune avec un match publié —
    l'une décisive, l'autre nulle, publiée après pour vérifier le tri."""
    decisive = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    draw = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)

    play_match(
        space_id,
        decisive,
        decisive["round_ids"][0],
        decisive["team_ids"][0],
        decisive["team_ids"][1],
        home_td=2,
        away_td=0,
    )
    _wait_round_completed(decisive["round_ids"][0])

    play_match(
        space_id,
        draw,
        draw["round_ids"][0],
        draw["team_ids"][0],
        draw["team_ids"][1],
        home_td=1,
        away_td=1,
    )
    _wait_round_completed(draw["round_ids"][0])

    return {"space_id": space_id, "decisive": decisive, "draw": draw}


def test_widget_affiche_les_resultats_de_plusieurs_competitions(
    page: Page, two_competitions_with_results, console_errors
):
    """Match décisif + match nul, publiés dans deux compétitions différentes
    du même espace : les deux apparaissent sur l'accueil, le nul (publié en
    second) avant le décisif, avec le bon highlight et un lien cliquable
    (DevCoach est admin des deux compétitions qu'il vient de créer)."""
    ctx = two_competitions_with_results
    page.goto(f"{BASE_URL}/app/{ctx['space_id']}/home", wait_until="load")
    page.wait_for_selector(
        "#latest-results-widget .match-result, #latest-results-widget .matches-panel-empty",
        timeout=10000,
    )

    decisive_row = page.locator(".match-result", has_text=ctx["decisive"]["name"])
    draw_row = page.locator(".match-result", has_text=ctx["draw"]["name"])
    expect(decisive_row).to_have_count(1)
    expect(draw_row).to_have_count(1)

    # Highlight vainqueur : le match décisif a exactement un score en évidence.
    expect(decisive_row.locator(".match-score.winner")).to_have_count(1)
    # Nul : aucun score n'est mis en évidence (règle validée en phase design).
    expect(draw_row.locator(".match-score.winner")).to_have_count(0)

    # Tri chronologique : le nul, publié après, apparaît avant le décisif.
    # text_content() (DOM brut) et non inner_text() : .match-league est en
    # `text-transform: uppercase`, inner_text() suivrait le rendu et casserait
    # la comparaison de casse avec ctx["..."]["name"].
    all_results = page.locator("#latest-results-widget .match-result")
    texts = [all_results.nth(i).text_content() or "" for i in range(all_results.count())]
    draw_index = next(i for i, t in enumerate(texts) if ctx["draw"]["name"] in t)
    decisive_index = next(i for i, t in enumerate(texts) if ctx["decisive"]["name"] in t)
    assert draw_index < decisive_index, "le résultat le plus récent doit apparaître en premier"

    # Autorisation : DevCoach est admin des deux compétitions qu'il a créées
    # (cf. competition_lifecycle.create_full_competition) — le résultat doit
    # donc être un lien cliquable, pas un simple <div>.
    expect(page.locator("a.match-result", has_text=ctx["decisive"]["name"])).to_have_count(1)

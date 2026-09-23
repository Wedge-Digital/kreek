"""La phase 1 d'un rapport existant, en édition (carte 565).

# Le `season_id` en double

En édition, l'URL du widget d'équipes porte déjà `season_id`, `selected_home` et
`selected_away`. Au chargement, le widget de compétition pré-rempli émet
`matchContextSelected`, et la page rechargeait le widget d'équipes en
**ajoutant** un second `season_id` à cette URL. axum refuse un champ en double
(`duplicate field season_id`) : la requête partait en 400, et changer ensuite de
saison ne rechargeait plus jamais les équipes.

Le test choisit une saison, ce qui relance la cascade jusqu'au widget
d'équipes, et exige que la requête ne porte la saison qu'une fois et aboutisse.
"""

from urllib.parse import parse_qs, urlparse

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db


@pytest.fixture(scope="module")
def competition(browser, space_id):
    return build_full_competition(browser, space_id, 2, 1)


def _rapport_de_la_journee(round_id: str) -> str:
    lignes = query_db(
        "SELECT match_report_id FROM match_report_proj "
        f"WHERE round_id = '{round_id}' LIMIT 1"
    )
    assert lignes, "la journée doit porter un rapport, créé avec son appariement"
    return lignes[0].strip()


def _est_le_widget_d_equipes(r) -> bool:
    """Le fragment, pas sa source JSON (`…/team-selection/json`)."""
    return urlparse(r.url).path.endswith("/team/widgets/team-selection")


def _choisir_la_saison(page: Page) -> None:
    """Choisir la saison relance la cascade : la journée se recharge, se
    sélectionne d'elle-même (`auto-select-first`) et émet
    `matchContextSelected` — l'événement qui rechargeait le widget d'équipes."""
    select = page.locator("kreek-select[name='season_id']")
    select.locator(".ks-control").click()
    select.locator(".ks-option:not(.ks-empty)").first.click()


def test_le_widget_d_equipes_ne_recoit_la_saison_qu_une_fois(
    page: Page, space_id, competition
):
    """**Le geste, pas le chargement.** Au chargement, l'événement ne part que
    si la journée a branché son écouteur avant que la saison ne s'annonce —
    une course, qui explique le « parfois » constaté en production et que ce
    poste perd. Choisir une saison l'émet à coup sûr.
    """
    mr_id = _rapport_de_la_journee(competition["round_ids"][0])
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}", wait_until="load")
    expect(page.locator("#teams-container .team-selection-widget")).to_be_attached(timeout=15000)

    with page.expect_response(_est_le_widget_d_equipes, timeout=15000) as attendue:
        _choisir_la_saison(page)
    r = attendue.value

    saisons = parse_qs(urlparse(r.url).query).get("season_id", [])
    assert len(saisons) == 1, f"season_id en double : {r.url}"
    assert r.status == 200, f"{r.status} sur {r.url}"

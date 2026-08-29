"""L'étape 3 du magicien — poules et calendrier (carte 412).

La phase finale a quitté cet écran : rien dans kreek ne s'en servait, et il
fallait pourtant la remplir pour créer une compétition.

**Ce que ces tests voient et qu'aucun test unitaire ne peut voir** : l'écran, et
la structure que le magicien réel écrit en base au bout de son parcours.

**Ce qu'ils ne voient pas, et qu'il faut savoir.** Un JS qui poserait encore
`play_offs_phase` dans sa charge utile passerait inaperçu : le serveur jette les
champs inconnus, et la structure enregistrée resterait propre. Mesuré en
falsifiant. C'est la contrepartie exacte de la tolérance dont cette carte dépend
pour lire les 3330 structures anciennes — elle vaut aussi pour le front, et
aucun contrôle ne s'y oppose.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import json

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db


@pytest.fixture(scope="module")
def competition(browser, space_id):
    """Une compétition créée **par le magicien réel**, étape 3 comprise.

    C'est ce parcours-là qu'on teste : une structure écrite à la main en base ne
    dirait rien du JS qui la compose.
    """
    ctx = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    ctx["space_id"] = space_id
    return ctx


def _url_etape_3(ctx: dict) -> str:
    return (
        f"{BASE_URL}/app/{ctx['space_id']}/competitions/create/"
        f"{ctx['competition_id']}/{ctx['season_id']}/structure"
    )


def test_l_etape_3_n_a_plus_de_section_phase_finale(page: Page, competition, console_errors):
    """L'écran, et sa renumérotation.

    Retirer la section sans renuméroter aurait laissé « 1, 3, 4 » — un saut que
    le lecteur attribue à un défaut d'affichage plutôt qu'à un choix.
    """
    page.goto(_url_etape_3(competition), wait_until="load")
    expect(page.locator(".section-title").first).to_be_visible(timeout=10000)

    sections = [t.strip() for t in page.locator(".section-title").all_inner_texts()]
    assert [s.split(" — ")[0] for s in sections] == ["1", "2", "3"], sections
    assert not any("PHASE FINALE" in s or "PLAY-OFF" in s.upper() for s in sections), sections

    assert page.locator("#playoff-mode-btns, #playoff-config").count() == 0
    assert page.locator("#playoff-start-date, #playoff-end-date").count() == 0
    assert "play-offs" not in page.locator(".sub").first.inner_text()


def test_la_structure_ecrite_ne_porte_plus_la_phase_finale(competition):
    """La structure écrite au bout du parcours réel du magicien.

    **Ce test ne dit rien de ce que le JS envoie** : le serveur jette les champs
    inconnus, donc un front resté en arrière produirait la même structure propre.
    Vérifié en falsifiant. Ce qu'il garde, c'est le bout de la chaîne — que le
    domaine, le handler et le dépôt s'accordent pour n'écrire que deux clés.
    """
    structure = json.loads(
        query_db(
            f"SELECT structure FROM competition_seasons WHERE id = '{competition['season_id']}'"
        )[0]
    )

    assert sorted(structure.keys()) == ["ranking_group", "schedule"], structure.keys()
    for cle in ("play_off_start_date", "play_off_end_date"):
        assert cle not in structure["schedule"], structure["schedule"].keys()


# ── Ce que ce fichier ne teste pas, et pourquoi ───────────────────────────────
#
# **Que les structures d'avant le retrait restent lisibles** n'est pas ici.
# C'est une affaire de serde, pas d'écran, et le test unitaire
# `retrait_phase_finale_tests::une_structure_deja_enregistree_se_lit_toujours`
# la couvre sur une structure copiée telle quelle de la base — en rougissant
# sous la mutation `deny_unknown_fields`.
#
# Une version e2e a été écrite, puis retirée : elle interrogeait la page
# d'administration d'une saison ancienne et se contentait d'un `200`. Or cette
# page rend `200` que la structure ait été lue ou non — vérifié en falsifiant :
# sous `deny_unknown_fields`, elle répondait toujours `200`, la structure ayant
# simplement disparu de son contenu. Le test passait dans les deux cas.

"""Tests E2E du widget de réglage des notifications (carte 332).

Le widget est monté dans l'onglet Synthèse de l'administration, en mode
auto-save : chaque bascule POSTe et rend `204`, sans re-rendre le fragment.

Ce que ces tests gardent, et qu'aucun test unitaire ne verrait :

- la bascule **persiste** — c'est le seul chemin d'édition de ces réglages, le
  POST du magicien ayant été jusqu'ici leur unique écrivain ;
- une case décochée n'est **pas envoyée** par un formulaire HTML. Le handler
  doit donc la lire comme `false` et non l'ignorer ; une erreur ici laisserait
  activer une notification sans jamais pouvoir la désactiver, ce qui
  ressemblerait à un défaut de persistance ;
- `update_notifications.sql` **n'écrit pas `status`**, contrairement à ses deux
  voisins : sans ça, régler une notification ferait retomber une compétition
  vivante dans le magicien.

`create_full_competition` ne pose que des journées **à date fixe** et ne fixe
jamais de date limite : les deux jeux de données dont ces tests ont besoin
s'obtiennent donc en faisant seulement varier `num_rounds`.

Prérequis : serveur kreek lancé en dev.
"""

import re

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import create_full_competition

BASE_URL = "http://localhost:3210"


def _ouvrir_synthese(page: Page, space_id: str, comp: dict) -> None:
    """Par la page d'admin et son onglet, jamais par `/admin/summary` en direct :
    cette URL rend un **fragment**, sans layout donc sans HTMX, et le
    `hx-trigger="load"` du conteneur du widget n'y partirait jamais."""
    page.goto(
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{comp['competition_id']}/{comp['season_id']}/admin",
        wait_until="load",
    )
    page.locator(".admin-tab", has_text="Résumé").click()
    page.wait_for_selector(".notification-settings", timeout=10000)
    # Alpine pose les motifs à son `init()` ; sans cette attente, les assertions
    # de grisage liraient un DOM encore nu.
    page.wait_for_timeout(500)


def _case(page: Page, cle: str):
    return page.locator(f".notification-settings .ns-check[name='{cle}']")


def _ligne(page: Page, cle: str):
    return page.locator(f".notification-settings .ns-row[data-cle='{cle}']")


def _basculer(page: Page, cle: str, cocher: bool) -> None:
    """Le POST est en `hx-swap="none"` et rend 204 : sans attendre la réponse,
    l'assertion de persistance courrait la requête."""
    with page.expect_response(
        lambda r: "/notifications" in r.url and r.request.method == "POST"
    ):
        if cocher:
            _case(page, cle).check()
        else:
            _case(page, cle).uncheck()


def _creer(browser, space_id: str, num_rounds: int) -> dict:
    """`competition_create_url` est de portée fonction : une fixture de module ne
    peut pas la demander. On refait l'URL, qui n'est qu'un `format`."""
    page = browser.new_page()
    try:
        return create_full_competition(
            page, f"{BASE_URL}/app/{space_id}/competitions/create", num_rounds=num_rounds
        )
    finally:
        page.close()


@pytest.fixture(scope="module")
def avec_calendrier(browser, space_id):
    """Deux journées à date fixe : `round_eve` est applicable, `round_closing`
    ne l'est pas — une journée à date fixe n'a pas de fenêtre à clore."""
    return _creer(browser, space_id, num_rounds=2)


@pytest.fixture(scope="module")
def sans_calendrier(browser, space_id):
    """Zéro journée : un calendrier vide n'est pas un calendrier."""
    return _creer(browser, space_id, num_rounds=0)


# ── Scénario 1 — une bascule persiste, dans les deux sens ────────────────────


def test_une_bascule_persiste_apres_rechargement(page: Page, space_id, avec_calendrier):
    _ouvrir_synthese(page, space_id, avec_calendrier)

    # Défaut d'une saison neuve : tout allumé (R8).
    expect(_case(page, "round_eve")).to_be_checked()

    # Décocher est le sens qui compte : une case non cochée n'est pas envoyée
    # par le formulaire, et c'est `#[serde(default)]` qui la rend `false`.
    _basculer(page, "round_eve", cocher=False)

    _ouvrir_synthese(page, space_id, avec_calendrier)
    expect(_case(page, "round_eve")).not_to_be_checked()
    # Les trois autres n'ont pas bougé : le POST porte bien les quatre champs.
    expect(_case(page, "registration_open")).to_be_checked()
    expect(_case(page, "round_closing")).to_be_checked()
    expect(_case(page, "registration_deadline")).to_be_checked()

    # Et le retour en arrière fonctionne — sans quoi on pourrait éteindre sans
    # jamais rallumer, ce qui est le symptôme exact d'un `Json` à la place d'un
    # `Form`.
    _basculer(page, "round_eve", cocher=True)
    _ouvrir_synthese(page, space_id, avec_calendrier)
    expect(_case(page, "round_eve")).to_be_checked()


# ── Scénario 2 — les lignes inapplicables sont grisées, avec leur motif ──────


def test_sans_calendrier_les_lignes_de_journee_sont_grisees_avec_leur_motif(
    page: Page, space_id, sans_calendrier
):
    _ouvrir_synthese(page, space_id, sans_calendrier)

    for cle in ("round_eve", "round_closing", "registration_deadline"):
        expect(_ligne(page, cle)).to_have_class(re.compile(r"ns-row--inapplicable"))
        expect(_ligne(page, cle).locator(".ns-motif")).to_be_visible()
        expect(_ligne(page, cle).locator(".ns-motif")).not_to_have_text("")

    # R6 en situation : grisée ne veut dire ni décochée, ni désactivée. La case
    # doit rester actionnable — on règle aujourd'hui ce qui s'appliquera quand
    # le calendrier existera.
    expect(_case(page, "round_eve")).to_be_checked()
    expect(_case(page, "round_eve")).to_be_enabled()

    # L'ouverture des inscriptions est applicable en toute circonstance : une
    # compétition a par construction une ouverture.
    expect(_ligne(page, "registration_open").locator(".ns-motif")).to_be_hidden()


def test_avec_des_journees_a_date_fixe_seule_la_cloture_est_grisee(
    page: Page, space_id, avec_calendrier
):
    """La distinction entre les deux notifications de journée : une journée
    `fixed_date` ne porte qu'une date de multiplexe, elle n'a pas de fenêtre à
    clore."""
    _ouvrir_synthese(page, space_id, avec_calendrier)

    expect(_ligne(page, "round_eve")).not_to_have_class(re.compile(r"ns-row--inapplicable"))
    expect(_ligne(page, "round_closing")).to_have_class(re.compile(r"ns-row--inapplicable"))


# ── Scénario 3 — le statut de la saison n'est pas touché ─────────────────────


def test_apres_une_bascule_la_competition_reste_atteignable(
    page: Page, space_id, avec_calendrier
):
    """Non-régression du statut. `update_structure` et `update_invitations`
    écrivent `status` ; `update_notifications` ne le fait pas. S'il le faisait,
    la saison retomberait dans le magicien — un dégât invisible depuis l'écran
    de réglage lui-même."""
    _ouvrir_synthese(page, space_id, avec_calendrier)
    _basculer(page, "round_closing", cocher=False)

    page.goto(
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{avec_calendrier['competition_id']}/{avec_calendrier['season_id']}",
        wait_until="load",
    )

    # Une saison retombée en brouillon redirigerait vers le magicien.
    assert "/create/" not in page.url, f"la saison est retombée dans le magicien : {page.url}"
    expect(page.locator("body")).to_contain_text(avec_calendrier["name"])

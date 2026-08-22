"""Tests E2E du widget de notifications à l'étape 4 du magicien (carte 333).

Le widget y est en mode **différé** : il n'enregistre pas lui-même, il émet, et
c'est `submitInvitations()` qui persiste — comme tout le reste de cette étape,
qui s'enregistre d'un bloc. Un auto-save y ferait cohabiter deux comportements
de sauvegarde dans le même écran : un « ← Retour » laisserait les cases
persistées et la date limite perdue.

Le scénario qui compte est `test_revalider_sans_toucher_aux_cases_ne_les_ecrase_pas`.
Le widget étant rendu par le serveur, les cases reviennent **toujours** justes ;
c'est l'objet `state` de la page qui peut être faux. Sans réhydratation, un
retour arrière suivi d'une re-validation envoie le défaut de la page au serveur
et écrase les réglages, pendant que l'écran affiche autre chose. Les quatre
autres tests passeraient avec ce bug.

Comme `test_phase4_invitations.py`, chaque test traverse le vrai parcours
phase 1 → 2 → 3 → 4 : l'accès direct masque les défauts de cette page.

Prérequis : serveur kreek lancé en dev.
"""

import re
import time

from playwright.sync_api import Page, expect

FAKE_LOGO_URL = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg"


def _atteindre_phase4(page: Page, competition_create_url: str) -> None:
    page.goto(competition_create_url, wait_until="load")
    page.fill("input[name='name']", f"Ligue E2E Notifs {time.time_ns()}")
    page.wait_for_selector(".coach-result-row", timeout=5000)
    page.locator(".coach-result-row").first.click()
    expect(page.locator(".coach-selected-badge")).to_have_count(1)
    page.evaluate(f"document.getElementById('logo_url').value = '{FAKE_LOGO_URL}'")
    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    page.wait_for_selector(".tier-block [data-slot='star'] .roster-chip", timeout=5000)
    page.fill("#season_name", "Saison E2E Notifs")
    page.click("button[onclick='submitRules()']")
    page.wait_for_selector("#groups-config", timeout=5000)

    page.click("button[onclick='submitStructure()']")
    _attendre_le_widget(page)


def _attendre_le_widget(page: Page) -> None:
    page.wait_for_selector("#access-mode-btns", timeout=5000)
    page.wait_for_selector(".notification-settings .ns-check", timeout=10000)
    # Alpine pose les motifs et émet son premier évènement à l'`init()`.
    page.wait_for_timeout(500)


def _case(page: Page, cle: str):
    return page.locator(f".notification-settings .ns-check[name='{cle}']")


def _ligne(page: Page, cle: str):
    return page.locator(f".notification-settings .ns-row[data-cle='{cle}']")


def _valider(page: Page) -> None:
    """Enregistre l'étape 4 et attend l'arrivée de l'étape 5."""
    page.click("button[onclick='submitInvitations()']")
    page.wait_for_selector(".recap-row", timeout=10000)


def _revenir_en_phase4(page: Page) -> None:
    """Par le bouton « ← Retour » de l'étape 5, et non par `go_back()` : le
    magicien navigue en `htmx.ajax` + `pushState` posés à la main, donc le
    retour navigateur ne re-rend pas la page."""
    page.click("text=← Retour")
    _attendre_le_widget(page)


# ── Scénario 1 — saison neuve : tout est allumé ──────────────────────────────


def test_une_saison_neuve_affiche_les_quatre_cases_cochees(page: Page, competition_create_url):
    _atteindre_phase4(page, competition_create_url)

    for cle in ("registration_open", "round_eve", "round_closing", "registration_deadline"):
        expect(_case(page, cle)).to_be_checked()

    # L'ancienne case unique a disparu — elle promettait des e-mails que rien
    # n'envoyait, et les quatre réglages l'absorbent.
    expect(page.locator("#notify-email")).to_have_count(0)


# ── Scénario 2 — l'état traverse l'aller-retour ──────────────────────────────


def test_une_case_decochee_est_affichee_au_retour(page: Page, competition_create_url):
    _atteindre_phase4(page, competition_create_url)
    _case(page, "round_eve").uncheck()
    _valider(page)

    _revenir_en_phase4(page)
    expect(_case(page, "round_eve")).not_to_be_checked()
    expect(_case(page, "round_closing")).to_be_checked()


# ── Scénario 3 — la réhydratation, par ses deux mécanismes ──────────────────
#
# `state` est tenu à jour par **deux** choses, et chacune couvre un trou de
# l'autre. Un seul test ne les distingue pas : le premier écrit ci-dessous passe
# même quand `INITIAL_NOTIFICATIONS` est retiré, parce que l'évènement d'`init()`
# du widget corrige `state` avant qu'on valide. Il faut donc empêcher le widget
# de charger pour que le JSON du serveur soit seul en cause.


def test_revalider_sans_toucher_aux_cases_ne_les_ecrase_pas(page: Page, competition_create_url):
    """Le chemin de l'évènement. Le widget est rendu par le serveur, donc les
    cases reviennent justes quoi qu'il arrive ; c'est `state` qui peut être faux,
    et c'est lui qui part au serveur."""
    _atteindre_phase4(page, competition_create_url)
    _case(page, "round_eve").uncheck()
    _case(page, "round_closing").uncheck()
    _valider(page)

    _revenir_en_phase4(page)
    # On ne touche à rien, et on revalide.
    _valider(page)

    _revenir_en_phase4(page)
    expect(_case(page, "round_eve")).not_to_be_checked()
    expect(_case(page, "round_closing")).not_to_be_checked()
    expect(_case(page, "registration_open")).to_be_checked()


def test_valider_avant_l_arrivee_du_widget_ne_perd_pas_les_reglages(
    page: Page, competition_create_url, console_errors
):
    """Le chemin du JSON, et le seul qui l'exerce. On coupe le fragment du
    widget : il ne rend rien, donc il n'émet rien, et `state` ne peut être juste
    que si le serveur l'a renseigné dès la première peinture.

    Sans `INITIAL_NOTIFICATIONS`, cette validation rejoue le défaut de la page —
    tout allumé — et écrase les réglages en silence."""
    _atteindre_phase4(page, competition_create_url)
    _case(page, "round_eve").uncheck()
    _valider(page)

    page.route("**/notifications-widget*", lambda route: route.abort())
    page.click("text=← Retour")
    page.wait_for_selector("#access-mode-btns", timeout=5000)
    # Le widget est absent : rien à attendre de son côté.
    expect(page.locator(".notification-settings")).to_have_count(0)
    _valider(page)

    page.unroute("**/notifications-widget*")
    _revenir_en_phase4(page)
    expect(_case(page, "round_eve")).not_to_be_checked()

    # Couper le fragment fait crier HTMX (`htmx:sendError`), et le garde-fou
    # global de `conftest.py` compte toute erreur de console comme un échec. On
    # le désamorce **ici seulement** : ces erreurs sont provoquées par le test.
    # Les tolérer globalement masquerait de vraies pannes HTMX ailleurs.
    console_errors.clear()


# ── Scénario 4 — le grisage suit la frappe, sans aller-retour ────────────────


def test_effacer_la_date_limite_grise_la_ligne_sans_rechargement(
    page: Page, competition_create_url
):
    _atteindre_phase4(page, competition_create_url)

    page.fill("#registration-deadline", "2026-09-15")
    page.wait_for_timeout(300)
    expect(_ligne(page, "registration_deadline")).not_to_have_class(
        re.compile(r"ns-row--inapplicable")
    )

    page.fill("#registration-deadline", "")
    page.wait_for_timeout(300)
    expect(_ligne(page, "registration_deadline")).to_have_class(
        re.compile(r"ns-row--inapplicable")
    )
    expect(_ligne(page, "registration_deadline").locator(".ns-motif")).to_be_visible()


# ── Scénario 5 — R6 : grisée ne veut pas dire décochée ───────────────────────


def test_une_ligne_grisee_par_la_frappe_reste_cochee(page: Page, competition_create_url):
    """R6. Décocher détruirait un choix explicite de l'organisateur en réaction
    à un geste qui n'a rien à voir, et il ne s'en apercevrait pas."""
    _atteindre_phase4(page, competition_create_url)

    page.fill("#registration-deadline", "2026-09-15")
    page.wait_for_timeout(300)
    expect(_case(page, "registration_deadline")).to_be_checked()

    page.fill("#registration-deadline", "")
    page.wait_for_timeout(300)

    expect(_case(page, "registration_deadline")).to_be_checked()
    expect(_case(page, "registration_deadline")).to_be_enabled()

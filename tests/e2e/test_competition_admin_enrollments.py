"""Tests E2E de l'onglet Inscriptions de l'admin compétition.

Crée une compétition, navigue vers /admin, clique sur l'onglet
Inscriptions et vérifie que les widgets se chargent.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

import re
import time

import pytest
import requests
from playwright.sync_api import Page, expect

from htmx_helpers import attendre_cablage

FAKE_LOGO_URL = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg"


def _create_competition_and_get_admin_url(page: Page, competition_create_url: str) -> str:
    page.goto(competition_create_url, wait_until="load")
    page.fill("input[name='name']", f"Enroll E2E {time.time_ns()}")

    page.wait_for_selector(".coach-result-row", timeout=5000)
    bagouze_row = page.locator(".coach-result-row", has_text="Bagouze")
    if bagouze_row.count() > 0:
        bagouze_row.first.click()
    else:
        page.locator(".coach-result-row").first.click()

    expect(page.locator(".coach-selected-badge")).to_have_count(1)
    page.evaluate(f"document.getElementById('logo_url').value = '{FAKE_LOGO_URL}'")

    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    url = page.url
    match = re.search(r"/app/([^/]+)/competitions/create/([^/]+)/([^/]+)/rules", url)
    assert match, f"Impossible d'extraire les IDs depuis l'URL: {url}"
    space_id = match.group(1)
    competition_id = match.group(2)
    season_id = match.group(3)

    return f"http://localhost:3210/app/{space_id}/competitions/{competition_id}/{season_id}/admin"


def test_ouvrir_l_administration_mene_au_resume(page: Page, competition_create_url):
    """Carte 419 — le Résumé est l'onglet d'accueil.

    Le tableau de bord l'occupait ; il a quitté l'administration, et le défaut de
    l'aiguillage rend désormais le Résumé. Trois cas e2e le vérifiaient sous son
    ancien nom — ils partent avec lui, et celui-ci prend leur place.

    L'assertion porte sur le **contenu servi**, pas seulement sur le fait que la
    page réponde : c'est l'onglet actif et le fragment rendu qui distinguent le
    Résumé de ce qui l'a précédé.
    """
    admin_url = _create_competition_and_get_admin_url(page, competition_create_url)

    page.goto(admin_url, wait_until="load")

    expect(page.locator(".admin-summary")).to_be_visible()
    onglet_actif = page.locator(".admin-tab.active")
    expect(onglet_actif).to_have_count(1)
    expect(onglet_actif).to_contain_text("Résumé")


def test_l_onglet_parametres_s_ouvre_sur_ses_conteneurs(page: Page, competition_create_url):
    """Carte 420 — la coquille de l'onglet Paramètres.

    Elle ne montre rien : les cinq panneaux arrivent avec les cartes 421 à 425.
    Ce que ce test affirme, c'est que l'aiguillage la sert — donc que les cartes
    suivantes auront bien un conteneur où se poser.

    L'assertion porte sur **les cinq conteneurs**, pas sur la seule présence de
    l'onglet : une branche d'aiguillage manquante rendrait le Résumé par défaut,
    et l'onglet aurait l'air de fonctionner.
    """
    admin_url = _create_competition_and_get_admin_url(page, competition_create_url)

    page.goto(admin_url + "/settings", wait_until="load")

    expect(page.locator(".competition-admin-settings")).to_be_visible()
    for conteneur in ("general", "ranking", "pools", "tiers", "visibility"):
        expect(page.locator(f"#settings-{conteneur}")).to_have_count(1)

    onglet_actif = page.locator(".admin-tab.active")
    expect(onglet_actif).to_have_count(1)
    expect(onglet_actif).to_contain_text("Paramètres")


def test_renommer_une_competition_depuis_les_parametres(page: Page, competition_create_url):
    """Carte 421 — le panneau « Informations générales », de bout en bout.

    Ce que ce test voit et qu'aucun test unitaire ne voit : que le panneau est
    **réellement servi** dans son conteneur, que le POST échange le widget par
    lui-même, et que le nouveau nom revient de la base — pas du formulaire.
    """
    admin_url = _create_competition_and_get_admin_url(page, competition_create_url)
    nouveau = f"Renommee E2E {time.time_ns()}"

    page.goto(admin_url + "/settings", wait_until="load")

    # Le conteneur se remplit par htmx : c'est le panneau qu'on attend, pas la page.
    expect(page.locator("#settings-general-panel")).to_be_visible(timeout=10000)
    page.fill("#settings-general-panel input[name='name']", nouveau)
    page.click("#settings-general-panel button[type='submit']")

    # Le widget est réécrit avec ce que porte la base.
    expect(page.locator("#settings-general-panel input[name='name']")).to_have_value(
        nouveau, timeout=10000
    )
    # Et la valeur survit à un rechargement complet — donc elle est enregistrée,
    # et non seulement réaffichée.
    page.goto(admin_url + "/settings", wait_until="load")
    expect(page.locator("#settings-general-panel input[name='name']")).to_have_value(
        nouveau, timeout=10000
    )


def test_un_nom_deja_pris_s_affiche_sous_le_champ(page: Page, competition_create_url):
    """L'erreur se lit **sous le champ fautif**, pas en bandeau.

    Et l'emplacement est réservé en permanence dans le flux : le formulaire ne
    saute pas au moment où l'utilisateur lit ce qu'on lui reproche.
    """
    premiere = _create_competition_and_get_admin_url(page, competition_create_url)
    page.goto(premiere + "/settings", wait_until="load")
    expect(page.locator("#settings-general-panel")).to_be_visible(timeout=10000)
    nom_pris = page.input_value("#settings-general-panel input[name='name']")

    # Une seconde compétition du même espace, qu'on tente de renommer comme la
    # première.
    seconde = _create_competition_and_get_admin_url(page, competition_create_url)
    page.goto(seconde + "/settings", wait_until="load")
    expect(page.locator("#settings-general-panel")).to_be_visible(timeout=10000)
    page.fill("#settings-general-panel input[name='name']", nom_pris)
    # **Le formulaire est câblé par htmx, et le panneau devient visible avant de
    # l'être.** Un clic tombé dans cette fenêtre ne produit rien : aucune
    # requête, aucune erreur, et l'attente ci-dessous expire sur un formulaire
    # qui n'a jamais été soumis. C'est ainsi que ce test échouait dans la suite
    # complète — jamais seul, où la machine a le temps.
    attendre_cablage(page, "#settings-general-panel form")
    page.click("#settings-general-panel button[type='submit']")

    erreur = page.locator("#settings-general-panel .form-row-error.shown")
    expect(erreur).to_be_visible(timeout=10000)
    expect(erreur).to_contain_text("déjà pris")


def test_l_onglet_parametres_est_garde(page: Page, competition_create_url):
    """Le `GET` du fragment est gardé, pas seulement la page complète.

    C'est **par le fragment qu'on navigue** : sans contrôle sur ce chemin, le
    changement d'onglet contournerait l'autorisation. Masquer l'onglet ne
    suffirait pas — l'URL est devinable.
    """
    admin_url = _create_competition_and_get_admin_url(page, competition_create_url)

    # `requests` et non `page.request` : ce dernier réutilise le cookie de
    # session du navigateur, et `bypass_auth` **ne remplace jamais une identité
    # déjà connectée** — l'en-tête serait ignoré, DevCoach répondrait, et le test
    # constaterait un `200` sans avoir rien exercé.
    refus = requests.get(
        admin_url + "/settings",
        headers={"HX-Request": "true", "X-Bypass-Auth-Profile": "simple"},
        timeout=10,
    )
    assert refus.status_code == 403, f"membre simple : {refus.status_code}"

    # Contre-épreuve : sans l'en-tête, la même requête passe. Sans elle, un 403
    # dû à une URL fautive se lirait comme un refus d'autorisation.
    admis = requests.get(
        admin_url + "/settings", headers={"HX-Request": "true"}, timeout=10
    )
    assert admis.status_code == 200, f"DevCoach : {admis.status_code}"


def test_les_onglets_retires_ne_repondent_plus(page: Page, competition_create_url):
    """Leurs routes ont disparu avec eux — un signet périmé rend `404`, il ne
    retombe pas sur une page à moitié rendue.

    Et la barre d'onglets ne les propose plus : masquer un lien sans retirer la
    route, ou l'inverse, laisserait la moitié du travail faite.
    """
    admin_url = _create_competition_and_get_admin_url(page, competition_create_url)

    for parti in ("dashboard", "results"):
        reponse = page.request.get(f"{admin_url}/{parti}")
        assert reponse.status == 404, f"/{parti} rend {reponse.status}"

    page.goto(admin_url, wait_until="load")
    barre = page.locator(".admin-tabs")
    expect(barre).not_to_contain_text("Tableau de bord")
    expect(barre).not_to_contain_text("Résultats")


def test_enrollments_tab_loads(page: Page, competition_create_url):
    admin_url = _create_competition_and_get_admin_url(page, competition_create_url)

    page.goto(admin_url + "/enrollments", wait_until="load")

    expect(page.locator(".admin-banner")).to_be_visible()
    page.wait_for_selector("#pending-container", timeout=5000)
    expect(page.locator("#pending-container")).to_be_visible()
    expect(page.locator("#enrolled-container")).to_be_visible()


def test_enrollments_shows_empty_states(page: Page, competition_create_url):
    admin_url = _create_competition_and_get_admin_url(page, competition_create_url)

    page.goto(admin_url + "/enrollments", wait_until="load")
    page.wait_for_selector("#pending-container .empty-state", timeout=5000)

    pending_empty = page.locator("#pending-container .empty-state")
    enrolled_empty = page.locator("#enrolled-container .empty-state")
    expect(pending_empty).to_be_visible()
    expect(enrolled_empty).to_be_visible()

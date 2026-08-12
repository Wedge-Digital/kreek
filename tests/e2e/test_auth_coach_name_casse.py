"""Tests E2E : le nom de coach identifie un compte sans distinction de casse.

Un coach qui ne se souvient plus s'il a saisi « Bagouze » ou « bagouze » à
l'inscription doit pouvoir se connecter et demander la réinitialisation de son
mot de passe malgré tout. Le pendant de cette tolérance est l'unicité : deux
comptes ne différant que par la casse rendraient la recherche ambiguë.

Le compte est créé par le test lui-même, via le formulaire d'inscription : la
suite ne dépend ainsi d'aucun mot de passe seedé, et reste rejouable.

Prérequis : serveur kreek lancé en dev, migrations appliquées (`make migrate`).
"""

import time

import pytest
from playwright.sync_api import Page, expect

from db_helpers import query_db

BASE_URL = "http://localhost:3210"

PASSWORD = "casse-e2e-12345"

# Préfixe commun aux comptes créés ici : il sert au nettoyage, et les distingue
# des coachs seedés, qu'aucun test ne doit toucher.
NAME_PREFIX = "E2eCasse"


def _delete_accounts():
    """Supprime les comptes créés par ce fichier, cache spaces compris.

    Le cache d'abord : il porte l'identifiant du compte, projeté depuis
    l'événement AccountCreated émis à l'inscription.
    """
    query_db(
        f"DELETE FROM auth__lost_login_token WHERE coach_name LIKE '{NAME_PREFIX}%';"
        f"DELETE FROM spaces__user_cache WHERE coach_name LIKE '{NAME_PREFIX}%';"
        f"DELETE FROM auth__users WHERE coach_name LIKE '{NAME_PREFIX}%';"
    )


@pytest.fixture
def coach_name():
    """Nom unique par exécution — l'unicité étant justement ce qu'on teste, un
    nom figé ferait échouer la seconde exécution sur la base de dev."""
    _delete_accounts()
    yield f"{NAME_PREFIX}{int(time.time())}"
    _delete_accounts()


def _register(page: Page, name: str, email: str | None = None) -> None:
    page.goto(f"{BASE_URL}/auth/register", wait_until="load")
    page.fill("input[name='coach_name']", name)
    page.fill("input[name='email']", email or f"{name.lower()}@example.test")
    page.fill("input[name='password']", PASSWORD)
    page.fill("input[name='password_confirm']", PASSWORD)
    page.click("button[type='submit']")


def _register_and_wait(page: Page, name: str) -> None:
    _register(page, name)
    page.wait_for_url("**/auth/register/success", timeout=5000)


def _login(page: Page, name: str, password: str = PASSWORD) -> None:
    page.goto(f"{BASE_URL}/auth/login", wait_until="load")
    page.fill("input[name='coach_name']", name)
    page.fill("input[name='password']", password)
    page.click("button[type='submit']")


def test_connexion_avec_le_nom_de_coach_dans_une_autre_casse(page: Page, coach_name):
    _register_and_wait(page, coach_name)

    _login(page, coach_name.upper())

    page.wait_for_url("**/auth/login/success", timeout=5000)
    expect(page.locator("text=Nous sommes ravis de vous revoir")).to_be_visible()


def test_mot_de_passe_errone_reste_refuse(page: Page, coach_name):
    """Contrôle négatif : sans lui, une page de succès atteinte par un tout
    autre chemin — session déjà ouverte, bypass d'authentification — ferait
    passer le test précédent pour une bonne raison qu'il n'a pas."""
    _register_and_wait(page, coach_name)

    _login(page, coach_name.upper(), password="mauvais-mot-de-passe")

    expect(page.locator("form .form-error")).to_contain_text(
        "Nom de coach ou mot de passe incorrect"
    )


def test_mot_de_passe_oublie_retrouve_le_compte_dans_une_autre_casse(
    page: Page, coach_name
):
    """La demande de réinitialisation ne dit jamais si le compte existe : la
    même page de confirmation répond dans les deux cas, pour ne pas révéler qui
    est inscrit. Le token créé en base est donc le seul témoin observable du
    fait que la recherche a bien retrouvé le compte."""
    _register_and_wait(page, coach_name)

    page.goto(f"{BASE_URL}/auth/forgot-password", wait_until="load")
    page.fill("input[name='coach_name']", coach_name.lower())
    page.click("button[type='submit']")
    page.wait_for_timeout(1000)

    tokens = query_db(
        "SELECT count(*) FROM auth__lost_login_token "
        f"WHERE coach_name = '{coach_name}';"
    )
    assert tokens == ["1"], (
        f"aucun token de réinitialisation pour « {coach_name} » : la recherche "
        "par nom de coach n'a pas retrouvé le compte."
    )


def test_inscription_refuse_un_nom_deja_pris_dans_une_autre_casse(
    page: Page, coach_name
):
    _register_and_wait(page, coach_name)

    # Email distinct : sinon l'insertion viole deux index uniques à la fois, et
    # l'erreur remontée dépendrait de celui que Postgres contrôle en premier.
    _register(page, coach_name.lower(), email=f"{coach_name.lower()}-bis@example.test")

    expect(page.locator("input[name='coach_name'] + .form-error")).to_contain_text(
        "Ce nom de coach est déjà utilisé"
    )

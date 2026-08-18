"""Tests E2E : chaque réponse porte l'identifiant de sa requête.

Le `rid` du span de requête est repris dans l'en-tête `x-request-id`. C'est le
seul chemin qui parte d'un symptôme constaté par un coach — l'identifiant lu
dans l'onglet réseau de son navigateur — pour retrouver la requête dans
`docker logs`. Sans lui, les lignes du journal se corrèlent bien entre elles
mais on ne connaît jamais la valeur à chercher.

Le second test vérifie l'autre moitié de la carte 344 : `/static` est servi à
côté du journal et n'y figure pas. C'est le placement de la couche sur
`auth_app` plutôt que sur le routeur externe qui le garantit — une propriété
qu'aucun test unitaire ne peut constater.

Prérequis : serveur kreek lancé en dev, migrations appliquées (`make migrate`).
"""

from playwright.sync_api import Page

BASE_URL = "http://localhost:3210"

# Un ULID fait 26 caractères en Crockford base32.
LONGUEUR_ULID = 26


def test_la_reponse_porte_un_identifiant_de_requete(page: Page):
    reponse = page.goto(f"{BASE_URL}/auth/login")

    assert reponse is not None
    rid = reponse.header_value("x-request-id")
    assert rid, "aucun en-tête x-request-id sur la réponse"
    assert len(rid) == LONGUEUR_ULID, f"identifiant inattendu : {rid!r}"


def test_deux_requetes_portent_deux_identifiants_distincts(page: Page):
    """Sans quoi l'identifiant ne corrèle rien : il désignerait tout."""
    premier = page.goto(f"{BASE_URL}/auth/login").header_value("x-request-id")
    second = page.goto(f"{BASE_URL}/auth/login").header_value("x-request-id")

    assert premier != second


def test_un_identifiant_entrant_n_est_jamais_repris(page: Page):
    """L'honorer laisserait n'importe qui injecter du texte dans le journal.

    Requête directe plutôt que navigation : `set_extra_http_headers` poserait
    l'en-tête sur **toutes** les sous-ressources de la page, polices Google
    comprises, dont le préflight CORS échouerait alors bruyamment.
    """
    reponse = page.request.get(
        f"{BASE_URL}/auth/login",
        headers={"x-request-id": "injecte-par-le-client"},
    )

    assert reponse.headers.get("x-request-id") != "injecte-par-le-client"


def test_les_fichiers_statiques_restent_hors_du_journal(page: Page):
    """`/static` est monté à côté de la couche : ni en-tête, ni ligne de log."""
    reponse = page.goto(f"{BASE_URL}/static/css/common.css")

    assert reponse.status == 200
    assert reponse.header_value("x-request-id") is None

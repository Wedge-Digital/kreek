"""Tests E2E — les routes d'administration refusent un membre simple (carte 573).

Masquer un bouton n'est pas un contrôle d'accès : chaque scénario forge la
requête qu'un membre simple n'a pas de bouton pour envoyer, et vérifie qu'elle
est refusée **et ne change rien en base**.

- les étapes du magicien d'une compétition qu'il n'administre pas ;
- mais le magicien de **sa** compétition : son créateur en est admin ;
- le magicien d'une compétition publiée renvoie vers son administration ;
- les réglages de notifications ;
- la création d'un compte coach.

Les requêtes passent par `requests` et non par le navigateur : le middleware
n'échange l'identité que sur une session vide — la raison donnée par le test de
la trésorerie.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import time

import pytest
import requests

from competition_lifecycle import BASE_URL, FAKE_LOGO_URL, create_full_competition
from db_helpers import query_db

MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple", "HX-Request": "true"}
ETAPES = ["rules", "structure", "invitations", "validation"]


def _url_magicien(space_id: str, comp: dict, etape: str | None = None) -> str:
    base = f"{BASE_URL}/app/{space_id}/competitions/create/{comp['competition_id']}"
    return base if etape is None else f"{base}/{comp['season_id']}/{etape}"


def _admins(competition_id: str) -> list[str]:
    return query_db(
        "SELECT coach_id FROM competitions_members "
        f"WHERE competition_id = '{competition_id}' AND competition_profile = 'CompetitionAdmin' "
        "ORDER BY coach_id"
    )


def _url_creation(space_id: str) -> str:
    return f"{BASE_URL}/app/{space_id}/competitions/create"


@pytest.fixture(scope="module")
def brouillon(browser, space_id):
    """Une compétition arrêtée à l'étape 3, administrée par DevCoach seul."""
    page = browser.new_page()
    try:
        return create_full_competition(page, _url_creation(space_id), stop_at_step=3)
    finally:
        page.close()


@pytest.fixture(scope="module")
def publiee(browser, space_id):
    page = browser.new_page()
    try:
        return create_full_competition(page, _url_creation(space_id), num_rounds=1)
    finally:
        page.close()


def test_le_magicien_d_une_competition_d_autrui_est_refuse(space_id, brouillon):
    for etape in [None, *ETAPES]:
        resp = requests.get(
            _url_magicien(space_id, brouillon, etape), headers=MEMBRE_SIMPLE,
            allow_redirects=False, timeout=20,
        )
        assert resp.status_code == 403, f"étape {etape or 'infos'} : {resp.status_code}"


def test_un_membre_simple_ne_peut_pas_se_declarer_admin(space_id, brouillon):
    """La forme la plus grave : l'étape 1 réécrit la liste des admins."""
    avant = _admins(brouillon["competition_id"])
    resp = requests.post(
        _url_magicien(space_id, brouillon),
        json={"name": f"Prise {time.time_ns()}", "logo_url": FAKE_LOGO_URL, "admin_ids": []},
        headers=MEMBRE_SIMPLE, timeout=20,
    )
    assert resp.status_code == 403, resp.status_code
    assert _admins(brouillon["competition_id"]) == avant, "la liste des admins n'a pas bougé"


def test_le_createur_d_une_competition_en_est_admin(space_id):
    """Tout membre peut créer une compétition, et en devient admin sans l'avoir
    demandé : sans quoi le magicien le refuserait dès l'étape 2."""
    resp = requests.post(
        _url_creation(space_id),
        json={"name": f"Ligue d'un membre {time.time_ns()}", "logo_url": FAKE_LOGO_URL, "admin_ids": []},
        headers=MEMBRE_SIMPLE, timeout=20,
    )
    cible = resp.headers.get("hx-redirect", "")
    assert "/rules" in cible, f"création refusée : {resp.status_code} — {resp.text[:200]}"

    etape_2 = requests.get(f"{BASE_URL}{cible}", headers=MEMBRE_SIMPLE, allow_redirects=False, timeout=20)
    assert etape_2.status_code == 200, f"le créateur doit accéder à l'étape 2 : {etape_2.status_code}"


def test_le_magicien_d_une_competition_publiee_renvoie_a_l_administration(space_id, publiee):
    resp = requests.get(
        _url_magicien(space_id, publiee, "structure"),
        headers={"HX-Request": "true"}, allow_redirects=False, timeout=20,
    )
    assert "/admin" in resp.headers.get("hx-redirect", ""), (
        f"une compétition publiée se modifie par son administration : {resp.status_code}"
    )

    resp = requests.post(_url_magicien(space_id, publiee, "validation"), headers={"HX-Request": "true"}, timeout=20)
    assert resp.status_code == 409, resp.status_code


def test_les_notifications_d_autrui_sont_refusees(space_id, publiee):
    base = f"{BASE_URL}/app/{space_id}/competitions/{publiee['competition_id']}/{publiee['season_id']}"
    avant = query_db(f"SELECT notifications::text FROM competition_seasons WHERE id = '{publiee['season_id']}'")

    lecture = requests.get(f"{base}/notifications-widget?mode=autosave", headers=MEMBRE_SIMPLE, timeout=20)
    assert lecture.status_code == 403, lecture.status_code

    # Un vrai champ de formulaire : sans corps, l'extracteur répondrait 415
    # avant même que la garde soit consultée, et le test ne prouverait rien.
    ecriture = requests.post(
        f"{base}/notifications", data={"registration_open": "on"}, headers=MEMBRE_SIMPLE, timeout=20
    )
    assert ecriture.status_code == 403, ecriture.status_code
    assert query_db(
        f"SELECT notifications::text FROM competition_seasons WHERE id = '{publiee['season_id']}'"
    ) == avant, "les réglages n'ont pas bougé"


def test_un_membre_simple_ne_cree_pas_de_compte(space_id):
    """Les routes d'`auth` sont hors du routeur protégé : le contournement
    d'authentification ne s'y applique pas. La session du membre simple est donc
    ouverte d'abord sur une page de l'application — comme le navigateur d'un vrai
    coach la porte —, puis présentée au widget. Sans elle, la réponse serait un
    401 d'anonyme, et la politique de l'hôte ne serait pas éprouvée."""
    pseudo = f"Forge{time.time_ns() % 10**8}"
    ouverture = requests.get(f"{BASE_URL}/app/{space_id}/competitions", headers=MEMBRE_SIMPLE, timeout=20)
    assert ouverture.status_code == 200, f"session du membre simple : {ouverture.status_code}"
    # Le cookie est `Secure` : `requests` ne le renverrait pas sur `http://`, là
    # où un navigateur tient `localhost` pour sûr. On le présente nous-mêmes.
    avec_session = {**MEMBRE_SIMPLE, "Cookie": f"id={ouverture.cookies['id']}"}

    affichage = requests.get(f"{BASE_URL}/auth/widgets/coach-creation", headers=avec_session, timeout=20)
    assert affichage.status_code == 403, affichage.status_code

    resp = requests.post(
        f"{BASE_URL}/auth/widgets/coach-creation",
        data={"coach_name": pseudo, "email": f"{pseudo.lower()}@kreek.test"},
        headers=avec_session, timeout=20,
    )
    assert resp.status_code == 403, resp.status_code
    assert query_db(f"SELECT count(*) FROM auth__users WHERE coach_name = '{pseudo}'") == ["0"]

"""Le panneau « Visibilité » (carte 425).

Sans navigateur : le panneau est un `<form>` ordinaire, sans une ligne de JS —
contrairement au panneau « Tiers », dont la collecte JS exigeait un vrai
navigateur. `requests` suffit et va cent fois plus vite.

**Ce que ces tests gardent réellement.** La carte prescrivait
`save_invitations`, qui écrit aussi `status = 'invitations_configured'` et
réécrit les notifications. Sur une saison en cours, changer un mode d'accès
aurait donc ramené la saison sous `ready` — et la carte 407 interdit la création
d'équipe sur une saison qui ne l'est pas : l'inscription de la compétition
entière aurait cessé, sans un mot. L'enregistrement, lui, aurait réussi.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import json

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import execute_db, query_db

HX = {"HX-Request": "true"}
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}


def _url(space_id: str, ctx: dict) -> str:
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/admin/settings/visibility"
    )


def _invitations(season_id: str) -> dict:
    return json.loads(
        query_db(f"SELECT invitations FROM competition_seasons WHERE id = '{season_id}'")[0]
    )


def _statut(season_id: str) -> str:
    return query_db(f"SELECT status FROM competition_seasons WHERE id = '{season_id}'")[0]


def _eteindre_un_rappel(season_id: str) -> None:
    """Écarte les notifications de leur défaut, **juste avant le test qui les
    observe**.

    Le défaut du domaine allume les quatre. Un test qui part de là est aveugle :
    réécrire la colonne avec `default()` produirait exactement le même JSON.
    Constaté en falsifiant — la mutation passait, le test restait vert.

    Et l'écriture ne peut pas vivre dans la fixture : les POST des autres tests
    du module remettent la colonne au défaut, donc elle n'y survivrait pas. Le
    test qui en dépend se la donne lui-même.
    """
    eteint = {
        "registration_open": True,
        "round_eve": False,
        "round_closing": True,
        "registration_deadline": True,
    }
    execute_db(
        "UPDATE competition_seasons SET notifications = '%s'::jsonb WHERE id = '%s'"
        % (json.dumps(eteint), season_id)
    )


def _notifications(season_id: str) -> str:
    return query_db(
        "SELECT coalesce(notifications::text, 'NULL') "
        f"FROM competition_seasons WHERE id = '{season_id}'"
    )[0]


@pytest.fixture(scope="module")
def saison_garnie(browser, space_id):
    """Une compétition fermée portant **tout ce que le panneau n'édite pas**.

    Le parcours de création laisse ces trois champs vides ; sans les garnir, les
    tests de préservation passeraient sur des listes vides — c'est-à-dire sans
    rien prouver.
    """
    ctx = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    season_id = ctx["season_id"]
    garni = {
        "access_mode": "invitation",
        "requires_validation": True,
        "invited_coaches": [
            {"id": "01M0000000000000000000000A", "coach_name": "Skarbrand", "initials": "SK"},
            {"id": "01M0000000000000000000000B", "coach_name": "Griff", "initials": "GR"},
        ],
        "max_participants": 12,
        "registration_deadline": "2026-09-30",
    }
    execute_db(
        "UPDATE competition_seasons SET invitations = '%s'::jsonb WHERE id = '%s'"
        % (json.dumps(garni).replace("'", "''"), season_id)
    )

    assert _statut(season_id) == "ready", "la fixture doit partir d'une saison prête"
    return {"space_id": space_id, "ctx": ctx, "season_id": season_id}


def test_ouvrir_la_competition_conserve_les_coachs_invites(saison_garnie):
    """Le test que la carte demande."""
    url = _url(saison_garnie["space_id"], saison_garnie["ctx"])
    season_id = saison_garnie["season_id"]
    avant = _invitations(season_id)
    assert len(avant["invited_coaches"]) == 2

    reponse = requests.post(
        url,
        data={"access_mode": "open", "requires_validation": "automatic"},
        headers=HX,
        timeout=30,
    )

    assert reponse.status_code == 200
    assert "Visibilité enregistrée" in reponse.text
    apres = _invitations(season_id)
    assert apres["access_mode"] == "open"
    assert apres["requires_validation"] is False
    assert apres["invited_coaches"] == avant["invited_coaches"], "la liste a été perdue"


def test_l_echeance_et_le_plafond_traversent_l_enregistrement(saison_garnie):
    """`max_participants` alimente la ligne « il reste N places » des relances."""
    url = _url(saison_garnie["space_id"], saison_garnie["ctx"])
    season_id = saison_garnie["season_id"]
    avant = _invitations(season_id)

    requests.post(
        url,
        data={"access_mode": "invitation", "requires_validation": "manual"},
        headers=HX,
        timeout=30,
    )

    apres = _invitations(season_id)
    assert apres["max_participants"] == avant["max_participants"] == 12
    assert apres["registration_deadline"] == avant["registration_deadline"] == "2026-09-30"


def test_la_saison_reste_prete_apres_un_changement_de_visibilite(saison_garnie):
    """**Le défaut que la carte ne voyait pas.**

    `save_invitations` aurait reposé `status = 'invitations_configured'`. La
    saison ne serait plus `ready`, et plus aucune équipe n'aurait pu s'inscrire
    dans la compétition — alors que l'enregistrement, lui, réussit.
    """
    url = _url(saison_garnie["space_id"], saison_garnie["ctx"])
    season_id = saison_garnie["season_id"]
    assert _statut(season_id) == "ready"

    requests.post(
        url,
        data={"access_mode": "open", "requires_validation": "manual"},
        headers=HX,
        timeout=30,
    )

    assert _statut(season_id) == "ready", "la saison a régressé sous ready"


def test_les_reglages_de_notification_ne_sont_pas_touches(saison_garnie):
    """L'autre moitié du même piège : les rappels d'échéance s'éteindraient."""
    url = _url(saison_garnie["space_id"], saison_garnie["ctx"])
    season_id = saison_garnie["season_id"]
    _eteindre_un_rappel(season_id)
    avant = _notifications(season_id)
    assert avant != "NULL", "la saison doit avoir des réglages de notification"
    assert '"round_eve": false' in avant, (
        "la fixture doit s'écarter du défaut, sinon une réécriture au défaut "
        "produirait le même JSON et ce test ne prouverait rien"
    )

    requests.post(
        url,
        data={"access_mode": "open", "requires_validation": "automatic"},
        headers=HX,
        timeout=30,
    )

    assert _notifications(season_id) == avant, "les notifications ont été réécrites"


@pytest.mark.parametrize(
    "champ,valeur",
    [("access_mode", "publique"), ("requires_validation", "peut-etre")],
)
def test_une_valeur_inconnue_est_refusee_sans_rien_ecrire(saison_garnie, champ, valeur):
    """**Un refus se voit ; un repli, non.**

    Se rabattre sur le défaut — `AccessMode::Invitation` — refermerait une
    compétition ouverte sur une simple faute de frappe, et l'écran afficherait
    un enregistrement réussi.
    """
    url = _url(saison_garnie["space_id"], saison_garnie["ctx"])
    season_id = saison_garnie["season_id"]
    donnees = {"access_mode": "open", "requires_validation": "automatic"}
    requests.post(url, data=donnees, headers=HX, timeout=30)
    avant = _invitations(season_id)

    donnees[champ] = valeur
    reponse = requests.post(url, data=donnees, headers=HX, timeout=30)

    assert reponse.status_code == 400
    assert _invitations(season_id) == avant, "une valeur refusée a tout de même écrit"


def test_le_compteur_d_invites_est_affiche(saison_garnie):
    """Il rend visible ce que l'enregistrement doit préserver."""
    reponse = requests.get(
        _url(saison_garnie["space_id"], saison_garnie["ctx"]), headers=HX, timeout=15
    )

    assert reponse.status_code == 200
    assert "2</strong>" in reponse.text
    assert "coachs invités" in reponse.text


def test_le_panneau_est_garde(saison_garnie):
    url = _url(saison_garnie["space_id"], saison_garnie["ctx"])

    assert requests.get(url, headers={**HX, **ENTETE_MEMBRE_SIMPLE}, timeout=10).status_code == 403
    assert (
        requests.post(
            url,
            data={"access_mode": "open", "requires_validation": "manual"},
            headers={**HX, **ENTETE_MEMBRE_SIMPLE},
            timeout=30,
        ).status_code
        == 403
    )
    assert requests.get(url, headers=HX, timeout=10).status_code == 200

"""Le panneau « Poules » — retirer une poule désaffecte ses équipes (carte 423).

Ce que ces tests voient et qu'aucun test unitaire ne voit : que la suppression
traverse réellement jusqu'à `competition_group_teams`, dont la cascade vide les
affectations. Les poules vivent à deux endroits — la déclaration dans le JSONB et
la table matérialisée — et la seconde n'est jamais purgée par son projecteur, qui
ne fait qu'`INSERT … ON CONFLICT DO UPDATE`.

Le troisième scénario est le plus important : **le calendrier survit à
l'enregistrement**. Son échec ne produirait aucune erreur, juste un calendrier
vide découvert des jours plus tard.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db

HX = {"HX-Request": "true"}
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}


def _admin(space_id: str, ctx: dict, suffixe: str) -> str:
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/admin/{suffixe}"
    )


def _poules(season_id: str) -> list[str]:
    return query_db(
        f"SELECT id || '=' || name FROM competition_groups "
        f"WHERE season_id = '{season_id}' ORDER BY position"
    )


def _affectations(season_id: str) -> int:
    lignes = query_db(
        "SELECT count(*) FROM competition_group_teams t "
        "JOIN competition_groups g ON g.id = t.group_id "
        f"WHERE g.season_id = '{season_id}'"
    )
    return int(lignes[0])


def _journees(season_id: str) -> int:
    lignes = query_db(
        "SELECT jsonb_array_length(coalesce("
        "structure->'schedule'->'scheduled_dates', '[]'::jsonb)) "
        f"FROM competition_seasons WHERE id = '{season_id}'"
    )
    return int(lignes[0])


@pytest.fixture(scope="module")
def saison_avec_poules(browser, space_id):
    """Une compétition dont les poules sont **matérialisées et peuplées**.

    Les deux étapes sont nécessaires et distinctes : ouvrir l'onglet Poules
    projette la déclaration dans la table (le projecteur est paresseux), le
    tirage au sort y affecte les équipes. Sans la seconde, la cascade n'aurait
    rien à défaire et les tests passeraient sans rien prouver.
    """
    ctx = build_full_competition(browser, space_id, num_teams=4, num_rounds=1)
    requests.get(_admin(space_id, ctx, "groups/cards"), headers=HX, timeout=10)
    requests.post(_admin(space_id, ctx, "groups/random-draw"), headers=HX, timeout=20)
    season_id = ctx["season_id"]
    assert _affectations(season_id) > 0, "le tirage doit avoir affecté des équipes"
    # Le nombre de journées est relevé **ici**, avant tout enregistrement : les
    # tests partagent cette fixture et s'exécutent dans l'ordre, donc comparer à
    # l'état courant ferait dépendre le verdict de ce que les précédents ont
    # fait — un calendrier déjà effacé passerait pour une prémisse manquante.
    journees = _journees(season_id)
    assert journees > 0, "la fixture doit avoir un calendrier"
    return {
        "space_id": space_id,
        "ctx": ctx,
        "season_id": season_id,
        "journees": journees,
    }


def test_retirer_une_poule_desaffecte_ses_equipes(saison_avec_poules):
    """Le cœur de la carte.

    Sans suppression explicite de la ligne de poule, le retrait serait
    **cosmétique** : la déclaration perdrait la poule, la table la garderait avec
    ses équipes, et l'onglet Poules continuerait de l'afficher.
    """
    space_id, ctx = saison_avec_poules["space_id"], saison_avec_poules["ctx"]
    season_id = saison_avec_poules["season_id"]
    poules = _poules(season_id)
    assert len(poules) >= 2, f"la fixture doit avoir deux poules : {poules}"
    gardee = poules[0].split("=")[0]
    avant = _affectations(season_id)

    reponse = requests.post(
        _admin(space_id, ctx, "settings/pools"),
        data={"use_pools": "true", "pool_id": gardee, "pool_name": "Seule rescapée"},
        headers=HX,
        timeout=30,
    )

    assert reponse.status_code == 200, reponse.text[:200]
    assert _poules(season_id) == [f"{gardee}=Seule rescapée"], "la table n'a pas suivi"
    assert _affectations(season_id) < avant, "aucune équipe n'a été désaffectée"


def test_le_calendrier_survit_a_l_enregistrement(saison_avec_poules):
    """**Le test le plus important de la carte.**

    `save_structure_and_prune_groups` écrit la structure **entière** : sans
    relecture du calendrier, l'enregistrement l'effacerait. Et sans erreur — juste
    un calendrier vide, découvert des jours plus tard.
    """
    space_id, ctx = saison_avec_poules["space_id"], saison_avec_poules["ctx"]
    season_id = saison_avec_poules["season_id"]
    avant = saison_avec_poules["journees"]
    gardee = _poules(season_id)[0].split("=")[0]

    requests.post(
        _admin(space_id, ctx, "settings/pools"),
        data={"use_pools": "true", "pool_id": gardee, "pool_name": "Toujours là"},
        headers=HX,
        timeout=30,
    )

    assert _journees(season_id) == avant, "le calendrier a été effacé"
    # Et la saison n'a pas régressé : la carte 407 interdit la création d'équipe
    # sur une saison qui n'est pas « prête ».
    assert query_db(f"SELECT status FROM competition_seasons WHERE id = '{season_id}'") == [
        "ready"
    ], "la saison a régressé sous « prête »"


def test_retirer_toutes_les_poules_est_autorise(saison_avec_poules):
    """Retirer tout n'est pas un cas particulier : aucune branche ne le traite,
    et c'est le signe que la forme est juste."""
    space_id, ctx = saison_avec_poules["space_id"], saison_avec_poules["ctx"]
    season_id = saison_avec_poules["season_id"]

    reponse = requests.post(
        _admin(space_id, ctx, "settings/pools"),
        data={"use_pools": "false"},
        headers=HX,
        timeout=30,
    )

    assert reponse.status_code == 200
    assert _poules(season_id) == [], "des poules subsistent"
    assert _affectations(season_id) == 0, "des affectations subsistent"
    assert _journees(season_id) == saison_avec_poules["journees"], "le calendrier a été emporté"


def test_une_poule_neuve_recoit_son_identifiant_du_serveur(saison_avec_poules):
    """Le formulaire soumet un identifiant **vide** pour une poule neuve. Un
    identifiant de domaine minté par le navigateur ne serait contrôlé ni en
    forme, ni en unicité, ni en provenance."""
    space_id, ctx = saison_avec_poules["space_id"], saison_avec_poules["ctx"]
    season_id = saison_avec_poules["season_id"]

    requests.post(
        _admin(space_id, ctx, "settings/pools"),
        data={"use_pools": "true", "pool_id": "", "pool_name": "Poule neuve"},
        headers=HX,
        timeout=30,
    )

    poules = _poules(season_id)
    assert len(poules) == 1, f"une seule poule attendue : {poules}"
    identifiant, nom = poules[0].split("=", 1)
    assert nom == "Poule neuve"
    assert identifiant.startswith("g") and identifiant[1:].islower(), (
        f"l'identifiant doit satisfaire ^g[0-9a-z]+$ : « {identifiant} »"
    )


def test_un_ecart_de_longueur_est_refuse(saison_avec_poules):
    """Deux tableaux parallèles de longueurs différentes sont un `400`, jamais un
    `zip` — qui s'arrête sur la plus courte et perdrait une poule sans rien dire.
    Le commissaire verrait son enregistrement réussir et une poule disparaître."""
    space_id, ctx = saison_avec_poules["space_id"], saison_avec_poules["ctx"]

    reponse = requests.post(
        _admin(space_id, ctx, "settings/pools"),
        data=[("use_pools", "true"), ("pool_id", ""), ("pool_id", ""), ("pool_name", "Seule")],
        headers=HX,
        timeout=30,
    )

    assert reponse.status_code == 400, f"obtenu {reponse.status_code}"


def test_le_panneau_est_garde(saison_avec_poules):
    """`require_admin_access` sur les deux verbes. `requests` et non le
    navigateur : `bypass_auth` ne remplace jamais une identité déjà connectée."""
    space_id, ctx = saison_avec_poules["space_id"], saison_avec_poules["ctx"]
    url = _admin(space_id, ctx, "settings/pools")

    assert requests.get(url, headers={**HX, **ENTETE_MEMBRE_SIMPLE}, timeout=10).status_code == 403
    assert (
        requests.post(
            url,
            data={"use_pools": "false"},
            headers={**HX, **ENTETE_MEMBRE_SIMPLE},
            timeout=30,
        ).status_code
        == 403
    )
    assert requests.get(url, headers=HX, timeout=10).status_code == 200

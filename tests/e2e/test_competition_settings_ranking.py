"""Le panneau « Points de classement » — le cas qui commande l'épic E14.

**Modifier le barème d'une saison en cours recalcule le classement publié**,
immédiatement, dans le même POST. Sans ce recalcul, changer un barème en cours de
saison produirait un classement qui mélange deux règles — et personne ne
l'apprendrait, les totaux restant plausibles.

Aucun test unitaire ne voit cette chaîne : elle traverse `competitions` vers
`ranking` par un port qui **ordonne**, et le classement affiché vient d'une
projection que seul un vrai rejeu réécrit.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import play_match, wait_ranking_points

JSON = {"Content-Type": "application/json", "HX-Request": "true"}
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}


def _bareme(victoire: int, nul: int = 1, defaite: int = 0) -> dict:
    """La charge utile épouse `RankingRules` : chaque champ est un nutype qui
    valide à la désérialisation, et le serveur n'a aucun contrôle à écrire."""
    return {
        "win_points": victoire,
        "draw_points": nul,
        "lose_points": defaite,
        "offensive_bonus": {"activated": False, "diff_td": 2, "points": 1},
        "defensive_bonus": {"activated": False, "max_td_conceded": 1, "points": 1},
        "aggressive_bonus": {"activated": False, "min_casualties": 3, "points": 1},
        "tiebreakers": [{"code": "nb_td", "activated": True}],
    }


def _url(space_id: str, ctx: dict) -> str:
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/admin/settings/ranking"
    )


@pytest.fixture(scope="module")
def saison_jouee(browser, space_id):
    """Deux équipes, **deux matchs joués et publiés**.

    Deux et non un : le décompte annoncé au retour doit distinguer les matchs des
    lignes de classement, qui sont deux fois plus nombreuses. Avec un seul match,
    « 1 » et « 2 » se confondraient dans l'esprit du lecteur autant que dans le
    code.
    """
    ctx = build_full_competition(browser, space_id, num_teams=2, num_rounds=2)
    home, away = ctx["team_ids"][0], ctx["team_ids"][1]
    play_match(space_id, ctx, ctx["round_ids"][0], home, away, home_td=2, away_td=0)
    play_match(space_id, ctx, ctx["round_ids"][1], home, away, home_td=1, away_td=0)
    return {"space_id": space_id, "ctx": ctx, "home": home, "away": away}


def test_changer_le_bareme_recalcule_le_classement_publie(saison_jouee):
    """**Le scénario central de toute la fonctionnalité.**

    Deux victoires à 2 points font 4. Le barème passe à 5, et le classement doit
    afficher 10 — sans qu'aucun match ne soit rejoué à la main, ni qu'un second
    geste soit demandé au commissaire.

    Les deux totaux sont **distincts et non multiples l'un de l'autre par un
    facteur trivial** : 4 et 10 ne se confondent avec aucun décompte de lignes,
    de matchs ou d'équipes qui traînerait dans le calcul.
    """
    space_id, ctx = saison_jouee["space_id"], saison_jouee["ctx"]
    season_id, home = ctx["season_id"], saison_jouee["home"]

    # Point de départ : deux victoires au barème par défaut.
    requests.post(_url(space_id, ctx), json=_bareme(2), headers=JSON, timeout=30)
    avant = wait_ranking_points(season_id, home)
    assert avant == 4, f"deux victoires à 2 points font 4, obtenu {avant}"

    reponse = requests.post(_url(space_id, ctx), json=_bareme(5), headers=JSON, timeout=30)

    assert reponse.status_code == 200, reponse.text[:200]
    apres = wait_ranking_points(season_id, home)
    assert apres == 10, f"deux victoires à 5 points font 10, obtenu {apres}"
    # Le pied annonce des **matchs**, pas des lignes : il y en a deux fois plus.
    assert "2</b> match" in reponse.text, "le décompte doit dire deux matchs"


def test_le_bareme_enregistre_survit_au_rechargement(saison_jouee):
    """Le panneau est relu depuis la base, jamais reconstruit depuis la charge
    utile : sans cela, un widget qui réafficherait la saisie sans l'enregistrer
    passerait au vert."""
    space_id, ctx = saison_jouee["space_id"], saison_jouee["ctx"]

    requests.post(_url(space_id, ctx), json=_bareme(7), headers=JSON, timeout=30)
    panneau = requests.get(_url(space_id, ctx), headers=JSON, timeout=10).text

    assert 'data-champ="win_points"' in panneau
    assert 'value="7"' in panneau, "le barème enregistré doit revenir de la base"
    assert (
        query_db(
            "SELECT rules->'ranking_rules'->>'win_points' FROM competition_seasons "
            f"WHERE id = '{ctx['season_id']}'"
        )
        == ["7"]
    )


def test_les_tiers_survivent_a_un_changement_de_bareme(saison_jouee):
    """`save_rules` écrit `CompetitionRules` entier, et ce panneau n'édite que le
    barème : sans relecture, budgets, rosters et coups de pouce de tous les tiers
    disparaîtraient — silencieusement."""
    space_id, ctx = saison_jouee["space_id"], saison_jouee["ctx"]
    avant = query_db(
        "SELECT jsonb_array_length(rules->'tiers') FROM competition_seasons "
        f"WHERE id = '{ctx['season_id']}'"
    )
    assert avant and int(avant[0]) > 0, "la fixture doit avoir au moins un tier"

    requests.post(_url(space_id, ctx), json=_bareme(3), headers=JSON, timeout=30)

    apres = query_db(
        "SELECT jsonb_array_length(rules->'tiers') FROM competition_seasons "
        f"WHERE id = '{ctx['season_id']}'"
    )
    assert apres == avant, f"les tiers ont été effacés : {avant} puis {apres}"


def test_un_bareme_hors_bornes_est_refuse_par_la_deserialisation(saison_jouee):
    """Le handler n'écrit aucune validation : chaque champ est un nutype qui
    valide à la désérialisation, et `TiebreakConfig` refuse la liste vide comme
    l'absence de critère actif.

    Trois formes de refus, pour que le test ne repose pas sur une seule."""
    space_id, ctx = saison_jouee["space_id"], saison_jouee["ctx"]

    for cas, charge in [
        ("points hors bornes", {**_bareme(3), "win_points": 999_999}),
        ("aucun départage", {**_bareme(3), "tiebreakers": []}),
        (
            "aucun critère actif",
            {**_bareme(3), "tiebreakers": [{"code": "nb_td", "activated": False}]},
        ),
    ]:
        reponse = requests.post(_url(space_id, ctx), json=charge, headers=JSON, timeout=30)
        assert reponse.status_code == 422, f"{cas} : {reponse.status_code}"


def test_le_panneau_est_garde(saison_jouee):
    """`require_admin_access` sur les deux verbes. `requests` et non le
    navigateur : `bypass_auth` ne remplace jamais une identité déjà connectée."""
    space_id, ctx = saison_jouee["space_id"], saison_jouee["ctx"]

    lecture = requests.get(
        _url(space_id, ctx), headers={**JSON, **ENTETE_MEMBRE_SIMPLE}, timeout=10
    )
    assert lecture.status_code == 403, f"GET membre simple : {lecture.status_code}"

    ecriture = requests.post(
        _url(space_id, ctx),
        json=_bareme(9),
        headers={**JSON, **ENTETE_MEMBRE_SIMPLE},
        timeout=30,
    )
    assert ecriture.status_code == 403, f"POST membre simple : {ecriture.status_code}"

    # Contre-épreuve, et preuve que le refus n'a rien écrit.
    assert requests.get(_url(space_id, ctx), headers=JSON, timeout=10).status_code == 200
    assert query_db(
        "SELECT rules->'ranking_rules'->>'win_points' FROM competition_seasons "
        f"WHERE id = '{ctx['season_id']}'"
    ) != ["9"], "un refus ne doit rien enregistrer"

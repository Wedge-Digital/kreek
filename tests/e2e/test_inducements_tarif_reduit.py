"""Le tarif réduit d'une règle spéciale — carte 560.

Un coup de pouce coûte moins cher à l'équipe qui porte la règle spéciale que le
corpus désigne dans `reducedCostFor`. En production, « Chantage et Corruption »
baisse les Pots-de-vin de 100 à 50 et le Représentant véreux de 120 à 80. Le
champ n'était pas même désérialisé : la règle n'était appliquée nulle part.

**Le corpus de production n'est pas versionné**, donc ce test ne peut pas s'y
référer. Le jeu de démonstration porte la même règle sur le Renfort Temporaire,
réduit de 20 à 10 pour un roster aux hommes de base bon marché : `DEMO_LANTERNE`
porte `LOW_COST_LINEMEN`, `DEMO_GRANIT` et `DEMO_ZEPHYR` non. C'est le même
mécanisme, exercé de bout en bout.

**Le test lit les deux endroits d'où sort le prix**, comme celui du cuisinier
halfling : ce que le sélecteur affiche, et ce que l'achat retient. N'en
vérifier qu'un laisserait passer un écran qui annonce 10 et une trésorerie
débitée de 20.

Prérequis : serveur kreek lancé en dev-demo.
"""

import json
import re

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import create_draft, ensure_pre_match

RENFORT = "DEMO_RENFORT_TEMPORAIRE"
PLEIN_TARIF = 20
TARIF_REDUIT = 10

AVEC_LA_REGLE = "DEMO_LANTERNE"
SANS_LA_REGLE = "DEMO_GRANIT"


def _prix_affiche(roster_id: str) -> int:
    """Le prix que le sélecteur montre, par un GET direct.

    Sans budget ni petite monnaie : c'est le seul point où deux rosters sont
    comparables sur le seul tarif.
    """
    html = requests.get(
        f"{BASE_URL}/references/inducement-selector",
        params={
            "allowed_inducement_uids": RENFORT,
            "allowed_star_player_uids": "",
            "roster_id": roster_id,
        },
    ).text
    prix = re.findall(r'mr-inducement-price">(\d+) kPo', html)
    assert prix, f"le sélecteur ne propose pas le renfort à {roster_id}"
    return int(prix[0])


def test_le_selecteur_applique_le_tarif_reduit_de_la_regle_speciale():
    assert _prix_affiche(AVEC_LA_REGLE) == TARIF_REDUIT
    assert _prix_affiche(SANS_LA_REGLE) == PLEIN_TARIF


def _ouvrir_les_coups_de_pouce(space_id: str, mr_id: str) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "2", "away_fan_roll": "3"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"facteur fans : {resp.status_code}"


def _acheter(space_id: str, mr_id: str, team_id: str) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/inducements/{team_id}",
        data={
            "intent": "buy",
            "selection": json.dumps([{"uid": RENFORT, "qty": 1}]),
            "mercenaries": "[]",
        },
        allow_redirects=False,
    )
    assert resp.status_code in (200, 302, 303), (
        f"achat du renfort pour {team_id} : {resp.status_code}"
    )


def _prix_paye(mr_id: str, colonne: str) -> int:
    brut = query_db(
        f"SELECT {colonne} FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )
    assert brut, f"aucune projection pour {mr_id}"
    lignes = json.loads(brut[0])
    achats = [l for l in lignes if l["uid"] == RENFORT]
    assert achats, f"le renfort n'a pas été acheté ({colonne} = {lignes})"
    return int(achats[0]["unit_cost"])


@pytest.fixture(scope="module")
def achat_ctx(browser, space_id):
    """Un match entre le roster qui a la règle et un témoin qui ne l'a pas."""
    full = build_full_competition(
        browser,
        space_id,
        num_teams=2,
        num_rounds=2,
        roster_uids=[AVEC_LA_REGLE, SANS_LA_REGLE],
    )
    avec, temoin = full["team_ids"][0], full["team_ids"][1]

    mr_id = create_draft(space_id, full, full["round_ids"][0], avec, temoin)
    ensure_pre_match(space_id, mr_id, full, full["round_ids"][0], avec, temoin)
    _ouvrir_les_coups_de_pouce(space_id, mr_id)
    _acheter(space_id, mr_id, avec)

    return {"mr_id": mr_id, "avec": avec, "temoin": temoin}


def test_le_roster_teste_est_bien_celui_du_corpus(achat_ctx):
    """Le socle du suivant : sans ce roster, il passerait en n'exerçant rien.

    La preuve passe par `roster_line_id`, qui porte l'uid du roster — celui que
    la règle nomme. `team_proj.roster_name` ne garde que le libellé.
    """
    lignes = query_db(
        "SELECT DISTINCT split_part(roster_line_id, '__', 1) FROM players_proj "
        f"WHERE team_id = '{achat_ctx['avec']}'"
    )
    assert lignes == [AVEC_LA_REGLE], f"obtenu {lignes}"


def test_le_prix_debite_est_le_tarif_reduit(achat_ctx):
    """L'autre endroit d'où sort le prix — celui qui part de la trésorerie."""
    assert _prix_paye(achat_ctx["mr_id"], "home_inducements") == TARIF_REDUIT

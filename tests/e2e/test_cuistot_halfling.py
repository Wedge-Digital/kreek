"""Le prix du maître cuisinier halfling — carte 507.

Il coûte 300 kPo, sauf à l'équipe halfling qui le paie 100. L'application le
facturait 300 à tout le monde : `reducedCost` était lu du corpus et **utilisé
nulle part**.

**Ce test lit ce qui a été appliqué, pas ce qui est affiché.** Le prix sortait
de deux endroits indépendants — celui que montre le sélecteur et celui que
débite le rapport de match. Un test qui lirait le montant à l'écran passerait
alors même que le coach serait prélevé de 300, ce qui serait pire que le défaut
d'origine.

La mesure est `match_report_proj.home_inducements[].unit_cost`, le prix
réellement retenu par l'achat. La trésorerie ne conviendrait pas : l'underdog
paie ses coups de pouce avec sa petite monnaie, et le débit ne dirait donc rien
du tarif.

**Le témoin partage la ligue du halfling.** `HALFLING` et `DEMO_GRANIT` sont
tous deux dans `LIGUE_DES_CIMES` : si le prix suivait la ligue — l'erreur
d'analyse qu'a corrigée cette carte — les deux paieraient pareil et le test
échouerait.

Prérequis : serveur kreek lancé en dev-demo.
"""

import json
import re

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import create_draft, ensure_pre_match

CUISTOT = "HALFLING_MASTER_CHEF"
PLEIN_TARIF = 300
TARIF_REDUIT = 100


def _ouvrir_les_coups_de_pouce(space_id: str, mr_id: str) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "2", "away_fan_roll": "3"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"facteur fans : {resp.status_code}"


def _acheter_le_cuistot(space_id: str, mr_id: str, team_id: str):
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/inducements/{team_id}",
        data={
            "intent": "buy",
            "selection": json.dumps([{"uid": CUISTOT, "qty": 1}]),
            "mercenaries": "[]",
        },
        allow_redirects=False,
    )
    assert resp.status_code in (200, 302, 303), (
        f"achat du cuistot pour {team_id} : {resp.status_code} — "
        "coup de pouce non autorisé par le tier, ou prix hors budget : c'est "
        "ce que produit le défaut de la carte 507, un cuistot facturé 300 à "
        "une équipe halfling dont le budget n'y suffit pas"
    )
    return resp


def _prix_paye(mr_id: str, colonne: str) -> int:
    """Le prix unitaire retenu par l'achat, tel que la projection le garde."""
    brut = query_db(
        f"SELECT {colonne} FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )
    assert brut, f"aucune projection pour {mr_id}"
    lignes = json.loads(brut[0])
    cuistots = [l for l in lignes if l["uid"] == CUISTOT]
    assert cuistots, f"le cuistot n'a pas été acheté ({colonne} = {lignes})"
    return int(cuistots[0]["unit_cost"])


@pytest.fixture(scope="module")
def cuistot_ctx(browser, space_id):
    """Un match halfling contre témoin, où les deux camps achètent le cuistot."""
    full = build_full_competition(
        browser,
        space_id,
        num_teams=2,
        num_rounds=2,
        roster_uids=["HALFLING", "DEMO_GRANIT"],
    )
    halfling, temoin = full["team_ids"][0], full["team_ids"][1]

    mr_id = create_draft(space_id, full, full["round_ids"][0], halfling, temoin)
    ensure_pre_match(space_id, mr_id, full, full["round_ids"][0], halfling, temoin)
    _ouvrir_les_coups_de_pouce(space_id, mr_id)

    # **Seul le halfling achète.** Le témoin est underdog ou top dog selon les
    # valeurs d'équipe, et le budget de coups de pouce de l'underdog est
    # plafonné à sa petite monnaie plus 50 kPo : lui faire acheter un cuistot à
    # 300 ferait dépendre le test d'un équilibre de valeurs d'équipe, pas du
    # tarif. Le plein tarif appliqué est couvert par le test de l'adapter.
    _acheter_le_cuistot(space_id, mr_id, halfling)

    return {"space_id": space_id, "mr_id": mr_id, "halfling": halfling, "temoin": temoin}


def test_le_roster_halfling_est_bien_celui_du_corpus(cuistot_ctx):
    """Le socle des deux suivants : sans ce roster, ils passeraient en
    n'exerçant rien.

    La preuve passe par `roster_line_id`, qui porte l'**uid** du roster — celui
    que la règle nomme. `team_proj.roster_name` ne garde que le libellé, qu'un
    renommage du corpus ferait varier sans rien dire de la règle.
    """
    lignes = query_db(
        "SELECT DISTINCT split_part(roster_line_id, '__', 1) FROM players_proj "
        f"WHERE team_id = '{cuistot_ctx['halfling']}'"
    )
    assert lignes == ["HALFLING"], f"obtenu {lignes}"


def test_l_equipe_halfling_paie_le_cuistot_a_prix_reduit(cuistot_ctx):
    assert _prix_paye(cuistot_ctx["mr_id"], "home_inducements") == TARIF_REDUIT


def _prix_affiche(roster_id: str) -> int:
    """Le prix que le sélecteur montre — l'autre endroit d'où sortait le tarif.

    Un GET, donc sans budget ni petite monnaie : c'est le seul point où les
    deux rosters sont comparables sur le seul tarif.
    """
    html = requests.get(
        f"{BASE_URL}/references/inducement-selector",
        params={
            "allowed_inducement_uids": CUISTOT,
            "allowed_star_player_uids": "",
            "roster_id": roster_id,
        },
    ).text
    prix = re.findall(r'mr-inducement-price">(\d+) kPo', html)
    assert prix, f"le sélecteur ne propose pas le cuistot à {roster_id}"
    return int(prix[0])


def test_le_selecteur_affiche_le_meme_prix_que_celui_applique():
    """**Les deux points d'entrée du prix**, et la raison d'être de la carte.

    Il sortait du référentiel à deux endroits indépendants — ici et dans
    l'adapter du rapport de match. N'en corriger qu'un ferait lire 100 au coach
    en lui prélevant 300, ce qui serait pire que le défaut d'origine.

    `DEMO_GRANIT` partage `LIGUE_DES_CIMES` avec le halfling : si le prix
    suivait la ligue — l'erreur d'analyse que cette carte a corrigée — les deux
    afficheraient la même chose.
    """
    assert _prix_affiche("HALFLING") == TARIF_REDUIT
    assert _prix_affiche("DEMO_GRANIT") == PLEIN_TARIF

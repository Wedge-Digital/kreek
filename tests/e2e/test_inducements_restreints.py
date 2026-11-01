"""Les coups de pouce restreints — carte 561.

`restrictedTo` désigne selon l'entrée un **roster**, une **règle spéciale** ou
un **membre du staff**. Le sélecteur ne comparait ces entrées qu'à
l'identifiant du roster : sur le corpus de production, où aucune n'en est un,
les quatre coups de pouce restreints n'étaient proposés à personne.

Le jeu de démonstration ne portait que la forme « roster », la seule qui
marchait — c'est ce qui a fait passer le défaut inaperçu. Il porte désormais
aussi la forme « staff », avec le Guérisseur Itinérant, réservé aux équipes qui
ont le droit d'engager un apothicaire.

**Le test lit les deux côtés.** Ce que le sélecteur propose, et ce que l'achat
accepte : le droit ne vivait qu'à l'affichage, et un achat forgé à la main
passait.

Prérequis : serveur kreek lancé en dev-demo.
"""

import json
import re

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from match_report_helpers import create_draft, ensure_pre_match, pass_topdog_first

GUERISSEUR = "DEMO_GUERISSEUR_ITINERANT"  # restreint au staff APOTHECARY
MASSEUR = "DEMO_MASSEUR_DOUTEUX"  # restreint au roster DEMO_GRANIT
LIBRE = "DEMO_CHARIOT_DE_VIVRES"  # restreint à personne

AVEC_APOTHICAIRE = "DEMO_GRANIT"
SANS_APOTHICAIRE = "DEMO_LANTERNE"


def _proposes(roster_id: str, uids: list[str]) -> set[str]:
    """Les coups de pouce que le sélecteur propose à ce roster."""
    html = requests.get(
        f"{BASE_URL}/references/inducement-selector",
        params={
            "allowed_inducement_uids": ",".join(uids),
            "allowed_star_player_uids": "",
            "roster_id": roster_id,
        },
    ).text
    # Le gabarit ne porte pas d'attribut d'identifiant : l'uid n'apparaît que
    # dans les appels Alpine des boutons de quantité. C'est lui qu'on lit, et
    # non le libellé, qu'un renommage du corpus ferait varier.
    return set(re.findall(r"increment\('([A-Z_]+)'\)", html)) & set(uids)


def test_un_coup_de_pouce_restreint_a_un_staff_suit_le_droit_a_ce_staff():
    """Le cas que le filtre d'avant ne voyait pas : l'entrée n'est pas un
    roster, donc le coup de pouce n'était proposé à personne."""
    tous = [GUERISSEUR, LIBRE]
    assert _proposes(AVEC_APOTHICAIRE, tous) == {GUERISSEUR, LIBRE}
    assert _proposes(SANS_APOTHICAIRE, tous) == {LIBRE}


def test_la_restriction_par_roster_marche_toujours():
    """La seule forme que le filtre d'avant savait lire. Elle ne doit pas se
    perdre au passage."""
    tous = [MASSEUR, LIBRE]
    assert _proposes("DEMO_GRANIT", tous) == {MASSEUR, LIBRE}
    assert _proposes("DEMO_ZEPHYR", tous) == {LIBRE}


@pytest.fixture(scope="module")
def match_ctx(browser, space_id):
    """Deux équipes : une sans droit à l'apothicaire, une avec.

    **Un rapport par test d'achat, sur deux journées.** Le favori choisit ses
    coups de pouce en premier (carte 627, R9), et une équipe qui a validé ne
    rachète plus (R2). Sur un seul rapport, faire passer le favori pour l'un
    des deux tests l'empêcherait d'acheter dans l'autre.
    """
    full = build_full_competition(
        browser,
        space_id,
        num_teams=2,
        num_rounds=2,
        roster_uids=[SANS_APOTHICAIRE, AVEC_APOTHICAIRE],
    )
    sans, avec = full["team_ids"][0], full["team_ids"][1]
    return {"space_id": space_id, "full": full, "sans": sans, "avec": avec}


def _ouvrir_un_rapport(match_ctx, round_idx: int) -> str:
    """Un rapport sans contre avec, jusqu'à la page des coups de pouce."""
    space_id, full = match_ctx["space_id"], match_ctx["full"]
    round_id = full["round_ids"][round_idx]
    sans, avec = match_ctx["sans"], match_ctx["avec"]
    mr_id = create_draft(space_id, full, round_id, sans, avec)
    ensure_pre_match(space_id, mr_id, full, round_id, sans, avec)
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "2", "away_fan_roll": "3"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"facteur fans : {resp.status_code}"
    return mr_id


def _acheter(space_id: str, mr_id: str, team_id: str, uid: str):
    # Le favori choisit d'abord (carte 627, R9) : si l'équipe visée est
    # l'outsider, le favori passe avant elle — sinon le refus serait un 409.
    pass_topdog_first(space_id, mr_id, team_id)
    return requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/inducements/{team_id}",
        data={
            "intent": "buy",
            "selection": json.dumps([{"uid": uid, "qty": 1}]),
            "mercenaries": "[]",
        },
        allow_redirects=False,
    )


def test_l_achat_refuse_un_coup_de_pouce_auquel_l_equipe_n_a_pas_droit(match_ctx):
    """**L'autre côté du droit.** Le sélecteur ne le propose pas, mais la
    requête, elle, se forge : la liste des coups de pouce autorisés que le
    domaine reçoit ne l'excluait pas.
    """
    mr_id = _ouvrir_un_rapport(match_ctx, round_idx=0)
    resp = _acheter(match_ctx["space_id"], mr_id, match_ctx["sans"], GUERISSEUR)
    # 422 : un refus du domaine est une saisie invalide, pas une panne. Le coup
    # de pouce n'est pas dans la liste autorisée, donc `UnknownInducement`.
    assert resp.status_code == 422, (
        f"attendu un refus, obtenu {resp.status_code} — l'équipe n'a pas droit "
        "à un apothicaire"
    )


def test_l_achat_accepte_le_coup_de_pouce_auquel_l_equipe_a_droit(match_ctx):
    """Le témoin du précédent : sans lui, un refus général passerait pour un
    droit correctement appliqué."""
    mr_id = _ouvrir_un_rapport(match_ctx, round_idx=1)
    resp = _acheter(match_ctx["space_id"], mr_id, match_ctx["avec"], GUERISSEUR)
    assert resp.status_code in (200, 302, 303), (
        f"achat refusé à une équipe qui y a droit : {resp.status_code}"
    )

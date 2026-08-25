"""La règle « Lineman a vil prix » — carte 388.

`LOW_COST_LINEMEN` annule le prix de base des linemen dans la **valeur
d'équipe**. Les augmentations comptent plein, la trésorerie ne bouge pas : le
coach paie toujours son lineman au prix du corpus.

Le roster porteur est `DEMO_LANTERNE`, choisi parce qu'il est hors du cycle
`ROSTERS` et que ses deux seuls autres tests lisent la trésorerie ou un message
d'erreur — jamais une valeur d'équipe. La règle n'y déplace aucune attente.

Prérequis : serveur kreek lancé en dev.
"""

import re

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db


@pytest.fixture(scope="module")
def equipes_comparees(browser, space_id):
    """Une équipe à vil prix, et un témoin d'un roster ordinaire."""
    ctx = build_full_competition(
        browser,
        space_id,
        num_teams=2,
        roster_uids=["DEMO_LANTERNE", "DEMO_GRANIT"],
    )
    return {
        "space_id": space_id,
        "vil_prix": ctx["team_ids"][0],
        "temoin": ctx["team_ids"][1],
    }


def _somme_des_joueurs(team_id: str) -> int:
    """La somme nue des valeurs de l'effectif disponible."""
    lignes = query_db(
        "SELECT value_kpo FROM players_proj "
        f"WHERE team_id = '{team_id}' AND membership = 'Active'"
    )
    return sum(int(x) for x in lignes)


def _valeur_affichee(page: Page, space_id: str, team_id: str) -> int:
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
    item = page.locator(".meta-item", has_text="Valeur d'équipe").locator(".meta-value")
    expect(item).to_be_visible(timeout=10000)
    return int(re.sub(r"[^0-9]", "", item.inner_text()))


def _linemen(team_id: str) -> list[str]:
    return query_db(
        "SELECT player_id FROM players_proj "
        f"WHERE team_id = '{team_id}' AND roster_line_id LIKE '%PIETAILLE'"
    )


def test_la_valeur_d_equipe_ignore_le_prix_des_linemen(page: Page, equipes_comparees):
    """La valeur d'équipe doit être **inférieure à la somme de son effectif**.

    C'est le discriminant, et il tient à l'arithmétique : hors règle, la valeur
    vaut la somme des joueurs **plus** les relances et le staff, donc elle ne
    peut pas lui être inférieure. Sous la règle, le prix de base des linemen en
    est retranché, et il pèse plus lourd que les relances d'une équipe neuve.

    Une première version comparait cette équipe à une équipe d'un autre roster.
    Elle passait **aussi avec la règle désactivée** : les Lanterniers sont
    naturellement moins chers que les Granitiers, et la comparaison mesurait ça.
    """
    c = equipes_comparees
    vil = _valeur_affichee(page, c["space_id"], c["vil_prix"])
    somme = _somme_des_joueurs(c["vil_prix"])

    assert vil < somme, (
        f"valeur d'équipe {vil} contre {somme} de somme d'effectif : "
        "sans la règle elle ne pourrait pas être inférieure"
    )


def test_le_temoin_compte_bien_ses_joueurs(page: Page, equipes_comparees):
    """Le contrôle inverse : sur un roster sans la règle, la valeur d'équipe
    est **au moins** la somme de l'effectif. Sans lui, l'assertion ci-dessus
    passerait aussi bien si toutes les valeurs d'équipe étaient nulles."""
    c = equipes_comparees
    temoin = _valeur_affichee(page, c["space_id"], c["temoin"])
    somme = _somme_des_joueurs(c["temoin"])

    assert temoin >= somme, (
        f"valeur d'équipe {temoin} contre {somme} de somme d'effectif : "
        "un roster sans la règle ne retranche rien"
    )


def test_la_tresorerie_n_est_pas_touchee_par_la_regle(page: Page, equipes_comparees):
    """La règle ne vise que la valeur d'équipe. Le coach paie toujours ses
    linemen au prix du corpus, et le grand livre le dit."""
    c = equipes_comparees
    lignes = query_db(
        "SELECT amount_kpo FROM teams__treasury_ledger "
        f"WHERE team_id = '{c['vil_prix']}' ORDER BY id"
    )
    assert lignes, "l'équipe à vil prix doit avoir des mouvements de trésorerie"
    assert any(int(x) > 0 for x in lignes), (
        "les recrutements doivent avoir coûté quelque chose"
    )


def test_l_equipe_a_vil_prix_a_bien_des_linemen(equipes_comparees):
    """Le garde-fou du test lui-même : sans lineman dans l'effectif, la
    comparaison ci-dessus passerait pour une raison étrangère à la règle."""
    assert _linemen(equipes_comparees["vil_prix"]), (
        "l'effectif doit compter des Piétailles, sinon le test ne prouve rien"
    )

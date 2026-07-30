"""Tests E2E — les coups de pouce et la trésorerie.

Le bug que ces tests verrouillent : les achats de coups de pouce étaient
**validés** contre la trésorerie, puis jamais débités. Le garde-fou existait, le
paiement non.

Aucun test ne l'avait vu parce qu'aucun test n'achetait quoi que ce soit :
`match_report_helpers.ensure_inducements` poste une sélection vide pour traverser
l'étape au plus vite. Ces scénarios-ci achètent pour de bon, des deux côtés.

La règle, qui n'est pas symétrique :

- le **top dog** paie l'intégralité de ses achats avec sa trésorerie, et rien
  d'autre ;
- l'**underdog** dispose d'une petite monnaie — l'écart de valeur d'équipe plus
  ce que le top dog vient de dépenser — qui ne sort d'aucune caisse. Seul le
  dépassement lui est facturé, et son budget le plafonne à 50 kPo.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) et base seedée.
"""

import json
import time

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import (
    create_draft,
    ensure_pre_match,
    post_step5,
    publish,
)

# Corpus de démonstration (`assets/references.example/inducements_fr.json`).
MAGE = "DEMO_MAGE_DES_BRUMES"      # 60
ARBITRE = "DEMO_ARBITRE_COMPLAISANT"  # 50
MASSEUR = "DEMO_MASSEUR_DOUTEUX"   # 30, max 2
RENFORT = "DEMO_RENFORT_TEMPORAIRE"  # 20, max 3

PLAFOND_APPOINT = 50
GAIN_MATCH_KPO = 5


# ── Helpers ───────────────────────────────────────────────────────────────────


def _attendre(predicat, quoi, timeout_s=20):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if predicat():
            return
        time.sleep(0.2)
    raise AssertionError(f"{quoi} : jamais satisfait après {timeout_s}s")


def _tresorerie(team_id: str) -> int:
    """Le solde vient du grand livre : `team_proj` ne porte pas la trésorerie."""
    rows = query_db(
        "SELECT balance_after_kpo FROM teams__treasury_ledger "
        f"WHERE team_id = '{team_id}' ORDER BY id DESC LIMIT 1"
    )
    assert rows, f"aucun mouvement de trésorerie pour {team_id}"
    return int(rows[0])


def _lignes_coups_de_pouce(team_id: str) -> list[str]:
    return query_db(
        "SELECT direction, amount_kpo, balance_after_kpo FROM teams__treasury_ledger "
        f"WHERE team_id = '{team_id}' AND reason = 'InducementPurchase' ORDER BY id"
    )


def _valeur_equipe(team_id: str) -> int:
    return int(query_db(f"SELECT team_value FROM team_proj WHERE team_id = '{team_id}'")[0])


def _ouvrir_les_coups_de_pouce(space_id: str, mr_id: str) -> None:
    """Le facteur fans ouvre l'étape des coups de pouce."""
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "2", "away_fan_roll": "3"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"facteur fans : {resp.status_code}"


def _acheter(space_id: str, mr_id: str, team_id: str, achats: list[tuple[str, int]]):
    """Achète pour une équipe donnée, sans dépendre de l'ordre des redirections.

    L'ordre importe : la petite monnaie de l'underdog contient les dépenses du
    top dog, donc le top dog doit acheter en premier pour que le second en
    bénéficie.
    """
    selection = json.dumps([{"uid": uid, "qty": qty} for uid, qty in achats])
    return requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/inducements/{team_id}",
        data={"intent": "buy", "selection": selection, "mercenaries": "[]"},
        allow_redirects=False,
    )


def _cout(achats: list[tuple[str, int]]) -> int:
    prix = {MAGE: 60, ARBITRE: 50, MASSEUR: 30, RENFORT: 20}
    return sum(prix[uid] * qty for uid, qty in achats)


# ── Fixture : un match où les deux camps achètent ─────────────────────────────


@pytest.fixture(scope="module")
def coups_de_pouce_ctx(browser, space_id):
    """Deux équipes de valeurs différentes — c'est l'écart qui fait la petite
    monnaie, et sans lui le scénario de l'underdog n'aurait aucun sujet.
    """
    full = build_full_competition(browser, space_id, num_teams=2, num_rounds=2)
    home, away = full["team_ids"][0], full["team_ids"][1]

    tv_home, tv_away = _valeur_equipe(home), _valeur_equipe(away)
    assert tv_home != tv_away, (
        f"les deux équipes ont la même valeur ({tv_home}) : ni top dog ni underdog, "
        "le scénario n'a plus d'objet — cf. la carte sur l'égalité de TV"
    )
    topdog, underdog = (home, away) if tv_home > tv_away else (away, home)

    caisse_topdog_avant = _tresorerie(topdog)
    caisse_underdog_avant = _tresorerie(underdog)

    mr_id = create_draft(space_id, full, full["round_ids"][0], home, away)
    ensure_pre_match(space_id, mr_id, full, full["round_ids"][0], home, away)
    _ouvrir_les_coups_de_pouce(space_id, mr_id)

    # Le top dog d'abord : sa dépense grossit la petite monnaie de l'autre.
    achats_topdog = [(MAGE, 1), (RENFORT, 2)]  # 100
    r = _acheter(space_id, mr_id, topdog, achats_topdog)
    assert r.status_code in (200, 302, 303), f"achats top dog : {r.status_code}"

    petite_monnaie = abs(tv_home - tv_away) + _cout(achats_topdog)
    # De quoi dépasser la petite monnaie, sans dépasser le budget.
    achats_underdog = [(ARBITRE, 1), (MAGE, 1), (MASSEUR, 2)]  # 170
    assert _cout(achats_underdog) > petite_monnaie, "le scénario doit faire payer l'underdog"
    assert _cout(achats_underdog) <= petite_monnaie + PLAFOND_APPOINT, "et rester dans le budget"
    r = _acheter(space_id, mr_id, underdog, achats_underdog)
    assert r.status_code in (200, 302, 303), f"achats underdog : {r.status_code}"

    post_step5(space_id, mr_id, home_gain=GAIN_MATCH_KPO, away_gain=GAIN_MATCH_KPO)
    publish(space_id, mr_id)

    _attendre(
        lambda: query_db(f"SELECT game_phase FROM team_proj WHERE team_id = '{topdog}'")
        == ["PlayerImprovement"],
        "la séquence d'après-match a démarré",
    )

    return {
        "space_id": space_id,
        "mr_id": mr_id,
        "topdog": topdog,
        "underdog": underdog,
        "caisse_topdog_avant": caisse_topdog_avant,
        "caisse_underdog_avant": caisse_underdog_avant,
        "achats_topdog": _cout(achats_topdog),
        "achats_underdog": _cout(achats_underdog),
        "petite_monnaie": petite_monnaie,
    }


# ── 1 · Le top dog paie tout ──────────────────────────────────────────────────


def test_01_le_topdog_paie_l_integralite_de_ses_achats(coups_de_pouce_ctx):
    ctx = coups_de_pouce_ctx
    attendue = ctx["caisse_topdog_avant"] + GAIN_MATCH_KPO - ctx["achats_topdog"]

    _attendre(
        lambda: _tresorerie(ctx["topdog"]) == attendue,
        f"trésorerie du top dog à {attendue} "
        f"(sans débit : {ctx['caisse_topdog_avant'] + GAIN_MATCH_KPO})",
    )

    lignes = _lignes_coups_de_pouce(ctx["topdog"])
    assert len(lignes) == 1, f"une ligne de grand livre, trouvé {lignes}"
    direction, montant, _ = lignes[0].split("|")
    assert direction == "Debit"
    assert int(montant) == ctx["achats_topdog"]


# ── 2 · L'underdog ne paie que le dépassement ─────────────────────────────────


def test_02_l_underdog_ne_paie_que_le_depassement(coups_de_pouce_ctx):
    """Le nombre qui distingue une correction juste d'un débit naïf.

    Trois issues possibles, et elles diffèrent :
      — aucun débit : la caisse ne bouge que du gain de match ;
      — débit naïf du montant acheté : 170 kPo retirés ;
      — débit juste : seul le dépassement de la petite monnaie, soit 15 kPo.
    """
    ctx = coups_de_pouce_ctx
    du = ctx["achats_underdog"] - ctx["petite_monnaie"]
    assert 0 < du <= PLAFOND_APPOINT, f"le dépassement doit être facturable, calculé {du}"

    base = ctx["caisse_underdog_avant"] + GAIN_MATCH_KPO
    attendue = base - du
    naif = base - ctx["achats_underdog"]
    assert len({attendue, naif, base}) == 3, "le test doit discriminer les trois issues"

    _attendre(
        lambda: _tresorerie(ctx["underdog"]) == attendue,
        f"trésorerie de l'underdog à {attendue} (sans débit {base}, débit naïf {naif})",
    )

    lignes = _lignes_coups_de_pouce(ctx["underdog"])
    assert len(lignes) == 1, f"une ligne de grand livre, trouvé {lignes}"
    assert int(lignes[0].split("|")[1]) == du


# ── 3 · Dépublier rend l'argent ───────────────────────────────────────────────


def test_03_depublier_rembourse_les_coups_de_pouce(coups_de_pouce_ctx):
    """Sans le remboursement, corriger un rapport rendrait les gains du match
    mais garderait l'argent des coups de pouce — un trou de trésorerie à chaque
    correction.
    """
    ctx = coups_de_pouce_ctx
    space_id, mr_id = ctx["space_id"], ctx["mr_id"]

    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap/unpublish",
        allow_redirects=False,
    )
    assert resp.status_code in (200, 302, 303), f"dépublication : {resp.status_code}"

    # On retrouve la caisse d'avant le match : gains repris, coups de pouce rendus.
    for camp, avant in (
        (ctx["topdog"], ctx["caisse_topdog_avant"]),
        (ctx["underdog"], ctx["caisse_underdog_avant"]),
    ):
        _attendre(
            lambda c=camp, a=avant: _tresorerie(c) == a,
            f"trésorerie de {camp} revenue à {avant}",
        )

    rendus = query_db(
        "SELECT amount_kpo FROM teams__treasury_ledger "
        f"WHERE team_id = '{ctx['underdog']}' AND reason = 'InducementRefunded'"
    )
    assert rendus, "le remboursement laisse sa trace au grand livre"

"""Tests E2E — le barème de SPP dépend de la règle spéciale du roster.

Une équipe portant `BRAWLIN_BRUTES` — « Brutes Bagarreuses » — suit un barème
inversé : chez elle, la sortie rapporte plus que l'essai. Six rosters du corpus
réel la portent ; côté démonstration, ce sont les Granitiers.

Ce que ce fichier vérifie et qu'aucun test unitaire ne peut montrer : que le
barème résolu par `references` traverse réellement l'app event jusqu'aux SPP
inscrits dans `players_proj`. Le barème était codé en dur en Rust, et
`spp_rules.json` n'était lu par personne.

Le test est **discriminant par construction** : les deux équipes du match
accomplissent les mêmes actions et doivent en retirer des SPP différents. Un
barème unique — quel qu'il soit — les rendrait égaux et ferait échouer le test.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) et base seedée.
"""

import time

import pytest

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    post_step5,
    publish,
    record_action_api,
)

# Corpus de démonstration. Les Granitiers portent `BRAWLIN_BRUTES`, les
# Zéphyriens non — c'est le seul écart qui doit expliquer la différence.
TD_BRUTES, CAS_BRUTES = 2, 4
TD_NORMAL, CAS_NORMAL = 4, 2


def _attendre(predicat, quoi, timeout_s=20):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if predicat():
            return
        time.sleep(0.2)
    raise AssertionError(f"{quoi} : jamais satisfait après {timeout_s}s")


def _spp(player_id: str) -> int:
    rows = query_db(f"SELECT spp FROM players_proj WHERE player_id = '{player_id}'")
    assert rows, f"joueur {player_id} absent de la projection"
    return int(rows[0])


def _premier_joueur(team_id: str) -> str:
    rows = query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{team_id}' "
        "AND membership = 'Active' ORDER BY jersey NULLS LAST, player_id LIMIT 1"
    )
    assert rows, f"aucun joueur dans {team_id}"
    return rows[0]


def _roster(team_id: str) -> str:
    return query_db(f"SELECT roster_name FROM team_proj WHERE team_id = '{team_id}'")[0]


@pytest.fixture(scope="module")
def bareme_ctx(browser, space_id):
    """Un match Granitiers contre Zéphyriens, aux gestes **asymétriques**.

    Une première version faisait marquer un essai et une sortie à chacun : les
    deux barèmes étant une permutation l'un de l'autre, les totaux se
    rejoignaient à 6 et deux des trois tests ne pouvaient pas échouer. Vérifié
    par mutation, puis corrigé — chaque camp accomplit maintenant un mélange
    d'actions dont le total diffère selon le barème appliqué.
    """
    full = build_full_competition(browser, space_id, num_teams=2, num_rounds=2)
    brutes, normaux = full["team_ids"][0], full["team_ids"][1]

    assert _roster(brutes) == "Granitiers", "l'équipe 0 doit porter BRAWLIN_BRUTES"
    assert _roster(normaux) == "Zéphyriens", "l'équipe 1 sert de témoin"

    heros_brutes = _premier_joueur(brutes)
    heros_normal = _premier_joueur(normaux)
    spp_avant = {heros_brutes: _spp(heros_brutes), heros_normal: _spp(heros_normal)}

    mr_id = create_draft(space_id, full, full["round_ids"][0], brutes, normaux)
    ensure_pre_match(space_id, mr_id, full, full["round_ids"][0], brutes, normaux)
    ensure_inducements(space_id, mr_id)

    # Un essai et deux sorties chez les Brutes, deux essais et une sortie en
    # face : les totaux ne se rejoignent qu'avec le bon barème de chaque côté.
    record_action_api(space_id, mr_id, "home", heros_brutes, turn=1, action_type="TOUCHDOWN")
    record_action_api(space_id, mr_id, "home", heros_brutes, turn=2, action_type="SORTIE")
    record_action_api(space_id, mr_id, "home", heros_brutes, turn=3, action_type="SORTIE")
    record_action_api(space_id, mr_id, "away", heros_normal, turn=1, action_type="TOUCHDOWN")
    record_action_api(space_id, mr_id, "away", heros_normal, turn=2, action_type="TOUCHDOWN")
    record_action_api(space_id, mr_id, "away", heros_normal, turn=3, action_type="SORTIE")

    post_step5(space_id, mr_id, home_gain=5, away_gain=5)
    publish(space_id, mr_id)

    # 1 essai + 2 sorties, puis 2 essais + 1 sortie.
    attendu_brutes = spp_avant[heros_brutes] + TD_BRUTES + 2 * CAS_BRUTES
    attendu_normal = spp_avant[heros_normal] + 2 * TD_NORMAL + CAS_NORMAL
    _attendre(
        lambda: _spp(heros_brutes) == attendu_brutes and _spp(heros_normal) == attendu_normal,
        f"SPP crédités : brutes {attendu_brutes}, normaux {attendu_normal} "
        f"(observé {_spp(heros_brutes)} et {_spp(heros_normal)})",
    )

    return {
        "heros_brutes": heros_brutes,
        "heros_normal": heros_normal,
        "spp_avant": spp_avant,
    }


def test_01_les_brutes_bagarreuses_valorisent_la_sortie_plus_que_l_essai(bareme_ctx):
    """Chez les Brutes, l'essai vaut 2 et la sortie 4 — l'inverse du barème
    ordinaire. C'est la règle que la carte 275 met en œuvre.
    """
    ctx = bareme_ctx
    gagne = _spp(ctx["heros_brutes"]) - ctx["spp_avant"][ctx["heros_brutes"]]
    juste = TD_BRUTES + 2 * CAS_BRUTES          # 2 + 8 = 10
    avec_bareme_ordinaire = TD_NORMAL + 2 * CAS_NORMAL  # 4 + 4 = 8
    assert juste != avec_bareme_ordinaire, "le test doit discriminer"
    assert gagne == juste, f"attendu {juste}, un barème unique donnerait {avec_bareme_ordinaire}"
    assert CAS_BRUTES > TD_BRUTES, "chez les Brutes, la sortie prime"


def test_02_le_bareme_ordinaire_reste_inchange(bareme_ctx):
    """Le contraste, sans lequel le premier test ne prouverait rien : un roster
    sans la règle garde l'essai à 4 et la sortie à 2.
    """
    ctx = bareme_ctx
    gagne = _spp(ctx["heros_normal"]) - ctx["spp_avant"][ctx["heros_normal"]]
    juste = 2 * TD_NORMAL + CAS_NORMAL        # 8 + 2 = 10
    avec_bareme_brutes = 2 * TD_BRUTES + CAS_BRUTES  # 4 + 4 = 8
    assert juste != avec_bareme_brutes, "le test doit discriminer"
    assert gagne == juste, f"attendu {juste}, le barème des Brutes donnerait {avec_bareme_brutes}"
    assert TD_NORMAL > CAS_NORMAL, "ailleurs, l'essai prime"


def test_03_les_deux_camps_ne_gagnent_pas_les_memes_spp_par_action(bareme_ctx):
    """Le test qui ne peut pas passer avec un barème unique.

    Les deux premiers scénarios comparent des totaux ; celui-ci descend au
    **détail par action**, seul endroit où l'inversion se lit directement. Un
    barème unique rendrait les deux journaux identiques.
    """
    ctx = bareme_ctx

    # Le journal du joueur porte le montant de chaque action : on le relit
    # depuis l'event store, seul endroit où les deux gains sont distingués —
    # la projection ne garde que le cumul.
    def gains(joueur: str) -> dict[str, int]:
        rows = query_db(
            "SELECT payload FROM players_events "
            f"WHERE player_id = '{joueur}' ORDER BY version"
        )
        import json

        trouves = {}
        for r in rows:
            e = json.loads(r)
            for cle, valeur in e.items():
                if isinstance(valeur, dict) and "spp_earned" in valeur:
                    trouves[cle] = valeur["spp_earned"]
        return trouves

    brutes = gains(ctx["heros_brutes"])
    normaux = gains(ctx["heros_normal"])
    assert brutes.get("TouchdownScored") == TD_BRUTES
    assert brutes.get("CasualtyInflicted") == CAS_BRUTES
    assert normaux.get("TouchdownScored") == TD_NORMAL
    assert normaux.get("CasualtyInflicted") == CAS_NORMAL
    assert brutes != normaux, "un barème unique rendrait les deux journaux identiques"

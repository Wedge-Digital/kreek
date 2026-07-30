"""Tests E2E — les SPP affichés par le récapitulatif de match.

Le récapitulatif créditait **10 SPP à tout le monde**, quoi qu'ait fait le
joueur. Ce n'était pas un oubli de câblage mais un stub assumé, qui annonçait
lui-même sa carte de remplacement.

Ce que ce fichier verrouille, et qu'aucun test ne regardait : la **valeur
affichée**. `test_match_report_recap` vérifie que la carte « Performances » est
là ou pas, jamais ce qu'elle contient — c'est précisément ce qui a laissé vivre
le 10 pendant toute la vie du stub.

Le récapitulatif s'affiche **avant publication**, quand rien n'a encore été
crédité à personne : il calcule, il ne lit pas `players`. Les deux chemins
doivent pourtant dire la même chose, et notamment appliquer le même barème —
celui du roster, que `BRAWLIN_BRUTES` inverse.

Les scénarios sont **discriminants par construction** : chaque total attendu
diffère de 10, du nombre d'actions, et de ce que donnerait l'autre barème.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) et base seedée.
"""

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    post_step5,
    record_action_api,
)

# Corpus de démonstration (`assets/references.example/spp_rules.json`).
BAREMES = {
    "brawlin_brutes": {"td": 2, "cas": 4, "mvp": 5},
    "normal": {"td": 4, "cas": 2, "mvp": 5},
}
STUB = 10  # la valeur forfaitaire d'avant, qu'aucun total ne doit valoir par hasard


def _roster(team_id: str) -> str:
    return query_db(f"SELECT roster_name FROM team_proj WHERE team_id = '{team_id}'")[0]


def _bareme(team_id: str) -> dict:
    """Seuls les Granitiers portent `BRAWLIN_BRUTES` dans le corpus de démo."""
    clef = "brawlin_brutes" if _roster(team_id) == "Granitiers" else "normal"
    return BAREMES[clef]


def _joueurs(team_id: str, combien: int) -> list[str]:
    rows = query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{team_id}' "
        f"AND membership = 'Active' ORDER BY jersey NULLS LAST, player_id LIMIT {combien}"
    )
    assert len(rows) >= combien, f"{combien} joueurs attendus dans {team_id}, trouvé {len(rows)}"
    return rows


def _spp_affiches(page: Page) -> list[int]:
    """Les montants de la carte « Performances », dans l'ordre où elle les rend."""
    return [
        int(txt.replace("+", "").replace("SPP", "").strip())
        for txt in page.locator(".ms-perf-spp").all_inner_texts()
    ]


@pytest.fixture(scope="module")
def recap_spp_ctx(browser, space_id):
    """Un match aux gestes **asymétriques**, et un agresseur qui ne marque rien.

    Le buteur enchaîne deux essais et trois sorties. Le compte est contraint de
    trois côtés à la fois : un essai et une sortie donneraient le même total
    sous les deux barèmes, qui sont une permutation l'un de l'autre ; un essai
    et deux sorties tomberaient sur 10 chez les Brutes — soit exactement le
    forfait du stub qu'on remplace. Deux et trois échappent aux deux pièges, et
    les assertions de chaque test le revérifient plutôt que de s'y fier.
    """
    full = build_full_competition(browser, space_id, num_teams=2, num_rounds=2)
    home, away = full["team_ids"][0], full["team_ids"][1]

    buteur, agresseur = _joueurs(home, 2)
    adverse = _joueurs(away, 1)[0]

    mr_id = create_draft(space_id, full, full["round_ids"][0], home, away)
    ensure_pre_match(space_id, mr_id, full, full["round_ids"][0], home, away)
    ensure_inducements(space_id, mr_id)

    for turn, action in enumerate(["TOUCHDOWN", "TOUCHDOWN", "SORTIE", "SORTIE", "SORTIE"], 1):
        record_action_api(space_id, mr_id, "home", buteur, turn=turn, action_type=action)
    record_action_api(space_id, mr_id, "home", agresseur, turn=6, action_type="AGRESSION")
    record_action_api(space_id, mr_id, "away", adverse, turn=7, action_type="MVP")

    # Le rapport reste en ReadyToPublish : le récapitulatif d'avant publication
    # est justement celui que le calcul sert, et le seul qui l'exerce.
    post_step5(space_id, mr_id)

    return {
        "mr_id": mr_id,
        "bareme_home": _bareme(home),
        "bareme_away": _bareme(away),
    }


def _ouvrir_le_recap(page: Page, space_id: str, mr_id: str) -> None:
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap", wait_until="load")
    expect(page.locator(".ms-card-header-title", has_text="Performances")).to_be_visible()


# ── 1 · Le total du buteur suit le barème de son roster ───────────────────────


def test_01_le_buteur_affiche_la_somme_de_ses_actions(page: Page, space_id, recap_spp_ctx):
    """Deux essais et trois sorties — pas un forfait, et pas non plus le
    décompte des actions.
    """
    ctx = recap_spp_ctx
    b = ctx["bareme_home"]
    total = lambda bareme: 2 * bareme["td"] + 3 * bareme["cas"]
    attendu = total(b)

    assert attendu != STUB, "le total doit se distinguer du forfait de l'ancien stub"
    assert attendu != 5, "et du simple nombre d'actions"
    autre = BAREMES["normal"] if b is BAREMES["brawlin_brutes"] else BAREMES["brawlin_brutes"]
    assert attendu != total(autre), "et de ce que l'autre barème donnerait"

    _ouvrir_le_recap(page, space_id, ctx["mr_id"])
    assert attendu in _spp_affiches(page), (
        f"{attendu} SPP attendus pour le buteur, affichés : {_spp_affiches(page)}"
    )


# ── 2 · L'agresseur ne gagne rien, et disparaît de la carte ───────────────────


def test_02_l_agresseur_n_a_pas_de_ligne(page: Page, space_id, recap_spp_ctx):
    """Une agression ne rapporte aucun SPP. Le stub lui accordait pourtant ses
    10 forfaitaires : c'est le cas où l'affichage était le plus faux.

    Une carte « Performances » n'a pas à lister qui n'a rien marqué — la ligne
    n'est donc pas affichée à zéro, elle n'existe pas.
    """
    ctx = recap_spp_ctx
    _ouvrir_le_recap(page, space_id, ctx["mr_id"])

    montants = _spp_affiches(page)
    assert len(montants) == 2, f"le buteur et le MVP adverse, pas l'agresseur : {montants}"
    assert 0 not in montants, "aucune ligne à zéro"
    assert STUB not in montants, "plus aucun forfait"


# ── 3 · Chaque camp est compté avec son propre barème ─────────────────────────


def test_03_le_mvp_adverse_suit_le_bareme_de_son_camp(page: Page, space_id, recap_spp_ctx):
    """Les deux camps n'ont pas forcément le même barème : le calcul en résout
    un par roster, et non un seul pour le match.
    """
    ctx = recap_spp_ctx
    attendu = ctx["bareme_away"]["mvp"]
    assert attendu != STUB

    _ouvrir_le_recap(page, space_id, ctx["mr_id"])
    assert attendu in _spp_affiches(page), (
        f"{attendu} SPP attendus pour le MVP adverse, affichés : {_spp_affiches(page)}"
    )

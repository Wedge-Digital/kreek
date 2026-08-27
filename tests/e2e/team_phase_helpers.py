"""Franchir les phases d'équipe que le scénario en cours ne porte pas.

**La phase des erreurs coûteuses s'intercale entre les renvois et « prête à
jouer »** (épic E13) dès que la trésorerie dépasse 100 kPo — c'est-à-dire pour
toute équipe de test, le gain de match par défaut de `play_match` valant 50 000
kPo. Les scénarios qui traversent le cycle de vie sans porter sur elle doivent
donc la franchir, sans quoi ils attendent en vain un `ReadyToPlay` qui ne vient
plus.

Le jet est **tiré par le serveur** : ce module ne promet donc rien sur la
trésorerie qu'il laisse. Une catastrophe ne laisse que la somme de deux dés en
dizaines de kPo, soit 20 à 120. Un scénario qui dépense après avoir appelé
`traverser_erreurs_couteuses` doit reprendre un revenu entre-temps — c'est le
cas dès qu'un match est joué — ou ne rien dépenser du tout.
"""

import time

import requests

from competition_lifecycle import BASE_URL
from db_helpers import query_db

SEUIL_KPO = 100


def phase(team_id: str) -> str | None:
    rows = query_db(f"SELECT game_phase FROM team_proj WHERE team_id = '{team_id}'")
    return rows[0] if rows else None


def attendre_une_phase(team_id: str, attendues: set[str], timeout_s: int = 20) -> str:
    """Attend l'**une** des phases données, et rend celle qui est atteinte.

    Attendre un ensemble et non une valeur est ce qui rend l'appelant robuste au
    seuil : une équipe pauvre saute la phase, une équipe riche la traverse, et le
    scénario n'a pas à savoir de quel côté il est tombé.
    """
    deadline = time.time() + timeout_s
    vue = None
    while time.time() < deadline:
        vue = phase(team_id)
        if vue in attendues:
            return vue
        time.sleep(0.2)
    raise AssertionError(
        f"équipe {team_id} en phase « {vue} » : aucune de {sorted(attendues)} après {timeout_s}s"
    )


def traverser_erreurs_couteuses(space_id: str, team_id: str) -> None:
    """Lance le dé si l'équipe le doit, et rend la main en « prête à jouer ».

    Sans effet si l'équipe est déjà passée : sous le seuil, la validation des
    renvois mène directement à `ReadyToPlay`.
    """
    if attendre_une_phase(team_id, {"CostlyMistakes", "ReadyToPlay"}) == "ReadyToPlay":
        return

    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{team_id}/costly-mistakes/roll",
        headers={"HX-Request": "true"},
    )
    assert resp.status_code == 200, (
        f"jet des erreurs coûteuses : {resp.status_code} — {resp.text[:200]}"
    )
    attendre_une_phase(team_id, {"ReadyToPlay"})

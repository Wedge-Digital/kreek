"""La garde des routes de saisie d'un compte rendu (carte 622).

Écrire un compte rendu, et ouvrir les écrans qui préparent l'écriture, est
réservé aux admins et aux coachs des deux équipes du match. Un autre membre
de l'espace reçoit 403.

Le membre simple de la suite (`X-Bypass-Auth-Profile: simple`, « E2E Coach
01 ») est aussi le coach de la **deuxième** équipe que monte
`build_full_competition` — les coachs y sont pris dans l'ordre alphabétique,
après DevCoach. Ce qui donne les deux côtés de la règle sans autre décor :
refusé sur un match qui n'est pas le sien, admis sur le sien.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from match_report_helpers import create_draft, ensure_pre_match

HX = {"HX-Request": "true"}
MEMBRE_SIMPLE = {**HX, "X-Bypass-Auth-Profile": "simple"}


@pytest.fixture(scope="module")
def rapports(browser, space_id):
    """Deux comptes rendus en avant-match : l'un entre les équipes 3 et 4,
    étranger au membre simple ; l'autre où joue son équipe, la 2."""
    ctx = build_full_competition(browser, space_id, num_teams=4, num_rounds=2)
    t, journees = ctx["team_ids"], ctx["round_ids"]
    etranger = create_draft(space_id, ctx, journees[0], t[2], t[3])
    ensure_pre_match(space_id, etranger, ctx, journees[0], t[2], t[3])
    sien = create_draft(space_id, ctx, journees[1], t[0], t[1])
    ensure_pre_match(space_id, sien, ctx, journees[1], t[0], t[1])
    return {"space_id": space_id, "etranger": etranger, "sien": sien}


def _url(ctx: dict, rapport: str, suffixe: str) -> str:
    return f"{BASE_URL}/app/{ctx['space_id']}/match-report/{ctx[rapport]}/{suffixe}"


def test_un_tiers_ne_voit_pas_l_ecran_de_saisie(rapports):
    reponse = requests.get(
        _url(rapports, "etranger", "step2"), headers=MEMBRE_SIMPLE,
        allow_redirects=False, timeout=15,
    )
    assert reponse.status_code == 403, reponse.status_code


def test_un_tiers_ne_peut_pas_saisir_d_action(rapports):
    reponse = requests.post(
        _url(rapports, "etranger", "step3/actions"), headers=MEMBRE_SIMPLE,
        data={}, allow_redirects=False, timeout=15,
    )
    assert reponse.status_code == 403, reponse.status_code


def test_le_coach_d_une_equipe_du_match_y_entre(rapports):
    reponse = requests.get(
        _url(rapports, "sien", "step2"), headers=MEMBRE_SIMPLE,
        allow_redirects=False, timeout=15,
    )
    assert reponse.status_code == 200, reponse.status_code


def test_l_admin_y_entre(rapports):
    """DevCoach, admin de l'espace — l'identité de toute la suite."""
    reponse = requests.get(
        _url(rapports, "etranger", "step2"), headers=HX, allow_redirects=False, timeout=15,
    )
    assert reponse.status_code == 200, reponse.status_code

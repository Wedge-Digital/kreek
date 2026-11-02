"""Les gardes d'un compte rendu (cartes 622, 634 et 636).

Lire un compte rendu en cours est réservé aux admins et aux coachs des deux
équipes du match ; un autre membre de l'espace reçoit 403 (carte 622).

Écrire — et ouvrir les écrans qui préparent l'écriture — est réservé, en « Je
saisis tout », au **démarreur** et aux admins (carte 634). L'autre coach suit :
403 sur une écriture, redirection 303 vers le match en direct sur une page
complète (carte 636), où il lit le score et le journal sans aucun bouton
d'écriture, journal sans ✕. Un démarreur qui ne coache aucune des deux équipes
— un admin qui a démarré pour eux — ne compte pas : les deux coachs écrivent.

Le membre simple de la suite (`X-Bypass-Auth-Profile: simple`, « E2E Coach
01 ») est le coach de la **deuxième** équipe que monte
`build_full_competition` — les coachs y sont pris dans l'ordre alphabétique :
DevCoach, admin de l'espace et identité de toute la suite, tient la première.
Les comptes rendus sont tous démarrés par DevCoach, ce qui donne les trois
côtés de la règle sans autre décor :

- `etranger` (équipes 3 et 4) : le membre simple n'y est rien ;
- `adverse` (équipes 1 et 2) : DevCoach a démarré et coache l'équipe 1 — le
  membre simple, coach de l'autre, suit ;
- `sien` (équipes 2 et 3) : DevCoach a démarré sans coacher aucune des deux
  équipes — le membre simple y écrit.

Prérequis : `make e2e-server` (port 3211, base kreek_e2e), `make init_e2e_db`.
"""

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    first_player_id,
    post_step5,
    record_action_api,
)

HX = {"HX-Request": "true"}
PROFIL_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}
MEMBRE_SIMPLE = {**HX, **PROFIL_SIMPLE}


def _demarre(space_id, ctx, journee, home, away) -> str:
    mr_id = create_draft(space_id, ctx, journee, home, away)
    ensure_pre_match(space_id, mr_id, ctx, journee, home, away)
    return mr_id


def _pret_a_publier(space_id, mr_id) -> None:
    """Une action saisie par DevCoach, puis l'étape 5 : le journal a une ligne
    à supprimer, et le récapitulatif un bouton de publication."""
    ensure_inducements(space_id, mr_id)
    joueur = first_player_id(mr_id, "home")
    record_action_api(space_id, mr_id, "home", joueur, turn=1, action_type="TOUCHDOWN")
    post_step5(space_id, mr_id)


@pytest.fixture(scope="module")
def rapports(browser, space_id):
    ctx = build_full_competition(browser, space_id, num_teams=4, num_rounds=3)
    t, journees = ctx["team_ids"], ctx["round_ids"]
    etranger = _demarre(space_id, ctx, journees[0], t[2], t[3])
    adverse = _demarre(space_id, ctx, journees[1], t[0], t[1])
    _pret_a_publier(space_id, adverse)
    sien = _demarre(space_id, ctx, journees[2], t[1], t[2])
    return {"space_id": space_id, "etranger": etranger, "adverse": adverse, "sien": sien}


def _url(ctx: dict, rapport: str, suffixe: str) -> str:
    return f"{BASE_URL}/app/{ctx['space_id']}/match-report/{ctx[rapport]}/{suffixe}"


# ── Le tiers (carte 622) ──────────────────────────────────────────────────────


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


def test_un_tiers_ne_lit_pas_le_journal(rapports):
    reponse = requests.get(
        _url(rapports, "etranger", "action-log"), headers=MEMBRE_SIMPLE,
        allow_redirects=False, timeout=15,
    )
    assert reponse.status_code == 403, reponse.status_code


# ── Le démarreur et les admins écrivent (carte 634) ───────────────────────────


def test_le_coach_y_entre_quand_un_admin_non_coach_a_demarre(rapports):
    """Migré par la carte 634 : le membre simple n'a pas démarré ce match.
    Il y entre parce que DevCoach, qui l'a démarré, ne coache aucune des deux
    équipes (décision du 2026-11-01)."""
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


# ── L'autre coach suit (carte 634) ────────────────────────────────────────────


def test_l_autre_coach_ne_peut_pas_saisir_d_action(rapports):
    reponse = requests.post(
        _url(rapports, "adverse", "step3/actions"), headers=MEMBRE_SIMPLE,
        data={}, allow_redirects=False, timeout=15,
    )
    assert reponse.status_code == 403, reponse.status_code


def test_l_autre_coach_ne_peut_pas_publier(rapports):
    reponse = requests.post(
        _url(rapports, "adverse", "recap/publish"), headers=MEMBRE_SIMPLE,
        allow_redirects=False, timeout=15,
    )
    assert reponse.status_code == 403, reponse.status_code


def test_l_autre_coach_est_mene_au_match_en_direct(rapports):
    """Une page complète, et non un 403 nu : la redirection vers le match en
    direct, où il suit (carte 636). Elle remplace la page « Ce match est saisi
    par … » de la carte 634."""
    reponse = requests.get(
        _url(rapports, "adverse", "step2"), headers=PROFIL_SIMPLE,
        allow_redirects=False, timeout=15,
    )
    assert reponse.status_code == 303, reponse.status_code
    assert reponse.headers["Location"].endswith(f"/{rapports['adverse']}/live")


def test_l_autre_coach_suit_le_match_sans_bouton_d_ecriture(rapports):
    """R5 : le même écran, sans « Nouvelle action » ni « Fin du match ». Lu
    par l'admin, il les porte : sans ce témoin, une page vide passerait."""
    autre = requests.get(
        _url(rapports, "adverse", "live"), headers=PROFIL_SIMPLE,
        allow_redirects=False, timeout=15,
    )
    assert autre.status_code == 200, autre.status_code
    assert "DevCoach saisit tout le match" in autre.text
    assert "Nouvelle action" not in autre.text
    assert "live-end-match" not in autre.text
    assert "/step3" not in autre.text

    admin = requests.get(
        _url(rapports, "adverse", "live"), allow_redirects=False, timeout=15,
    )
    assert admin.status_code == 200, admin.status_code
    assert "Nouvelle action" in admin.text
    assert "saisit tout le match" not in admin.text


def test_un_tiers_ne_suit_pas_le_match(rapports):
    reponse = requests.get(
        _url(rapports, "etranger", "live"), headers=PROFIL_SIMPLE,
        allow_redirects=False, timeout=15,
    )
    assert reponse.status_code == 403, reponse.status_code


def test_l_autre_coach_voit_le_recapitulatif_sans_bouton_de_publication(rapports):
    reponse = requests.get(
        _url(rapports, "adverse", "recap"), headers=PROFIL_SIMPLE,
        allow_redirects=False, timeout=15,
    )
    assert reponse.status_code == 200, reponse.status_code
    assert "recap/publish" not in reponse.text, "l'autre coach ne publie pas"


def test_l_autre_coach_voit_le_journal_sans_croix(rapports):
    """R6 : le journal montre les deux camps à l'autre coach, ✕ en moins. Le
    même journal, lu par l'admin, porte la croix : sans ce témoin, un journal
    vide passerait le test."""
    autre = requests.get(
        _url(rapports, "adverse", "action-log"), headers=MEMBRE_SIMPLE,
        allow_redirects=False, timeout=15,
    )
    assert autre.status_code == 200, autre.status_code
    assert "mr-log-entry" in autre.text, "l'action saisie par DevCoach est visible"
    assert "mr-log-delete" not in autre.text, "pas de ✕ pour qui ne peut pas écrire"

    admin = requests.get(
        _url(rapports, "adverse", "action-log"), headers=HX,
        allow_redirects=False, timeout=15,
    )
    assert "mr-log-delete" in admin.text, "le ✕ reste à l'admin"

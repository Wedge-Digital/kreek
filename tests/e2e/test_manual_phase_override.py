"""Tests E2E — ouvrir une phase manuellement (carte 578).

Scénarios :
- un commissaire ouvre chacune des trois phases depuis le bandeau d'une équipe
  prête à jouer ; l'équipe passe dans la phase, avec son bandeau habituel, et
  l'ouverture est enregistrée avec son motif. La validation habituelle de la
  phase la ramène à « prête à jouer » — sans enchaîner ;
- un membre simple ne voit pas le bouton, et sa requête forgée est refusée.

Les sorties détaillées et les refus serveur restants sont à la carte 579.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from team_phase_helpers import attendre_une_phase

ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}

# Ce que chaque phase affiche une fois ouverte, et la route qui la ferme.
PHASES = [
    ("player_improvement", "Dépense des SPP", "Ouvrir la dépense des SPP",
     "PlayerImprovement", "Phase d'amélioration des joueurs.",
     "validate-improvement-phase", "ManualImprovementPhaseOpened"),
    ("recruitment", "Recrutement", "Ouvrir le recrutement",
     "Recruitment", "Phase de recrutement.",
     "validate-recruitment-phase", "ManualRecruitmentPhaseOpened"),
    ("dismissals", "Renvois", "Ouvrir les renvois",
     "Dismissals", "Phase de renvois.",
     "validate-dismissals-phase", "ManualDismissalsPhaseOpened"),
]


@pytest.fixture(scope="module")
def ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    team_id = full["team_ids"][0]
    attendre_une_phase(team_id, {"ReadyToPlay"})
    return {"space_id": space_id, "team_id": team_id}


def _url_equipe(ctx: dict) -> str:
    return f"{BASE_URL}/app/{ctx['space_id']}/teams/{ctx['team_id']}"


def _fermer(ctx: dict, route: str) -> None:
    resp = requests.post(
        f"{_url_equipe(ctx)}/{route}", headers={"HX-Request": "true"}, timeout=20
    )
    assert resp.status_code == 200, f"{route} : {resp.status_code} — {resp.text[:200]}"


def _motif_enregistre(team_id: str, event_type: str) -> str:
    rows = query_db(
        "SELECT coalesce(payload->>'reason', '') FROM team_event_store "
        f"WHERE team_id = '{team_id}' AND event_type = '{event_type}' ORDER BY version DESC LIMIT 1"
    )
    assert rows, f"aucun événement {event_type} pour {team_id}"
    return rows[0]


def _ouvrir_depuis_le_bandeau(page: Page, ctx: dict, nom: str, bouton: str, motif: str) -> None:
    page.goto(_url_equipe(ctx))
    expect(page.locator(".state-banner")).to_contain_text("Équipe prête à jouer.")
    page.locator(".phase-override-btn").click()
    panneau = page.locator(".phase-override")
    expect(panneau).to_be_visible()
    soumettre = panneau.locator(".phase-override-submit")
    expect(soumettre).to_be_disabled()

    panneau.locator(".phase-choice", has_text=nom).click()
    expect(soumettre).to_be_enabled()
    expect(soumettre).to_have_text(bouton)
    panneau.locator("textarea[name='reason']").fill(motif)
    soumettre.click()


def test_un_commissaire_ouvre_chacune_des_trois_phases(page: Page, ctx):
    for valeur, nom, bouton, phase, titre, sortie, evenement in PHASES:
        motif = f"Correction e2e — {valeur}"
        _ouvrir_depuis_le_bandeau(page, ctx, nom, bouton, motif)

        # `HX-Refresh` recharge la fiche : le bandeau est celui de la phase.
        expect(page.locator(".state-banner")).to_contain_text(titre, timeout=10_000)
        assert attendre_une_phase(ctx["team_id"], {phase}) == phase
        assert _motif_enregistre(ctx["team_id"], evenement) == motif

        # La validation habituelle ferme la phase et ramène à « prête à jouer »,
        # sans passer à la suivante : c'est la sortie manuelle.
        _fermer(ctx, sortie)
        assert attendre_une_phase(ctx["team_id"], {"ReadyToPlay"}) == "ReadyToPlay", valeur


def test_un_membre_simple_ne_voit_pas_le_bouton_et_sa_requete_est_refusee(ctx):
    """En HTTP, et non au navigateur : le middleware n'échange l'identité que
    sur une session vide — la raison donnée par le test de la trésorerie."""
    corps = requests.get(_url_equipe(ctx), headers=ENTETE_MEMBRE_SIMPLE, timeout=20).text
    assert "Équipe prête à jouer." in corps, "la fiche reste lisible par tous"
    assert "phase-override-btn" not in corps, "un membre simple ne doit pas voir le bouton"

    resp = requests.post(
        f"{_url_equipe(ctx)}/phases/override",
        data={"phase": "recruitment", "reason": ""},
        headers={**ENTETE_MEMBRE_SIMPLE, "HX-Request": "true"},
        timeout=20,
    )
    assert resp.status_code == 403, f"requête forgée : {resp.status_code}"
    assert query_db(
        f"SELECT game_phase FROM team_proj WHERE team_id = '{ctx['team_id']}'"
    ) == ["ReadyToPlay"], "le refus n'a rien ouvert"

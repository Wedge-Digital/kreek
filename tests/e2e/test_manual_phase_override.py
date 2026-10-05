"""Tests E2E — ouvrir une phase manuellement (carte 578).

Scénarios :
- un commissaire ouvre chacune des trois phases depuis le bandeau d'une équipe
  prête à jouer ; l'équipe passe dans la phase, avec son bandeau habituel, et
  l'ouverture est enregistrée avec son motif. La validation habituelle de la
  phase la ramène à « prête à jouer » — sans enchaîner ;
- un membre simple ne voit pas le bouton, et sa requête forgée est refusée.

Sorties et refus (carte 579) :
- pendant une phase manuelle, le dernier rapport de match n'est pas
  corrigeable ;
- le coach dépense ses SPP pendant une phase de dépense ouverte à la main ; la
  validation ramène à « prête à jouer » sans passer par le recrutement, et la
  valeur d'équipe intègre l'achat ;
- une sortie de renvois manuelle au-dessus du seuil ramène à « prête à jouer »,
  sans erreurs coûteuses ;
- une équipe qui n'est plus prête à jouer reçoit le message au pied du panneau.

Une équipe distincte par scénario qui la fait changer de phase : une équipe
laissée dans une phase ne peut pas servir au suivant.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import attendre_que, query_db
from htmx_helpers import cliquer_quand_cable
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    first_player_id,
    post_step5,
    publish,
    record_action_api,
)
from team_phase_helpers import SEUIL_KPO, attendre_une_phase, traverser_erreurs_couteuses

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
def competition(browser, space_id):
    """Six équipes, une journée. Chaque scénario prend la sienne :
    0 · ouverture des trois phases ; 1 · renvois manuels ; 2 contre 3 · le
    match joué ; 4 · le refus d'une équipe qui n'est plus prête à jouer."""
    full = build_full_competition(browser, space_id, num_teams=6, num_rounds=1)
    for team_id in full["team_ids"]:
        attendre_une_phase(team_id, {"ReadyToPlay"})
    return full


@pytest.fixture(scope="module")
def ctx(competition, space_id):
    return {"space_id": space_id, "team_id": competition["team_ids"][0]}


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


# ── Sorties et refus (carte 579) ──────────────────────────────────────────────


def _ouvrir_par_http(ctx: dict, phase: str) -> None:
    """L'ouverture elle-même est éprouvée à l'écran plus haut ; ici, elle n'est
    que le point de départ du scénario."""
    resp = requests.post(
        f"{_url_equipe(ctx)}/phases/override",
        data={"phase": phase, "reason": ""},
        headers={"HX-Request": "true"},
        timeout=20,
    )
    assert resp.status_code == 200, f"ouverture : {resp.status_code} — {resp.text[:200]}"
    assert resp.headers.get("hx-refresh") == "true", "l'ouverture doit avoir réussi"


def _evenements_depuis(team_id: str, event_type: str) -> list[str]:
    """Les événements écrits après le dernier `event_type`, dans l'ordre."""
    return query_db(
        "SELECT event_type FROM team_event_store "
        f"WHERE team_id = '{team_id}' AND version > ("
        f"  SELECT max(version) FROM team_event_store "
        f"  WHERE team_id = '{team_id}' AND event_type = '{event_type}') "
        "ORDER BY version"
    )


def _valeur_equipe(team_id: str) -> int:
    return int(query_db(f"SELECT team_value FROM team_proj WHERE team_id = '{team_id}'")[0])


def _tresorerie(team_id: str) -> int:
    """Le solde vient du grand livre : `team_proj` ne porte pas la trésorerie."""
    rows = query_db(
        "SELECT balance_after_kpo FROM teams__treasury_ledger "
        f"WHERE team_id = '{team_id}' ORDER BY id DESC LIMIT 1"
    )
    assert rows, f"aucun mouvement de trésorerie pour {team_id}"
    return int(rows[0])


@pytest.fixture(scope="module")
def joue_ctx(competition, space_id):
    """Une équipe qui a joué, traversé son après-match **sans rien dépenser**, et
    à qui un commissaire rouvre la dépense des SPP.

    Le visiteur marque et prend le MVP : 7 SPP, de quoi s'offrir une compétence
    (niveau 1 : 6). C'est lui l'équipe du scénario ; le domicile reste dans son
    vrai après-match, sans rapport avec ce qu'on éprouve.
    """
    full = competition
    round_id = full["round_ids"][0]
    home, away = full["team_ids"][2], full["team_ids"][3]
    ctx_match = {**full, "teams": full["team_ids"]}

    mr_id = create_draft(space_id, ctx_match, round_id, home, away)
    ensure_pre_match(space_id, mr_id, ctx_match, round_id, home, away)
    ensure_inducements(space_id, mr_id)
    buteur = first_player_id(mr_id, "away")
    record_action_api(space_id, mr_id, "away", buteur, turn=1, action_type="TOUCHDOWN")
    record_action_api(space_id, mr_id, "away", buteur, turn=2, action_type="MVP")
    post_step5(space_id, mr_id)
    publish(space_id, mr_id)

    ctx = {"space_id": space_id, "team_id": away, "mr_id": mr_id}
    attendre_une_phase(away, {"PlayerImprovement"})
    for route, suivante in [
        ("validate-improvement-phase", {"Recruitment"}),
        ("validate-recruitment-phase", {"Dismissals"}),
        ("validate-dismissals-phase", {"CostlyMistakes", "ReadyToPlay"}),
    ]:
        _fermer(ctx, route)
        attendre_une_phase(away, suivante)
    traverser_erreurs_couteuses(space_id, away)

    def buteur_credite() -> list[str]:
        return query_db(
            f"SELECT player_id FROM players_proj WHERE team_id = '{away}' AND spp >= 6 LIMIT 1"
        )

    attendre_que(lambda: bool(buteur_credite()), quoi="un joueur à 6 SPP au moins")
    ctx["buteur"] = buteur_credite()[0]
    _ouvrir_par_http(ctx, "player_improvement")
    attendre_une_phase(away, {"PlayerImprovement"})
    return ctx


def test_pendant_une_phase_manuelle_le_dernier_rapport_n_est_pas_corrigeable(page: Page, joue_ctx):
    """Avant la carte 576, cet écran aurait proposé la correction : la phase
    affichée était bien celle de l'amélioration, et le dernier après-match
    toujours en mémoire. Il doit précéder toute dépense, sans quoi le refus
    viendrait des SPP dépensés et ne prouverait plus rien."""
    page.goto(
        f"{BASE_URL}/app/{joue_ctx['space_id']}/match-report/{joue_ctx['mr_id']}/recap",
        wait_until="load",
    )
    expect(page.locator(".ms-correct-btn")).to_be_disabled()
    raison = page.locator(".ms-correct-blocked")
    expect(raison).to_be_visible()
    assert "phase d'amélioration" in raison.inner_text()


def test_la_depense_manuelle_revient_a_pret_a_jouer_avec_l_achat(page: Page, joue_ctx):
    team_id = joue_ctx["team_id"]
    valeur_avant = _valeur_equipe(team_id)

    page.goto(
        f"{BASE_URL}/app/{joue_ctx['space_id']}/players/{joue_ctx['buteur']}/detail",
        wait_until="load",
    )
    cliquer_quand_cable(page, ".btn-toggle-spp")
    page.wait_for_selector(".skill-list-table", timeout=10_000)
    with page.expect_navigation(wait_until="load"):
        page.locator(".btn-add-skill:visible", has_text="Choisir").first.click()

    _fermer(joue_ctx, "validate-improvement-phase")
    assert attendre_une_phase(team_id, {"ReadyToPlay", "Recruitment"}) == "ReadyToPlay", (
        "une phase ouverte à la main ne doit pas enchaîner sur le recrutement"
    )
    assert "PlayerImprovementPhaseValidated" not in _evenements_depuis(
        team_id, "ManualImprovementPhaseOpened"
    )
    attendre_que(
        lambda: _valeur_equipe(team_id) > valeur_avant,
        quoi=f"une valeur d'équipe au-dessus de {valeur_avant} kPo",
    )


def test_une_sortie_de_renvois_manuelle_saute_les_erreurs_couteuses(page: Page, competition, space_id):
    ctx = {"space_id": space_id, "team_id": competition["team_ids"][1]}
    assert _tresorerie(ctx["team_id"]) >= SEUIL_KPO, (
        "le scénario suppose une caisse au-dessus du seuil — sinon il ne prouve rien"
    )
    _ouvrir_par_http(ctx, "dismissals")
    attendre_une_phase(ctx["team_id"], {"Dismissals"})

    page.goto(f"{_url_equipe(ctx)}/dismissals", wait_until="load")
    expect(page.locator(".dis-cart")).to_be_visible()
    with page.expect_navigation(wait_until="load"):
        cliquer_quand_cable(page, ".dis-cart .cta-primary")

    assert "costly-mistakes" not in page.url, "aucun écran d'erreurs coûteuses"
    assert attendre_une_phase(ctx["team_id"], {"ReadyToPlay", "CostlyMistakes"}) == "ReadyToPlay"
    assert "CostlyMistakesPhaseStarted" not in _evenements_depuis(
        ctx["team_id"], "ManualDismissalsPhaseOpened"
    )


def test_une_equipe_qui_n_est_plus_prete_recoit_le_message(page: Page, competition, space_id):
    ctx = {"space_id": space_id, "team_id": competition["team_ids"][4]}
    page.goto(_url_equipe(ctx))
    page.locator(".phase-override-btn").click()
    panneau = page.locator(".phase-override")
    panneau.locator(".phase-choice", has_text="Recrutement").click()
    panneau.locator("textarea[name='reason']").fill("Motif conservé")

    # Pendant que l'écran attend, l'équipe quitte « prête à jouer ».
    _ouvrir_par_http(ctx, "dismissals")
    attendre_une_phase(ctx["team_id"], {"Dismissals"})

    panneau.locator(".phase-override-submit").click()
    erreur = page.locator("#phase-override-error")
    expect(erreur).to_contain_text("Impossible d'ouvrir le recrutement")
    expect(erreur).to_contain_text("n'est plus prête à jouer")
    expect(panneau).to_be_visible()
    expect(panneau.locator("textarea[name='reason']")).to_have_value("Motif conservé")
    assert query_db(
        f"SELECT game_phase FROM team_proj WHERE team_id = '{ctx['team_id']}'"
    ) == ["Dismissals"], "le refus n'a rien ouvert"


# ── La valeur d'équipe compte les recrues (carte 593) ─────────────────────────


@pytest.fixture(scope="module")
def granit_ctx(browser, space_id):
    """Une équipe `DEMO_GRANIT` : sa ligne de recrutement et son prix sont
    connus d'avance (Piétaille, 50 kPo)."""
    full = build_full_competition(
        browser, space_id, num_teams=2, num_rounds=1, roster_uids=["DEMO_GRANIT"] * 2
    )
    team_id = full["team_ids"][0]
    attendre_une_phase(team_id, {"ReadyToPlay"})
    return {"space_id": space_id, "team_id": team_id}


def test_un_recrutement_manuel_compte_la_recrue_dans_la_valeur_d_equipe(granit_ctx):
    """Mesuré avant la carte 593 : 12 joueurs, mais une valeur restée à celle
    de 11. Le recalcul de `ManualPhaseClosed` précédait la création de la
    recrue par `players`, et rien ne le refaisait."""
    ctx, team_id = granit_ctx, granit_ctx["team_id"]
    avant = _valeur_equipe(team_id)
    _ouvrir_par_http(ctx, "recruitment")
    attendre_une_phase(team_id, {"Recruitment"})

    resp = requests.post(
        f"{_url_equipe(ctx)}/recruitment/players/add",
        data={"roster_line_id": "DEMO_GRANIT__PIETAILLE", "version": 0},
        headers={"HX-Request": "true"}, timeout=20,
    )
    assert resp.status_code == 200, resp.text[:200]
    _fermer(ctx, "validate-recruitment-phase")
    assert attendre_une_phase(team_id, {"ReadyToPlay"}) == "ReadyToPlay"

    attendre_que(
        lambda: _valeur_equipe(team_id) == avant + 50,
        quoi=f"une valeur d'équipe de {avant + 50} kPo, recrue comprise",
    )

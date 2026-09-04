"""Tests E2E — correction d'un rapport de match publié.

Couvre le parcours complet en navigateur et la propagation réelle des
compensations à travers les cinq BCs : match_report, competitions, ranking,
teams et players.

Ce que ces tests gardent, qu'aucun test unitaire ne peut atteindre :

- le rendu réel de la zone de correction et de ses états bloqués
- la chaîne asynchrone complète, du clic jusqu'aux quatre compensations
- la convergence d'un cycle publier / corriger / republier sur des données
  réelles, y compris l'absence de pairing dupliqué
- la survie du bandeau à une modification d'action, donc au passage transitoire
  par l'état PreMatch

Les compensations transitent par l'app event bus : toutes les vérifications
d'effet passent par une attente explicite sur l'état voulu, jamais par un délai
fixe.

Une paire d'équipes distincte par scénario, et une journée distincte. Ce n'est
pas de la précaution : une équipe qui a joué reste en phase d'amélioration — ou
en saisie de rapport après une correction — et ne peut pas enchaîner un second
match. La compétition étant construite par ce module, ses indices d'équipes ne
collisionnent avec aucun autre fichier de test.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) — cf. README.
"""

import html
import re
import time

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL
from db_helpers import query_db
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


# ── Helpers ───────────────────────────────────────────────────────────────────


def _unpublish(space_id: str, mr_id: str) -> requests.Response:
    return requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap/unpublish",
        allow_redirects=False,
    )


def _motif_de_blocage(space_id: str, mr_id: str) -> str | None:
    """Le motif que le récapitulatif affiche, ou `None` si le rapport est corrigeable.

    C'est la seule chose qui distingue un refus d'un succès. Mesuré : les deux
    rendent `HTTP 200`, `hx-refresh: true` et un corps vide — `unpublish_response`
    fait passer `Ok(())` et `Err(NotEligible(_))` par la même sortie `refresh()`.
    Le code de retour ne dit donc rien, et c'est délibéré côté produit : la page
    rechargée montre le motif recalculé. Il faut aller le lire là.
    """
    # `page` et non `html` : la variable masquerait le module du même nom, dont
    # on a besoin deux lignes plus bas pour déséchapper le motif.
    page = requests.get(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap",
        headers={"HX-Request": "true"},
        timeout=15,
    ).text
    bloc = re.search(r'ms-correct-blocked.*?</div>', page, re.S)
    if not bloc:
        return None
    spans = re.findall(r"<span[^>]*>(.*?)</span>", bloc.group(0), re.S)
    # Le premier `<span>` porte l'icône ⓘ ; le motif est le suivant.
    # Askama échappe le rendu : sans ce décodage, le message de l'échec porte
    # « d&#x27;amélioration » — lisible, mais pas par quelqu'un qui lit vite
    # une sortie de CI.
    motif = html.unescape(spans[-1]).strip() if spans else ""
    return motif or "blocage sans motif affiché"


def _attendre_ou_dire_le_motif(space_id, mr_id, condition, quoi, timeout_s=30):
    """Attend, et si rien ne vient, **va lire pourquoi** au lieu d'accuser le délai."""
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if condition():
            return
        time.sleep(0.2)
    motif = _motif_de_blocage(space_id, mr_id)
    raise AssertionError(
        f"{quoi}, après {timeout_s} s. "
        + (
            f"Le récapitulatif dit : « {motif} »"
            if motif
            else "Le récapitulatif ne montre aucun blocage — la cause est ailleurs."
        )
    )


def depublier(space_id: str, mr_id: str, quoi: str = "le rapport") -> None:
    """Dépublie **et constate** — les deux comptent autant.

    **Ce que ce helper remplace.** Les quatre sites d'appel postaient sans
    regarder la réponse, puis attendaient trente secondes que la phase change,
    et concluaient « non satisfait après 30s ». Un refus arrivé en six
    millisecondes se lisait donc comme une lenteur, et la CI de `demo` a été
    rouge trois runs sur quatre sur ce message-là.

    **Pourquoi attendre d'abord.** L'entrée d'une équipe en `PlayerImprovement`
    vient d'un app event **cross-BC**, asynchrone par construction. La phase du
    rapport, elle, vit dans une projection intra-BC écrite dans la même
    transaction que l'événement : elle dit « publié » *avant* que `teams` ait
    réagi. Dans cette fenêtre, `is_team_in_player_improvement` rend `false` — non
    parce que la phase est dépassée, mais parce qu'elle n'est pas encore
    arrivée — et la dépublication est refusée. Mesuré localement : l'app event
    gagne la course en moins de 40 ms, trois fois sur trois. En CI chargée, il
    peut perdre.

    Attendre que le rapport soit *corrigeable* plutôt qu'une phase d'équipe
    couvre les trois bloqueurs d'un coup, et sans connaître les identifiants des
    deux camps : c'est exactement la condition que le produit va évaluer.

    **Les deux attentes citent le motif**, pas seulement la seconde. Un rapport
    définitivement bloqué — un coach a validé ses améliorations — n'arrive jamais
    à la dépublication, et le message rendrait alors le même « non satisfait
    après 30 s » que celui qu'on remplace.
    """
    _attendre_ou_dire_le_motif(
        space_id, mr_id,
        lambda: _motif_de_blocage(space_id, mr_id) is None,
        f"{quoi} — le rapport n'est jamais devenu corrigeable",
    )
    _unpublish(space_id, mr_id)
    _attendre_ou_dire_le_motif(
        space_id, mr_id,
        lambda: _phase(mr_id) == "ReadyToPublish",
        f"{quoi} — la dépublication n'a rien changé",
    )


def _phase(mr_id: str) -> str | None:
    rows = query_db(f"SELECT phase FROM match_report_proj WHERE match_report_id = '{mr_id}'")
    return rows[0] if rows else None


def _ranking_line_count(mr_id: str) -> int:
    rows = query_db(f"SELECT count(*) FROM ranking_lines WHERE match_report_id = '{mr_id}'")
    return int(rows[0]) if rows else 0


def _calendar_status(mr_id: str) -> str | None:
    rows = query_db(
        "SELECT match_status FROM competition_match_display_proj "
        f"WHERE match_report_url LIKE '%{mr_id}%'"
    )
    return rows[0] if rows else None


def _team_of(mr_id: str, side: str) -> str:
    return query_db(f"SELECT {side}_team_id FROM match_report_proj WHERE match_report_id = '{mr_id}'")[0]


def _team_phase(team_id: str) -> str | None:
    rows = query_db(f"SELECT game_phase FROM team_proj WHERE team_id = '{team_id}'")
    return rows[0] if rows else None


def _team_treasury(team_id: str) -> int:
    """La trésorerie ne vit que dans l'event store — la projection ne la porte pas."""
    rows = query_db(
        "SELECT payload FROM team_event_store "
        f"WHERE team_id = '{team_id}' ORDER BY version ASC"
    )
    treasury = 0
    for payload in rows:
        if '"TeamCreated"' in payload:
            treasury = _extract_int(payload, '"treasury":')
        elif '"PostMatchSequenceStarted"' in payload:
            treasury += _extract_int(payload, '"treasury_income":')
        elif '"PostMatchSequenceReverted"' in payload:
            treasury -= _extract_int(payload, '"treasury_refund":')
    return treasury


def _extract_int(payload: str, key: str) -> int:
    start = payload.index(key) + len(key)
    digits = ""
    for ch in payload[start:]:
        if ch.isdigit():
            digits += ch
        elif digits:
            break
    return int(digits or 0)


def _wait(condition, description: str, timeout_s: int = 30):
    deadline = time.time() + timeout_s
    last = None
    while time.time() < deadline:
        last = condition()
        if last:
            return last
        time.sleep(0.3)
    raise AssertionError(f"{description} — non satisfait après {timeout_s}s (dernier : {last})")


def _play_and_publish(
    space_id, ctx, round_id, home_idx, away_idx, *, home_td=1, away_td=0, away_mvp=False
):
    home_team_id = ctx["teams"][home_idx]
    away_team_id = ctx["teams"][away_idx]
    mr_id = create_draft(space_id, ctx, round_id, home_team_id, away_team_id)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home_team_id, away_team_id)
    ensure_inducements(space_id, mr_id)

    home_player = first_player_id(mr_id, "home")
    for turn in range(home_td):
        record_action_api(space_id, mr_id, "home", home_player, turn=turn + 1, action_type="TOUCHDOWN")
    if away_td or away_mvp:
        away_player = first_player_id(mr_id, "away")
        for turn in range(away_td):
            record_action_api(space_id, mr_id, "away", away_player, turn=turn + 1, action_type="TOUCHDOWN")
        if away_mvp:
            # Essai + MVP = 7 SPP, le minimum pour s'offrir une compétence
            # primaire choisie (coût niveau 1 = 6).
            record_action_api(space_id, mr_id, "away", away_player, turn=away_td + 1, action_type="MVP")

    post_step5(space_id, mr_id)
    publish(space_id, mr_id)
    _wait(lambda: _phase(mr_id) == "Published", "le rapport doit être publié")
    # **Le rapport publié ne suffit pas.** L'entrée d'une équipe en
    # `PlayerImprovement` est le fait d'un listener cross-BC, dans une autre
    # tâche : la publication est enregistrée avant que les équipes aient bougé.
    #
    # Or la correction n'est permise que si les deux camps y sont encore
    # (`CorrectionBlocker::PhaseAdvanced`), et le domaine échoue **fermé** —
    # à raison. Interroger le récapitulatif trop tôt rend donc un bouton
    # désactivé, et le test accuse la règle pour un défaut d'attente.
    #
    # **Les deux camps**, pas seulement le domicile : le blocage vient de l'un
    # ou de l'autre, et n'attendre qu'un seul laisserait la moitié de la course
    # ouverte.
    for team_id in (home_team_id, away_team_id):
        _wait(
            lambda t=team_id: _team_phase(t) == "PlayerImprovement",
            f"l'équipe {team_id} doit passer en amélioration",
        )
    return mr_id


# ── Fixtures ──────────────────────────────────────────────────────────────────


@pytest.fixture(scope="module")
def correction_ctx(browser, space_id):
    from competition_lifecycle import build_full_competition

    full = build_full_competition(browser, space_id, num_teams=12)
    return {
        "competition_id": full["competition_id"],
        "season_id":      full["season_id"],
        "round_ids":      full["round_ids"],
        "teams":          full["team_ids"],
    }


# ── TC-01 — Parcours nominal ──────────────────────────────────────────────────


def test_correction_ramene_le_rapport_en_etat_corrigeable(page: Page, space_id, correction_ctx):
    mr_id = _play_and_publish(space_id, correction_ctx, correction_ctx["round_ids"][0], 0, 1)

    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap", wait_until="load")
    bouton = page.locator(".ms-correct-btn")
    expect(bouton).to_be_visible()
    expect(bouton).to_be_enabled()

    depublier(space_id, mr_id)

    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap", wait_until="load")
    expect(page.locator(".ms-unpublished-banner")).to_be_visible()
    expect(page.locator(".ms-btn-primary", has_text="Publier")).to_be_visible()


# ── TC-02 — Bloqué par les SPP de l'adversaire ────────────────────────────────


def test_bouton_desactive_quand_une_equipe_a_depense_ses_spp(page: Page, space_id, correction_ctx):
    # L'équipe adverse doit marquer : sans essai, aucun de ses joueurs n'a de SPP
    # à dépenser, et le scénario ne pourrait pas se jouer.
    mr_id = _play_and_publish(
        space_id, correction_ctx, correction_ctx["round_ids"][1], 2, 3, away_td=1, away_mvp=True
    )
    away_team = _team_of(mr_id, "away")

    # Le buteur adverse a des SPP : on lui fait acheter une compétence.
    scorer = _wait(
        lambda: (query_db(
            f"SELECT player_id FROM players_proj WHERE team_id = '{away_team}' AND spp > 0 LIMIT 1"
        ) or [None])[0],
        "un joueur adverse doit avoir des SPP",
    )
    page.goto(f"{BASE_URL}/app/{space_id}/players/{scorer}/detail", wait_until="load")
    # Le panneau droit affiche le journal par défaut (cf. test_player_spp_spending).
    # `cliquer_quand_cable` et non `click` : c'est **ce bouton** que le
    # `CLAUDE.md` donne en exemple du piège de la fenêtre de câblage — peint et
    # cliquable quelques dizaines de millisecondes avant qu'htmx ne l'ait câblé,
    # un clic s'y perd sans requête ni erreur.
    cliquer_quand_cable(page, ".btn-toggle-spp")
    expect(page.locator(".tabs")).to_be_visible(timeout=10000)
    page.wait_for_selector(".skill-list-table", timeout=10000)
    with page.expect_navigation(wait_until="load"):
        page.locator(".btn-add-skill:visible", has_text="Choisir").first.click()

    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap", wait_until="load")
    expect(page.locator(".ms-correct-btn")).to_be_disabled()
    raison = page.locator(".ms-correct-blocked")
    expect(raison).to_be_visible()
    assert "SPP" in raison.inner_text()


# ── TC-03 — Bloqué par une phase avancée ──────────────────────────────────────


def test_bouton_desactive_quand_une_equipe_a_valide_sa_phase(page: Page, space_id, correction_ctx):
    mr_id = _play_and_publish(space_id, correction_ctx, correction_ctx["round_ids"][2], 4, 5)
    home_team = _team_of(mr_id, "home")
    _wait(lambda: _team_phase(home_team) == "PlayerImprovement", "l'équipe doit passer en amélioration")

    requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{home_team}/validate-improvement-phase",
        allow_redirects=False,
    )
    _wait(lambda: _team_phase(home_team) != "PlayerImprovement", "la phase doit avoir avancé")

    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap", wait_until="load")
    expect(page.locator(".ms-correct-btn")).to_be_disabled()
    raison = page.locator(".ms-correct-blocked").inner_text()
    assert "phase d'amélioration" in raison


# ── TC-04/05/06 — Propagation des quatre compensations ────────────────────────


def test_les_quatre_compensations_s_appliquent_puis_se_rejouent(space_id, correction_ctx):
    round_id = correction_ctx["round_ids"][3]
    mr_id = _play_and_publish(space_id, correction_ctx, round_id, 6, 7, home_td=2)
    home_team = _team_of(mr_id, "home")

    _wait(lambda: _ranking_line_count(mr_id) == 2, "le classement doit compter le match")
    _wait(lambda: _calendar_status(mr_id) == "completed", "le calendrier doit afficher terminé")
    tresorerie_apres_publication = _team_treasury(home_team)

    depublier(space_id, mr_id, "match_report compensé")

    _wait(lambda: _ranking_line_count(mr_id) == 0, "ranking compensé")
    _wait(lambda: _calendar_status(mr_id) == "in_progress", "competitions compensé")
    _wait(lambda: _team_phase(home_team) == "MatchReporting", "teams compensé")
    assert _team_treasury(home_team) < tresorerie_apres_publication, "le gain doit être retiré"

    # Re-publication : le rejeu doit reproduire l'état, sans résidu ni doublon.
    publish(space_id, mr_id)
    _wait(lambda: _ranking_line_count(mr_id) == 2, "le classement doit être rejoué")
    assert _ranking_line_count(mr_id) == 2, "deux lignes, pas quatre"
    _wait(lambda: _calendar_status(mr_id) == "completed", "le calendrier doit repasser à terminé")
    assert _team_treasury(home_team) == tresorerie_apres_publication, "la trésorerie doit converger"


# ── TC-07 — Corrections successives ───────────────────────────────────────────


def test_deux_corrections_successives_aboutissent(space_id, correction_ctx):
    mr_id = _play_and_publish(space_id, correction_ctx, correction_ctx["round_ids"][4], 8, 9)

    for tour in (1, 2):
        depublier(space_id, mr_id, f"correction {tour}")
        publish(space_id, mr_id)
        _wait(lambda: _phase(mr_id) == "Published", f"re-publication {tour}")

    assert _ranking_line_count(mr_id) == 2, "aucune ligne de classement en double"


# ── TC-08 — Le bandeau survit à une modification d'action ─────────────────────


def test_le_bandeau_survit_a_une_modification_d_action(page: Page, space_id, correction_ctx):
    round_id = correction_ctx["round_ids"][5]
    mr_id = _play_and_publish(space_id, correction_ctx, round_id, 10, 11)

    depublier(space_id, mr_id)

    # Une action ajoutée puis l'après-match resaisi : le parcours réel de
    # correction, qui repasse transitoirement par PreMatch côté use cases.
    home_player = first_player_id(mr_id, "home")
    record_action_api(space_id, mr_id, "home", home_player, turn=9, action_type="TOUCHDOWN")
    post_step5(space_id, mr_id)

    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap", wait_until="load")
    expect(page.locator(".ms-unpublished-banner")).to_be_visible()

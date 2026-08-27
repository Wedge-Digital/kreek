"""Les erreurs coûteuses, de la validation des renvois au jet (carte 411).

Une équipe qui garde plus de 100 kPo après ses renvois doit un jet ; en dessous,
elle repart jouer sans rien voir.

**Le dé est tiré par le serveur, pour de vrai.** Un test qui attendrait « incident
majeur » serait instable une fois sur six. Ces scénarios ne portent donc que sur
ce qui ne dépend pas du jet : qu'un résultat s'affiche, que la trésorerie
corresponde au montant annoncé à l'écran, qu'un second jet soit refusé. La table
des incidents est vérifiée par les 36 tests unitaires de la carte 408 — c'est
leur raison d'être.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import re
import time

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import build_full_competition
from match_report_helpers import play_match
from db_helpers import query_db

BASE_URL = "http://localhost:3210"

PIETAILLE = "DEMO_GRANIT__PIETAILLE"
PERCUTEUR = "DEMO_GRANIT__PERCUTEUR"
COLOSSE = "DEMO_GRANIT__COLOSSE"
# Le staff est le second levier de dépense : les recrues seules ne suffisent
# pas à descendre sous le seuil, trois contraintes les limitant.
ASSISTANT = "ASSISTANT"      # 10 kPo, max 6
CHEERLEADER = "CHEERLEADER"  # 10 kPo, max 6
SEUIL = 100
# `DevCoach` est administrateur de l'espace e2e : sous son identité l'autorisation
# est toujours accordée, et aucun refus n'est observable. L'en-tête fait connecter
# un membre simple par `bypass_auth` — le seul moyen d'exercer le garde-fou.
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}


# ── Lectures ─────────────────────────────────────────────────────────────────

def _phase(team_id: str) -> str | None:
    rows = query_db(f"SELECT game_phase FROM team_proj WHERE team_id = '{team_id}'")
    return rows[0] if rows else None


def _tresorerie(team_id: str) -> int:
    """Le solde vient du grand livre : `team_proj` ne porte pas la trésorerie."""
    rows = query_db(
        "SELECT balance_after_kpo FROM teams__treasury_ledger "
        f"WHERE team_id = '{team_id}' ORDER BY id DESC LIMIT 1"
    )
    assert rows, f"aucun mouvement de trésorerie pour {team_id}"
    return int(rows[0])


def _attendre_phase(team_id: str, phase: str, timeout_s: int = 20) -> None:
    deadline = time.time() + timeout_s
    vue = None
    while time.time() < deadline:
        vue = _phase(team_id)
        if vue == phase:
            return
        time.sleep(0.2)
    raise AssertionError(f"{team_id} : phase « {vue} » au lieu de « {phase} »")


# ── Écritures ────────────────────────────────────────────────────────────────

def _valider_phase(space_id: str, team_id: str, route: str) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{team_id}/{route}",
        headers={"HX-Request": "true"},
    )
    assert resp.status_code == 200, f"{route} : {resp.status_code}"


def _recruter(space_id: str, team_id: str, ligne: str, version: int) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{team_id}/recruitment/players/add",
        data={"roster_line_id": ligne, "version": version},
        headers={"HX-Request": "true"},
    )
    # Le serveur explique son refus dans le corps — le taire obligerait à
    # redemander à la main à chaque échec.
    assert resp.status_code == 200, (
        f"recrutement de {ligne} : {resp.status_code} — {resp.text[:200]}"
    )


def _acheter_staff(space_id: str, team_id: str, uid: str, version: int) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{team_id}/recruitment/staff/add",
        data={"staff_uid": uid, "version": version},
        headers={"HX-Request": "true"},
    )
    assert resp.status_code == 200, f"staff {uid} : {resp.status_code} — {resp.text[:200]}"


def _jeter(space_id: str, team_id: str, entetes: dict | None = None) -> requests.Response:
    return requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{team_id}/costly-mistakes/roll",
        headers={"HX-Request": "true", **(entetes or {})},
    )


def _amener_en_renvois(
    space_id: str,
    ctx: dict,
    team_id: str,
    achats: list[tuple[str, int]],
    staff: list[tuple[str, int]] | None = None,
) -> None:
    """Joue un match, puis dépense ce qu'il faut pour viser une trésorerie.

    Les achats fixent le côté du seuil : c'est le seul levier, la dotation de
    départ et le gain de match étant les mêmes pour toutes les équipes.
    """
    _attendre_phase(team_id, "PlayerImprovement")
    _valider_phase(space_id, team_id, "validate-improvement-phase")
    _attendre_phase(team_id, "Recruitment")

    version = 0
    for ligne, combien in achats:
        for _ in range(combien):
            _recruter(space_id, team_id, ligne, version)
            version += 1
    for uid, combien in staff or []:
        for _ in range(combien):
            _acheter_staff(space_id, team_id, uid, version)
            version += 1
    _valider_phase(space_id, team_id, "validate-recruitment-phase")
    _attendre_phase(team_id, "Dismissals")


# ── Fixtures ─────────────────────────────────────────────────────────────────

@pytest.fixture(scope="module")
def contexte(browser, space_id):
    """Deux équipes **du même roster**.

    Le générateur alterne sinon Granitiers et Zéphyriens, et les lignes de
    recrutement d'un roster ne valent rien pour l'autre — le POST répond 422
    sans dire lequel des deux est en cause.
    """
    return build_full_competition(
        browser,
        space_id,
        num_teams=2,
        num_rounds=1,
        roster_uids=["DEMO_GRANIT", "DEMO_GRANIT"],
    )


@pytest.fixture(scope="module")
def riche(space_id, contexte):
    """Une équipe au-dessus du seuil : deux Percuteurs seulement."""
    home, away = contexte["team_ids"][0], contexte["team_ids"][1]
    # **Gain de match au minimum.** Le défaut de `play_match` est de 50 000 kPo :
    # avec lui, aucune dépense possible ne ramène sous le seuil — l'effectif
    # plafonne à 16 et `cross_limit` borne les postes chers. Zéro n'est pas
    # possible non plus, `MatchGain` étant `greater = 0` ; 10 est donc le plus
    # petit gain qui passe. Ce sont les achats qui décident du côté du seuil, à
    # partir des 520 kPo que valent la dotation et ce gain.
    play_match(
        space_id, contexte, contexte["round_ids"][0], home, away,
        home_gain=10, away_gain=10,
    )
    _amener_en_renvois(space_id, contexte, home, [(PERCUTEUR, 2)])
    assert _tresorerie(home) >= SEUIL, "cette équipe doit être au-dessus du seuil"
    # La fixture mène jusqu'à l'entrée en phase : sans quoi les tests
    # dépendraient de leur ordre d'exécution, et `-k test_3` échouerait sur un
    # écran que `test_2` n'aurait pas ouvert.
    _valider_phase(space_id, home, "validate-dismissals-phase")
    _attendre_phase(home, "CostlyMistakes")
    return home


@pytest.fixture(scope="module")
def pauvre(space_id, contexte, riche):
    """Une équipe sous le seuil : cinq recrues et six membres de staff.

    L'équipe extérieure du même match — elle a encaissé le même gain, et ses
    achats sont le seul levier pour changer de côté du seuil.
    """
    away = contexte["team_ids"][1]
    # Le seul panier qui descende sous le seuil sans rien enfreindre : trois
    # contraintes se croisent chez les Granitiers, et le corpus les porte.
    #
    #   - l'effectif plafonne à 16, onze joueurs étant déjà là → 5 recrues ;
    #   - `cross_limit` en autorise **2 au total** entre Percuteur et Colosse ;
    #   - 520 kPo à dépenser, et il faut retomber sous 100.
    #
    # 140 + 90 + 3 × 50 = 380 de recrues, plus 60 de staff : il reste 80 kPo sur
    # les 520. Un plan qui ignorait le cumul échouait sur « La limite de cumul
    # entre ces postes est atteinte », et les recrues seules ne suffisent pas —
    # le plafond d'effectif et `cross_limit` bornent la dépense à 380.
    _amener_en_renvois(
        space_id,
        contexte,
        away,
        [(COLOSSE, 1), (PERCUTEUR, 1), (PIETAILLE, 3)],
        staff=[(ASSISTANT, 3), (CHEERLEADER, 3)],
    )
    assert _tresorerie(away) < SEUIL, (
        f"cette équipe doit être sous le seuil, elle a {_tresorerie(away)} kPo"
    )
    return away


# ── Les sept scénarios ───────────────────────────────────────────────────────

def test_1_sous_le_seuil_l_equipe_est_prete_a_jouer_sans_ecran(space_id, pauvre):
    """Le seuil, et une **absence d'écran** — ce qu'aucun test unitaire ne voit :
    la logique serveur est identique, seule la sortie change."""
    _valider_phase(space_id, pauvre, "validate-dismissals-phase")
    _attendre_phase(pauvre, "ReadyToPlay")

    page = requests.get(f"{BASE_URL}/app/{space_id}/teams/{pauvre}/costly-mistakes")
    assert page.status_code == 422, "il n'y a pas de jet à faire sous le seuil"


def test_2_au_dessus_du_seuil_le_bandeau_propose_le_jet(page: Page, space_id, riche):
    assert _phase(riche) == "CostlyMistakes", "la validation des renvois a ouvert la phase"

    page.goto(f"{BASE_URL}/app/{space_id}/teams/{riche}", wait_until="load")
    # Le CTA navigue par htmx, pas par `href` : la cible est dans `hx-get`.
    cta = page.locator(".state-banner-cta", has_text="Lancer le dé")
    cta.first.wait_for(timeout=10000)
    expect(cta.first).to_have_attribute("hx-get", re.compile(r"/costly-mistakes$"))


def test_3_le_jet_affiche_son_resultat_et_la_tresorerie_suit(page: Page, space_id, riche):
    """Chemin nominal. Aucune assertion sur l'issue : le dé est tiré par le
    serveur, et attendre un incident précis serait instable une fois sur six."""
    avant = _tresorerie(riche)
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{riche}/costly-mistakes", wait_until="load")
    expect(page.locator(".cm-table")).to_be_visible()

    page.locator(".cm-btn-roll").click()

    # L'animation tient un plancher d'un peu plus d'une seconde : le fragment est
    # reçu bien avant d'être montré, et le guetter « visible » attend l'échéance
    # au lieu de la course du dé.
    verdict = page.locator(".cm-verdict-title")
    verdict.wait_for(state="visible", timeout=10000)
    assert verdict.inner_text().strip(), "un verdict doit s'afficher"

    # La cohérence : ce que l'écran annonce et ce que la base porte.
    reste = page.locator(".cm-calc-line--rest .cm-calc-value").inner_text()
    annonce = int(re.sub(r"[^0-9]", "", reste))
    _attendre_phase(riche, "ReadyToPlay")
    assert _tresorerie(riche) == annonce, (
        f"l'écran annonce {annonce} kPo, la trésorerie en porte {_tresorerie(riche)}"
    )
    assert annonce <= avant, "un jet ne rend jamais d'argent"


@pytest.fixture(scope="module")
def jet_fait(space_id, riche):
    """Le jet, effectué une fois pour les scénarios qui le supposent déjà fait.

    Ils ne peuvent pas s'en passer — un second jet n'existe qu'après un premier —
    mais ils ne doivent pas dépendre de l'**ordre** des tests pour l'obtenir.
    """
    if _phase(riche) == "CostlyMistakes":
        assert _jeter(space_id, riche).status_code == 200
        _attendre_phase(riche, "ReadyToPlay")
    return riche


def test_4_un_second_jet_est_refuse_et_ne_reprend_rien(space_id, riche, jet_fait):
    """La raison d'être de cette carte : un double jet **retirerait de l'argent
    deux fois**. Le bouton est désactivé après le premier — il faut donc poster
    sans passer par l'interface, ce qu'un utilisateur mal intentionné ferait."""
    avant = _tresorerie(riche)

    resp = _jeter(space_id, riche)

    assert resp.status_code == 409, f"un second jet doit être refusé : {resp.status_code}"
    assert _tresorerie(riche) == avant, "la trésorerie ne doit pas rebouger"


def test_5_un_coach_tiers_ne_peut_pas_jeter_le_de(space_id, riche, jet_fait):
    """Le jet a un effet financier et son URL est devinable : le droit garde le
    **POST**, pas seulement l'affichage."""
    avant = _tresorerie(riche)

    refus = _jeter(space_id, riche, ENTETE_MEMBRE_SIMPLE)

    assert refus.status_code == 403, f"membre simple : {refus.status_code}"
    assert _tresorerie(riche) == avant, "un refus ne doit rien prélever"

    # Contre-épreuve. Sans elle, un 403 dû à une URL fautive ou à une garde
    # antérieure se lirait comme un refus d'autorisation. Le 409 dit deux choses :
    # l'URL est la bonne, et le droit a bien été consulté **avant** la phase —
    # sinon les deux requêtes rendraient le même code.
    assert _jeter(space_id, riche).status_code == 409


def test_6_la_page_hors_phase_est_refusee(space_id, riche, jet_fait):
    """Après le jet, l'équipe est repartie jouer : l'écran n'a plus d'objet."""
    assert _phase(riche) == "ReadyToPlay"
    page = requests.get(f"{BASE_URL}/app/{space_id}/teams/{riche}/costly-mistakes")
    assert page.status_code == 422


def test_7_une_equipe_inconnue_ne_donne_pas_d_ecran(space_id):
    inconnue = "01ARZ3NDEKTSV4RRFFQ69G5FAV"
    assert requests.get(
        f"{BASE_URL}/app/{space_id}/teams/{inconnue}/costly-mistakes"
    ).status_code == 404
    assert _jeter(space_id, inconnue).status_code == 404

"""Tests E2E — dépense de SPP sur la fiche joueur (slot journal / panneau de dépense).

Scénarios couverts :
- Équipe hors phase PlayerImprovement → widget journal (lecture seule) affiché,
  bouton "Activer la dépense de SPP" absent.
- Équipe en phase → journal affiché par défaut, bouton visible ; le clic sur
  le bouton bascule explicitement vers le panneau de dépense (pas d'activation
  automatique au chargement de la page).
- Équipe en phase, coach de l'équipe, dépense activée → achat d'une
  compétence → réserve SPP diminuée, tag de compétence acquise en plus.
- Joueur sans aucun SPP → aucune compétence achetable dans skill_picker (son
  comportement existant : bouton "Budget insuf.").
- Augmentation de caractéristique → stat + valeur mises à jour.
- team_value (fiche équipe) incrémenté après achat — vérifie le pipeline app
  event players → teams.

Non testé ici : utilisateur non autorisé (ni coach, ni admin) sur une équipe en
phase PlayerImprovement. `bypass_auth` connecte toujours le même utilisateur
(« DevCoach »), et ce compte est coach de toutes les équipes seedées — le
simuler nécessiterait de modifier directement le payload JSON de l'événement
TeamCreated dans l'event store pour changer le coach_id effectif (l'agrégat
est rejoué depuis team_event_store, pas depuis la projection team_proj), ce
qui reviendrait à fabriquer un état plutôt qu'à piloter une vraie action
applicative — contraire au principe déjà posé par
test_team_detail_state_banner.py. La règle elle-même (`can_spend_spp`) vit
dans purchase_skill_controller.rs, hors du périmètre des tests unitaires
existants ; elle reste donc non couverte automatiquement pour l'instant.

Setup via HTTP direct (comme test_player_detail.py) : création + confirmation
+ inducements + step5 + publication d'un vrai rapport de match fait entrer
l'équipe home en PlayerImprovement.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée
avec au moins 12 équipes inscrites et 7 journées pour la première
compétition/saison du space (make init_db WITH_SEED=1 sur une base fraîche).
Utilise les indices d'équipes 6/7 et l'avant-avant-dernière journée
disponible (round_ids[-3]) — paire et journée dédiées à ce module, jamais
utilisées par les autres suites e2e (0-5 par test_match_report_recap.py,
0/1×[-2] par test_team_detail_state_banner.py, 10/11×[-1] par
test_player_detail.py). L'équipe d'indice 8 (hors de toute action dans ce
module) sert de témoin "hors phase PlayerImprovement".
"""

import re

import pytest
import requests
from playwright.sync_api import Page, expect

from htmx_helpers import cliquer_quand_cable

from db_helpers import query_db as _query_db

BASE_URL = "http://localhost:3210"
_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")


def _create_draft(space_id: str, ctx: dict, round_id: str, home_idx: int, away_idx: int) -> str:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/new",
        data={
            "competition_id": ctx["competition_id"],
            "season_id": ctx["season_id"],
            "round_id": round_id,
            "home_team_id": ctx["teams"][home_idx],
            "away_team_id": ctx["teams"][away_idx],
        },
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"create: {resp.status_code}\n{resp.text[:200]}"
    m = _ULID_RE.search(resp.headers.get("Location", ""))
    assert m, f"match_report_id introuvable dans Location: {resp.headers.get('Location')!r}"
    return m.group(1)


def _ensure_inducements(space_id: str, mr_id: str) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "2", "away_fan_roll": "3"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"fan factor: {resp.status_code}"
    location = resp.headers.get("Location", "")

    for _ in range(3):
        if not location or "/inducements/" not in location:
            break
        resp = requests.post(f"{BASE_URL}{location}", data={"selection": ""}, allow_redirects=False)
        if resp.status_code not in (302, 303):
            break
        location = resp.headers.get("Location", "")


def _record_action_api(space_id: str, mr_id: str, side: str, player_id: str, turn: int,
                        action_type: str, injury_type: str | None = None) -> None:
    endpoint = "step3" if side == "home" else "step4"
    data = {
        "turn": str(turn),
        "player_id": player_id,
        "player_type": "regular",
        "action_type": action_type,
    }
    if injury_type:
        data["injury_type"] = injury_type
    resp = requests.post(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{endpoint}/actions", data=data)
    assert resp.status_code == 200, f"record_action {action_type}: {resp.status_code}\n{resp.text[:200]}"


def _team_player_ids(team_id: str) -> list[str]:
    return _query_db(f"SELECT player_id FROM players_proj WHERE team_id = '{team_id}' ORDER BY player_id;")


def _post_step5(space_id: str, mr_id: str, **overrides) -> requests.Response:
    data = {"home_gain": "50000", "away_gain": "40000", "home_fan_mod": "1", "away_fan_mod": "-1"}
    data.update(overrides)
    return requests.post(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step5", data=data, allow_redirects=False)


def _publish(space_id: str, mr_id: str) -> None:
    resp = requests.post(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/recap/publish", allow_redirects=False)
    assert resp.status_code in (302, 303), f"publish: {resp.status_code}\n{resp.text[:200]}"


def _wait_for(check, attempts=30, delay_s=0.2):
    import time
    for _ in range(attempts):
        if check():
            return
        time.sleep(delay_s)
    pytest.fail("condition jamais satisfaite (pipeline app event pas propagé à temps)")


def _activate_spp_spending(page: Page) -> None:
    """Le panneau droit charge le journal par défaut — il faut cliquer sur
    "Activer la dépense de SPP" pour basculer vers le panneau de dépense.

    `#pd-right-panel` est monté par un `hx-trigger="load"` : le bouton n'existe
    qu'une fois ce fragment inséré, et il est **inerte** le temps qu'htmx le
    câble — cf. `htmx_helpers`. Un clic tombé dans cette fenêtre se perd sans
    émettre de requête, et l'échec tombe dix secondes plus tard sur un symptôme
    muet."""
    cliquer_quand_cable(page, ".btn-toggle-spp")
    expect(page.locator(".tabs")).to_be_visible(timeout=10000)


# ── Fixture : équipe publiée, en PlayerImprovement, joueur crédité en SPP ──────

@pytest.fixture(scope="module")
def spp_ctx(browser, space_id):
    from competition_lifecycle import build_full_competition
    full = build_full_competition(browser, space_id, num_teams=3)
    ctx = {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_ids": full["round_ids"],
        "teams": full["team_ids"],
    }
    round_id = ctx["round_ids"][0]
    home_idx, away_idx = 0, 1

    mr_id = _create_draft(space_id, ctx, round_id, home_idx, away_idx)
    _ensure_inducements(space_id, mr_id)

    home_team_id = ctx["teams"][home_idx]
    players = _team_player_ids(home_team_id)
    assert len(players) >= 6, f"au moins 6 joueurs attendus pour {home_team_id}"
    rich_player, poor_player, value_player, stat_player, elite_player, sheet_player = (
        players[0],
        players[1],
        players[2],
        players[3],
        players[4],
        players[5],
    )

    _record_action_api(space_id, mr_id, "home", rich_player, turn=1, action_type="TOUCHDOWN")
    _record_action_api(space_id, mr_id, "home", rich_player, turn=2, action_type="SORTIE")
    _record_action_api(space_id, mr_id, "home", rich_player, turn=3, action_type="MVP")
    # Joueur dédié au scénario team_value — indépendant de rich_player pour ne
    # pas dépendre de l'ordre d'exécution des tests (réserve SPP non partagée).
    # TOUCHDOWN + MVP = 7 SPP, suffisant pour une compétence primaire choisie
    # (coût niveau 1 = 6) sans devoir changer de mode dans skill_picker.
    _record_action_api(space_id, mr_id, "home", value_player, turn=1, action_type="TOUCHDOWN")
    _record_action_api(space_id, mr_id, "home", value_player, turn=2, action_type="MVP")
    # Joueur dédié au scénario d'augmentation de caractéristique — indépendant
    # de rich_player, budget confortable pour couvrir le coût niveau 1 d'une
    # caractéristique (14), quel que soit l'ordre d'exécution des tests.
    #
    # Le nombre d'essais dépend du **roster** depuis la carte 275 : l'équipe
    # domicile est Granitiers, qui porte `BRAWLIN_BRUTES` et ne compte l'essai
    # que 2 SPP au lieu de 4. Cinq essais donnaient 20 SPP avant, 10 depuis —
    # sous le seuil. Huit en donnent 16, et le test redevient confortable.
    for turn in range(1, 9):
        _record_action_api(space_id, mr_id, "home", stat_player, turn=turn, action_type="TOUCHDOWN")
    # Joueur dédié au barème Élite — un de plus, pour la même raison que les
    # trois précédents : la réserve de SPP n'est pas partagée, et un joueur
    # déjà dépensé rend le bouton « Choisir » invisible plutôt qu'absent, ce
    # qui échoue loin de sa cause. Une Élite primaire de niveau 1 coûte 8 SPP
    # contre 6 pour une Standard ; quatre essais à 2 SPP en donnent 8.
    for turn in range(1, 5):
        _record_action_api(space_id, mr_id, "home", elite_player, turn=turn, action_type="TOUCHDOWN")
    # Joueur dédié à la colonne SPP de la feuille d'équipe (carte 492) — même
    # raison que les quatre précédents : la réserve n'est pas partagée, et un
    # joueur sur lequel un autre test a déjà dépensé rendrait l'achat impossible
    # ou le solde imprévisible. Quatre essais à 2 SPP en donnent 8, de quoi
    # payer une Standard primaire de niveau 1 (6).
    for turn in range(1, 5):
        _record_action_api(space_id, mr_id, "home", sheet_player, turn=turn, action_type="TOUCHDOWN")

    resp = _post_step5(space_id, mr_id, summary_title="Match E2E dépense SPP", summary_body="Généré par les tests.")
    assert resp.status_code in (302, 303), f"step5: {resp.status_code}\n{resp.text[:200]}"
    _publish(space_id, mr_id)

    def team_in_improvement():
        rows = _query_db(f"SELECT game_phase FROM team_proj WHERE team_id = '{home_team_id}';")
        return rows and rows[0] == "PlayerImprovement"

    _wait_for(team_in_improvement)

    def players_credited():
        ids = "', '".join([rich_player, value_player, stat_player, elite_player, sheet_player])
        rows = _query_db(f"SELECT spp FROM players_proj WHERE player_id IN ('{ids}');")
        return len(rows) == 5 and all(int(r) > 0 for r in rows)

    _wait_for(players_credited)

    return {
        "teams": ctx["teams"],
        "home_team_id": home_team_id,
        "rich_player_id": rich_player,
        "poor_player_id": poor_player,
        "value_player_id": value_player,
        "stat_player_id": stat_player,
        "elite_player_id": elite_player,
        "sheet_player_id": sheet_player,
    }


@pytest.fixture(scope="module")
def untouched_team_player(spp_ctx):
    """3e équipe de la compétition dédiée — jamais touchée par le match du
    module, sert de témoin 'hors phase PlayerImprovement'."""
    team_id = spp_ctx["teams"][2]
    players = _team_player_ids(team_id)
    assert players, f"aucun joueur trouvé pour {team_id}"
    return {"team_id": team_id, "player_id": players[0]}


# ── Scénarios ──────────────────────────────────────────────────────────────────

def test_journal_widget_shown_outside_player_improvement_phase(page: Page, space_id, untouched_team_player):
    player_id = untouched_team_player["player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    expect(page.locator(".lock-banner")).to_be_visible()
    expect(page.locator(".tabs")).to_have_count(0)
    expect(page.locator(".btn-toggle-spp")).to_have_count(0)


def test_toggle_button_hidden_outside_player_improvement_phase(page: Page, space_id, untouched_team_player):
    player_id = untouched_team_player["player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    expect(page.locator(".badge-locked")).to_be_visible()
    expect(page.locator(".btn-toggle-spp")).to_have_count(0)


def test_journal_shown_by_default_then_toggle_reveals_spending_panel(page: Page, space_id, spp_ctx):
    player_id = spp_ctx["rich_player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")

    expect(page.locator(".badge-locked")).to_have_count(0)
    expect(page.locator(".btn-toggle-spp")).to_be_visible()
    expect(page.locator(".tabs")).to_have_count(0)

    _activate_spp_spending(page)
    expect(page.locator(".lock-banner")).to_have_count(0)


def test_coach_sees_spending_panel_and_can_purchase_a_skill(page: Page, space_id, spp_ctx):
    player_id = spp_ctx["rich_player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    _activate_spp_spending(page)

    reserve_before = int(page.locator(".spend-panel-remaining-val").inner_text())
    tags_before = page.locator(".skill-tag--acquired").count()

    page.wait_for_selector(".skill-list-table", timeout=10000)
    choosable = page.locator(".btn-add-skill:visible", has_text="Choisir")
    expect(choosable.first).to_be_visible(timeout=10000)

    with page.expect_navigation(wait_until="load"):
        choosable.first.click()

    reserve_after = int(page.locator(".spend-panel-remaining-val").inner_text())
    tags_after = page.locator(".skill-tag--acquired").count()
    assert reserve_after < reserve_before
    assert tags_after == tags_before + 1


def test_player_with_no_spp_cannot_afford_any_skill(page: Page, space_id, spp_ctx):
    player_id = spp_ctx["poor_player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    _activate_spp_spending(page)

    page.wait_for_selector(".skill-list-table", timeout=10000)

    choosable = page.locator(".btn-add-skill:visible", has_text="Choisir")
    random_pick = page.locator(".btn-add-skill:visible", has_text="Tirer")
    expect(choosable).to_have_count(0)
    expect(random_pick).to_have_count(0)


def test_stat_increase_updates_stat_and_reserve(page: Page, space_id, spp_ctx):
    player_id = spp_ctx["stat_player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    _activate_spp_spending(page)

    page.locator(".tab", has_text="Caractéristiques").click()
    ma_before = int(page.locator(".pstat-val").first.inner_text())
    reserve_before = int(page.locator(".spend-panel-remaining-val").inner_text())

    increase_btn = page.locator(".stat-card-btn:not([disabled])").first
    expect(increase_btn).to_be_visible(timeout=10000)
    with page.expect_navigation(wait_until="load"):
        increase_btn.click()

    ma_after = int(page.locator(".pstat-val").first.inner_text())
    reserve_after = int(page.locator(".spend-panel-remaining-val").inner_text())
    assert ma_after == ma_before + 1
    assert reserve_after < reserve_before


def test_team_value_is_frozen_during_the_post_match_phases(page: Page, space_id, spp_ctx):
    """La TV n'est plus une accumulation de deltas : elle est recalculée à
    l'entrée en « Prête à jouer », et ne bouge pas pendant que le coach dépense
    ses SPP. Ce test garde ce gel — c'est la contrepartie assumée d'une valeur
    qui reflète l'effectif réel au moment où l'équipe se déclare prête
    (carte 251)."""
    team_id = spp_ctx["home_team_id"]
    player_id = spp_ctx["value_player_id"]

    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
    value_item = page.locator(".meta-item", has_text="Valeur d'équipe").locator(".meta-value")
    value_before = int(value_item.inner_text().replace("kPo", "").strip())

    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    _activate_spp_spending(page)
    page.wait_for_selector(".skill-list-table", timeout=10000)
    choosable = page.locator(".btn-add-skill:visible", has_text="Choisir")
    expect(choosable.first).to_be_visible(timeout=10000)
    with page.expect_navigation(wait_until="load"):
        choosable.first.click()

    # Laisse au pipeline d'app events le temps de ne rien faire : c'est
    # l'absence de changement qu'on vérifie, elle ne peut pas être « attendue ».
    import time
    time.sleep(2)

    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
    value_after = int(value_item.inner_text().replace("kPo", "").strip())
    assert value_after == value_before, (
        "la TV doit rester figée pendant la phase d'amélioration des joueurs"
    )

def test_une_competence_elite_ajoute_dix_kpo_de_plus_qu_une_standard(page: Page, space_id, spp_ctx):
    """Carte 387 — le barème passe à 30 kPo pour une Élite en accès primaire,
    contre 20 pour une Standard.

    L'assertion porte sur la valeur du joueur **lue à l'écran** après l'achat :
    c'est le seul niveau où l'on constate que le barème du corpus a traversé le
    port, le service de coût, l'événement et la projection.
    """
    player_id = spp_ctx["elite_player_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")

    valeur = page.locator(".meta-item", has_text="Valeur actuelle").locator(".meta-value")
    avant = int(valeur.inner_text().replace("kPo", "").strip())

    _activate_spp_spending(page)
    page.wait_for_selector(".skill-list-table", timeout=10000)

    # « Second Souffle » : GENERAL, donc primaire pour ce poste, et Élite.
    ligne = page.locator("tr", has=page.locator(".skill-name", has_text="Second Souffle"))
    expect(ligne.locator(".skill-elite-badge")).to_be_visible(timeout=10000)
    with page.expect_navigation(wait_until="load"):
        ligne.locator(".btn-add-skill", has_text="Choisir").click()

    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    apres = int(valeur.inner_text().replace("kPo", "").strip())

    assert apres - avant == 30, (
        f"une Élite primaire vaut 30 kPo, pas {apres - avant} — "
        "le barème du corpus n'a pas traversé toute la chaîne"
    )


# ── La colonne SPP de la feuille d'équipe (carte 492) ─────────────────────────


def _colonne_spp_de_la_feuille(page: Page, space_id: str, team_id: str, player_id: str) -> str:
    """Ce que la feuille d'équipe affiche dans la colonne SPP de ce joueur.

    Rendue en texte et non en nombre : c'est aussi ce qui permet de distinguer
    un solde de zéro d'un tiret, quand l'agrégat manque.
    """
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
    ligne = page.locator(f'tr.player-table-row[data-player-detail*="{player_id}"]')
    ligne.first.wait_for(state="attached", timeout=15000)
    return ligne.first.locator("td.player-spp").inner_text().strip()


def test_la_colonne_spp_de_la_liste_est_le_solde_pas_le_cumul(page: Page, space_id, spp_ctx):
    """**Deux écrans, un seul chiffre.**

    `players_proj.spp` cumule les gains : `PlayerSkillPurchased` n'en retire
    rien, seuls les chemins d'annulation le font. La liste affichait donc le
    cumul quand la fiche du joueur affichait la réserve — un joueur ayant tout
    dépensé s'y lisait encore riche.

    Le test achète une compétence et vérifie les deux moitiés : la colonne
    **baisse**, et elle **égale** la réserve de la fiche. La seconde est la plus
    forte — une baisse seule passerait encore si les deux écrans divergeaient
    d'une constante.
    """
    player_id = spp_ctx["sheet_player_id"]
    team_id = spp_ctx["home_team_id"]

    avant = _colonne_spp_de_la_feuille(page, space_id, team_id, player_id)

    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    _activate_spp_spending(page)
    reserve_avant = page.locator(".spend-panel-remaining-val").inner_text().strip()
    assert avant == reserve_avant, (
        f"les deux écrans divergent avant tout achat : liste {avant}, fiche {reserve_avant}"
    )

    page.wait_for_selector(".skill-list-table", timeout=10000)
    choisir = page.locator(".btn-add-skill:visible", has_text="Choisir")
    expect(choisir.first).to_be_visible(timeout=10000)
    with page.expect_navigation(wait_until="load"):
        choisir.first.click()

    reserve_apres = page.locator(".spend-panel-remaining-val").inner_text().strip()
    apres = _colonne_spp_de_la_feuille(page, space_id, team_id, player_id)

    assert int(apres) < int(avant), (
        f"la colonne n'a pas baissé après l'achat : {avant} puis {apres} — "
        "c'est le cumul des gains qui s'affiche, pas le solde"
    )
    assert apres == reserve_apres, (
        f"liste {apres} et fiche {reserve_apres} ne disent pas la même chose"
    )

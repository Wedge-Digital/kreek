"""Tests E2E — rapport de match, steps 3 et 4 (saisie des actions).

Scénarios couverts :
- S1  — Chargement page step3 : 5 zones présentes
- S2  — Sélectionner un tour : tour 3 marqué actif
- S3  — Sélectionner un joueur : action-panel visible
- S4  — Enregistrer un TD : log affiche 1+ entrée Touchdown
- S5  — Enregistrer un Blessé Amoché : log affiche entrée Blessé
- S6  — Enregistrer un Blessé Séquelle −AV : log affiche entrée Blessé
- S7  — Supprimer une action : entrée supprimée du log
- S8  — Plusieurs actions même tour : 2+ entrées dans le log
- S9  — Joueur temp visible si inducements soumis (skip sinon)
- S10 — Journaliers automatiques si équipe < 11 (skip sinon)
- S11 — Page step4 (équipe away) : mêmes zones présentes

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée.
"""

import json
import re

import pytest
import requests
from playwright.sync_api import Page, expect

from db_helpers import query_db as _query_db

BASE_URL = "http://localhost:3210"
_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")
_INDUCEMENTS_TEAM_RE = re.compile(r"/inducements/([0-9A-Z]{26})")


# ── Helpers ───────────────────────────────────────────────────────────────────

def _create_draft(space_id: str, ctx: dict, home_idx: int, away_idx: int) -> str:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/new",
        data={
            "competition_id": ctx["competition_id"],
            "season_id": ctx["season_id"],
            "round_id": ctx["round_id"],
            "home_team_id": ctx["teams"][home_idx],
            "away_team_id": ctx["teams"][away_idx],
        },
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"create: {resp.status_code}\n{resp.text[:200]}"
    m = _ULID_RE.search(resp.headers.get("Location", ""))
    assert m, f"match_report_id introuvable dans Location: {resp.headers.get('Location')!r}"
    return m.group(1)


def _ensure_pre_match(space_id: str, mr_id: str, ctx: dict,
                      home_idx: int, away_idx: int) -> None:
    check = requests.get(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        allow_redirects=False,
    )
    if check.status_code == 200:
        return
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}",
        data={
            "competition_id": ctx["competition_id"],
            "season_id": ctx["season_id"],
            "round_id": ctx["round_id"],
            "home_team_id": ctx["teams"][home_idx],
            "away_team_id": ctx["teams"][away_idx],
        },
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"confirm: {resp.status_code}"


def _ensure_fan_factor(space_id: str, mr_id: str) -> str:
    """Soumet le fan factor si pas encore fait. Retourne le Location header."""
    rows = _query_db(
        f"SELECT home_team_value FROM match_report_proj "
        f"WHERE match_report_id = '{mr_id}'"
    )
    if rows and rows[0] and rows[0] != "":
        return ""  # déjà enregistré
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "1", "away_fan_roll": "2"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"fan factor POST: {resp.status_code}"
    return resp.headers.get("Location", "")


def _ensure_inducements(space_id: str, mr_id: str) -> None:
    """Soumet des inducements vides pour les deux équipes si pas encore fait."""
    rows = _query_db(
        f"SELECT home_inducements, away_inducements FROM match_report_proj "
        f"WHERE match_report_id = '{mr_id}'"
    )
    if rows:
        parts = rows[0].split("|")
        if len(parts) == 2 and parts[0] and parts[1]:
            return  # inducements déjà enregistrés

    location = _ensure_fan_factor(space_id, mr_id)
    if not location:
        rows = _query_db(
            f"SELECT home_team_id, away_team_id FROM match_report_proj "
            f"WHERE match_report_id = '{mr_id}'"
        )
        if rows:
            parts = rows[0].split("|")
            home_id, away_id = parts[0], parts[1]
            location = f"/app/{space_id}/match-report/{mr_id}/inducements/{home_id}"

    for _ in range(3):
        if not location or "/inducements/" not in location:
            break
        loc = location if location.startswith("/") else ""
        if not loc:
            break
        resp = requests.post(
            f"{BASE_URL}{loc}",
            data={"selection": ""},
            allow_redirects=False,
        )
        if resp.status_code not in (302, 303):
            break
        location = resp.headers.get("Location", "")


def _record_action_api(space_id: str, mr_id: str, team_side: str,
                       player_id: str, turn: int, action_type: str) -> str | None:
    """Enregistre une action via l'API. Retourne l'action_id ou None."""
    endpoint = "step3" if team_side == "home" else "step4"
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/{endpoint}/actions",
        data={
            "turn": str(turn),
            "player_id": player_id,
            "player_type": "regular",
            "action_type": action_type,
        },
    )
    if resp.status_code == 200:
        trigger = resp.headers.get("HX-Trigger", "")
        try:
            return json.loads(trigger).get("actionRecorded", {}).get("action_id")
        except (json.JSONDecodeError, AttributeError):
            return None
    return None


def _first_player_id(mr_id: str, team_side: str) -> str | None:
    """Retourne le premier player_id de l'équipe dans la projection."""
    rows = _query_db(
        f"SELECT {team_side}_team_id FROM match_report_proj "
        f"WHERE match_report_id = '{mr_id}'"
    )
    if not rows:
        return None
    team_id = rows[0]
    player_rows = _query_db(
        f"SELECT player_id FROM players_proj "
        f"WHERE team_id = '{team_id}' LIMIT 1"
    )
    return player_rows[0] if player_rows else None


def _wait_for_widgets(page: Page) -> None:
    """Attend que les 3 widgets HTMX soient chargés."""
    page.wait_for_selector("#turn-selector .mr-turn-btn", timeout=6000)
    page.wait_for_selector("#player-selector .mr-player-chip", timeout=6000)


def _select_turn_and_player(page: Page, turn: int = 3) -> None:
    """Sélectionne le tour et le premier joueur visible."""
    page.locator(f".mr-turn-btn[data-turn='{turn}']").first.click()
    page.locator(".mr-player-chip").first.click()
    page.wait_for_selector("#action-panel .mr-action-grid", timeout=6000)


# ── Fixtures ──────────────────────────────────────────────────────────────────

@pytest.fixture(scope="module")
def step3_ctx(browser, space_id):
    from competition_lifecycle import build_full_competition
    full = build_full_competition(browser, space_id, num_teams=4)
    return {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_id": full["round_ids"][0],
        "teams": full["team_ids"],
    }


@pytest.fixture(scope="module")
def mr_step3(space_id, step3_ctx):
    """Match report PreMatch accessible à step3 et step4."""
    mr_id = _create_draft(space_id, step3_ctx, home_idx=0, away_idx=1)
    _ensure_pre_match(space_id, mr_id, step3_ctx, home_idx=0, away_idx=1)
    resp = requests.get(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step3",
        allow_redirects=False,
    )
    assert resp.status_code == 200, (
        f"step3 inaccessible ({resp.status_code}) — "
        "vérifier que le match report est en état PreMatch"
    )
    return mr_id


@pytest.fixture(scope="module")
def mr_step3_full(space_id, step3_ctx):
    """Match report avec fan factor + inducements soumis (pour S9/S10).

    Utilise les équipes 2/3 si disponibles, sinon 0/1 en fallback.
    """
    teams = step3_ctx["teams"]
    home_idx, away_idx = (2, 3) if len(teams) >= 4 else (0, 1)
    mr_id = _create_draft(space_id, step3_ctx, home_idx=home_idx, away_idx=away_idx)
    _ensure_pre_match(space_id, mr_id, step3_ctx, home_idx=home_idx, away_idx=away_idx)
    _ensure_inducements(space_id, mr_id)
    return mr_id


# ── S1 — Chargement ───────────────────────────────────────────────────────────

def test_s1_page_step3_loads(page: Page, space_id, mr_step3):
    """GET step3 → 5 zones présentes (turn-selector, player-selector,
    temp-player-selector, action-panel, action-log)."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")

    expect(page.locator(".mr-header-title")).to_be_visible()
    expect(page.locator("#turn-selector")).to_be_visible()
    expect(page.locator("#player-selector")).to_be_visible()
    # Ces deux divs sont des conteneurs HTMX initialement vides → to_be_attached
    expect(page.locator("#temp-player-selector")).to_be_attached()
    expect(page.locator("#action-panel")).to_be_attached()
    expect(page.locator("#action-log")).to_be_visible()


# ── S2 — Sélection tour ───────────────────────────────────────────────────────

def test_s2_select_turn(page: Page, space_id, mr_step3):
    """Clic sur le tour 3 → bouton marqué actif."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")
    page.wait_for_selector("#turn-selector .mr-turn-btn", timeout=6000)

    page.locator(".mr-turn-btn[data-turn='3']").first.click()

    expect(page.locator(".mr-turn-btn[data-turn='3']").first).to_have_class(re.compile(r"\bactive\b"))


# ── S3 — Sélection joueur ─────────────────────────────────────────────────────

def test_s3_select_player_shows_action_panel(page: Page, space_id, mr_step3):
    """Sélection tour + joueur → action-panel contient la grille d'actions."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")
    _wait_for_widgets(page)
    _select_turn_and_player(page, turn=1)

    expect(page.locator("#action-panel .mr-action-grid")).to_be_visible()


# ── S4 — Enregistrer Touchdown ────────────────────────────────────────────────

def test_s4_record_touchdown(page: Page, space_id, mr_step3):
    """Clic TD → action-log affiche une entrée Touchdown."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")
    _wait_for_widgets(page)
    _select_turn_and_player(page, turn=2)

    page.locator(".mr-action-btn").filter(has_text="Touchdown").click()

    page.wait_for_selector("#action-log .mr-log-entry", timeout=6000)
    entries = page.locator("#action-log .mr-log-entry")
    texts = [e.inner_text() for e in entries.all()]
    assert any("Touchdown" in t for t in texts), (
        f"Aucune entrée Touchdown dans le log : {texts}"
    )


# ── S5 — Enregistrer Blessé Amoché ───────────────────────────────────────────

def test_s5_record_blesse_amoche(page: Page, space_id, mr_step3):
    """Flux Blessé → Amoché → log affiche une entrée Blessé."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")
    _wait_for_widgets(page)
    _select_turn_and_player(page, turn=3)

    page.locator(".mr-action-btn--damage").click()
    page.wait_for_selector(".mr-injury-panel", state="visible", timeout=3000)

    page.locator(".mr-injury-btn").filter(has_text="Amoché").click()

    # Depuis la carte 402, une blessure qui peut donner la Haine ne se confirme
    # pas tant que la question n'est pas tranchée : le bouton reste masqué.
    page.locator(".mr-hate-btn--no").click()
    page.locator(".mr-injury-btn--full").click()

    # Une entrée existe déjà (TD du tour précédent, module partagé) : on
    # attend spécifiquement l'entrée T3/Blessé plutôt que "une entrée
    # existe", sans quoi l'assertion passerait trivialement sur l'ancienne.
    entry_t3 = page.locator("#action-log .mr-log-entry", has_text="T3")
    expect(entry_t3).to_contain_text("Blessé", timeout=6000)


# ── S6 — Enregistrer Blessé Séquelle −AV ─────────────────────────────────────

def test_s6_record_blesse_sequel_av(page: Page, space_id, mr_step3):
    """Flux Blessé → Blessure Grave → séquelle −AV → Confirmer → log affiche Blessé."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")
    _wait_for_widgets(page)
    _select_turn_and_player(page, turn=4)

    page.locator(".mr-action-btn--damage").click()
    page.wait_for_selector(".mr-injury-panel", state="visible", timeout=3000)

    page.locator(".mr-injury-btn").filter(has_text="13–14").click()
    page.wait_for_selector("select.mr-sequel-select", state="visible", timeout=3000)

    page.locator("select.mr-sequel-select").select_option("AV")
    page.locator(".mr-hate-btn--no").click()
    page.locator(".mr-injury-btn--full").click()

    page.wait_for_selector("#action-log .mr-log-entry", timeout=6000)
    entries = page.locator("#action-log .mr-log-entry")
    texts = [e.inner_text() for e in entries.all()]
    assert any("Blessé" in t for t in texts), (
        f"Aucune entrée Blessé (séquelle) dans le log : {texts}"
    )


# ── S7 — Supprimer une action ─────────────────────────────────────────────────

def test_s7_delete_action(page: Page, space_id, mr_step3):
    """Suppression d'une action via le bouton ✕ → entrée disparaît du log."""
    player_id = _first_player_id(mr_step3, "home")
    if not player_id:
        pytest.skip("Aucun joueur trouvé pour l'équipe domicile")

    _record_action_api(space_id, mr_step3, "home", player_id, turn=8, action_type="MVP")

    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")
    page.wait_for_selector("#action-log .mr-log-entry", timeout=6000)

    count_before = page.locator("#action-log .mr-log-entry").count()
    assert count_before >= 1

    page.locator("#action-log .mr-log-delete").last.click()
    page.wait_for_timeout(1000)

    count_after = page.locator("#action-log .mr-log-entry").count()
    assert count_after == count_before - 1, (
        f"Attendu {count_before - 1} entrées après suppression, trouvé {count_after}"
    )


# ── S8 — Plusieurs actions même tour ─────────────────────────────────────────

def test_s8_multiple_actions_same_turn(page: Page, space_id, mr_step3):
    """Enregistrer TD + MVP au tour 5 → log contient 2+ entrées."""
    player_id = _first_player_id(mr_step3, "home")
    if not player_id:
        pytest.skip("Aucun joueur trouvé pour l'équipe domicile")

    _record_action_api(space_id, mr_step3, "home", player_id, turn=5, action_type="TOUCHDOWN")
    _record_action_api(space_id, mr_step3, "home", player_id, turn=5, action_type="MVP")

    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step3",
              wait_until="load")
    page.wait_for_selector("#action-log .mr-log-entry", timeout=6000)

    t5_entries = page.locator("#action-log .mr-log-entry").filter(has_text="T5")
    assert t5_entries.count() >= 2, (
        f"Attendu 2+ entrées au tour 5, trouvé {t5_entries.count()}"
    )


# ── S9 — Joueur temporaire visible ────────────────────────────────────────────

def test_s9_temp_player_visible_after_inducements(page: Page, space_id, mr_step3_full):
    """Star player ou journalier visible dans temp-player-selector après inducements."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3_full}/step3",
              wait_until="load")
    page.wait_for_timeout(2000)

    chips = page.locator("#temp-player-selector .mr-player-chip")
    if chips.count() == 0:
        pytest.skip(
            "Aucun joueur temporaire disponible (ni journalier ni star player) — "
            "vérifier seed data et configuration inducements"
        )

    expect(chips.first).to_be_visible()


# ── S10 — Journaliers automatiques ────────────────────────────────────────────

def test_s10_journeymen_auto(page: Page, space_id, mr_step3_full):
    """Si équipe < 11 joueurs, des journaliers sont présents dans temp-player-selector."""
    rows = _query_db(
        f"SELECT home_team_id FROM match_report_proj "
        f"WHERE match_report_id = '{mr_step3_full}'"
    )
    if not rows:
        pytest.skip("Match report introuvable en DB")

    home_team_id = rows[0]
    count_rows = _query_db(
        f"SELECT COUNT(*) FROM players_proj "
        f"WHERE team_id = '{home_team_id}'"
    )
    player_count = int(count_rows[0]) if count_rows else 11

    if player_count >= 11:
        pytest.skip(f"Équipe domicile a {player_count} joueurs (≥ 11) — pas de journaliers attendus")

    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3_full}/step3",
              wait_until="load")
    page.wait_for_timeout(2000)

    journeymen = page.locator("#temp-player-selector .mr-player-chip--journeyman")
    assert journeymen.count() > 0, (
        f"Équipe a {player_count} joueurs mais aucun journalier dans temp-player-selector"
    )


# ── S11 — Page step4 (équipe away) ────────────────────────────────────────────

def test_s11_step4_away_team(page: Page, space_id, mr_step3):
    """GET step4 → mêmes 5 zones présentes, titre contient le nom de l'équipe away."""
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_step3}/step4",
              wait_until="load")

    expect(page.locator(".mr-header-title")).to_be_visible()
    expect(page.locator("#turn-selector")).to_be_visible()
    expect(page.locator("#player-selector")).to_be_visible()
    expect(page.locator("#temp-player-selector")).to_be_attached()
    expect(page.locator("#action-panel")).to_be_attached()
    expect(page.locator("#action-log")).to_be_visible()

    page.wait_for_selector("#turn-selector .mr-turn-btn", timeout=6000)
    page.wait_for_selector("#player-selector .mr-player-chip", timeout=6000)

# ── Haine (carte 404) ─────────────────────────────────────────────────────────
#
# Les scénarios H3 et H4 sont la raison d'être de cette section : ils vérifient
# qu'une chose **n'apparaît pas**. La logique serveur est identique dans les deux
# cas, seule la conditionnelle du template change — aucun test unitaire ne peut
# le voir.
#
# Ils vivent dans ce fichier plutôt que dans un fichier à eux pour réutiliser la
# fixture `mr_step3`, de portée module : un fichier neuf reconstruirait équipes,
# compétition et rapport, et allongerait une suite que la carte 312 cherche à
# raccourcir.

# Les rosters de démonstration portent DWARF, ELF et SKAVEN (carte 399). Un
# mot-clef d'une autre espèce est donc nécessairement dans le repli.
MOT_DU_REPLI = "Vampire"


def _ouvrir_blessure(page: Page, space_id: str, mr_id: str, turn: int) -> None:
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step3", wait_until="load")
    _wait_for_widgets(page)
    _select_turn_and_player(page, turn=turn)
    page.locator(".mr-action-btn--damage").click()
    page.wait_for_selector(".mr-injury-panel", state="visible", timeout=3000)


def _choisir(page: Page, libelle: str) -> None:
    page.locator(".mr-injury-btn").filter(has_text=libelle).click()


def _panneau_haine(page: Page):
    return page.locator(".mr-hate-panel")


def _confirmation(page: Page):
    return page.locator(".mr-injury-btn--full")


def test_h1_amoche_montre_la_section_et_non_enregistre_sans_haine(
    page: Page, space_id, mr_step3
):
    """R1 et R2 : la section apparaît sur Amoché, et « Non » suffit à confirmer."""
    # Tour 13 : les tours 1 à 5 et 8 portent déjà des actions posées par
    # les tests S4 à S8, et un filtre « T5 » en attraperait plusieurs.
    _ouvrir_blessure(page, space_id, mr_step3, turn=13)
    _choisir(page, "Amoché")
    # `wait_for` avant l'assertion : `not_to_be_visible()` serait vraie à
    # l'instant du clic, avant qu'Alpine n'ait appliqué `x-show`, et
    # l'assertion inverse passerait aussi. Attendre l'état stable est ce qui
    # rend ce test capable d'échouer.
    _panneau_haine(page).wait_for(state="visible", timeout=3000)
    expect(_panneau_haine(page)).to_be_visible()

    page.locator(".mr-hate-btn--no").click()
    expect(_confirmation(page)).to_be_visible()
    _confirmation(page).click()

    entree = page.locator("#action-log .mr-log-entry", has_text="T13").last
    expect(entree).to_contain_text("Blessé", timeout=6000)
    expect(entree).not_to_contain_text("Haine")


def test_h2_une_sequelle_avec_haine_apparait_au_journal(page: Page, space_id, mr_step3):
    """Chemin nominal : Séquelle → Oui → mot-clef → le journal porte la Haine."""
    _ouvrir_blessure(page, space_id, mr_step3, turn=6)
    _choisir(page, "13–14")
    page.wait_for_selector("select.mr-sequel-select", state="visible", timeout=3000)
    page.locator("select.mr-sequel-select").select_option("AV")

    page.locator(".mr-hate-btn").filter(has_text="Oui").click()
    mot = page.locator(".mr-hate-kw").first
    libelle = mot.inner_text().strip()
    mot.click()
    _confirmation(page).click()

    entree = page.locator("#action-log .mr-log-entry", has_text="T6").last
    expect(entree).to_contain_text(f"Haine\u00a0: {libelle}", timeout=6000)


def test_h3_une_commotion_ne_montre_pas_la_section(page: Page, space_id, mr_step3):
    """R1 côté front. Ce test ne peut pas exister ailleurs qu'en navigateur."""
    _ouvrir_blessure(page, space_id, mr_step3, turn=7)
    _choisir(page, "Commotion")
    expect(_panneau_haine(page)).not_to_be_visible()
    # La confirmation, elle, doit être offerte : la règle interdit la Haine sur
    # une Commotion, pas la Commotion.
    expect(_confirmation(page)).to_be_visible()


def test_h4_oui_sans_mot_clef_masque_la_confirmation(page: Page, space_id, mr_step3):
    """R3 côté front : répondre « Oui » ne suffit pas, il faut choisir."""
    _ouvrir_blessure(page, space_id, mr_step3, turn=8)
    _choisir(page, "Amoché")
    page.locator(".mr-hate-btn").filter(has_text="Oui").click()
    expect(_confirmation(page)).not_to_be_visible()

    page.locator(".mr-hate-kw").first.click()
    expect(_confirmation(page)).to_be_visible()


def test_h5_le_filtre_ouvre_le_repli_de_lui_meme(page: Page, space_id, mr_step3):
    """Sinon le coach tape un mot, voit une liste vide, et le croit inexistant."""
    _ouvrir_blessure(page, space_id, mr_step3, turn=9)
    _choisir(page, "Amoché")
    page.locator(".mr-hate-btn").filter(has_text="Oui").click()

    repli = page.locator(".mr-hate-more")
    assert not repli.evaluate("e => e.open"), "le repli doit être fermé au départ"

    page.locator(".mr-hate-search").fill(MOT_DU_REPLI)
    expect(page.locator(".mr-hate-kw", has_text=MOT_DU_REPLI).first).to_be_visible()
    assert repli.evaluate("e => e.open"), (
        f"« {MOT_DU_REPLI} » n'existe que dans le repli : il doit s'ouvrir seul"
    )


def test_h6_le_mot_choisi_reste_visible_malgre_le_filtre(page: Page, space_id, mr_step3):
    """Sans quoi le coach perdrait de vue ce qu'il vient de sélectionner."""
    _ouvrir_blessure(page, space_id, mr_step3, turn=10)
    _choisir(page, "Amoché")
    page.locator(".mr-hate-btn").filter(has_text="Oui").click()

    choisi = page.locator(".mr-hate-kw").first
    libelle = choisi.inner_text().strip()
    choisi.click()

    page.locator(".mr-hate-search").fill("zzzz-aucun-mot-clef")
    expect(page.locator(".mr-hate-kw", has_text=libelle).first).to_be_visible()


def test_h7_deux_fois_le_meme_mot_clef_est_accepte(page: Page, space_id, mr_step3):
    """R7 : aucune garde de doublon, ni au domaine ni à l'écran."""
    libelles = []
    for turn in (11, 12):
        _ouvrir_blessure(page, space_id, mr_step3, turn=turn)
        _choisir(page, "Amoché")
        page.locator(".mr-hate-btn").filter(has_text="Oui").click()
        mot = page.locator(".mr-hate-kw").first
        libelles.append(mot.inner_text().strip())
        mot.click()
        _confirmation(page).click()
        entree = page.locator("#action-log .mr-log-entry", has_text=f"T{turn}").last
        expect(entree).to_contain_text("Haine", timeout=6000)

    assert libelles[0] == libelles[1], "le test doit rejouer le même mot-clef"

"""Briques réutilisables pour produire une compétition complète de bout en
bout : création (phases 1-5), N équipes construites et soumises (auto-
enrôlées), calendrier synchronisé et pairings générés.

Ce module n'est pas collecté par pytest (pas de préfixe `test_`) — il expose
des fonctions utilisées par `test_competition_full_lifecycle.py` (qui vérifie
que ce pipeline fonctionne réellement) et par `build_full_competition()`, que
chaque fichier e2e ayant besoin d'une compétition avec des équipes inscrites
appelle pour obtenir SA PROPRE compétition dédiée (pas de partage entre
fichiers : le verrou d'une équipe dès confirmation d'un match report est
global, pas par journée — cf. `update_match_selection_use_case.rs`, partager
une compétition entre fichiers créerait des collisions d'indisponibilité).

Pourquoi ça existe : avant ce module, cette donnée (journées, équipes
inscrites) n'était produite par aucun script reproductible — seulement
accumulée à la main dans la base de dev au fil du temps. Rien ne vérifiait
que ce pipeline applicatif fonctionnait, et rien ne permettait de la
reconstruire après un reset de base.
"""

import re
import time

from playwright.sync_api import Page, expect

BASE_URL = "http://localhost:3210"
FAKE_LOGO_URL = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg"
# Nom du compte connecté par bypass_auth (legacy_id=1) — cf. scripts/seed_accounts.example.json.
BYPASS_AUTH_COACH_NAME = "DevCoach"

# Rosters du jeu de démonstration (assets/references.example) sans
# FAVOURED_OF_CHOOSE_* : pas de choix de règle spéciale à gérer dans ce flux.
# DEMO_LANTERNE en est donc exclu — c'est le seul roster à choix du jeu, et il
# est couvert par test_special_rule_selector.py.
# Consommé en modulo (cf. build_and_submit_team), la longueur est libre.
ROSTERS = [
    ("DEMO_GRANIT", "Granitiers"),
    ("DEMO_ZEPHYR", "Zéphyriens"),
]


def create_full_competition(
    page: Page,
    competition_create_url: str,
    *,
    num_rounds: int = 7,
    tier_xp: int = 6,
    with_default_bonuses: bool = True,
) -> dict:
    """Phases 1 à 5 : crée une compétition publiée, avec `num_rounds` journées
    programmées et acceptation automatique des inscriptions
    (requires_validation=false). Retourne {competition_id, season_id, name}.

    `with_default_bonuses=False` décoche les bonus offensif et défensif, cochés
    par défaut dans le formulaire. Nécessaire dès qu'un test veut des équipes à
    **égalité de points** : avec les bonus, un vainqueur 3-0 et un vainqueur 1-0
    ne totalisent pas la même chose, et toute égalité recherchée est illusoire."""
    competition_name = f"Ligue E2E Lifecycle {time.time_ns()}"

    # ── Phase 1 : infos + admin ──────────────────────────────────────────
    # L'admin de compétition doit être l'utilisateur bypass_auth lui-même
    # (legacy_id=1, "Bagouze") : /recap et d'autres pages exigent d'être
    # admin d'espace, admin de compétition, ou coach d'une des deux équipes
    # (cf. recap_controller.rs::is_authorized) — un admin choisi au hasard
    # dans les ~100 coachs seedés rendrait ces pages inaccessibles au compte
    # qui exécute les tests.
    page.goto(competition_create_url, wait_until="load")
    page.fill("input[name='name']", competition_name)
    page.wait_for_selector(".coach-result-row", timeout=5000)
    # hx-trigger="keyup changed delay:300ms" — page.fill() ne déclenche pas
    # keyup (juste input/change), la recherche ne partirait jamais.
    page.locator("input[name='q']").press_sequentially(BYPASS_AUTH_COACH_NAME, delay=30)
    page.wait_for_timeout(600)
    page.locator(".coach-result-row", has_text=BYPASS_AUTH_COACH_NAME).first.click()
    page.evaluate(f"document.getElementById('logo_url').value = '{FAKE_LOGO_URL}'")
    with page.expect_navigation(wait_until="load"):
        page.click("button[type='submit']")

    match = re.search(r"/competitions/create/([0-9A-Za-z]+)/([0-9A-Za-z]+)/rules", page.url)
    assert match, f"competition_id/season_id introuvables dans {page.url}"
    competition_id, season_id = match.group(1), match.group(2)

    # ── Phase 2 : règles + tiers (SPP > 0 pour exercer finalize-team.html) ─
    page.wait_for_selector(".tier-block [data-slot='star'] .roster-chip", timeout=5000)
    xp_input = page.locator(".tier-block").first.locator("input.tier-xp")
    xp_input.fill(str(tier_xp))
    xp_input.dispatch_event("change")
    page.fill("#season_name", f"Saison E2E Lifecycle {time.time_ns()}")
    if not with_default_bonuses:
        page.uncheck("#off_activated")
        page.uncheck("#def_activated")
    page.click("button[onclick='submitRules()']")
    page.wait_for_selector("#groups-config", timeout=10000)

    # ── Phase 3 : structure — num_rounds journées à date fixe ────────────
    for i in range(num_rounds):
        page.click("button[onclick=\"addDate('fixed_date')\"]")
        date_inputs = page.locator("#dates-list .date-input")
        date_inputs.nth(i).fill(f"2026-0{(i % 9) + 1}-01")
    page.click("button[onclick='submitStructure()']")
    page.wait_for_selector("#access-mode-btns", timeout=10000)

    # ── Phase 4 : invitations — acceptation automatique ──────────────────
    page.click("#validation-mode-btns .choice-btn[data-val='false']")
    page.click("button[onclick='submitInvitations()']")
    page.wait_for_selector(".recap-row", timeout=10000)

    # ── Phase 5 : publication ─────────────────────────────────────────────
    page.click(".btn-cta")
    page.wait_for_timeout(1000)

    return {"competition_id": competition_id, "season_id": season_id, "name": competition_name}


def _select_competition_by_name(page: Page, competition_name: str) -> None:
    comp_select = page.locator("kreek-select[name='competition_id']")
    comp_select.wait_for(timeout=5000)
    comp_select.locator(".ks-control").click()
    comp_option = comp_select.locator(".ks-option", has_text=competition_name)
    comp_option.wait_for(timeout=5000)
    comp_option.first.click()
    season_hidden = page.locator("kreek-select[name='season_id'] input[type='hidden']")
    expect(season_hidden).not_to_have_value("", timeout=5000)


def build_and_submit_team(page: Page, space_id: str, competition_name: str, coach_option_index: int, roster_index: int) -> str:
    """Crée un draft, choisit un roster (parmi ROSTERS, sans choix de règle
    spéciale à gérer), recrute 11 joueurs, finalise et soumet — auto-enrôlée
    puisque la compétition a requires_validation=false. Retourne le team_id."""
    roster_uid, roster_name = ROSTERS[roster_index % len(ROSTERS)]

    page.goto(f"{BASE_URL}/app/{space_id}/team/create", wait_until="load")
    page.fill("input[name='team_name']", f"Team Lifecycle {roster_name} {time.time_ns()}")

    page.wait_for_selector(".coach-select .ts-control", timeout=5000)
    page.locator(".coach-select .ts-control").click()
    page.wait_for_selector(".ts-dropdown .option", timeout=3000)
    page.locator(".ts-dropdown .option").nth(coach_option_index).click()
    page.wait_for_timeout(300)

    _select_competition_by_name(page, competition_name)

    page.click("button[type='submit']")
    page.wait_for_url(re.compile(r".*/build$"), timeout=10000)
    team_match = re.search(r"/team/([0-9A-Za-z]+)/build", page.url)
    assert team_match, f"team_id introuvable dans {page.url}"
    team_id = team_match.group(1)

    # #player-table-body existe toujours dès le rendu initial (hx-trigger
    # "load" sur #player-table-container, en plus de "rosterSelected") —
    # l'attendre garantit que htmx a fini d'attacher ses listeners sur toute
    # la page avant qu'on déclenche rosterSelected. Sans ça, l'event peut
    # partir avant que #player-table-container écoute déjà, et le widget ne
    # se met jamais à jour (course intermittente, cf. test_special_rule_selector.py).
    page.wait_for_selector("#player-table-body", timeout=5000)
    page.evaluate(
        "([uid, name]) => htmx.trigger(document.body, 'rosterSelected', {uid, name})",
        [roster_uid, roster_name],
    )
    page.wait_for_selector("#player-table-container .tbl-btn", timeout=10000)

    hired = 0
    attempts = 0
    while hired < 11 and attempts < 40:
        buttons = page.locator("#player-table-container .tbl-btn:not([disabled])").all()
        clicked = False
        for btn in buttons:
            if btn.inner_text().strip() == "+":
                btn.click()
                page.wait_for_timeout(150)
                hired += 1
                clicked = True
                break
        if not clicked:
            break
        attempts += 1
    assert hired >= 11, f"N'a pu recruter que {hired} joueurs pour {team_id}"

    page.click("text=Terminer la construction →")
    # "Terminer la construction" est un lien HTMX (hx-get, hx-push-url), pas une
    # navigation classique — un wait_for_timeout fixe pariait sur une durée de
    # swap au lieu d'attendre le DOM réel. Sous charge (12 équipes construites
    # d'affilée), le swap peut dépasser 1s : .submit-bar n'existe pas encore,
    # count()==0, l'équipe restait silencieusement en brouillon (jamais soumise,
    # jamais Enrolled — bug découvert via 11/12 équipes Enrolled au lieu de 12).
    page.wait_for_selector(".submit-bar", timeout=10000)

    # .submit-bar existe dès le rendu initial du swap, mais les widgets
    # player-list et spp-budget (hx-trigger="load", pas de placeholder de
    # hauteur réservée) chargent leur contenu juste après et poussent le
    # bouton plus bas dans la page (~540px observé) — un clic pile pendant
    # cette fenêtre de quelques dizaines de ms tombe dans le vide (bouton non
    # cliqué, aucune requête POST envoyée, la page reste bloquée sur
    # /finalize). Attendre le contenu réel de ces deux widgets avant de
    # cliquer élimine la course, sans parier sur une durée arbitraire.
    page.wait_for_selector(".player-list .player-row", timeout=5000)
    page.wait_for_selector(".spp-budget", timeout=5000)

    # SPP > 0 : atterrit sur finalize-team.html, soumission sans dépense.
    submit_btn = page.locator(".submit-bar button")
    if submit_btn.count() > 0:
        submit_btn.click()
        # post_finalize_team répond par HX-Redirect vers /teams/{team_id} (pluriel)
        # en cas de succès — navigation de toute la page, indépendante du
        # hx-target du bouton. Signal de succès fiable, contrairement à un
        # wait_for_timeout fixe qui pariait sur la durée de la chaîne
        # soumission + auto-enrôlement (asynchrone, plus lente sous charge).
        page.wait_for_url(re.compile(r".*/teams/.*"), timeout=10000)

    return team_id


def sync_and_generate_schedule(page: Page, space_id: str, competition_id: str, season_id: str) -> None:
    """Synchronise `competition_match_days` depuis la structure postée en
    phase 3, puis génère les pairings de toutes les journées non-repos."""
    rounds_url = f"{BASE_URL}/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/rounds"
    page.request.get(rounds_url)

    generate_url = f"{BASE_URL}/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/generate-all"
    resp = page.request.post(generate_url, headers={"HX-Request": "true"})
    assert resp.ok, f"generate-all a échoué : {resp.status} {resp.text()[:300]}"


def build_full_competition(
    browser, space_id: str, num_teams: int, num_rounds: int = 7, *, with_default_bonuses: bool = True
) -> dict:
    """Construit une compétition dédiée (pas partagée entre fichiers de test —
    cf. docstring du module) avec `num_teams` équipes auto-enrôlées et
    `num_rounds` journées programmées et pairées.

    `page` (fixture pytest-playwright) n'est pas réutilisable dans une
    fixture de portée module/session (scope function par défaut) — on ouvre
    notre propre page depuis `browser` (scope session), fermée à la fin.
    """
    from db_helpers import query_db

    page = browser.new_page()
    try:
        competition_create_url = f"{BASE_URL}/app/{space_id}/competitions/create"
        competition = create_full_competition(
            page,
            competition_create_url,
            num_rounds=num_rounds,
            with_default_bonuses=with_default_bonuses,
        )
        team_ids = [
            build_and_submit_team(page, space_id, competition["name"], coach_option_index=i, roster_index=i)
            for i in range(num_teams)
        ]
        sync_and_generate_schedule(page, space_id, competition["competition_id"], competition["season_id"])
        round_ids = query_db(
            f"SELECT id FROM competition_match_days WHERE season_id = '{competition['season_id']}' ORDER BY position;"
        )
        return {
            "competition_id": competition["competition_id"],
            "season_id": competition["season_id"],
            "name": competition["name"],
            "team_ids": team_ids,
            "round_ids": round_ids,
        }
    finally:
        page.close()

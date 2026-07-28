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

Deux constructions d'équipe cohabitent, et ce n'est pas un doublon :

- `build_and_submit_team` passe par le navigateur. C'est le parcours réel,
  exercé par `test_competition_full_lifecycle`, `test_build_and_finalize_team`
  et `test_special_rule_selector`.
- `build_and_submit_team_http` appelle les mêmes routes en HTTP direct. C'est
  ce que `build_full_competition` utilise pour les fixtures des autres
  fichiers : rejouer le parcours au clic n'y teste rien de plus et coûtait
  ~2,4 s par équipe (dont 1,65 s de `wait_for_timeout` fixes) contre quelques
  dizaines de millisecondes. Même raisonnement que `match_report_helpers`,
  qui pilote les rapports en HTTP pour la même raison.
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
# Marqueur d'un refus métier rendu en fragment HTMX. Ne pas tester la simple
# sous-chaîne « table-error » : tout fragment de succès embarque le conteneur
# `class="table-error-zone"` en swap-oob, qui la contient.
ERREUR_FRAGMENT = 'class="table-error"'

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
    deactivated_tiebreaks: list[str] | None = None,
) -> dict:
    """Phases 1 à 5 : crée une compétition publiée, avec `num_rounds` journées
    programmées et acceptation automatique des inscriptions
    (requires_validation=false). Retourne {competition_id, season_id, name}.

    `deactivated_tiebreaks` décoche les critères de départage dont le libellé est
    donné — les sept sont actifs par défaut. Décocher suffit à tester la sélection
    des colonnes : les critères restants gardent l'ordre canonique, et le
    glisser-déposer HTML5 du formulaire est trop fragile pour un test E2E.

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
    for label in deactivated_tiebreaks or []:
        row = page.locator("#tiebreak-list .tiebreak-row").filter(has_text=label)
        row.locator(".tiebreak-check").uncheck()
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


def _space_coach_ids(space_id: str) -> list[str]:
    """Coachs membres du space, dans le même ordre que le sélecteur de l'UI —
    c'est cette liste que `build_and_submit_team` parcourt par index."""
    from db_helpers import query_db

    return query_db(
        "SELECT DISTINCT u.id, u.coach_name FROM spaces__user_space us "
        "JOIN auth__users u ON u.id = us.coach_id "
        f"WHERE us.space_id = '{space_id}' ORDER BY u.coach_name"
    )


def build_and_submit_team_http(
    page: Page, space_id: str, competition_id: str, season_id: str,
    coach_id: str, roster_index: int,
) -> str:
    """Même parcours que `build_and_submit_team`, en HTTP direct au lieu du clic.

    Même justification que `match_report_helpers` : le parcours au navigateur
    est déjà couvert par `test_build_and_finalize_team`,
    `test_special_rule_selector` et `test_draft_team_errors` — le rejouer au
    clic dans les fixtures des autres fichiers ne teste rien de plus et coûte
    ~2,4 s par équipe, dont 1,65 s de `wait_for_timeout` fixes.

    Ce sont bien les vraies routes qui sont appelées, pas des écritures en
    base : la chaîne draft → roster → recrutement → soumission → enrôlement
    reste exercée de bout en bout.
    """
    roster_uid, roster_name = ROSTERS[roster_index % len(ROSTERS)]
    api = page.request

    # `post_draft_team` attend du JSON (Json<DraftTeamForm>), pas de
    # l'urlencoded : côté UI c'est l'extension htmx json-enc qui s'en charge.
    resp = api.post(
        f"{BASE_URL}/app/{space_id}/team/create",
        data={
            "team_name": f"Team HTTP {roster_name} {time.time_ns()}",
            "coach_id": coach_id,
            "coach_name": "",
            "logo_url": FAKE_LOGO_URL,
            "competition_id": competition_id,
            "season_id": season_id,
        },
        headers={"HX-Request": "true"},
    )
    assert resp.ok, f"création du draft : {resp.status} {resp.text()[:300]}"
    location = resp.headers.get("hx-redirect", "")
    m = re.search(r"/team/([0-9A-Za-z]+)/build", location)
    assert m, f"team_id introuvable dans HX-Redirect : {location!r}"
    team_id = m.group(1)

    # Le widget de table des joueurs persiste le choix du roster au passage
    # (choose_roster + save) : c'est lui qui joue le rôle de l'événement
    # `rosterSelected` côté UI.
    table = api.get(
        f"{BASE_URL}/app/{space_id}/team/{team_id}/widgets/player-table",
        params={"roster_uid": roster_uid},
    )
    assert table.ok, f"widget player-table : {table.status}"
    positions = re.findall(r'"player_id":"([^"]+)"', table.text())
    assert positions, f"aucune position recrutable pour {roster_uid}"

    # Un recrutement refusé (quota de poste atteint) répond 200 avec un
    # fragment d'erreur, pas un statut d'échec : c'est le corps qu'il faut
    # regarder. L'UI, elle, voit le bouton « + » passer en disabled.
    # Même composition que l'UI, qui reclique le premier « + » encore actif :
    # on épuise chaque poste dans l'ordre du tableau avant de passer au
    # suivant. Un round-robin donnerait une autre valeur d'équipe, donc
    # d'autres calculs de tiers et d'outsider.
    hired = 0
    for player_id in positions:
        while hired < 11:
            hire = api.post(
                f"{BASE_URL}/app/{space_id}/team/{team_id}/players/hire",
                data={"player_id": player_id},
                headers={"HX-Request": "true"},
            )
            assert hire.ok, f"recrutement : {hire.status} {hire.text()[:200]}"
            # Un refus (quota de poste atteint) répond 200 avec un fragment
            # d'erreur, pas un statut d'échec : c'est le corps qu'il faut lire.
            if ERREUR_FRAGMENT in hire.text():
                break
            hired += 1
        if hired == 11:
            break
    assert hired == 11, f"seulement {hired} joueurs recrutés pour {roster_uid}"

    submit = api.post(
        f"{BASE_URL}/app/{space_id}/team/{team_id}/finalize",
        headers={"HX-Request": "true"},
    )
    # Idem : la soumission refusée répond 200 avec le fragment d'erreur, le
    # succès répond par un HX-Redirect vers la fiche d'équipe.
    assert submit.ok, f"soumission : {submit.status}"
    assert ERREUR_FRAGMENT not in submit.text(), f"soumission refusée : {submit.text()[:200]}"
    return team_id


def _wait_teams_enrolled(team_ids: list[str], timeout_s: int = 20) -> None:
    """L'enrôlement suit la soumission par app event : il est asynchrone.

    Au clic, les ~2,4 s de construction de l'équipe suivante masquaient ce
    délai ; en HTTP il ne reste plus rien entre la dernière soumission et la
    génération des appariements, qui ne verrait alors aucune équipe inscrite.
    """
    from db_helpers import query_db

    liste = "','".join(team_ids)
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        n = query_db(
            f"SELECT count(*) FROM team_proj WHERE team_id IN ('{liste}') "
            "AND status = 'Enrolled'"
        )
        if n and int(n[0]) == len(team_ids):
            return
        time.sleep(0.2)
    raise AssertionError(
        f"toutes les équipes ne sont pas enrôlées après {timeout_s}s : {n}/{len(team_ids)}"
    )


def sync_and_generate_schedule(page: Page, space_id: str, competition_id: str, season_id: str) -> None:
    """Synchronise `competition_match_days` depuis la structure postée en
    phase 3, puis génère les pairings de toutes les journées non-repos."""
    rounds_url = f"{BASE_URL}/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/rounds"
    page.request.get(rounds_url)

    generate_url = f"{BASE_URL}/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/generate-all"
    resp = page.request.post(generate_url, headers={"HX-Request": "true"})
    assert resp.ok, f"generate-all a échoué : {resp.status} {resp.text()[:300]}"


def build_full_competition(
    browser,
    space_id: str,
    num_teams: int,
    num_rounds: int = 7,
    *,
    with_default_bonuses: bool = True,
    deactivated_tiebreaks: list[str] | None = None,
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
            deactivated_tiebreaks=deactivated_tiebreaks,
        )
        coachs = [ligne.split("|")[0] for ligne in _space_coach_ids(space_id)]
        assert len(coachs) >= num_teams, (
            f"{num_teams} équipes demandées mais {len(coachs)} coachs dans "
            f"le space (make seed_e2e)"
        )
        team_ids = [
            build_and_submit_team_http(
                page, space_id, competition["competition_id"],
                competition["season_id"], coachs[i], roster_index=i,
            )
            for i in range(num_teams)
        ]
        _wait_teams_enrolled(team_ids)
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

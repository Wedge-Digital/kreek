"""Tests E2E — page "Mes équipes" restructurée (BC `team_creation`) et widget
"Mes équipes actives/archivées" (BC `teams`, carte 287).

C'est précisément le genre de bug (statut affiché incohérent avec le statut
domaine réel, ou une équipe fraîchement soumise encore visible comme
brouillon) qu'aucun test unitaire n'aurait détecté seul — cf. carte 289.

Toute la donnée est produite en pilotant les vraies routes applicatives (pas
de fabrication SQL), sous le même coach que `bypass_auth` (`DevCoach`) : c'est
lui qui navigue sur `/team/list`, donc les cartes qu'on y vérifie doivent lui
appartenir.

Limite assumée : `bypass_auth` connecte toujours le même coach (`DevCoach`),
et `Espace E2E` est partagé par toute la suite — d'autres fichiers peuvent
donc déjà y avoir créé des équipes pour ce même coach (ex. `coachs[i] ==
DevCoach` dans `build_full_competition`, qui répartit les équipes par index).
Les cartes de cette fixture sont donc localisées par `team_id` (attribut
`hx-get` du rendu réel), jamais par un décompte total de section — un
décompte strict serait fragile sur un espace partagé.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import re
import time

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import (
    BASE_URL,
    ROSTER_NAMES,
    build_and_submit_team_http,
    create_full_competition,
    sync_and_generate_schedule,
)
from db_helpers import query_db
from match_report_helpers import create_draft as create_match_report_draft

MY_TEAMS_ROSTER_UID = "DEMO_GRANIT"
FAKE_LOGO_URL = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg"


def _dev_coach_id() -> str:
    rows = query_db("SELECT id FROM auth__users WHERE coach_name = 'DevCoach'")
    assert rows, "coach DevCoach introuvable — lance `make seed_e2e`"
    return rows[0]


def _other_coach_id(space_id: str, exclude_coach_id: str) -> str:
    rows = query_db(
        "SELECT u.id FROM spaces__user_space us JOIN auth__users u ON u.id = us.coach_id "
        f"WHERE us.space_id = '{space_id}' AND u.id != '{exclude_coach_id}' LIMIT 1;"
    )
    assert rows, "aucun autre coach dans le space — lance `make seed_e2e`"
    return rows[0]


def _create_draft(page, space_id, coach_id, competition_id, season_id, *, roster_uid=None):
    """Crée un brouillon (`team_creation`) sans le soumettre. `roster_uid`
    persiste le choix de roster via le widget player-table — même mécanisme
    que `build_and_submit_team_http`, sans recrutement ni finalisation."""
    resp = page.request.post(
        f"{BASE_URL}/app/{space_id}/team/create",
        data={
            "team_name": f"Draft E2E MyTeams {time.time_ns()}",
            "coach_id": coach_id,
            "coach_name": "",
            "logo_url": "",
            "competition_id": competition_id,
            "season_id": season_id,
        },
        headers={"HX-Request": "true"},
    )
    assert resp.ok, f"création du brouillon : {resp.status} {resp.text()[:300]}"
    m = re.search(r"/team/([0-9A-Za-z]+)/build", resp.headers.get("hx-redirect", ""))
    assert m, f"team_id introuvable dans HX-Redirect : {resp.headers.get('hx-redirect')!r}"
    team_id = m.group(1)

    if roster_uid:
        table = page.request.get(
            f"{BASE_URL}/app/{space_id}/team/{team_id}/widgets/player-table",
            params={"roster_uid": roster_uid},
        )
        assert table.ok, f"widget player-table : {table.status}"

    return team_id


def _wait_status(team_id: str, status: str, timeout_s: int = 20) -> None:
    """L'agrégat `teams` est alimenté par app event : la création de l'équipe
    comme son enrôlement suivent la soumission de façon asynchrone. Sans cette
    attente, `dismiss` arrive alors que l'équipe est encore `PendingEnrollment`
    et le domaine refuse la transition (`Team::dismiss()` n'accepte que
    `Enrolled`) — un 422 qui ressemble à un bug applicatif sans en être un.

    Même délai que celui documenté par `_wait_teams_enrolled()` dans
    `competition_lifecycle.py` ; ce helper-ci prend un statut quelconque, la
    fixture devant aussi attendre `PendingEnrollment` avant de rejeter.
    """
    rows: list[str] = []
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        rows = query_db(f"SELECT status FROM team_proj WHERE team_id = '{team_id}';")
        if rows and rows[0] == status:
            return
        time.sleep(0.2)
    raise AssertionError(
        f"équipe {team_id} : statut {rows[0] if rows else '(absente de team_proj)'} "
        f"au lieu de {status} après {timeout_s}s"
    )


def _dismiss_enrollment(page, space_id, team_id):
    resp = page.request.post(
        f"{BASE_URL}/app/{space_id}/team/{team_id}/enrollment/dismiss",
        headers={"HX-Request": "true"},
    )
    assert resp.ok, f"dismiss_enrollment : {resp.status}"


def _reject_enrollment(page, space_id, team_id):
    resp = page.request.post(
        f"{BASE_URL}/app/{space_id}/team/{team_id}/enrollment/reject",
        headers={"HX-Request": "true"},
    )
    assert resp.ok, f"reject_enrollment : {resp.status}"


# ── Fixture : coach DevCoach avec des équipes dans les 4 ParticipationStatus,
# plus deux brouillons ────────────────────────────────────────────────────────

@pytest.fixture(scope="module")
def my_teams_ctx(browser, space_id):
    page = browser.new_page()
    try:
        dev_coach_id = _dev_coach_id()
        other_coach_id = _other_coach_id(space_id, dev_coach_id)
        create_url = f"{BASE_URL}/app/{space_id}/competitions/create"

        # ── Compétition à acceptation automatique : Enrolled/ReadyToPlay,
        # Enrolled/MatchReporting, Dismissed ──────────────────────────────
        auto = create_full_competition(page, create_url, num_rounds=1, requires_validation=False)

        team_ready_id = build_and_submit_team_http(
            page, space_id, auto["competition_id"], auto["season_id"], dev_coach_id, roster_index=0,
        )
        team_reporting_id = build_and_submit_team_http(
            page, space_id, auto["competition_id"], auto["season_id"], dev_coach_id, roster_index=1,
        )
        # Adversaire du rapport de match — appartient à un autre coach pour ne
        # pas polluer les cartes actives de DevCoach vérifiées ci-dessous.
        opponent_id = build_and_submit_team_http(
            page, space_id, auto["competition_id"], auto["season_id"], other_coach_id, roster_index=0,
        )
        team_dismissed_id = build_and_submit_team_http(
            page, space_id, auto["competition_id"], auto["season_id"], dev_coach_id, roster_index=0,
        )
        # Les quatre équipes sont enrôlées par app event : rien ne garantit que
        # ce soit fait quand la dernière soumission HTTP rend la main. On attend
        # avant de renvoyer l'une d'elles et avant d'ouvrir le rapport de match.
        for enrolled_id in (team_ready_id, team_reporting_id, opponent_id, team_dismissed_id):
            _wait_status(enrolled_id, "Enrolled")

        # `competition_match_days` n'est pas alimenté par la phase 3 : c'est la
        # génération du calendrier qui synchronise les journées depuis la
        # structure postée, puis crée les pairings (cf. `build_full_competition`,
        # qui enchaîne enrôlement -> génération -> lecture des round_ids). Sans
        # cet appel, la requête ci-dessous ne ramène rien.
        sync_and_generate_schedule(page, space_id, auto["competition_id"], auto["season_id"])

        # Après la génération, pour que les quatre équipes soient appariables au
        # moment du calendrier — le renvoi n'a d'effet que sur les cartes.
        _dismiss_enrollment(page, space_id, team_dismissed_id)

        round_id = query_db(
            f"SELECT id FROM competition_match_days WHERE season_id = '{auto['season_id']}' "
            "ORDER BY position LIMIT 1;"
        )[0]
        create_match_report_draft(space_id, auto, round_id, team_reporting_id, opponent_id)

        # ── Compétition à validation manuelle : PendingEnrollment -> Rejected ─
        manual = create_full_competition(page, create_url, num_rounds=1, requires_validation=True)
        team_rejected_id = build_and_submit_team_http(
            page, space_id, manual["competition_id"], manual["season_id"], dev_coach_id, roster_index=0,
        )
        # Même course, à l'autre bout : l'équipe doit exister dans `teams` avant
        # qu'on la rejette. Le statut attendu est `PendingEnrollment` — la
        # compétition est à validation manuelle, donc pas d'enrôlement auto.
        _wait_status(team_rejected_id, "PendingEnrollment")
        _reject_enrollment(page, space_id, team_rejected_id)

        # ── Brouillons : un avec roster choisi, un sans ───────────────────
        draft_with_roster_id = _create_draft(
            page, space_id, dev_coach_id, auto["competition_id"], auto["season_id"],
            roster_uid=MY_TEAMS_ROSTER_UID,
        )
        draft_without_roster_id = _create_draft(
            page, space_id, dev_coach_id, auto["competition_id"], auto["season_id"],
        )

        return {
            "team_ready_id": team_ready_id,
            "team_reporting_id": team_reporting_id,
            "team_dismissed_id": team_dismissed_id,
            "team_rejected_id": team_rejected_id,
            "draft_with_roster_id": draft_with_roster_id,
            "draft_without_roster_id": draft_without_roster_id,
        }
    finally:
        page.close()


def _goto_my_teams(page: Page, space_id: str) -> None:
    page.goto(f"{BASE_URL}/app/{space_id}/team/list", wait_until="load")
    # Le widget actif/archivé charge en hx-trigger="load" : attendre que son
    # swap ait eu lieu avant d'asserter sur son contenu.
    #
    # Pas `networkidle` : les logos d'équipe sont distants (Cloudinary), le
    # réseau ne retombe donc jamais au calme et l'attente expire — c'est aussi
    # le seul endroit de la suite qui l'utilisait. Le repère est le `<link>` du
    # widget, injecté par son seul fragment, et présent même quand les deux
    # listes sont vides (indispensable pour l'espace sans équipe du scénario 4).
    page.wait_for_selector(
        'link[href*="my-teams-widget.css"]', state="attached", timeout=10000
    )


def _card_by_team_id(page: Page, css_class: str, team_id: str):
    return page.locator(f'{css_class}[hx-get*="{team_id}"]')


# ── Scénario 1 — répartition et libellés des 3 sections ───────────────────────

def test_sections_show_correct_counts_and_labels(page: Page, space_id, my_teams_ctx):
    _goto_my_teams(page, space_id)

    draft_with_roster = _card_by_team_id(page, ".draft-card", my_teams_ctx["draft_with_roster_id"])
    expect(draft_with_roster).to_be_visible()
    expect(draft_with_roster.locator(".draft-roster")).to_contain_text(ROSTER_NAMES[MY_TEAMS_ROSTER_UID])
    expect(draft_with_roster.locator(".draft-status")).to_contain_text("Brouillon")

    draft_without_roster = _card_by_team_id(page, ".draft-card", my_teams_ctx["draft_without_roster_id"])
    expect(draft_without_roster).to_be_visible()
    expect(draft_without_roster.locator(".draft-roster")).to_have_count(0)

    ready_card = _card_by_team_id(page, ".team-card", my_teams_ctx["team_ready_id"])
    expect(ready_card).to_be_visible()
    expect(ready_card.locator(".team-card-status")).to_contain_text("Prête à jouer")

    reporting_card = _card_by_team_id(page, ".team-card", my_teams_ctx["team_reporting_id"])
    expect(reporting_card).to_be_visible()
    expect(reporting_card.locator(".team-card-status")).to_contain_text("Rapport en cours")

    rejected_card = _card_by_team_id(page, ".archived-card", my_teams_ctx["team_rejected_id"])
    expect(rejected_card).to_be_visible()
    expect(rejected_card.locator(".archived-tag")).to_contain_text("Inscription refusée")

    dismissed_card = _card_by_team_id(page, ".archived-card", my_teams_ctx["team_dismissed_id"])
    expect(dismissed_card).to_be_visible()
    expect(dismissed_card.locator(".archived-tag")).to_contain_text("Renvoyée")


# ── Scénario 2 — navigation brouillon → build ──────────────────────────────────

def test_draft_continue_button_navigates_to_build(page: Page, space_id, my_teams_ctx):
    _goto_my_teams(page, space_id)

    draft_id = my_teams_ctx["draft_without_roster_id"]
    draft_card = _card_by_team_id(page, ".draft-card", draft_id)
    draft_card.locator(".btn-draft-continue").click()

    page.wait_for_url(re.compile(rf".*/team/{draft_id}/build"), timeout=10000)


# ── Scénario 3 — navigation active/archivée → détail d'équipe ─────────────────

def test_active_card_navigates_to_team_detail(page: Page, space_id, my_teams_ctx):
    _goto_my_teams(page, space_id)

    team_id = my_teams_ctx["team_ready_id"]
    _card_by_team_id(page, ".team-card", team_id).click()

    page.wait_for_url(re.compile(rf".*/teams/{team_id}"), timeout=10000)


def test_archived_card_navigates_to_team_detail(page: Page, space_id, my_teams_ctx):
    _goto_my_teams(page, space_id)

    team_id = my_teams_ctx["team_dismissed_id"]
    _card_by_team_id(page, ".archived-card", team_id).click()

    page.wait_for_url(re.compile(rf".*/teams/{team_id}"), timeout=10000)


# ── Scénario 4 — état vide : aucune des 3 sections ne s'affiche ───────────────

def test_empty_state_shows_no_section(page: Page):
    empty_space_name = f"E2E MyTeams Vide {time.time_ns()}"
    # `form=` et non `data=` : `register_space_submit` prend un
    # `Form<RegisterSpaceFormPayload>`, alors qu'un dict passé à `data=` part en
    # JSON — 415. `_create_draft` utilise `data=` à raison, lui : son handler
    # `post_draft_team` prend un `Json<DraftTeamForm>`.
    resp = page.request.post(
        f"{BASE_URL}/app/space/create",
        form={"space_name": empty_space_name, "logo_url": FAKE_LOGO_URL},
        headers={"HX-Request": "true"},
    )
    assert resp.ok, f"création de l'espace : {resp.status} {resp.text()[:300]}"

    rows = query_db(f"SELECT id FROM spaces WHERE space_name = '{empty_space_name}';")
    assert rows, f"space « {empty_space_name} » introuvable après création"
    empty_space_id = rows[0]

    _goto_my_teams(page, empty_space_id)
    expect(page.locator(".section-title")).to_have_count(0)


# ── Scénario 5 — non-régression : une équipe fraîchement soumise disparaît
# immédiatement de "En cours de création" (le bug initial de la carte 288) ────

def test_freshly_submitted_team_does_not_appear_as_draft(page: Page, space_id, my_teams_ctx):
    _goto_my_teams(page, space_id)

    for team_id in (
        my_teams_ctx["team_ready_id"],
        my_teams_ctx["team_reporting_id"],
        my_teams_ctx["team_dismissed_id"],
        my_teams_ctx["team_rejected_id"],
    ):
        expect(_card_by_team_id(page, ".draft-card", team_id)).to_have_count(0)

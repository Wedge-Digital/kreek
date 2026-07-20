"""Tests E2E — bandeau d'état contextuel sur la page de détail d'équipe.

Scénarios couverts :
- Rapport en cours (MatchReporting) : bandeau avec lien "Reprendre le rapport →"
  qui navigue vers le rapport de match réellement en cours pour l'équipe.
- Phase d'amélioration (PlayerImprovement) : bandeau "Évolutions terminées" →
  clic → passage réel en phase de recrutement (badge d'en-tête mis à jour).
- Phase de recrutement (Recruitment) : bandeau "Terminer les achats" → clic →
  passage réel en phase de renvois.
- Phase de renvois (Dismissals) : bandeau "Valider les renvois" → clic →
  retour réel en "Prête à jouer" (bandeau impression visible à ce stade).

Toute la séquence MatchReporting → PlayerImprovement → Recruitment →
Dismissals → ReadyToPlay est pilotée par de vraies actions applicatives
(création + confirmation + publication d'un rapport de match, puis les 3
routes POST de validation de phase) sur une même équipe suivie de bout en
bout — pas de fabrication d'état en base.

PendingEnrollment (bandeau informatif) : vérifié dynamiquement sur une
équipe existante dans cet état si la base seedée en contient une ; sinon le
test est ignoré (skip) avec message explicite plutôt que d'inventer un
setup fragile pour ce cas.

TemporaryRetirement / OffSeason (aucun bandeau) : non testés ici — en l'état
actuel de l'application, aucune voie applicative (hors admin override-phase,
carte 46 non développée) ne permet d'atteindre ces phases. Une insertion SQL
directe d'événement le permettrait mais casserait le principe de ce test
(piloter via de vraies actions plutôt que fabriquer un état).

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), base initialisée
avec au moins 12 équipes inscrites et 7 journées pour la première
compétition/saison du space (make init_db WITH_SEED=1 sur une base fraîche).
Utilise une journée dédiée (avant-dernière disponible) pour éviter toute
collision de round avec les autres suites e2e match-report.
"""

import re

import pytest
import requests
from playwright.sync_api import Page, expect

from db_helpers import query_db as _query_db

BASE_URL = "http://localhost:3210"
_ULID_RE = re.compile(r"/app/[0-9A-Z]{26}/match-report/([0-9A-Z]{26})")


def _create_draft(space_id: str, ctx: dict, round_id: str, home_idx: int, away_idx: int) -> str:
    """Crée un rapport — la création confirme directement en PreMatch (aucune
    étape Draft séparée dans cette route), ce qui déclenche MatchReportConfirmed."""
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
    pytest.fail("condition jamais satisfaite (listener app event pas propagé à temps)")


# ── Fixture : équipe suivie du rapport de match jusqu'à sa publication ─────────

@pytest.fixture(scope="module")
def banner_ctx(browser, space_id):
    from competition_lifecycle import build_full_competition
    full = build_full_competition(browser, space_id, num_teams=2)
    return {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_ids": full["round_ids"],
        "teams": full["team_ids"],
    }


@pytest.fixture(scope="module")
def match_report_in_progress(space_id, banner_ctx):
    """Crée un rapport de match réel (équipes 0/1, avant-dernière journée
    disponible — dédiée à ce module) et le laisse en MatchReporting, sans le
    publier : les tests suivants pilotent la suite de la séquence."""
    round_id = banner_ctx["round_ids"][-2]
    home_idx, away_idx = 0, 1
    mr_id = _create_draft(space_id, banner_ctx, round_id, home_idx, away_idx)
    home_team_id = banner_ctx["teams"][home_idx]
    return {"mr_id": mr_id, "home_team_id": home_team_id, "round_id": round_id}


# ── Scénario : en attente d'inscription (dynamique, skip si absent) ───────────

def test_pending_enrollment_banner_is_informational(page: Page, space_id):
    rows = _query_db(
        f"SELECT team_id FROM team_proj WHERE status = 'PendingEnrollment' LIMIT 1;"
    )
    if not rows:
        pytest.skip("Aucune équipe en attente d'inscription dans la base seedée")
    team_id = rows[0]
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
    banner = page.locator(".state-banner--pending")
    expect(banner).to_be_visible()
    expect(banner).to_contain_text("en attente d'inscription")
    expect(banner.locator(".state-banner-cta")).to_have_count(0)


# ── Scénario : rapport en cours → reprise ─────────────────────────────────────

def test_match_reporting_banner_resume_link_navigates(page: Page, space_id, match_report_in_progress):
    team_id = match_report_in_progress["home_team_id"]
    mr_id = match_report_in_progress["mr_id"]

    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
    banner = page.locator(".state-banner--phase")
    expect(banner).to_be_visible()
    expect(banner).to_contain_text("Rapport de match en cours")

    banner.locator(".state-banner-cta").click()
    page.wait_for_url(re.compile(rf".*/match-report/{mr_id}.*"), timeout=10000)


# ── Scénario : phase d'amélioration → recrutement ─────────────────────────────

def test_player_improvement_banner_validates_to_recruitment(page: Page, space_id, match_report_in_progress):
    team_id = match_report_in_progress["home_team_id"]
    mr_id = match_report_in_progress["mr_id"]

    _ensure_inducements(space_id, mr_id)
    resp = _post_step5(
        space_id, mr_id,
        summary_title="Match capté par les tests E2E — bandeau d'état",
        summary_body="Compte-rendu généré automatiquement.",
    )
    assert resp.status_code in (302, 303), f"step5: {resp.status_code}\n{resp.text[:200]}"
    _publish(space_id, mr_id)

    def team_in_improvement():
        page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
        return page.locator(".state-banner--phase").count() > 0 and \
            "amélioration" in page.locator(".state-banner-text").inner_text()

    _wait_for(team_in_improvement)

    banner = page.locator(".state-banner--phase")
    expect(banner).to_contain_text("Phase d'amélioration")
    with page.expect_navigation(wait_until="load"):
        banner.locator(".state-banner-cta").click()

    expect(page.locator(".team-status-badge")).to_contain_text("recrutement")


# ── Scénario : phase de recrutement → renvois ─────────────────────────────────

def test_recruitment_banner_validates_to_dismissals(page: Page, space_id, match_report_in_progress):
    team_id = match_report_in_progress["home_team_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")

    banner = page.locator(".state-banner--phase")
    expect(banner).to_contain_text("Phase de recrutement")
    with page.expect_navigation(wait_until="load"):
        banner.locator(".state-banner-cta").click()

    expect(page.locator(".team-status-badge")).to_contain_text("renvois")


# ── Scénario : phase de renvois → prête à jouer ───────────────────────────────

def test_dismissals_banner_validates_to_ready_to_play(page: Page, space_id, match_report_in_progress):
    team_id = match_report_in_progress["home_team_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")

    banner = page.locator(".state-banner--phase")
    expect(banner).to_contain_text("Phase de renvois")
    with page.expect_navigation(wait_until="load"):
        banner.locator(".state-banner-cta").click()

    expect(page.locator(".team-status-badge")).to_contain_text("Prête à jouer")
    ready_banner = page.locator(".state-banner--ready")
    expect(ready_banner).to_be_visible()
    expect(ready_banner.locator(".state-banner-cta--outline")).to_contain_text("Imprimer en PDF")

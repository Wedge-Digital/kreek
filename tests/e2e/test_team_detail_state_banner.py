"""Tests E2E — bandeau d'état contextuel sur la page de détail d'équipe.

Scénarios couverts :
- Rapport en cours (MatchReporting) : bandeau avec lien "Reprendre le rapport →"
  qui navigue vers le rapport de match réellement en cours pour l'équipe.
- Phase d'amélioration (PlayerImprovement) : bandeau "Évolutions terminées" →
  clic → passage réel en phase de recrutement (badge d'en-tête mis à jour).
- Phase de recrutement (Recruitment) : bandeau "Recruter →" → clic → page de
  recrutement, dont le panier valide la phase → passage réel en renvois.
  Seule phase à ne pas se clore depuis son bandeau (carte 264).
- Phase de renvois (Dismissals) : bandeau "Gérer les renvois →" → clic → page
  de renvois, dont le panier valide la phase. Comme le recrutement, cette phase
  ne se clôt pas depuis son bandeau (carte 269).
- Phase des erreurs coûteuses (CostlyMistakes) : bandeau "Lancer le dé →" →
  clic → écran du jet, dont le lien de sortie ramène en "Prête à jouer"
  (bandeau impression visible à ce stade). Au-dessus de 100 kPo cette phase
  n'est pas contournable, et l'équipe suivie ici y passe (épic E13).

Toute la séquence MatchReporting → PlayerImprovement → Recruitment →
Dismissals → CostlyMistakes → ReadyToPlay est pilotée par de vraies actions applicatives
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

**Chaque phase vérifie aussi qu'un membre simple n'y voit aucune action**
(carte 500) : le bandeau, son texte et son bouton d'impression lui restent,
les boutons qui agissent disparaissent. La vérification est posée dans les
tests de phase plutôt que dans un module à part, parce que la séquence est
pilotée par de vraies actions et qu'aucune phase ne se fabrique isolément.

**Tous les CTA de ce module passent par `cliquer_quand_cable_locator`** : ce
sont des `hx-get`/`hx-post` qui arrivent par un échange htmx, donc visibles
quelques dizaines de millisecondes avant d'être câblés. Un clic tombé dans
cette fenêtre ne produit rien — ni requête, ni erreur — et l'assertion suivante
expire sur une phase qui n'a pas avancé. Constaté sur la validation des
renvois, qui échouait dans la suite complète et passait seule.

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
from htmx_helpers import cliquer_quand_cable_locator

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


# ── Carte 500 : le bandeau n'offre que ce qu'on a le droit de faire ───────────

ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}


def _aucune_action_pour_un_tiers(browser, space_id, team_id, texte_attendu):
    """Un membre simple voit le bandeau de la phase, sans aucun bouton d'action.

    La carte 389 n'avait gardé que « ✎ Modifier l'effectif » ; « Recruter → »,
    « Gérer les renvois → », « Évolutions terminées » et « Lancer le dé → »
    s'affichaient pour n'importe quel visiteur. La vérification est donc posée
    **dans chaque test de phase**, au moment où cette phase est réellement
    vivante : la fabriquer à part demanderait de rejouer toute la séquence.

    Un contexte de navigateur à part, et non la fixture `page` : l'en-tête de
    profil se pose à la création du contexte, et le partager connecterait tous
    les autres tests en membre simple.

    `:not([onclick])` écarte « Imprimer en PDF », seul CTA sans URL ni action
    serveur — il n'agit sur rien et reste pour tout le monde. C'est ce qui
    distingue « retirer un raccourci » de « cacher la page », et sans cette
    moitié-là un correctif qui viderait le bandeau entier passerait le test.
    """
    contexte = browser.new_context(extra_http_headers=ENTETE_MEMBRE_SIMPLE)
    try:
        vue = contexte.new_page()
        vue.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
        bandeau = vue.locator(".state-banner")
        expect(bandeau).to_be_visible(timeout=10000)
        expect(bandeau).to_contain_text(texte_attendu)
        expect(bandeau.locator(".state-banner-cta:not([onclick])")).to_have_count(0)
    finally:
        contexte.close()


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
    # **Filtrer par espace.** La requête prenait n'importe quelle équipe de la
    # base, puis l'ouvrait sous `space_id` : tant que la base ne contenait que
    # l'espace e2e, les deux coïncidaient. Une base chargée depuis la production
    # — ce que `make import_prod_db` permet désormais — rend une équipe d'un
    # autre espace, la page ne la trouve pas, et le test échoue en accusant le
    # bandeau.
    rows = _query_db(
        "SELECT team_id FROM team_proj "
        f"WHERE status = 'PendingEnrollment' AND space_id = '{space_id}' LIMIT 1;"
    )
    if not rows:
        pytest.skip("Aucune équipe en attente d'inscription dans cet espace")
    team_id = rows[0]
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
    banner = page.locator(".state-banner--pending")
    expect(banner).to_be_visible()
    expect(banner).to_contain_text("en attente d'inscription")
    expect(banner.locator(".state-banner-cta")).to_have_count(0)


# ── Scénario : rapport en cours → reprise ─────────────────────────────────────

def test_match_reporting_banner_resume_link_navigates(browser, page: Page, space_id, match_report_in_progress):
    team_id = match_report_in_progress["home_team_id"]
    mr_id = match_report_in_progress["mr_id"]

    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
    banner = page.locator(".state-banner--phase")
    expect(banner).to_be_visible()
    expect(banner).to_contain_text("Rapport de match en cours")
    _aucune_action_pour_un_tiers(browser, space_id, team_id, "Rapport de match en cours")

    cliquer_quand_cable_locator(page, banner.locator(".state-banner-cta"))
    page.wait_for_url(re.compile(rf".*/match-report/{mr_id}.*"), timeout=10000)


# ── Scénario : phase d'amélioration → recrutement ─────────────────────────────

def test_player_improvement_banner_validates_to_recruitment(browser, page: Page, space_id, match_report_in_progress):
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
    _aucune_action_pour_un_tiers(browser, space_id, team_id, "Phase d'amélioration")
    with page.expect_navigation(wait_until="load"):
        cliquer_quand_cable_locator(page, banner.locator(".state-banner-cta"))

    expect(page.locator(".team-status-badge")).to_contain_text("recrutement")


# ── Scénario : phase de recrutement → renvois ─────────────────────────────────

def test_recruitment_banner_leads_to_the_recruitment_page(browser, page: Page, space_id, match_report_in_progress):
    """Depuis la carte 264, cette bannière **navigue** au lieu de valider.

    Les trois autres phases se closent depuis leur bandeau ; celle-ci fait
    exception, et c'est délibéré : valider depuis la fiche d'équipe clôturerait
    les achats sans que le coach ait vu ce qu'il valide. La validation vit donc
    dans le panier, et c'est lui qui fait passer l'équipe en renvois — ce que ce
    test suit jusqu'au bout pour que la séquence de phases reste continue.

    Le détail de la page de recrutement appartient à `test_recruitment_phase` ;
    ici on ne vérifie que la charnière.
    """
    team_id = match_report_in_progress["home_team_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")

    banner = page.locator(".state-banner--phase")
    expect(banner).to_contain_text("Phase de recrutement")
    _aucune_action_pour_un_tiers(browser, space_id, team_id, "Phase de recrutement")

    # `<a hx-get hx-push-url>` : HTMX échange `#app-content`, il n'y a pas de
    # navigation du navigateur à attendre.
    cliquer_quand_cable_locator(page, banner.locator(".state-banner-cta"))
    expect(page.locator(".rec-catalog")).to_be_visible(timeout=10000)

    # Panier vide : le CTA termine la phase sans rien acheter.
    cta = page.locator(".rec-cart .cta-primary")
    expect(cta).to_contain_text("Terminer les achats")
    cliquer_quand_cable_locator(page, cta)

    expect(page.locator(".team-status-badge")).to_contain_text("renvois", timeout=15000)


# ── Scénario : phase de renvois → prête à jouer ───────────────────────────────

def test_dismissals_banner_leads_to_the_dismissals_page(browser, page: Page, space_id, match_report_in_progress):
    """Depuis la carte 269, cette bannière **navigue** au lieu de valider.

    Comme le recrutement avant elle : valider depuis la fiche d'équipe
    clôturerait la phase sans que le coach ait vu qui il renvoie. La validation
    vit dans le panier, et c'est lui qui déclare l'équipe prête à jouer — ce que
    ce test suit jusqu'au bout pour que la séquence de phases reste continue.

    Le détail de la page de renvois viendra avec la carte 271.
    """
    team_id = match_report_in_progress["home_team_id"]
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")

    banner = page.locator(".state-banner--phase")
    expect(banner).to_contain_text("Phase de renvois")
    _aucune_action_pour_un_tiers(browser, space_id, team_id, "Phase de renvois")

    # `<a hx-get hx-push-url>` : HTMX échange `#app-content`, il n'y a pas de
    # navigation du navigateur à attendre.
    cliquer_quand_cable_locator(page, banner.locator(".state-banner-cta"))
    expect(page.locator(".dis-roster")).to_be_visible(timeout=10000)

    # Panier vide : le CTA clôt la phase sans renvoyer personne.
    cta = page.locator(".dis-cart .cta-primary")
    expect(cta).to_contain_text("Valider sans renvoyer personne")
    cliquer_quand_cable_locator(page, cta)

    # **La phase des erreurs coûteuses s'intercale ici** (épic E13) : au-dessus
    # de 100 kPo, valider les renvois ne rend pas l'équipe prête à jouer, elle
    # lui doit un jet. Ce test suit la séquence de bout en bout par de vraies
    # actions — c'est sa raison d'être —, donc il lance le dé plutôt que de
    # contourner l'étape.
    expect(page.locator(".team-status-badge")).to_contain_text(
        "Erreurs coûteuses", timeout=15000
    )
    banner = page.locator(".state-banner--phase")
    expect(banner).to_contain_text("Erreurs coûteuses.")
    _aucune_action_pour_un_tiers(browser, space_id, team_id, "Erreurs coûteuses.")
    cliquer_quand_cable_locator(page, banner.locator(".state-banner-cta"))

    expect(page.locator(".cm-table")).to_be_visible(timeout=10000)
    page.locator(".cm-btn-roll").click()
    # Aucune assertion sur l'issue : le dé est tiré par le serveur, et attendre
    # un incident précis serait instable une fois sur six. La table est vérifiée
    # par les tests unitaires de la carte 408.
    expect(page.locator(".cm-verdict-title")).to_be_visible(timeout=10000)

    # Le lien de sortie n'apparaît qu'une fois le jet fait : c'est lui qui ramène
    # le coach sur sa fiche, et donc lui qui poursuit la séquence.
    page.locator(".cm-footer .cta-primary").click()

    expect(page.locator(".team-status-badge")).to_contain_text("Prête à jouer", timeout=15000)
    ready_banner = page.locator(".state-banner--ready")
    expect(ready_banner).to_be_visible()
    # Bouton désigné par son libellé, et non par sa classe : le bandeau « Prête
    # à jouer » porte désormais trois CTA `--outline` (carte 293 y a ajouté
    # « Modifier l'effectif » et « Annuler »), et la classe seule est ambiguë.
    expect(ready_banner.get_by_role("button", name="Imprimer en PDF")).to_be_visible()

    # L'entrée en « Prête à jouer » déclenche le recalcul de la TV (carte 251).
    # Ce qu'on couvre ici est le câblage — listener intra-BC, use case,
    # projection écrite dans la transaction de l'append —, pas l'arithmétique,
    # qui relève des tests unitaires de `compute_team_value`. Sans ce câblage la
    # fiche resterait sur la valeur d'avant la séquence d'après-match.
    value_item = page.locator(".meta-item", has_text="Valeur d'équipe").locator(".meta-value")

    def value_is_recomputed():
        page.reload(wait_until="load")
        texte = value_item.inner_text().replace("kPo", "").strip()
        return texte.isdigit() and int(texte) > 0

    for _ in range(30):
        if value_is_recomputed():
            break
        page.wait_for_timeout(200)
    else:
        raise AssertionError("la TV n'a pas été recalculée à l'entrée en « Prête à jouer »")

    recomputee = int(value_item.inner_text().replace("kPo", "").strip())
    page.reload(wait_until="load")
    stable = int(value_item.inner_text().replace("kPo", "").strip())
    assert stable == recomputee, "la TV projetée doit être stable une fois recalculée"

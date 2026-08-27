"""Tests E2E — édition de l'effectif depuis la fiche d'équipe (carte 295).

La fonctionnalité est à coordination cross-BC par événements DOM : le bandeau
d'état appartient à `teams`, le tableau éditable à `players`, et les deux ne se
connaissent que par `rosterEditRequested` / `rosterEditSaveRequested` /
`rosterEditValidityChanged`. Aucun test unitaire ne couvre cette couture.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import re

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import play_match
from team_phase_helpers import traverser_erreurs_couteuses

# Coach seedé sans droit d'administration (`seed_e2e.rs::SIMPLE_COACH_NAME`).
# `bypass_auth` le connecte sur présentation de cet en-tête ; sans lui, c'est
# DevCoach — admin d'espace — qui répond, et aucun refus n'est observable.
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}

PIETAILLE = "DEMO_GRANIT__PIETAILLE"
FORM_URLENCODED = "application/x-www-form-urlencoded"


# ── Helpers ───────────────────────────────────────────────────────────────────


def _phase(team_id: str) -> str:
    lignes = query_db(f"SELECT game_phase FROM team_proj WHERE team_id = '{team_id}'")
    return lignes[0] if lignes else ""


def _attendre_phase(page: Page, team_id: str, attendue: str) -> None:
    for _ in range(100):
        if _phase(team_id) == attendue:
            return
        page.wait_for_timeout(200)
    raise AssertionError(f"équipe {team_id} en {_phase(team_id)!r} au lieu de {attendue!r}")


def _valider_phase(space_id: str, team_id: str, route: str) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{team_id}/{route}",
        headers={"HX-Request": "true"},
        allow_redirects=False,
    )
    assert resp.status_code in (200, 302, 303), f"{route} : {resp.status_code}"


def _recruter(space_id: str, team_id: str, ligne: str, version: int) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{team_id}/recruitment/players/add",
        data={"roster_line_id": ligne, "version": version},
        headers={"HX-Request": "true"},
    )
    assert resp.status_code == 200, f"recrutement : {resp.status_code}"


def _attendre_effectif(page: Page, team_id: str, taille: int) -> None:
    """La recrue arrive dans `players` par app event : la validation de phase
    rend la main avant qu'elle n'y soit."""
    for _ in range(100):
        if len(_effectif(team_id)) == taille:
            return
        page.wait_for_timeout(200)
    raise AssertionError(f"effectif de {len(_effectif(team_id))} joueurs au lieu de {taille}")


def _effectif(team_id: str) -> list[str]:
    """`jersey|personal_name|display_order` de l'effectif actif, dans l'ordre
    d'affichage — celui-là même que la nouvelle clé de tri produit."""
    return query_db(
        "SELECT coalesce(jersey::text, ''), personal_name, coalesce(display_order::text, '') "
        f"FROM players_proj WHERE team_id = '{team_id}' AND membership = 'Active' "
        "ORDER BY display_order NULLS LAST, jersey NULLS LAST, player_id"
    )


def _ouvrir(page: Page, space_id: str, team_id: str) -> None:
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
    # La racine du widget, et non plus son `<link>` : la carte 342 a réuni les
    # feuilles en un fichier unique, aucun fragment n'en porte plus.
    page.wait_for_selector(".players-widget", state="attached", timeout=10000)


# Le tableau du staff porte lui aussi la classe `player-table` : toutes les
# cibles sont donc préfixées par la racine du widget joueurs.
TABLE = ".players-widget .player-table"


def _entrer_en_edition(page: Page) -> None:
    page.locator(".roster-edit-trigger-btn").click()
    expect(page.locator(TABLE)).to_have_class(re.compile(r"\bedit-mode\b"))


def _enregistrer(page: Page, team_id: str, attendu: list[str]) -> None:
    """Clique Enregistrer et attend que la projection porte l'état visé — la
    persistance passe par un POST puis un swap, rien n'est synchrone."""
    page.locator(".roster-edit-save-btn").click()
    for _ in range(100):
        if _effectif(team_id) == attendu:
            return
        page.wait_for_timeout(200)
    raise AssertionError(f"effectif {_effectif(team_id)} au lieu de {attendu}")


# ── Fixture : équipe « Prête à jouer » avec un joueur renvoyé ─────────────────


@pytest.fixture(scope="module")
def roster_ctx(browser, space_id):
    """Le renvoi impose de traverser tout le cycle de phases : c'est le seul
    chemin applicatif qui produit un `Dismissed`, et la validation des renvois
    ramène l'équipe en « Prête à jouer » (la retraite temporaire n'existe pas
    encore, cf. `Team::apply`)."""
    page = browser.new_page()
    try:
        full = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
        home, away = full["team_ids"][0], full["team_ids"][1]

        play_match(space_id, full, full["round_ids"][0], home, away)
        _attendre_phase(page, home, "PlayerImprovement")
        _valider_phase(space_id, home, "validate-improvement-phase")
        _attendre_phase(page, home, "Recruitment")

        # Une recrue est indispensable : le domaine impose un plancher de onze
        # joueurs éligibles, et une équipe fraîchement créée en compte
        # exactement onze — aucun renvoi n'y serait possible.
        _recruter(space_id, home, PIETAILLE, version=0)
        _valider_phase(space_id, home, "validate-recruitment-phase")
        _attendre_phase(page, home, "Dismissals")
        _attendre_effectif(page, home, 12)

        # Renvoi du joueur au plus grand numéro : c'est son maillot que le
        # scénario 6 réattribuera.
        renvoye = query_db(
            "SELECT player_id, jersey FROM players_proj "
            f"WHERE team_id = '{home}' AND membership = 'Active' "
            "ORDER BY jersey DESC NULLS LAST LIMIT 1"
        )[0]
        player_id, maillot_libere = renvoye.split("|")

        page.goto(f"{BASE_URL}/app/{space_id}/teams/{home}/dismissals", wait_until="load")
        ligne = page.locator(".dis-table tbody tr").filter(
            has=page.locator(f".col-num:text-is('{maillot_libere}')")
        )
        ligne.locator(".fire-btn").click()
        expect(page.locator(".dis-cart .recap-row")).to_have_count(1, timeout=10000)
        page.locator(".dis-cart .cta-primary").click()

        # La phase des erreurs coûteuses s'intercale ici depuis l'épic E13 : cette
        # équipe a encaissé le gain de match par défaut, elle doit donc un jet.
        # Rien de ce que ces scénarios vérifient — nom, numéro, ordre — ne dépend
        # de la trésorerie qu'il laisse, et aucun n'achète quoi que ce soit.
        traverser_erreurs_couteuses(space_id, home)
        _attendre_phase(page, home, "ReadyToPlay")
        return {
            "space_id": space_id,
            "team_id": home,
            "maillot_libere": int(maillot_libere),
        }
    finally:
        page.close()


# ── Scénario 1 — le renommage persiste ───────────────────────────────────────


def test_renommage_persiste_apres_rechargement(page: Page, roster_ctx):
    space_id, team_id = roster_ctx["space_id"], roster_ctx["team_id"]
    _ouvrir(page, space_id, team_id)
    _entrer_en_edition(page)

    page.locator(".players-widget .name-input").first.fill("Grok Fracasse")
    avant = _effectif(team_id)
    attendu = [ligne if i else _remplacer_nom(ligne, "Grok Fracasse") for i, ligne in enumerate(avant)]
    _enregistrer(page, team_id, _avec_ordres(attendu))

    _ouvrir(page, space_id, team_id)
    expect(page.locator(".cell-name .display-value").first).to_contain_text("Grok Fracasse")


def _remplacer_nom(ligne: str, nom: str) -> str:
    jersey, _, ordre = ligne.split("|")
    return f"{jersey}|{nom}|{ordre}"


def _avec_ordres(lignes: list[str]) -> list[str]:
    """Le premier enregistrement pose un rang sur chaque joueur : aucun n'en
    avait, et le use case assigne `display_order` depuis l'index."""
    return [
        f"{ligne.split('|')[0]}|{ligne.split('|')[1]}|{i}" for i, ligne in enumerate(lignes)
    ]


# ── Scénario 2 — la renumérotation persiste ──────────────────────────────────


def test_renumerotation_persiste(page: Page, roster_ctx):
    space_id, team_id = roster_ctx["space_id"], roster_ctx["team_id"]
    _ouvrir(page, space_id, team_id)
    _entrer_en_edition(page)

    page.locator(".players-widget .jersey-input").first.fill("77")
    page.locator(".players-widget .jersey-input").first.dispatch_event("input")
    page.locator(".roster-edit-save-btn").click()

    _attendre_valeur(page, team_id, lambda e: e[0].startswith("77|"), "le maillot 77 en tête")

    _ouvrir(page, space_id, team_id)
    expect(page.locator(".cell-jersey .display-value").first).to_have_text("77")


def _attendre_valeur(page: Page, team_id: str, predicat, quoi: str) -> None:
    for _ in range(100):
        if predicat(_effectif(team_id)):
            return
        page.wait_for_timeout(200)
    raise AssertionError(f"{quoi} — effectif : {_effectif(team_id)}")


# ── Scénario 3 — un numéro vidé s'affiche « — » en lecture ───────────────────


def test_numero_vide_affiche_un_tiret(page: Page, roster_ctx):
    space_id, team_id = roster_ctx["space_id"], roster_ctx["team_id"]
    _ouvrir(page, space_id, team_id)
    _entrer_en_edition(page)

    page.locator(".players-widget .jersey-input").first.fill("")
    page.locator(".players-widget .jersey-input").first.dispatch_event("input")
    page.locator(".roster-edit-save-btn").click()

    # Sans numéro, la ligne passe en fin de tri (`jersey NULLS LAST`) mais garde
    # son rang : c'est `display_order` qui commande désormais.
    _attendre_valeur(page, team_id, lambda e: e[0].startswith("|"), "un joueur sans maillot en tête")

    _ouvrir(page, space_id, team_id)
    expect(page.locator(".cell-jersey .display-value").first).to_have_text("—")


# ── Scénario 4 — le réordonnancement persiste ────────────────────────────────


def test_reordonnancement_persiste(page: Page, roster_ctx):
    space_id, team_id = roster_ctx["space_id"], roster_ctx["team_id"]
    _ouvrir(page, space_id, team_id)
    _entrer_en_edition(page)

    lignes = page.locator(".players-widget .player-table-row")
    # Les deux noms sont posés par le test, jamais lus dans le DOM : la fixture
    # est de portée module, et rien ne garantit que les deux premières lignes
    # soient distinguables — sans le scénario 1, elles sont vides toutes deux,
    # et l'assertion du glissement devient vraie sans rien vérifier.
    premier, second = "Grish Un", "Nobbla Deux"
    lignes.nth(0).locator(".name-input").fill(premier)
    lignes.nth(1).locator(".name-input").fill(second)

    # Glisser la deuxième ligne au-dessus de la première.
    source = lignes.nth(1).locator(".drag-handle-cell")
    cible = lignes.nth(0)
    source.drag_to(cible, target_position={"x": 10, "y": 2})

    expect(lignes.nth(0).locator(".name-input")).to_have_value(second)
    page.locator(".roster-edit-save-btn").click()

    # Attendre la persistance avant de rouvrir : sans cela le GET du widget
    # double le POST encore en vol, la page est peinte sur l'ancien ordre et
    # HTMX ne la rafraîchit plus. Le formulaire poste les noms avec l'ordre,
    # donc les deux persistent ensemble.
    _attendre_valeur(
        page, team_id, lambda e: e[0].split("|")[1] == second, f"« {second} » passé en tête"
    )
    _ouvrir(page, space_id, team_id)
    expect(page.locator(".players-widget .player-table-row").nth(0).locator(".cell-name")).to_contain_text(
        second
    )


# ── Scénario 5 — doublon bloqué côté front, sans requête ─────────────────────


def test_doublon_bloque_sans_requete_reseau(page: Page, roster_ctx):
    space_id, team_id = roster_ctx["space_id"], roster_ctx["team_id"]
    _ouvrir(page, space_id, team_id)
    _entrer_en_edition(page)

    appels: list[str] = []
    page.on("request", lambda r: appels.append(r.url) if "/roster" in r.url else None)

    # Les deux numéros sont posés par le test, jamais lus dans le DOM : la
    # fixture est de portée module, et le scénario 3 vide un maillot que le
    # scénario 4 déplace en deuxième position. Lire cette ligne rendait le
    # doublon vide — or deux vides n'en sont pas un, à dessein (cf. scénario 3).
    # Le maillot libéré par le renvoi est libre par construction.
    numero = str(roster_ctx["maillot_libere"])
    maillots = page.locator(".players-widget .jersey-input")
    for rang in (1, 0):
        maillots.nth(rang).fill(numero)
        maillots.nth(rang).dispatch_event("input")

    expect(page.locator(".players-widget .cell-jersey.has-duplicate")).to_have_count(2)
    expect(page.locator(".roster-edit-save-btn")).to_be_disabled()

    page.wait_for_timeout(500)
    assert appels == [], f"aucune requête ne doit partir sur un doublon : {appels}"


# ── Scénario 6 — le numéro d'un renvoyé est réattribuable ────────────────────


def test_numero_d_un_renvoye_est_reattribuable(page: Page, roster_ctx):
    space_id, team_id = roster_ctx["space_id"], roster_ctx["team_id"]
    libere = roster_ctx["maillot_libere"]
    _ouvrir(page, space_id, team_id)
    _entrer_en_edition(page)

    page.locator(".players-widget .jersey-input").first.fill(str(libere))
    page.locator(".players-widget .jersey-input").first.dispatch_event("input")

    # Le renvoyé ne fait plus partie de l'effectif : son ancien numéro ne doit
    # déclencher aucun doublon, ni côté front ni côté serveur.
    expect(page.locator(".players-widget .cell-jersey.has-duplicate")).to_have_count(0)
    expect(page.locator(".roster-edit-save-btn")).to_be_enabled()

    page.locator(".roster-edit-save-btn").click()
    _attendre_valeur(
        page, team_id, lambda e: e[0].startswith(f"{libere}|"), f"le maillot {libere} réattribué"
    )
    expect(page.locator(".players-widget .roster-save-error")).to_have_count(0)


# ── Scénario 7 — un membre sans droit est refusé ─────────────────────────────


def test_membre_sans_droit_est_refuse(roster_ctx):
    """`DevCoach` est admin de l'espace E2E : sous son identité, `can_spend_spp`
    accorde toujours le droit et aucun refus n'est observable. L'en-tête fait
    connecter par `bypass_auth` un membre simple, seul moyen d'exercer le
    garde-fou."""
    space_id, team_id = roster_ctx["space_id"], roster_ctx["team_id"]
    url = f"{BASE_URL}/app/{space_id}/players/by-team/{team_id}/roster"

    # Corps vide mais **typé** : sans `Content-Type`, l'extracteur de
    # formulaire rejette en 415 avant que l'autorisation ne soit consultée,
    # et le test vérifierait un rejet de format au lieu d'un refus de droit.
    entetes = {"HX-Request": "true", "Content-Type": FORM_URLENCODED}
    refuse = requests.post(url, headers={**entetes, **ENTETE_MEMBRE_SIMPLE}, data="")
    assert refuse.status_code == 403, f"membre simple : {refuse.status_code}"

    # Contre-épreuve : sans l'en-tête, la même requête passe. Sans elle, un 403
    # dû à une URL fautive se lirait comme un refus d'autorisation.
    accepte = requests.post(url, headers=entetes, data="")
    assert accepte.status_code == 200, f"DevCoach : {accepte.status_code}"

def test_un_coach_tiers_ne_voit_pas_le_bouton_d_edition(browser, space_id, roster_ctx):
    """Carte 389 — l'affichage rejoint l'autorisation.

    L'écriture était déjà gardée : le test ci-dessus le montre par un 403. Mais
    le bouton s'affichait pour tout visiteur, qui entrait en édition, saisissait
    un effectif entier, et découvrait le refus à l'enregistrement.

    Un contexte de navigateur à part, et non la fixture `page` : l'en-tête de
    profil se pose à la création du contexte, et le partager avec les autres
    tests les connecterait tous en membre simple.
    """
    home = roster_ctx["team_id"]
    url = f"{BASE_URL}/app/{space_id}/teams/{home}"

    contexte = browser.new_context(extra_http_headers=ENTETE_MEMBRE_SIMPLE)
    try:
        vue_tiers = contexte.new_page()
        vue_tiers.goto(url, wait_until="load")
        expect(vue_tiers.locator(".state-banner")).to_be_visible(timeout=10000)
        expect(vue_tiers.locator(".roster-edit-trigger-btn")).to_have_count(0)
        # Le bandeau garde tout le reste : on retire un raccourci, pas la page.
        expect(vue_tiers.get_by_text("Imprimer", exact=False).first).to_be_visible()
    finally:
        contexte.close()

    # Contre-épreuve indispensable : sans elle, le test passerait aussi bien si
    # le bouton avait disparu pour tout le monde.
    vue_admin = browser.new_page()
    try:
        vue_admin.goto(url, wait_until="load")
        expect(vue_admin.locator(".roster-edit-trigger-btn")).to_have_count(1)
    finally:
        vue_admin.close()


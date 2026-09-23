"""L'option « Autoriser les matchs hors calendrier » et ses trois effets (carte 550).

Décochée, elle ferme trois choses : les points d'entrée vers la saisie manuelle,
la création côté serveur, et la modification de la phase 1 pour un non-admin.

# Desktop et mobile sont deux markups, donc deux vérifications

`app-menu.html` porte le sous-menu desktop **et** la tabbar mobile — les deux
coexistent dans le même gabarit, et c'est le CSS qui bascule l'affichage. Un test
qui ne regarderait qu'une seule taille de fenêtre laisserait l'autre entrée en
place sans rien dire.

# La garde ignore, elle ne refuse pas

Le POST de la phase 1 reste ouvert à tous : c'est le seul chemin de `Draft` vers
`PreMatch`, donc le refuser empêcherait un coach de commencer son rapport. Ce
sont les **valeurs** qui sont écartées. Le test envoie donc deux autres équipes
et relit la base : elle ne doit pas avoir bougé, et la réponse ne doit pas être
une erreur.

# Ce que ce fichier ne couvre pas

Le rapport orphelin de l'incident G. B. L. R — carte 540. Interdire le
hors-calendrier ferme la porte ; ça ne recolle pas le lien manquant entre un
rapport manuel et l'appariement fabriqué pour lui.
"""

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import execute_db, query_db

HX = {"HX-Request": "true"}
#: `requests` ne pose pas de `Content-Type` quand le corps est **vide**, et le
#: serveur rend alors 415. Or le corps vide est précisément le cas « case
#: décochée » : sans cet en-tête posé à la main, le seul geste que cette carte
#: existe pour offrir serait le seul que le test ne saurait pas envoyer.
FORM = {**HX, "Content-Type": "application/x-www-form-urlencoded"}
MEMBRE_SIMPLE = {**HX, "X-Bypass-Auth-Profile": "simple"}

DESKTOP = {"width": 1440, "height": 900}
MOBILE = {"width": 390, "height": 844}


@pytest.fixture(scope="module")
def competition(browser, space_id):
    """Deux équipes, une journée — une seule rencontre suffit à tout éprouver."""
    return build_full_competition(browser, space_id, 2, 1)


@pytest.fixture(autouse=True)
def options_etanches(space_id):
    """Chaque test part d'un espace où **rien** n'interdit, et le rend tel qu'il l'a trouvé.

    ## Pourquoi c'est indispensable ici, et pas ailleurs

    La règle du menu est « une seule suffit » : dès qu'une compétition de
    l'espace interdit le hors-calendrier, l'entrée disparaît **pour tout
    l'espace**. Or la suite accumule les compétitions — une par exécution, jamais
    purgées.

    Une seule compétition laissée en interdiction, par un test interrompu ou une
    falsification, cache donc l'entrée **définitivement**, et plus aucun test ne
    peut observer son état « présente ». C'est arrivé : trois compétitions
    résiduelles ont rendu les deux cas de `test_l_entree_de_menu_disparait…`
    infalsifiables, qui échouaient tous deux sur leur **première** assertion.

    ## Remettre, et non vider

    Le teardown restaure l'état exact relevé au départ plutôt que de tout mettre
    à `NULL` : la suite tourne sur la base de travail (`db_helpers` lit
    `.env.dev`), où un réglage volontaire ne doit pas être effacé par un test.
    """
    avant = _options_de_l_espace(space_id)
    _poser_options(space_id, None)
    yield
    for season_id, valeur in avant.items():
        _poser_options(space_id, valeur, season_id)


# ── Les adresses ──────────────────────────────────────────────────────────────


def _settings(space_id, ctx) -> str:
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/admin/settings"
    )


def _regler(space_id, ctx, autorise: bool) -> None:
    """Coche ou décoche l'option, par la route du panneau.

    Une case décochée **n'est pas envoyée** par un navigateur : l'absence du
    champ est ce qui porte l'interdiction. Le corps vide reproduit ce que fait
    le formulaire, et non une valeur `false` que le serveur ne lirait pas.
    """
    corps = {"autorise_hors_calendrier": "on"} if autorise else {}
    r = requests.post(
        f"{_settings(space_id, ctx)}/general-options",
        data=corps,
        headers=FORM,
        timeout=15,
    )
    assert r.status_code == 200, r.text[:300]


# ── L'étanchéité ──────────────────────────────────────────────────────────────


def _options_de_l_espace(space_id: str) -> dict[str, str | None]:
    lignes = query_db(
        "SELECT s.id || '|' || coalesce(s.options::text, 'NULL') "
        "FROM competition_seasons s JOIN competitions c ON c.id = s.competition_id "
        f"WHERE c.space_id = '{space_id}'"
    )
    etat: dict[str, str | None] = {}
    for ligne in lignes:
        if not ligne.strip():
            continue
        sid, brut = ligne.strip().split("|", 1)
        etat[sid] = None if brut == "NULL" else brut
    return etat


def _poser_options(space_id: str, valeur: str | None, season_id: str | None = None) -> None:
    """`valeur is None` remet la colonne à `NULL` — l'état « jamais réglée »."""
    quoi = "NULL" if valeur is None else f"'{valeur}'::jsonb"
    ou = (
        f"id = '{season_id}'"
        if season_id
        else f"competition_id IN (SELECT id FROM competitions WHERE space_id = '{space_id}')"
    )
    execute_db(f"UPDATE competition_seasons SET options = {quoi} WHERE {ou}")


# ── Ce que la base sait ───────────────────────────────────────────────────────


def _option_en_base(season_id: str) -> str:
    return query_db(
        "SELECT coalesce(options ->> 'autorise_hors_calendrier', 'NULL') "
        f"FROM competition_seasons WHERE id = '{season_id}'"
    )[0].strip()


def _rapport_de_la_journee(round_id: str) -> str:
    lignes = query_db(
        "SELECT match_report_id FROM match_report_proj "
        f"WHERE round_id = '{round_id}' LIMIT 1"
    )
    assert lignes, "la journée doit porter un rapport, créé avec son appariement"
    return lignes[0].strip()


def _equipes_du_rapport(mr_id: str) -> tuple[str, str]:
    ligne = query_db(
        "SELECT home_team_id || '|' || away_team_id "
        f"FROM match_report_proj WHERE match_report_id = '{mr_id}'"
    )[0]
    return tuple(ligne.strip().split("|"))


# ══ 1 · Le réglage lui-même ════════════════════════════════════════════════════


def test_la_colonne_est_nulle_avant_tout_reglage(competition):
    """Le défaut n'est pas écrit en base : il est rendu par serde à la lecture.

    C'est ce qui fait qu'aucune ligue existante ne change de comportement au
    déploiement — et si quelqu'un remplaçait le défaut serde par un `DEFAULT`
    SQL, ce test le dirait.
    """
    assert _option_en_base(competition["season_id"]) == "NULL"


def test_decocher_puis_recocher_ecrit_les_deux_etats(space_id, competition):
    _regler(space_id, competition, False)
    assert _option_en_base(competition["season_id"]) == "false"

    _regler(space_id, competition, True)
    assert _option_en_base(competition["season_id"]) == "true"


def test_un_membre_simple_ne_regle_rien(space_id, competition):
    r = requests.post(
        f"{_settings(space_id, competition)}/general-options",
        data={},
        headers={**MEMBRE_SIMPLE, "Content-Type": "application/x-www-form-urlencoded"},
        timeout=15,
    )
    assert r.status_code in (403, 404), r.status_code


# ══ 2 · Les points d'entrée ════════════════════════════════════════════════════


@pytest.mark.parametrize("taille,selecteur", [
    ("desktop", ".sub-menu-btn"),
    ("mobile", ".mobile-tab"),
])
def test_l_entree_de_menu_disparait_dans_les_deux_tailles(
    page: Page, space_id, competition, taille, selecteur
):
    """**Deux markups, deux vérifications.**

    Le sous-menu desktop et la tabbar mobile coexistent dans `app-menu.html` ;
    en conditionner un seul laisse l'entrée en place sur l'autre taille d'écran,
    et aucun test à une seule fenêtre ne le verrait.
    """
    page.set_viewport_size(DESKTOP if taille == "desktop" else MOBILE)

    _regler(space_id, competition, True)
    page.goto(f"{BASE_URL}/app/{space_id}/competitions", wait_until="load")
    expect(page.locator(selecteur, has_text="Match").first).to_be_attached(timeout=15000)

    _regler(space_id, competition, False)
    page.goto(f"{BASE_URL}/app/{space_id}/competitions", wait_until="load")
    expect(page.locator(selecteur, has_text="Match")).to_have_count(0, timeout=15000)

    _regler(space_id, competition, True)


# ══ 3 · La garde à la création ════════════════════════════════════════════════


def test_la_creation_manuelle_est_refusee_quand_l_option_est_decochee(
    space_id, competition
):
    """Retirer l'entrée de menu cache la fonction ; seule la garde la ferme."""
    _regler(space_id, competition, False)

    r = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/new",
        data={
            "competition_id": competition["competition_id"],
            "season_id": competition["season_id"],
            "round_id": competition["round_ids"][0],
            "home_team_id": competition["team_ids"][0],
            "away_team_id": competition["team_ids"][1],
        },
        headers=FORM,
        timeout=15,
    )

    assert r.status_code == 403, r.status_code
    _regler(space_id, competition, True)


# ══ 4 · La phase 1 en lecture seule ═══════════════════════════════════════════


def test_un_post_trafique_ne_change_pas_les_equipes(space_id, competition):
    """**La garde ignore, elle ne refuse pas.**

    Le POST reste ouvert — c'est le seul chemin de `Draft` vers `PreMatch`. Ce
    sont les deux équipes envoyées qui sont écartées au profit de celles déjà
    en base.
    """
    _regler(space_id, competition, False)

    round_id = competition["round_ids"][0]
    mr_id = _rapport_de_la_journee(round_id)
    avant = _equipes_du_rapport(mr_id)

    # Les deux équipes inversées : un corps qu'un formulaire honnête n'enverrait
    # pas, et que la garde doit traiter comme s'il ne l'avait pas envoyé.
    r = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}",
        data={
            "competition_id": competition["competition_id"],
            "season_id": competition["season_id"],
            "round_id": round_id,
            "home_team_id": avant[1],
            "away_team_id": avant[0],
        },
        headers={**MEMBRE_SIMPLE, **HX},
        timeout=15,
        allow_redirects=False,
    )

    assert r.status_code < 500, f"le POST ne doit pas être refusé : {r.status_code}"
    assert _equipes_du_rapport(mr_id) == avant, "les équipes ont été changées"

    _regler(space_id, competition, True)


@pytest.fixture
def brouillon_neuf(browser, space_id):
    """Une compétition à elle, pour un rapport encore en `Draft`.

    Celle du module ne convient pas : `test_un_post_trafique…` confirme son unique
    rapport, qui passe en `PreMatch` et n'affiche plus la phase 1.
    """
    return build_full_competition(browser, space_id, 2, 1)


def _en_membre_simple(page: Page) -> None:
    """Connecte la page en membre simple, **sur les seules requêtes de l'app**.

    `set_extra_http_headers` poserait l'en-tête sur les polices Google aussi,
    dont le préflight CORS échouerait — cf. `test_player_customisation.py`.
    """
    page.route(
        f"{BASE_URL}/**",
        lambda route: route.continue_(
            headers={**route.request.headers, "x-bypass-auth-profile": "simple"}
        ),
    )


def test_un_membre_simple_commence_son_rapport_depuis_la_phase_figee(
    page: Page, space_id, brouillon_neuf
):
    """**Le vrai bouton, pas un POST forgé** (carte 564).

    En lecture seule, le formulaire n'a plus aucun champ et part vide. Le
    serveur exigeait les cinq champs de la création, et répondait `missing
    field competition_id` : le coach ne pouvait plus commencer son rapport. Le
    test précédent ne le voyait pas, parce qu'il envoie un corps complet.
    """
    _regler(space_id, brouillon_neuf, False)
    mr_id = _rapport_de_la_journee(brouillon_neuf["round_ids"][0])
    avant = _equipes_du_rapport(mr_id)

    _en_membre_simple(page)
    page.goto(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}", wait_until="load")
    expect(page.locator(".mr-figee").first).to_be_visible(timeout=15000)

    page.get_by_role("button", name="Commencer").click()

    expect(page).to_have_url(f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2", timeout=15000)
    assert _equipes_du_rapport(mr_id) == avant, "les équipes ont été changées"

    _regler(space_id, brouillon_neuf, True)

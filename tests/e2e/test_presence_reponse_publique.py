"""Le parcours du coach depuis son lien, dans un navigateur — sans connexion.

C'est le chemin que personne ne verra avant un coach : la page publique
`/presence/{jeton}/{oui|non}` n'est atteignable ni depuis le menu, ni depuis
l'onglet de l'organisateur. Aucun clic de développement n'y mène.

# Le jeton se lit en base, faute de boîte mail

Le test ne peut pas partir d'un clic dans un e-mail. Il lit `token` dans
`competition_presence_answers` — c'est exactement l'usage que `db_helpers`
documente : un état que l'écran ne montre pas, et ne doit pas montrer (R7, le
jeton *est* l'autorisation).

# Les quatre journées, et pourquoi elles sont allouées

Chaque journée porte l'état que ses scénarios supposent, sur le modèle de
`test_competition_presences.py`. Une campagne close ne se réouvre pas pour le
scénario suivant, et une journée figée ne se dégèle pas du tout.

| Journée | Scénarios |
|---|---|
| J1 | `/oui` et l'onglet de l'organisateur · le bouton opposé · deux visites · sans cookie |
| J2 | close par **décision** |
| J3 | close par **échéance** |
| J4 | journée **figée** par un rapport publié, campagne pourtant ouverte |

**Les scénarios de J1 sont ordonnés** et chacun le dit : le second part de la
réponse que le premier a posée.

# Pourquoi J3 passe par `execute_db`

`ouvrir` refuse une échéance passée, et `rouvrir` aussi — aucun parcours
utilisateur ne mène à une campagne échue le jour même. Reculer la date en base
est le seul moyen d'observer le motif « échéance », et c'est le second usage que
`execute_db` déclare : un état qu'aucun parcours ne peut atteindre.

Les deux motifs sont éprouvés séparément parce que ce sont deux branches
distinctes de `libelle_du_motif`, et parce que **c'est l'échéance que le coach
rencontre en vrai** : la plupart des campagnes se ferment d'elles-mêmes.

# Pourquoi J4 fait un vrai tirage

`etat_de_la_journee` calcule `figee` depuis les **appariements** de la journée qui
portent un rapport publié. Un rapport posé sur une journée sans appariement ne
figerait donc peut-être rien, et le test serait vert sans éprouver la règle. J4
suit le chemin réel : présents → clore → tirer → valider → **rouvrir** → publier.

L'ordre compte : `rouvrir` refuse une journée figée, donc la réouverture précède
la publication. C'est ce que fait un organisateur qui repousse son échéance le
matin et voit un match se jouer en avance l'après-midi.

# Pas de `cliquer_quand_cable` sur la page publique

C'est un rendu serveur complet, sans HTMX : la fenêtre où un élément est peint,
visible et **inerte** ne s'y présente pas. C'est la seule page du projet dans ce
cas, et le noter évite qu'on y ajoute l'attente par habitude.

Sur l'onglet de l'organisateur, en revanche, elle reste obligatoire — le panneau
est remplacé à chaque action.

Prérequis : serveur lancé en `make dev-e2e`, base au gabarit (`make e2e_db`).
"""

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import attendre_que, execute_db, query_db
from htmx_helpers import cliquer_quand_cable
from match_report_helpers import play_match

ENTETES = {"HX-Request": "true"}
LOINTAIN = "2099-12-31"

# Un ULID valide dans sa forme, qui n'a jamais désigné de réponse.
JETON_INVENTE = "01JJJJJJJJJJJJJJJJJJJJJJJJ"
# Et un jeton que `SurveyToken::try_new` refuse : R26 veut la **même** page pour
# les deux, sans quoi essayer des jetons dirait lesquels ont eu la bonne forme.
JETON_DIFFORME = "pas-un-ulid"


@pytest.fixture(scope="module")
def competition(browser, space_id):
    """Quatre équipes, quatre journées, sans appariements.

    Les appariements de J4 sont créés par son propre tirage : les recevoir de la
    fixture aurait sauté l'étape que le scénario éprouve.
    """
    return build_full_competition(browser, space_id, 4, 4)


# ── Les adresses ──────────────────────────────────────────────────────────────


def _base(space_id, comp):
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{comp['competition_id']}/{comp['season_id']}/admin"
    )


def _onglet(space_id, comp):
    return f"{_base(space_id, comp)}/presences"


def _poster(space_id, comp, nom, corps):
    return requests.post(
        f"{_base(space_id, comp)}/presences/{nom}",
        json=corps,
        headers=ENTETES,
        timeout=30,
    )


def _lien(jeton, verbe):
    return f"{BASE_URL}/presence/{jeton}/{verbe}"


def _ouvrir_la_journee(page: Page, space_id, comp, round_id):
    """Ouvre l'onglet sur une journée, comme le ferait un clic dans la barre
    latérale — en posant `activeRoundId`, que la page hôte lit."""
    page.goto(_onglet(space_id, comp), wait_until="load")
    expect(page.locator("#presences-sidebar")).to_be_visible(timeout=15000)
    page.evaluate(f"document.body.dataset.activeRoundId = '{round_id}'")
    page.evaluate(
        "htmx.trigger(document.body, 'roundSelected', " f"{{round_id: '{round_id}'}})"
    )
    expect(page.locator(".presences-panel")).to_be_visible(timeout=15000)


# ── Les états de campagne ─────────────────────────────────────────────────────


def _lancer(space_id, comp, round_id, deadline=LOINTAIN):
    reponse = _poster(
        space_id,
        comp,
        "launch",
        {"round_id": round_id, "deadline": deadline, "auto_remind": False},
    )
    assert reponse.status_code == 200, reponse.text[:400]


def _tous_presents(space_id, comp, round_id):
    for equipe in comp["team_ids"]:
        _poster(
            space_id,
            comp,
            "answer",
            {"round_id": round_id, "team_id": equipe, "presence": "presente"},
        )


def _clore(space_id, comp, round_id):
    assert (
        _poster(space_id, comp, "close", {"round_id": round_id}).status_code == 200
    )


def _rouvrir(space_id, comp, round_id, deadline=LOINTAIN):
    reponse = _poster(
        space_id, comp, "reopen", {"round_id": round_id, "deadline": deadline}
    )
    assert reponse.status_code == 200, reponse.text[:400]


# ── Ce que la base sait, et que l'écran ne montre pas ─────────────────────────


def _jetons(round_id):
    """Les jetons de la campagne, dans un ordre stable.

    Trié sur `team_id` et non laissé au hasard du `SELECT` : deux scénarios de la
    même journée doivent parler de la même équipe, sinon leur enchaînement mesure
    l'ordre de la base.
    """
    return query_db(
        "SELECT a.token FROM competition_presence_answers a "
        "JOIN competition_presence_surveys s ON s.id = a.survey_id "
        f"WHERE s.round_id = '{round_id}' ORDER BY a.team_id"
    )


def _reponse_en_base(jeton):
    """`(presence, saisi_par_admin)` — les deux colonnes que R6 distingue."""
    lignes = query_db(
        "SELECT presence, COALESCE(saisi_par_admin, '-') "
        f"FROM competition_presence_answers WHERE token = '{jeton}'"
    )
    assert len(lignes) == 1, f"jeton introuvable en base : {jeton}"
    return tuple(lignes[0].split("|"))


def _appariements(round_id):
    return query_db(
        "SELECT id FROM competition_match_day_pairings "
        f"WHERE match_day_id = '{round_id}'"
    )


def _equipes_du_premier_appariement(round_id):
    ligne = query_db(
        "SELECT home_team_id, away_team_id FROM competition_match_day_pairings "
        f"WHERE match_day_id = '{round_id}' ORDER BY id LIMIT 1"
    )
    assert ligne, "aucun appariement sur cette journée"
    return tuple(ligne[0].split("|"))


# ══ J1 — le parcours qui marche ════════════════════════════════════════════════


def test_j1_visiter_le_lien_oui_confirme_et_l_organisateur_le_voit(
    page: Page, space_id, competition
):
    """Le chemin complet : le lien enregistre, et la présence remonte à l'onglet.

    **Et il n'y a pas de badge « saisi par vous ».** R6 le réserve à la saisie de
    l'organisateur ; le chemin du jeton doit laisser `saisi_par_admin` à `NULL`.
    Rien d'autre ne le vérifie, et c'est la seule chose qui distingue « le coach a
    dit qu'il venait » de « on a dit pour lui » quand une rencontre est contestée.
    """
    round_id = competition["round_ids"][0]
    _lancer(space_id, competition, round_id)
    jeton = _jetons(round_id)[0]

    page.goto(_lien(jeton, "oui"), wait_until="load")

    expect(page.locator(".presence-titre")).to_have_text("C'est noté, tu viens")
    expect(page.locator(".presence-card--ok")).to_be_visible()
    assert _reponse_en_base(jeton) == ("presente", "-")

    _ouvrir_la_journee(page, space_id, competition, round_id)
    expect(page.locator(".colonne--presents .team-card")).to_have_count(1, timeout=15000)
    expect(page.locator(".colonne--presents .team-badge-admin")).to_have_count(
        0, timeout=15000
    )


def test_j1_le_bouton_oppose_fait_basculer_la_reponse(page: Page, space_id, competition):
    """R4 — la parade au préfetch n'est pas un second clic, c'est le bouton opposé.

    Un antivirus qui visite le lien enregistre « oui » ; le coach qui ouvre la page
    voit alors immédiatement de quoi dire le contraire, en un clic.

    Dépend du scénario précédent, qui a posé « présente » sur ce jeton.
    """
    round_id = competition["round_ids"][0]
    jeton = _jetons(round_id)[0]
    assert _reponse_en_base(jeton)[0] == "presente", "le scénario précédent d'abord"

    page.goto(_lien(jeton, "oui"), wait_until="load")
    page.click(".presence-oppose")

    expect(page.locator(".presence-titre")).to_have_text("C'est noté, tu ne viens pas")
    expect(page.locator(".presence-card--absent")).to_be_visible()
    assert _reponse_en_base(jeton) == ("absente", "-")


def test_j1_visiter_deux_fois_le_meme_lien_ne_change_rien(
    page: Page, space_id, competition
):
    """Idempotent : un lien visité deux fois dit deux fois la même chose.

    C'est ce qui rend le `GET` de R4 tenable — SafeLinks et les antivirus
    inspectent les URL, parfois plusieurs fois.

    Dépend des deux scénarios précédents ; il repose « présente ».
    """
    round_id = competition["round_ids"][0]
    jeton = _jetons(round_id)[0]

    for _ in range(2):
        page.goto(_lien(jeton, "oui"), wait_until="load")
        expect(page.locator(".presence-titre")).to_have_text("C'est noté, tu viens")

    assert _reponse_en_base(jeton) == ("presente", "-")


def test_j1_le_lien_repond_sans_cookie_de_session(browser, space_id, competition):
    """Le routeur public, vu du navigateur.

    **Double délibérément** `test_route_publique_presence.rs` : le test unitaire
    vérifie que la route n'est pas sous `require_auth`, celui-ci qu'un navigateur
    sans cookie voit bien la page. Les deux ont échoué séparément ailleurs dans ce
    projet — une route juste rendue inaccessible par le layout, et un layout juste
    servi sur une route morte.

    Un contexte neuf, et non la fixture `page` : celle-ci porte la session de
    l'organisateur, et le test passerait sans rien prouver.
    """
    round_id = competition["round_ids"][0]
    jeton = _jetons(round_id)[0]

    contexte = browser.new_context()
    try:
        assert contexte.cookies() == [], "le contexte doit être vierge"
        anonyme = contexte.new_page()
        anonyme.goto(_lien(jeton, "oui"), wait_until="load")

        expect(anonyme.locator(".presence-titre")).to_have_text("C'est noté, tu viens")
        assert "/auth/login" not in anonyme.url, "aucune redirection vers la connexion"
    finally:
        contexte.close()


# ══ J2 — close par décision ════════════════════════════════════════════════════


def test_j2_une_campagne_close_par_decision_le_dit(page: Page, space_id, competition):
    """R27 — « L'organisateur a clos le sondage avant l'échéance. »

    L'échéance étant lointaine, un message parlant de date limite serait démenti
    par la campagne elle-même.
    """
    round_id = competition["round_ids"][1]
    _lancer(space_id, competition, round_id)
    jeton = _jetons(round_id)[0]
    _clore(space_id, competition, round_id)

    page.goto(_lien(jeton, "oui"), wait_until="load")

    expect(page.locator(".presence-titre")).to_have_text("Ce sondage est clos")
    expect(page.locator(".presence-motif")).to_have_text(
        "L'organisateur a clos le sondage avant l'échéance."
    )
    expect(page.locator(".presence-card--clos")).to_be_visible()
    # R21 — le chemin du coach est fermé, la réponse n'a pas bougé.
    assert _reponse_en_base(jeton) == ("sans_reponse", "-")


# ══ J3 — close par échéance ════════════════════════════════════════════════════


def test_j3_une_campagne_echue_nomme_sa_date_limite(page: Page, space_id, competition):
    """R23 et R27 — la clôture est **calculée**, et l'écran nomme la date.

    Rien n'écrit « close » en base : c'est `statut(maintenant)` qui croise
    l'échéance et la décision. Reculer la date suffit donc à fermer la campagne,
    sans qu'aucune action ne soit passée — et c'est précisément ce que la règle
    promet.
    """
    round_id = competition["round_ids"][2]
    _lancer(space_id, competition, round_id)
    jeton = _jetons(round_id)[0]

    execute_db(
        "UPDATE competition_presence_surveys SET deadline = '2020-01-01' "
        f"WHERE round_id = '{round_id}'"
    )

    page.goto(_lien(jeton, "oui"), wait_until="load")

    expect(page.locator(".presence-titre")).to_have_text("Ce sondage est clos")
    expect(page.locator(".presence-motif")).to_have_text(
        "La date limite de réponse était le 2020-01-01."
    )
    assert _reponse_en_base(jeton) == ("sans_reponse", "-")


# ══ J4 — la journée figée, campagne pourtant ouverte ═══════════════════════════


def test_j4_une_journee_deja_jouee_le_dit_alors_que_la_campagne_est_ouverte(
    page: Page, space_id, competition
):
    """**Le scénario qui a fait écrire R27**, et que personne ne rencontre en
    développement : il demande un rapport publié avant l'échéance.

    « Le sondage est clos » serait ici faux deux fois — la campagne est ouverte et
    son échéance à venir. Ce que le coach doit lire, c'est que la journée s'est
    jouée sans lui.

    L'ordre des six étapes est contraint : `rouvrir` refuse une journée figée, donc
    la réouverture précède la publication.
    """
    round_id = competition["round_ids"][3]
    _lancer(space_id, competition, round_id)
    jeton = _jetons(round_id)[0]
    _tous_presents(space_id, competition, round_id)
    _clore(space_id, competition, round_id)

    # Tirer et valider au clic : c'est le chemin qui écrit les appariements, et
    # `confirm-draw` reçoit ses rencontres du panneau, pas d'un corps forgé ici.
    _ouvrir_la_journee(page, space_id, competition, round_id)
    cliquer_quand_cable(page, ".panel-footer .btn-primary-sm")
    expect(page.locator(".panel-title")).to_contain_text("Tirage proposé", timeout=15000)
    cliquer_quand_cable(page, ".panel-footer .btn-primary-sm")
    attendre_que(
        lambda: len(_appariements(round_id)) == 2,
        quoi="les deux rencontres écrites au calendrier",
    )

    # La campagne redevient ouverte **avant** que la journée ne se fige.
    _rouvrir(space_id, competition, round_id)

    domicile, exterieur = _equipes_du_premier_appariement(round_id)
    play_match(space_id, competition, round_id, domicile, exterieur)

    page.goto(_lien(jeton, "oui"), wait_until="load")

    expect(page.locator(".presence-titre")).to_have_text("Ce sondage est clos")
    expect(page.locator(".presence-motif")).to_have_text(
        "Cette journée a déjà été jouée : un rapport de match y est publié."
    )
    # R13 — la présence posée avant le tirage n'a pas été réécrite par la visite.
    assert _reponse_en_base(jeton)[0] == "presente"


# ══ R26 — un lien qui ne mène à rien ═══════════════════════════════════════════


@pytest.mark.parametrize("jeton", [JETON_INVENTE, JETON_DIFFORME])
def test_un_jeton_inconnu_ou_difforme_donne_la_meme_page_muette(
    page: Page, space_id, jeton
):
    """R26 — la page ne révèle **jamais** si un jeton a existé.

    Les deux cas passent par le même rendu : un ULID bien formé qui n'a jamais
    désigné de réponse, et une chaîne que `SurveyToken::try_new` refuse. Deux pages
    distinctes diraient à qui essaie des jetons lesquels ont eu la bonne forme.

    La page ne dit pas non plus « ce lien a expiré » : ce serait affirmer qu'il a
    existé.
    """
    page.goto(_lien(jeton, "oui"), wait_until="load")

    expect(page.locator(".presence-titre")).to_have_text("Ce lien ne mène à rien")
    expect(page.locator(".presence-card--inconnu")).to_be_visible()
    # Rien du contexte ne doit filtrer : ni équipe, ni journée, ni compétition.
    expect(page.locator(".presence-recap")).to_have_count(0)
    expect(page.locator("body")).not_to_contain_text("Journée")
    expect(page.locator("body")).not_to_contain_text("expiré")

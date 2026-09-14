"""L'encart du coach connecté, dans un navigateur.

Le second chemin de réponse : le coach ne reçoit pas d'e-mail — filtré,
supprimé, ou jamais parti faute d'adresse — et répond depuis la page de détail
qu'il consultait déjà.

# Le coach connecté **est** DevCoach, et c'est ce qui rend ces tests possibles

`build_full_competition` attribue ses équipes aux coachs de l'espace pris dans
l'ordre de leur nom, et `DevCoach` est le premier. La première équipe de la
fixture lui appartient donc, et c'est celle que l'encart lui montre.

Le coach « simple » (`E2E Coach 01`) est le **second** de cette liste : sur une
compétition à quatre équipes il en possède une, et verrait l'encart. D'où la
seconde fixture, à une seule équipe, pour le scénario du coach sans équipe.

# Chaque scénario ferme sa campagne, et ce n'est pas une politesse

**L'encart agrège toutes les campagnes ouvertes de la saison.** Un scénario qui
laisserait la sienne ouverte se verrait dans tous les suivants : celui qui attend
deux cartes en compterait trois, et l'échec accuserait le code au lieu du test.

Les huit scénarios ne sont donc pas indépendants, et c'est **structurel** — pas
un accident d'écriture. Chacun clôt sa campagne en sortant, et ceux qui
dépendent d'un état posé par le précédent le disent dans leur docstring.

L'alternative — une compétition par scénario — aurait coûté huit constructions de
fixture pour éviter huit appels à `close`.

# « La page ne bouge pas » ne se regarde pas, il se mesure

Vérifier qu'un écran est le même après un clic ne prouve rien : un rechargement
complet redonnerait exactement le même écran. Le scénario 2 pose donc un témoin
dans `window` avant de cliquer et vérifie qu'il survit. Un `outerHTML` sur le seul
encart le laisse intact ; un rechargement l'efface.

# Le piège habituel

`cliquer_quand_cable` sur tous les boutons de l'encart : il est injecté par htmx,
donc la fenêtre où il est peint, visible et **inerte** s'y présente. Aucun
`sleep` — une durée fixe n'a pas de marge sur une machine chargée, et c'est là
que la suite échoue.

Prérequis : serveur lancé en `make dev-e2e`, base au gabarit (`make e2e_db`).
"""

from contextlib import contextmanager

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import (
    BASE_URL,
    build_and_submit_team_http,
    build_full_competition,
)
from db_helpers import attendre_que, query_db
from htmx_helpers import cliquer_quand_cable

ENTETES = {"HX-Request": "true"}
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}
LOINTAIN = "2099-12-31"
PROCHE = "2099-01-31"


@pytest.fixture(scope="module")
def competition(browser, space_id):
    """Quatre équipes, six journées. La première équipe est celle de DevCoach."""
    return build_full_competition(browser, space_id, 4, 6)


@pytest.fixture(scope="module")
def competition_solo(browser, space_id):
    """**Une seule** équipe, celle de DevCoach.

    Le coach simple n'y est donc pas engagé — c'est la seule façon d'observer
    l'absence totale d'encart, puisque sur la compétition à quatre équipes il en
    possède une.
    """
    return build_full_competition(browser, space_id, 1, 2)


# ── Les adresses ──────────────────────────────────────────────────────────────


def _detail(space_id, comp):
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{comp['competition_id']}/{comp['season_id']}"
    )


def _admin(space_id, comp, nom):
    return f"{_detail(space_id, comp)}/admin/presences/{nom}"


def _poster_admin(space_id, comp, nom, corps):
    """Les actions de l'organisateur sont appelées en HTTP : ce test éprouve
    l'encart du coach, pas l'onglet d'administration, qui a ses propres tests."""
    return requests.post(
        _admin(space_id, comp, nom), json=corps, headers=ENTETES, timeout=30
    )


def _campagne_ouverte(round_id):
    return query_db(
        "SELECT count(*) FROM competition_presence_surveys "
        f"WHERE round_id = '{round_id}' AND close_le IS NULL"
    ) == ["1"]


def _lancer(space_id, comp, round_id, deadline=LOINTAIN):
    """**Vérifie l'effet, pas le code HTTP.**

    Un refus métier rend lui aussi `200`, avec un fragment explicatif : relancer
    une campagne déjà existante — même close — répond « une campagne existe déjà »
    sans rien ouvrir, et R2 veut qu'on la **rouvre** au lieu de la relancer.

    Un `assert` sur le seul code de statut a laissé passer ce cas à l'écriture de
    ce fichier, et le test qui suivait accusait l'encart de ne pas s'afficher.
    """
    reponse = _poster_admin(
        space_id,
        comp,
        "launch",
        {"round_id": round_id, "deadline": deadline, "auto_remind": False},
    )
    assert reponse.status_code == 200, reponse.text[:400]
    assert _campagne_ouverte(round_id), (
        "aucune campagne ouverte après le lancement — refus métier rendu en 200 ?"
    )


def _clore(space_id, comp, round_id):
    reponse = _poster_admin(space_id, comp, "close", {"round_id": round_id})
    assert reponse.status_code == 200, reponse.text[:400]
    assert not _campagne_ouverte(round_id), "la campagne est restée ouverte"


def _rouvrir(space_id, comp, round_id, deadline=LOINTAIN):
    reponse = _poster_admin(
        space_id, comp, "reopen", {"round_id": round_id, "deadline": deadline}
    )
    assert reponse.status_code == 200, reponse.text[:400]
    assert _campagne_ouverte(round_id), "la campagne est restée close"


@contextmanager
def campagne(space_id, comp, round_id, deadline=LOINTAIN):
    """Ouvre une campagne, et la referme **quoi qu'il arrive**.

    La clôture était écrite en dernière ligne du corps du test. Elle ne
    s'exécutait donc pas quand le test échouait — c'est-à-dire exactement quand
    elle compte : en CI, l'échec de J4 a laissé sa campagne ouverte, et J5 a
    compté trois cartes au lieu de deux. Le second échec n'était que l'ombre du
    premier, et il a fallu les deux messages pour le voir.

    Le `finally` referme ce trou. Un `try` sans `finally` documente une
    intention ; celui-ci l'exécute.
    """
    _lancer(space_id, comp, round_id, deadline)
    try:
        yield round_id
    finally:
        if _campagne_ouverte(round_id):
            _clore(space_id, comp, round_id)


def _campagnes_ouvertes(comp):
    return query_db(
        "SELECT round_id FROM competition_presence_surveys s "
        "JOIN competition_match_days d ON d.id = s.round_id "
        f"WHERE d.season_id = '{comp['season_id']}' AND s.close_le IS NULL"
    )


def _mon_equipe_appariee(comp, round_id):
    """L'équipe de DevCoach qui **figure dans une rencontre** de cette journée.

    Sans cette lecture, le test cliquait sur la première ligne venue. Or R9 exempte
    une équipe quand le compte est impair — cinq ici, DevCoach en ayant deux — et
    se décommander depuis l'exemptée ne défait aucune rencontre : pas d'avis, et un
    test rouge une fois sur cinq. Il passait en local par chance de tirage.
    """
    miennes = query_db(
        "SELECT t.team_id FROM team_proj t JOIN auth__users u ON u.id = t.coach_id "
        f"WHERE t.season_id = '{comp['season_id']}' AND u.coach_name = 'DevCoach'"
    )
    appariees = query_db(
        "SELECT home_team_id, away_team_id FROM competition_match_day_pairings "
        f"WHERE match_day_id = '{round_id}'"
    )
    engagees = {c for ligne in appariees for c in ligne.split("|")}
    candidates = [e for e in miennes if e in engagees]
    assert candidates, "aucune équipe de DevCoach n'est appariée sur cette journée"
    return candidates[0]


def _ouvrir_le_detail(page: Page, space_id, comp):
    """La page de détail, l'encart chargé.

    L'encart arrive par `hx-trigger="load"` : attendre sa racine, c'est attendre
    que la requête soit revenue **et** que htmx ait posé le fragment.
    """
    page.goto(_detail(space_id, comp), wait_until="load")
    expect(page.locator(".tabs")).to_be_visible(timeout=15000)


def _presence_en_base(round_id, team_id):
    lignes = query_db(
        "SELECT a.presence FROM competition_presence_answers a "
        "JOIN competition_presence_surveys s ON s.id = a.survey_id "
        f"WHERE s.round_id = '{round_id}' AND a.team_id = '{team_id}'"
    )
    return lignes[0] if lignes else None


# ══ J1 — une équipe : voir, répondre, changer d'avis ═══════════════════════════


def test_j1_l_encart_apparait_avec_ses_deux_boutons(page: Page, space_id, competition):
    """La campagne ouverte, et le coach qui possède une équipe la voit."""
    round_id = competition["round_ids"][0]
    _lancer(space_id, competition, round_id)

    _ouvrir_le_detail(page, space_id, competition)

    expect(page.locator(".presence-call")).to_be_visible(timeout=15000)
    expect(page.locator(".presence-call .pc-card")).to_have_count(1)
    expect(page.locator(".presence-call .pc-btn--yes")).to_be_visible()
    expect(page.locator(".presence-call .pc-btn--no")).to_be_visible()


def test_j1_repondre_ne_recharge_pas_la_page(page: Page, space_id, competition):
    """**Le test qui mesure au lieu de regarder.**

    « La page ne bouge pas » ne se prouve pas en constatant que l'écran est le
    même : un rechargement complet redonnerait le même écran. Un témoin posé dans
    `window` avant le clic tranche — un `outerHTML` sur le seul encart le laisse
    intact, un rechargement l'efface.

    Dépend du scénario précédent, qui a lancé la campagne de J1.
    """
    round_id = competition["round_ids"][0]
    _ouvrir_le_detail(page, space_id, competition)
    expect(page.locator(".presence-call .pc-btn--yes")).to_be_visible(timeout=15000)

    page.evaluate("window.__temoin_encart = 'intact'")
    cliquer_quand_cable(page, ".presence-call .pc-btn--yes")

    expect(page.locator(".presence-call .pc-card--answered")).to_be_visible(
        timeout=15000
    )
    assert page.evaluate("window.__temoin_encart") == "intact", (
        "la page a été rechargée : seul l'encart devait être remplacé"
    )
    # Et l'onglet ouvert l'est resté.
    expect(page.locator(".tabs")).to_be_visible()
    assert _presence_en_base(round_id, competition["team_ids"][0]) == "presente"


def test_j1_changer_d_avis_bascule_la_reponse(page: Page, space_id, competition):
    """R12 dans l'autre sens : on peut revenir sur sa réponse tant que le sondage
    est ouvert.

    Dépend des deux scénarios précédents ; la réponse est « présente » en entrant.
    """
    round_id = competition["round_ids"][0]
    _ouvrir_le_detail(page, space_id, competition)
    expect(page.locator(".presence-call .pc-card--answered")).to_be_visible(
        timeout=15000
    )

    cliquer_quand_cable(page, ".presence-call .pc-btn--switch")

    expect(page.locator(".presence-call .pc-card--declined")).to_be_visible(
        timeout=15000
    )
    assert _presence_en_base(round_id, competition["team_ids"][0]) == "absente"

    # La campagne de J1 est close en sortant : sans quoi elle s'ajouterait à
    # toutes les cartes attendues par les scénarios suivants.
    _clore(space_id, competition, round_id)


# ══ J2 — deux équipes pour le même coach ══════════════════════════════════════


@pytest.fixture(scope="module")
def seconde_equipe(browser, space_id, competition):
    """Une seconde équipe **pour DevCoach** dans la même saison.

    La fixture n'en donne qu'une par coach ; R1 portant sur l'équipe et non sur
    le coach, il faut en engager deux pour éprouver la règle.
    """
    coach = query_db(
        "SELECT u.id FROM auth__users u WHERE u.coach_name = 'DevCoach'"
    )[0]
    page = browser.new_page()
    try:
        team_id = build_and_submit_team_http(
            page,
            space_id,
            competition["competition_id"],
            competition["season_id"],
            coach,
            roster_index=2,
            team_name="La Seconde de DevCoach",
        )
    finally:
        page.close()
    attendre_que(
        lambda: query_db(
            f"SELECT count(*) FROM team_proj WHERE team_id = '{team_id}' "
            "AND status = 'Enrolled'"
        )
        == ["1"],
        quoi="la seconde équipe inscrite",
    )
    return team_id


def test_j2_deux_equipes_donnent_deux_lignes_repondables_separement(
    page: Page, space_id, competition, seconde_equipe
):
    """R1 — la réponse porte sur l'équipe, jamais sur le coach.

    Il peut venir avec l'une et pas l'autre : chaque ligne porte donc sa propre
    paire de boutons, et répondre pour l'une laisse l'autre en attente.
    """
    round_id = competition["round_ids"][1]
    with campagne(space_id, competition, round_id):
        _ouvrir_le_detail(page, space_id, competition)
        expect(page.locator(".presence-call .pc-team-row")).to_have_count(
            2, timeout=15000
        )

        # Répondre pour la première ligne seulement.
        cliquer_quand_cable(page, ".presence-call .pc-team-row .pc-btn--yes")

        expect(page.locator(".presence-call .pc-team-state--yes")).to_have_count(
            1, timeout=15000
        )
        # L'autre équipe attend toujours sa réponse : elle garde ses deux boutons.
        expect(page.locator(".presence-call .pc-btn--yes")).to_have_count(1)


# ══ J3 — la campagne close entre l'affichage et le clic ═══════════════════════


def test_j3_une_campagne_close_apres_l_affichage_rend_une_carte_et_non_le_vide(
    page: Page, space_id, competition, seconde_equipe
):
    """**La course que personne ne rencontre en développement.**

    Son mode d'échec ressemble à un succès : sans carte explicative, le fragment
    réaffiché serait celui d'un encart devenu vide, il disparaîtrait sous le
    curseur, et le coach lirait ce vide comme une réponse enregistrée.
    """
    round_id = competition["round_ids"][2]
    with campagne(space_id, competition, round_id):
        _ouvrir_le_detail(page, space_id, competition)
        expect(page.locator(".presence-call .pc-btn--yes").first).to_be_visible(
            timeout=15000
        )
        # L'organisateur clôt pendant que le coach lit sa page.
        _clore(space_id, competition, round_id)

        cliquer_quand_cable(page, ".presence-call .pc-btn--yes")

        expect(page.locator(".presence-call .pc-card--clos")).to_be_visible(
            timeout=15000
        )
        expect(page.locator(".presence-call")).to_contain_text(
            "Ta réponse n'a pas pu être enregistrée"
        )
        # Ce que le vide aurait donné, et qu'il ne faut surtout pas voir.
        expect(page.locator(".presence-call")).to_be_visible()


# ══ J4 — le désistement après tirage ══════════════════════════════════════════


def test_j4_se_decommander_apres_le_tirage_le_dit_sans_nommer_d_adversaire(
    page: Page, space_id, competition, seconde_equipe
):
    """R30 — la mention, et **aucun adversaire annoncé**.

    La réparation est une proposition que l'organisateur valide : en annoncer un
    produirait deux coachs qui se croient appariés sur un match qui n'existe pas.
    Et pas de silence non plus — le coach vient de défaire un match que son
    adversaire avait noté.

    L'ordre des étapes est contraint : `rouvrir` refuse une journée figée, et il
    faut la campagne **ouverte** pour que le coach puisse se décommander.
    """
    round_id = competition["round_ids"][3]
    with campagne(space_id, competition, round_id):
        _jouer_le_desistement(page, space_id, competition, round_id)


def _jouer_le_desistement(page: Page, space_id, competition, round_id):
    for equipe in query_db(
        f"SELECT team_id FROM team_proj WHERE season_id = '{competition['season_id']}' "
        "AND status = 'Enrolled'"
    ):
        _poster_admin(
            space_id,
            competition,
            "answer",
            {"round_id": round_id, "team_id": equipe, "presence": "presente"},
        )
    _clore(space_id, competition, round_id)

    # Tirer et valider par l'onglet de l'organisateur.
    page.goto(f"{_detail(space_id, competition)}/admin/presences", wait_until="load")
    page.evaluate(f"document.body.dataset.activeRoundId = '{round_id}'")
    page.evaluate(
        "htmx.trigger(document.body, 'roundSelected', "
        f"{{round_id: '{round_id}'}})"
    )
    cliquer_quand_cable(page, ".panel-footer .btn-primary-sm")
    expect(page.locator(".panel-title")).to_contain_text("Tirage proposé", timeout=15000)
    cliquer_quand_cable(page, ".panel-footer .btn-primary-sm")
    attendre_que(
        lambda: len(
            query_db(
                "SELECT id FROM competition_match_day_pairings "
                f"WHERE match_day_id = '{round_id}'"
            )
        )
        > 0,
        quoi="les rencontres écrites au calendrier",
    )

    _rouvrir(space_id, competition, round_id)

    # **L'équipe qui figure dans une rencontre**, et non la première ligne venue.
    # Cinq équipes présentes donnent deux rencontres et une exemptée ; se
    # décommander depuis l'exemptée ne défait rien, et c'est ce que la CI a tiré.
    mienne = _mon_equipe_appariee(competition, round_id)

    _ouvrir_le_detail(page, space_id, competition)
    expect(page.locator(".presence-call .pc-team-state--yes").first).to_be_visible(
        timeout=15000
    )
    cliquer_quand_cable(
        page, f'.presence-call .pc-btn--switch[hx-vals*="{mienne}"]'
    )

    expect(page.locator(".presence-call .pc-avis")).to_be_visible(timeout=15000)
    expect(page.locator(".presence-call .pc-avis")).to_contain_text(
        "Ta rencontre était déjà tirée"
    )
    # Aucun adversaire n'est nommé : l'avis parle de la rencontre, pas d'une équipe.
    avis = page.locator(".presence-call .pc-avis").inner_text()
    for nom in query_db(
        f"SELECT team_name FROM team_proj WHERE season_id = '{competition['season_id']}' "
        "AND status = 'Enrolled'"
    ):
        assert nom not in avis, f"l'avis nomme une équipe : {nom}"


# ══ J5 et J6 — deux campagnes ouvertes en même temps ══════════════════════════


def test_j5_deux_campagnes_ouvertes_donnent_deux_cartes_la_plus_pressee_d_abord(
    page: Page, space_id, competition, seconde_equipe
):
    """R2 porte sur la **journée**, pas sur la compétition.

    Deux journées peuvent être sondées en même temps, et taire l'une parce que
    l'autre existe ferait manquer une échéance. La plus proche échéance passe en
    premier : c'est celle qui presse.
    """
    tardive, pressee = competition["round_ids"][4], competition["round_ids"][5]
    # **La précondition qui nomme la cause.** Ce test compte les cartes ; une
    # campagne laissée ouverte par un scénario précédent en ajoute une, et le
    # « 3 au lieu de 2 » accuserait alors l'encart. Vu en CI, où l'échec de J4 a
    # produit exactement ça et masqué son propre motif.
    restantes = _campagnes_ouvertes(competition)
    assert not restantes, f"campagnes laissées ouvertes par un scénario précédent : {restantes}"

    with campagne(space_id, competition, tardive, deadline=LOINTAIN), campagne(
        space_id, competition, pressee, deadline=PROCHE
    ):
        _ouvrir_le_detail(page, space_id, competition)

        expect(page.locator(".presence-call .pc-card")).to_have_count(2, timeout=15000)
        premier = page.locator(".presence-call .pc-card").first
        expect(premier).to_contain_text(PROCHE)


# ══ Le coach qui n'est pas concerné ═══════════════════════════════════════════


def test_le_proprietaire_de_l_equipe_voit_l_encart(page: Page, space_id, competition_solo):
    """Le premier volet de la paire, et sans lui l'absence ne prouverait rien.

    Une campagne absente, une route cassée ou un gabarit muet donneraient le même
    « aucun encart » au coach suivant. Celui-ci ouvre la campagne et constate que
    le propriétaire de l'unique équipe la voit — **c'est la même campagne** que le
    scénario suivant regarde, et c'est ce qui rend sa comparaison valide.

    Il ne clôt pas en sortant : le scénario suivant en a besoin ouverte.
    """
    round_id = competition_solo["round_ids"][0]
    _lancer(space_id, competition_solo, round_id)

    _ouvrir_le_detail(page, space_id, competition_solo)

    expect(page.locator(".presence-call")).to_be_visible(timeout=15000)
    expect(page.locator(".presence-call .pc-btn--yes")).to_be_visible()


def test_un_coach_sans_equipe_engagee_ne_voit_aucun_encart(
    browser, space_id, competition_solo
):
    """**Une absence, et c'est ce qu'on oublie de tester.**

    Pas seulement qu'aucun texte n'apparaît : qu'aucun élément n'existe. C'est le
    `hx-swap="outerHTML"` qui le garantit — le conteneur d'appel disparaît avec sa
    réponse vide, sans laisser un `<div>` de zéro hauteur et sa marge.

    Contexte neuf : `bypass_auth` ne remplace jamais une session déjà connectée,
    donc l'autre identité suppose un navigateur sans cookie.

    Dépend du scénario précédent, qui a ouvert la campagne et l'a laissée ouverte.
    """
    round_id = competition_solo["round_ids"][0]
    assert _campagne_ouverte(round_id), "le scénario précédent doit l'avoir ouverte"

    contexte = browser.new_context(extra_http_headers=ENTETE_MEMBRE_SIMPLE)
    try:
        autre = contexte.new_page()
        autre.goto(_detail(space_id, competition_solo), wait_until="load")
        expect(autre.locator(".tabs")).to_be_visible(timeout=15000)

        expect(autre.locator(".presence-call")).to_have_count(0)
    finally:
        contexte.close()

    _clore(space_id, competition_solo, round_id)

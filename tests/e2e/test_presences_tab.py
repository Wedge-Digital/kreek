"""L'onglet Présences : la coquille s'ouvre, la barre latérale liste les journées.

**Le premier test e2e de l'épic E16.** Les treize cartes précédentes n'avaient
posé aucune route : la suite ne vérifiait jusqu'ici que l'absence de régression
sur le Calendrier.

Ce que ce test couvre et qu'aucun test unitaire ne voit : l'onglet est atteignable
par le chemin htmx du changement d'onglet, le fragment de barre latérale se charge
sur son `hx-trigger="load"`, et il liste les journées de la saison.

Ce qu'il ne couvre pas encore : le panneau, qui rend une invite jusqu'à la carte
520.

Prérequis : serveur lancé en `make dev-e2e`, base au gabarit (`make e2e_db`).
"""

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from htmx_helpers import cliquer_quand_cable

# Coach seedé sans droit d'administration (`seed_e2e.rs::SIMPLE_COACH_NAME`).
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}


@pytest.fixture(scope="module")
def competition(browser, space_id):
    """Une compétition finalisée, avec son calendrier — mais **sans appariements**.

    `with_pairings` reste à son défaut : la carte 508 a montré qu'un test qui
    n'en a pas besoin ne doit pas en créer, le tirage étant aléatoire (R17) et
    l'orientation des rapports en découlant.
    """
    # Quatre équipes et trois journées : le minimum pour que la barre latérale ait
    # de quoi lister sans allonger la construction de la fixture.
    return build_full_competition(browser, space_id, 4, 3)


def _url_onglet(space_id, competition):
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{competition['competition_id']}/{competition['season_id']}/admin/presences"
    )


# ── L'onglet s'ouvre, par les deux chemins ────────────────────────────────────


def test_l_onglet_presences_s_ouvre_en_chargement_complet(page: Page, space_id, competition):
    page.goto(_url_onglet(space_id, competition), wait_until="load")

    expect(page.locator(".competition-admin-presences").first).to_be_visible(timeout=10000)
    expect(page.locator("#presences-sidebar")).to_be_visible()
    expect(page.locator("#presences-panel")).to_be_visible()


def test_l_onglet_est_atteignable_depuis_le_calendrier_par_htmx(page: Page, space_id, competition):
    """Le chemin htmx, et non un rechargement : c'est celui que l'organisateur
    emprunte, et c'est aussi celui qui contournerait la garde si un fragment ne
    l'appelait pas (carte 416)."""
    schedule = _url_onglet(space_id, competition).replace("/presences", "/schedule")
    page.goto(schedule, wait_until="load")
    expect(page.locator(".competition-admin-schedule").first).to_be_visible(timeout=10000)

    cliquer_quand_cable(page, ".admin-tab:has-text('Présences')")

    expect(page.locator("#presences-sidebar")).to_be_visible(timeout=10000)


# ── La barre latérale liste les journées ──────────────────────────────────────


def test_la_barre_laterale_liste_les_journees_de_la_saison(page: Page, space_id, competition):
    page.goto(_url_onglet(space_id, competition), wait_until="load")

    journees = page.locator(".presences-round")
    expect(journees.first).to_be_visible(timeout=10000)
    assert journees.count() > 0, "la saison porte des journées, elles doivent être listées"
    # Aucune campagne n'est ouverte : toutes les journées jouables le disent.
    expect(page.locator(".presences-round--aucun").first).to_be_visible()
    expect(page.locator(".presences-round--aucun").first).to_contain_text("Aucun sondage")


def test_le_panneau_invite_a_choisir_une_journee(page: Page, space_id, competition):
    page.goto(_url_onglet(space_id, competition), wait_until="load")

    expect(page.locator(".presences-panel--invite")).to_be_visible(timeout=10000)
    expect(page.locator(".presences-panel-invite")).to_contain_text("Choisissez une journée")


def test_choisir_une_journee_montre_l_etat_aucun_sondage(page: Page, space_id, competition):
    """Le seul des cinq états atteignable sans les actions de la carte 521.

    Il porte trois choses qu'aucun test unitaire ne voit : le clic de la barre
    latérale émet bien `roundSelected`, le panneau le reçoit avec son `round_id`,
    et le compte des destinataires vient du roster — pas d'un nombre en dur.
    """
    page.goto(_url_onglet(space_id, competition), wait_until="load")
    expect(page.locator(".presences-round").first).to_be_visible(timeout=10000)

    cliquer_quand_cable(page, ".presences-round:not(.presences-round--repos)")

    expect(page.locator(".panel-title")).to_contain_text("Sonder les présences", timeout=10000)
    # La fixture engage quatre équipes : le panneau doit le dire, et non l'inventer.
    expect(page.locator(".recipients-box")).to_contain_text("4")
    expect(page.locator(".launch-steps .launch-step")).to_have_count(3)


# ── La garde, sur la page comme sur les fragments ─────────────────────────────


@pytest.mark.parametrize("suffixe", ["", "/rounds", "/panel"])
def test_un_membre_simple_est_refuse_sur_l_onglet_et_ses_fragments(
    space_id, competition, suffixe
):
    """Les trois routes, fragments compris.

    `space_scope` garantit qu'une ressource appartient à l'espace de l'URL — pas
    que l'appelant en est administrateur. Sans `require_admin_access` sur chaque
    handler, un membre simple lirait la barre latérale de l'administration.
    """
    reponse = requests.get(
        _url_onglet(space_id, competition) + suffixe,
        headers={**ENTETE_MEMBRE_SIMPLE, "HX-Request": "true"},
        timeout=15,
    )

    assert reponse.status_code == 403, (
        f"la route {suffixe or '/presences'} a répondu {reponse.status_code} "
        "à un membre sans droit d'administration"
    )


# ── Le protocole des actions (carte 521) ──────────────────────────────────────
#
# Deux cas seulement, et c'est délibéré : le parcours complet — lancement, tirage,
# validation — est le sujet de la carte 522. Ceux-ci éprouvent le **protocole**, qui
# vaut pour les dix actions, et qu'aucun test unitaire ne voit :
#
#   succès  → corps vide + `HX-Trigger: presenceChanged`, les widgets se rechargent
#   refus   → le panneau porteur du motif, `HX-Retarget` + `HX-Reswap`, sans trigger
#
# Les vérifier une fois suffit : le reste des actions passe par les mêmes trois
# helpers.


def _url_action(space_id, competition, action):
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{competition['competition_id']}/{competition['season_id']}/admin/presences/{action}"
    )


def _premiere_journee(space_id, competition):
    """La première journée jouable de la saison, lue en base.

    En base et non à l'écran : le test du protocole n'a pas à dépendre du rendu de
    la barre latérale, que d'autres tests couvrent déjà.
    """
    from db_helpers import query_db

    lignes = query_db(
        "SELECT id FROM competition_match_days "
        f"WHERE season_id = '{competition['season_id']}' AND day_type <> 'rest' "
        "ORDER BY position LIMIT 1"
    )
    assert lignes, "la fixture doit avoir programmé des journées"
    return lignes[0].strip()


@pytest.fixture(scope="module")
def journee(space_id, competition):
    return _premiere_journee(space_id, competition)


def test_un_succes_rend_un_corps_vide_et_declenche_presence_changed(
    space_id, competition, journee
):
    """Le succès ne remplace rien : les deux widgets se rechargent d'eux-mêmes sur
    l'événement. Rendre le fragment *et* déclencher le rechargement peindrait le
    panneau deux fois."""
    reponse = requests.post(
        _url_action(space_id, competition, "launch"),
        json={"round_id": journee, "deadline": "2099-12-31", "auto_remind": True},
        headers={"HX-Request": "true"},
        timeout=20,
    )

    assert reponse.status_code == 200, reponse.text[:400]
    assert reponse.headers.get("HX-Trigger") == "presenceChanged"
    assert reponse.text == "", "le succès ne rend aucun corps"
    assert "HX-Retarget" not in reponse.headers, "rien à rediriger sur un succès"


def test_le_panneau_annonce_ce_que_l_expedition_a_produit(
    space_id, competition, journee
):
    """Carte 544 — les compteurs d'expédition atteignent enfin un écran.

    Ils étaient calculés par le use case, journalisés, puis perdus : un
    organisateur dont le serveur de messagerie tombait voyait son panneau se
    recharger normalement et croyait ses e-mails partis.

    Le panneau les **relit du journal**, ce qui les rend durables — cette requête
    est un `GET` neuf, sans rapport avec le POST qui a lancé la campagne, et la
    ligne est pourtant là.

    En e2e le fournisseur d'e-mail est `console` : les envois réussissent, le
    journal les atteste, et le compte est donc non nul. Un « 0 coach joint » ici
    signifierait que l'expédition n'a pas eu lieu du tout.

    Dépend du test qui lance la campagne, plus haut dans ce fichier.
    """
    reponse = requests.get(
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{competition['competition_id']}/{competition['season_id']}"
        f"/admin/presences/panel?round_id={journee}",
        headers={"HX-Request": "true"},
        timeout=20,
    )

    assert reponse.status_code == 200, reponse.text[:400]
    assert "Envoi initial" in reponse.text, "la ligne d'expédition doit être rendue"
    # Le compte est celui du journal, et il porte sur des **coachs**.
    assert "joint" in reponse.text and "sur" in reponse.text
    # Le libellé accorde : « 4 coachs joints », « 1 coach joint ». Un zéro ici
    # signifierait que l'expédition n'a pas eu lieu du tout.
    assert "0 coach" not in reponse.text, (
        "aucun envoi attesté : l'expédition n'a pas eu lieu"
    )


def test_un_refus_metier_rend_le_panneau_avec_son_motif(space_id, competition, journee):
    """R2 — une seconde campagne sur la même journée est refusée, et le motif
    s'affiche **dans le panneau**, jamais dans une boîte du navigateur.

    Dépend du test précédent, qui a lancé la campagne : le refus n'existe qu'une
    fois la première ouverte. Les tests d'un fichier s'exécutent dans l'ordre, et
    c'est déjà ce dont dépend `test_dismissals_banner…` ailleurs dans la suite.
    """
    reponse = requests.post(
        _url_action(space_id, competition, "launch"),
        json={"round_id": journee, "deadline": "2099-12-31", "auto_remind": False},
        headers={"HX-Request": "true"},
        timeout=20,
    )

    assert reponse.status_code == 200, "un refus métier n'est pas une erreur HTTP"
    assert reponse.headers.get("HX-Retarget") == "#presences-panel"
    assert reponse.headers.get("HX-Reswap") == "innerHTML"
    assert (
        "HX-Trigger" not in reponse.headers
    ), "un refus ne déclenche rien : le panneau se rechargerait par-dessus le motif"
    assert "panel-refus" in reponse.text, "le motif est dans le panneau"
    assert "existe déjà" in reponse.text

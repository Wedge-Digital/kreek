"""Tests E2E — une compétence gratuite ne renchérit pas la suivante (carte 482).

`next_improvement_level` comptait toutes les compétences acquises, customisations
et Haines comprises, alors que celles-ci n'ont rien coûté. Le niveau est **baké
dans l'URL du sélecteur** : il ne tarifait donc pas seulement la compétence
achetée, mais **toutes les lignes du picker**. Un joueur customisé se voyait
proposer l'écran entier au tarif du niveau supérieur.

# L'assertion est différentielle, et c'est ce qui la rend solide

Deux joueurs de la même équipe, tous deux crédités en SPP par le même match,
tous deux sans achat : l'un a reçu une compétence d'un commissaire, l'autre non.
**Leur tarif affiché doit être identique.**

Comparer plutôt que d'affirmer « 6 SPP » met le test à l'abri du barème : si la
matrice de coût change un jour, il continue de vérifier ce qu'il prétend — que
le cadeau ne déplace rien — au lieu d'échouer sur un chiffre qui n'était pas son
sujet. La valeur est tout de même vérifiée, mais en second.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) et base seedée.
"""

import re
import time

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from htmx_helpers import cliquer_quand_cable
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    post_step5,
    publish,
    record_action_api,
)

HX = {"HX-Request": "true"}
#: Un touchdown vaut 4 SPP au barème normal — cinq en donnent vingt, de quoi
#: acheter au moins deux compétences sans jamais frôler le refus pour budget.
TOUCHDOWNS = 5


def _attendre(predicat, quoi: str, timeout_s: int = 20) -> None:
    """Le crédit des SPP suit la publication par app event : il est asynchrone."""
    limite = time.time() + timeout_s
    while time.time() < limite:
        if predicat():
            return
        time.sleep(0.2)
    raise AssertionError(f"{quoi} : jamais satisfait après {timeout_s}s")


# ── Helpers de customisation (mêmes routes que test_player_customisation) ─────


def _url(space_id: str, player_id: str, suffixe: str) -> str:
    return f"{BASE_URL}/app/{space_id}/players/{player_id}/{suffixe}"


def _panneau(space_id: str, player_id: str) -> requests.Response:
    """Le `GET` crée le panier s'il n'existe pas — c'est lui qui ouvre le mode."""
    return requests.get(_url(space_id, player_id, "widgets/customisation"), timeout=10)


def _version(html: str) -> int:
    m = re.search(r'name="expected_version" value="(\d+)"', html)
    assert m, "le panneau ne porte pas de version — a-t-il seulement été rendu ?"
    return int(m.group(1))


def _offrir_une_competence(space_id: str, player_id: str) -> str:
    """Ajoute une compétence au panier de customisation, puis valide le lot."""
    html = _panneau(space_id, player_id).text
    m = re.search(r'"skill_id": "([A-Z_]+)"', html)
    assert m, "aucune compétence ajoutable dans le panneau de customisation"
    competence = m.group(1)

    ajout = requests.post(
        _url(space_id, player_id, "customisation/skills/add"),
        data={"skill_id": competence, "expected_version": _version(html)},
        headers=HX,
        timeout=10,
    )
    assert ajout.status_code == 200, f"ajout au panier : {ajout.status_code}"

    version = _version(_panneau(space_id, player_id).text)
    validation = requests.post(
        _url(space_id, player_id, "customisation/validate"),
        data={"expected_version": version},
        headers=HX,
        timeout=10,
    )
    assert validation.status_code == 200, f"validation : {validation.status_code}"
    return competence


# ── Lectures d'écran ──────────────────────────────────────────────────────────


def _ouvrir_le_picker(page: Page, space_id: str, player_id: str) -> None:
    """Le panneau droit charge le journal par défaut ; le sélecteur de
    compétences n'apparaît qu'après la bascule.

    `cliquer_quand_cable` : le bouton est monté par un `hx-trigger="load"` et
    reste inerte le temps qu'htmx le câble. Un clic tombé dans cette fenêtre se
    perd sans émettre de requête.
    """
    page.goto(f"{BASE_URL}/app/{space_id}/players/{player_id}/detail", wait_until="load")
    cliquer_quand_cable(page, ".btn-toggle-spp")
    page.wait_for_selector(".skill-list-table", timeout=10000)


def _tarif_affiche(page: Page) -> str:
    """Le prix qu'un coach lit en haut du sélecteur — « 6 SPP ».

    C'est `pricing.chosen.primary`, dérivé du niveau baké dans l'URL du picker :
    l'assertion porte donc sur ce que le niveau produit, et non sur le niveau
    lui-même, que les tests unitaires couvrent déjà.
    """
    return page.locator(".cost-chosen").first.inner_text().strip()


def _niveau_du_picker(page: Page) -> str:
    conteneur = page.locator("[hx-get*='skill-picker'], [hx-get*='level=']").first
    return conteneur.get_attribute("hx-get") or ""


# ── Montage ───────────────────────────────────────────────────────────────────


@pytest.fixture(scope="module")
def niveau_ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    equipe = full["team_ids"][0]
    round_id = full["round_ids"][0]

    joueurs = query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{equipe}' "
        "AND membership = 'Active' ORDER BY player_id"
    )
    assert len(joueurs) >= 2, f"deux joueurs attendus dans {equipe}, {len(joueurs)} trouvés"
    customise, temoin = joueurs[0], joueurs[1]

    mr_id = create_draft(space_id, full, round_id, equipe, full["team_ids"][1])
    ensure_pre_match(space_id, mr_id, full, round_id, equipe, full["team_ids"][1])
    ensure_inducements(space_id, mr_id)
    # Les deux joueurs marquent : il leur faut le même crédit, sans quoi une
    # différence de tarif pourrait venir du budget et non du niveau.
    for tour in range(TOUCHDOWNS):
        record_action_api(space_id, mr_id, "home", customise, tour + 1, "TOUCHDOWN")
        record_action_api(space_id, mr_id, "home", temoin, TOUCHDOWNS + tour + 1, "TOUCHDOWN")
    post_step5(space_id, mr_id, home_gain=50, away_gain=40)
    publish(space_id, mr_id)

    def credites() -> bool:
        lignes = query_db(
            f"SELECT spp FROM players_proj WHERE player_id IN ('{customise}','{temoin}')"
        )
        return len(lignes) == 2 and all(int(l) > 0 for l in lignes)

    _attendre(credites, "les deux joueurs crédités en SPP")

    competence = _offrir_une_competence(space_id, customise)

    return {
        "space_id": space_id,
        "customise": customise,
        "temoin": temoin,
        "competence_offerte": competence,
    }


# ── Scénarios ─────────────────────────────────────────────────────────────────


def test_le_tarif_est_le_meme_avec_ou_sans_competence_customisee(page: Page, niveau_ctx):
    """**Le cœur de la carte 482.**

    Avant la correction, le joueur customisé affichait le tarif du niveau 2 —
    8 SPP au lieu de 6 — sur *toutes* les lignes du sélecteur.
    """
    _ouvrir_le_picker(page, niveau_ctx["space_id"], niveau_ctx["temoin"])
    tarif_temoin = _tarif_affiche(page)
    niveau_temoin = _niveau_du_picker(page)

    _ouvrir_le_picker(page, niveau_ctx["space_id"], niveau_ctx["customise"])
    tarif_customise = _tarif_affiche(page)

    assert tarif_customise == tarif_temoin, (
        f"le cadeau a déplacé le tarif : {tarif_customise} contre {tarif_temoin} "
        "pour un joueur qui n'a rien reçu"
    )
    # Vérifié en second, pour que l'échec dise « le barème a changé » et non
    # « le cadeau compte » si la matrice bougeait un jour.
    assert tarif_customise == "6 SPP", f"tarif du niveau 1 attendu, lu « {tarif_customise} »"
    assert "level=1" in niveau_temoin, f"le sélecteur doit tarifer au niveau 1 : {niveau_temoin}"


def test_la_competence_offerte_est_bien_la_et_reste_gratuite(page: Page, niveau_ctx):
    """La contre-épreuve du montage : sans elle, le test précédent passerait
    aussi bien si la customisation n'avait jamais été appliquée."""
    _ouvrir_le_picker(page, niveau_ctx["space_id"], niveau_ctx["customise"])

    acquises = page.locator(".skill-tag--acquired")
    expect(acquises).not_to_have_count(0)
    # Elle n'a rien coûté : la réserve du joueur customisé égale celle du témoin.
    reserve_customise = int(page.locator(".spend-panel-remaining-val").inner_text())
    _ouvrir_le_picker(page, niveau_ctx["space_id"], niveau_ctx["temoin"])
    reserve_temoin = int(page.locator(".spend-panel-remaining-val").inner_text())

    assert reserve_customise == reserve_temoin, (
        "la compétence offerte a été facturée : "
        f"{reserve_customise} SPP contre {reserve_temoin}"
    )


def test_un_achat_reel_fait_monter_le_tarif(page: Page, niveau_ctx):
    """**La contre-épreuve du barème.**

    Sans elle, les deux tests précédents passeraient tout aussi bien sur un
    sélecteur figé au niveau 1 pour tout le monde. Elle s'exécute sur le témoin
    — le joueur customisé reste intact pour les tests qui le lisent.
    """
    _ouvrir_le_picker(page, niveau_ctx["space_id"], niveau_ctx["temoin"])
    assert _tarif_affiche(page) == "6 SPP"

    choisissable = page.locator(".btn-add-skill:visible", has_text="Choisir")
    expect(choisissable.first).to_be_visible(timeout=10000)
    with page.expect_navigation(wait_until="load"):
        choisissable.first.click()

    _ouvrir_le_picker(page, niveau_ctx["space_id"], niveau_ctx["temoin"])
    assert _tarif_affiche(page) == "8 SPP", "un achat réel doit faire monter le tarif"

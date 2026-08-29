"""Le panneau « Tiers & coups de pouce » (carte 424).

**Le seul e2e de l'onglet qui exige un vrai navigateur.** Les trois autres
panneaux passent par `requests` : leur formulaire porte ses valeurs dans le
markup. Celui-ci non — le sélecteur d'inducements n'a **aucun champ caché**, il
garde sa sélection dans son état Alpine et n'émet qu'un événement DOM.

Sans le JS de collecte, le panneau enverrait des tiers aux listes vides sans
qu'aucune erreur ne le signale : chaque enregistrement effacerait tous les coups
de pouce. Aucun test unitaire ne peut le voir, et un test HTTP non plus.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import json
import re

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db

HX = {"HX-Request": "true"}
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}


def _url(space_id: str, ctx: dict) -> str:
    """L'endpoint du panneau — un **fragment**, sans layout."""
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/admin/settings/tiers"
    )


def _onglet(space_id: str, ctx: dict) -> str:
    """La page par laquelle un commissaire arrive.

    Ouvrir le fragment directement au navigateur rend un HTML sans layout —
    donc **sans htmx**, et le script du panneau échoue sur `htmx is not
    defined`. Les tests par `requests` ne s'en aperçoivent pas : ils n'exécutent
    aucun script.
    """
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/admin/settings"
    )


PANNEAU = "#settings-tiers-panel"
"""**L'onglet porte les quatre panneaux.** `[data-role="save"]` et
`.consequence-idle` y sont partagés : un locator non scopé prend le premier du
DOM — celui du panneau « Classement ». Le clic recalculait le classement et le
test lisait *son* message de conséquence, sans qu'aucun POST ne parte vers les
tiers. Tout locator de ce fichier passe par cet identifiant."""


def _attendre_les_selecteurs(page: Page) -> None:
    """Rend la main quand **tous** les sélecteurs ont émis leur état.

    Le bouton, lui, n'est pas un élément htmx : son écouteur est posé dès
    l'exécution du script, il n'y a pas de câblage à attendre. Ce qu'il faut
    attendre, c'est le remplissage de la carte de collecte — et le panneau
    charge ses sélecteurs **en série**, donc voir le premier ne dit rien des
    suivants. Enregistrer trop tôt retomberait sur les valeurs figées et le
    test passerait sans avoir rien prouvé.
    """
    page.wait_for_function(
        """() => {
            const p = document.querySelector('#settings-tiers-panel');
            if (!p) { return false; }
            const slots = p.querySelectorAll('[data-slot="inducement"],[data-slot="star"]');
            return slots.length > 0 &&
                   [...slots].every(s => s.querySelector('.inducement-chip, .roster-chip'));
        }""",
        timeout=20000,
    )


def _coups_de_pouce(season_id: str) -> list[str]:
    lignes = query_db(
        "SELECT coalesce(rules->'tiers'->0->>'inducements', '[]') "
        f"FROM competition_seasons WHERE id = '{season_id}'"
    )
    return re.findall(r'"([^"]+)"', lignes[0] if lignes else "")


@pytest.fixture(scope="module")
def saison_avec_tiers(browser, space_id):
    ctx = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    season_id = ctx["season_id"]
    assert _coups_de_pouce(season_id), "la fixture doit avoir des coups de pouce"
    return {"space_id": space_id, "ctx": ctx, "season_id": season_id}


def test_retirer_un_coup_de_pouce_l_enregistre(page: Page, saison_avec_tiers):
    """**Le test que la carte désigne comme indispensable.**

    Il clique une puce dans le sélecteur, enregistre, et vérifie la base. C'est
    le seul chemin qui exerce la collecte : le sélecteur n'écrit rien dans le
    formulaire, il n'émet qu'un événement.
    """
    space_id, ctx = saison_avec_tiers["space_id"], saison_avec_tiers["ctx"]
    season_id = saison_avec_tiers["season_id"]
    avant = _coups_de_pouce(season_id)
    assert len(avant) >= 2, f"il faut au moins deux coups de pouce : {avant}"

    page.goto(_onglet(space_id, ctx), wait_until="load")
    # Les sélecteurs sont chargés en série par le panneau : on attend le premier.
    _attendre_les_selecteurs(page)

    premiere = page.locator(f"{PANNEAU} .inducement-chip.selected").first
    uid = premiere.get_attribute("data-uid")
    premiere.click()
    expect(
        page.locator(f'{PANNEAU} .inducement-chip[data-uid="{uid}"]')
    ).not_to_have_class(
        re.compile(r"\bselected\b"), timeout=5000
    )

    page.click(f'{PANNEAU} [data-role="save"]')
    expect(page.locator(f"{PANNEAU} .consequence-idle")).to_contain_text(
        "Coups de pouce enregistrés", timeout=10000
    )

    apres = _coups_de_pouce(season_id)
    assert uid not in apres, f"« {uid} » devait être retiré : {apres}"
    assert len(apres) == len(avant) - 1, f"un seul retrait attendu : {avant} → {apres}"


def test_enregistrer_sans_rien_toucher_ne_vide_pas_les_coups_de_pouce(
    page: Page, saison_avec_tiers
):
    """**Le défaut que la carte annonce comme invisible.**

    Sans collecte, le panneau enverrait des listes vides — et l'enregistrement
    réussirait. C'est ce test qui l'attrape : ouvrir, enregistrer, et constater
    que rien n'a bougé.

    Le sélecteur émet son état dès son montage (`x-init="notify()"`), ce qui
    remplit la carte de collecte même sans interaction. Ce test vérifie
    exactement cela.
    """
    space_id, ctx = saison_avec_tiers["space_id"], saison_avec_tiers["ctx"]
    season_id = saison_avec_tiers["season_id"]
    avant = _coups_de_pouce(season_id)
    assert avant, "la fixture doit avoir des coups de pouce"

    page.goto(_onglet(space_id, ctx), wait_until="load")
    _attendre_les_selecteurs(page)
    page.click(f'{PANNEAU} [data-role="save"]')
    expect(page.locator(f"{PANNEAU} .consequence-idle")).to_contain_text(
        "Coups de pouce enregistrés", timeout=10000
    )

    assert _coups_de_pouce(season_id) == avant, "les coups de pouce ont été vidés"


def test_les_champs_figes_sont_refuses_par_le_domaine(saison_avec_tiers):
    """Un budget forgé est **refusé**, pas corrigé.

    Accepter la valeur reçue rendrait modifiable par requête forgée ce que
    l'écran n'ouvre pas ; la corriger en silence ferait croire à un
    enregistrement qui n'a pas eu lieu.
    """
    space_id, ctx = saison_avec_tiers["space_id"], saison_avec_tiers["ctx"]
    season_id = saison_avec_tiers["season_id"]
    tiers = query_db(
        f"SELECT rules->'tiers' FROM competition_seasons WHERE id = '{season_id}'"
    )[0]
    charge = {"tiers": json.loads(tiers)}
    budget_avant = charge["tiers"][0]["budget"]
    charge["tiers"][0]["budget"] = 999_999

    reponse = requests.post(_url(space_id, ctx), json=charge, headers=HX, timeout=30)

    assert reponse.status_code == 200, "un refus métier n'est pas une erreur de protocole"
    assert "ne se modifie pas" in reponse.text, "le motif doit nommer le champ"
    apres = json.loads(
        query_db(f"SELECT rules->'tiers' FROM competition_seasons WHERE id = '{season_id}'")[0]
    )
    assert apres[0]["budget"] == budget_avant, "le budget a été modifié"


def test_le_panneau_est_garde(saison_avec_tiers):
    space_id, ctx = saison_avec_tiers["space_id"], saison_avec_tiers["ctx"]
    url = _url(space_id, ctx)

    assert requests.get(url, headers={**HX, **ENTETE_MEMBRE_SIMPLE}, timeout=10).status_code == 403
    assert (
        requests.post(
            url, json={"tiers": []}, headers={**HX, **ENTETE_MEMBRE_SIMPLE}, timeout=30
        ).status_code
        == 403
    )
    assert requests.get(url, headers=HX, timeout=10).status_code == 200

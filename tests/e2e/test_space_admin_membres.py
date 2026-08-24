"""Tests E2E — onglet Membres de l'administration d'espace (cartes 368 à 373).

Ce que ces tests couvrent et que rien d'autre ne couvre : le re-rendu de ligne.
Changer un rôle renvoie la ligne modifiée, et c'est le serveur — seul à connaître
le nombre d'administrateurs postérieur — qui décide si le sélecteur du survivant
doit se figer. Aucun test unitaire ne voit cette couture HTMX.

**Chaque exécution crée son propre espace**, et ce n'est pas du confort.
`bypass_auth` connecte toujours DevCoach, et `Espace E2E` est partagé par toute
la suite : y promouvoir, rétrograder ou retirer quelqu'un casserait les autres
fichiers, dont plusieurs s'appuient sur ses douze coachs. Un espace jetable rend
les mutations sans conséquence.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import time

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL
from db_helpers import query_db

# Coach seedé sans droit d'administration (`seed_e2e.rs::SIMPLE_COACH_NAME`).
# Sans cet en-tête, c'est DevCoach — administrateur — qui répond, et aucun refus
# n'est observable.
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}
MEMBRE_SIMPLE = "E2E Coach 01"
FORM_URLENCODED = "application/x-www-form-urlencoded"
LOGO = "https://res.cloudinary.com/demo/image/upload/sample.jpg"


# ── Fixture ───────────────────────────────────────────────────────────────────

@pytest.fixture(scope="module")
def espace_jetable() -> str:
    """Un espace à deux membres : DevCoach administrateur, E2E Coach 01 membre.

    Créé en pilotant les vraies routes — créer un espace en fait son
    administrateur, et rejoindre y entre comme membre simple. Aucune fabrication
    SQL : c'est le chemin que suivrait un utilisateur.
    """
    nom = f"E2E Admin {time.time_ns()}"
    r = requests.post(
        f"{BASE_URL}/app/space/create",
        data={"space_name": nom, "logo_url": LOGO},
        headers={"HX-Request": "true", "Content-Type": FORM_URLENCODED},
        timeout=15,
    )
    assert r.status_code < 400, f"création de l'espace : {r.status_code} {r.text[:300]}"

    lignes = query_db(f"SELECT id FROM spaces WHERE space_name = '{nom}' LIMIT 1")
    assert lignes, f"l'espace « {nom} » doit exister après sa création"
    space_id = lignes[0][0] if isinstance(lignes[0], (list, tuple)) else lignes[0]

    _faire_rejoindre(space_id)
    return space_id


def _faire_rejoindre(space_id: str) -> None:
    """Le membre simple rejoint l'espace, sous **son** identité.

    En JSON : le formulaire de l'écran porte `hx-ext="json-enc"`, et la route
    refuse l'urlencodé par un 415.
    """
    r = requests.post(
        f"{BASE_URL}/app/space/join",
        json={"space_ids": [space_id]},
        headers={**ENTETE_MEMBRE_SIMPLE, "HX-Request": "true"},
        timeout=15,
    )
    assert r.status_code < 400, f"adhésion : {r.status_code} {r.text[:300]}"


def _ouvrir(page: Page, space_id: str) -> None:
    page.goto(f"{BASE_URL}/app/{space_id}/admin", wait_until="load")
    # Le widget arrive par un aller-retour HTMX : attendre la liste, pas la page.
    expect(page.locator(".space-admin-members")).to_be_visible(timeout=10000)


def _ligne(page: Page, pseudo: str):
    return page.locator(".sam-row").filter(has_text=pseudo)


# ── Affichage ─────────────────────────────────────────────────────────────────

def test_la_page_presente_ses_quatre_onglets(page: Page, espace_jetable: str):
    _ouvrir(page, espace_jetable)
    for libelle in ("Membres", "Ajout direct", "Invitations", "Paramètres"):
        expect(page.locator(".space-admin-tab").filter(has_text=libelle)).to_be_visible()
    expect(page.locator(".space-admin-tab.is-active")).to_have_text("👥 Membres")


def test_la_liste_affiche_pseudo_email_et_role(page: Page, espace_jetable: str):
    _ouvrir(page, espace_jetable)

    expect(page.locator(".sam-row")).to_have_count(2)
    ligne = _ligne(page, MEMBRE_SIMPLE)
    expect(ligne.locator(".sam-email")).to_contain_text("@")
    expect(ligne.locator(".sam-role")).to_be_visible()


def test_sa_propre_ligne_n_est_ni_modifiable_ni_retirable(page: Page, espace_jetable: str):
    _ouvrir(page, espace_jetable)
    moi = _ligne(page, "DevCoach")

    expect(moi.locator(".sam-vous")).to_have_text("(vous)")
    expect(moi.locator("kreek-select[disabled]")).to_have_count(1)
    expect(moi.locator(".sam-btn--danger")).to_have_count(0)


def test_la_recherche_filtre_sans_requete(page: Page, espace_jetable: str):
    _ouvrir(page, espace_jetable)
    requetes: list[str] = []
    page.on("request", lambda r: requetes.append(r.url))

    page.locator(".sam-search-input").fill("DevCoach")

    expect(_ligne(page, MEMBRE_SIMPLE)).to_be_hidden()
    expect(_ligne(page, "DevCoach")).to_be_visible()
    assert not [u for u in requetes if "/admin/widgets/members" in u], (
        "le filtre est local : aucune requête ne doit partir"
    )


# ── Mutations ─────────────────────────────────────────────────────────────────

def test_promouvoir_un_membre_rerend_sa_ligne(page: Page, espace_jetable: str):
    _ouvrir(page, espace_jetable)

    # L'assertion porte sur le **rôle affiché**, pas sur la présence du bouton de
    # retrait : celui-ci est là avant comme après la promotion, et le test
    # passerait sans que rien n'ait changé.
    _promouvoir(page, MEMBRE_SIMPLE)

    expect(_ligne(page, MEMBRE_SIMPLE).locator(".ks-display")).to_contain_text(
        "Admin", timeout=10000
    )
    # Remis dans son état d'origine : les tests de ce module partagent l'espace.
    _retrograder(page, MEMBRE_SIMPLE)


def test_promouvoir_puis_retrograder_ramene_l_etat_initial(page: Page, espace_jetable: str):
    """Deux mutations enchaînées, chacune re-rendant sa ligne sans rechargement.

    Ce test remplace celui que la carte annonçait — « le dernier administrateur
    voit son sélecteur se figer » — qui **ne peut pas exister**.

    `role_locked` vaut `is_self || (is_admin && admins == 1)`. Pour qu'une ligne
    soit figée par la seconde clause sans l'être par la première, il faudrait une
    cible seule administratrice, un spectateur distinct d'elle, et ce spectateur
    administrateur — sinon la page rend 403. Le spectateur serait donc un second
    administrateur, et la cible ne serait plus seule. La clause ne s'applique
    jamais qu'à sa propre ligne, que `is_self` fige déjà.

    C'est le même raisonnement que celui qui a montré, en carte 371, que
    `DernierAdministrateur` est inatteignable depuis le web. Ce qui reste
    observable, et que ce test vérifie, c'est l'échange lui-même : le serveur
    renvoie la ligne, elle se remplace en place, deux fois de suite.
    """
    _ouvrir(page, espace_jetable)

    _promouvoir(page, MEMBRE_SIMPLE)
    expect(_ligne(page, MEMBRE_SIMPLE).locator(".ks-display")).to_contain_text("Admin")

    _retrograder(page, MEMBRE_SIMPLE)
    expect(_ligne(page, MEMBRE_SIMPLE).locator(".ks-display")).to_contain_text("Membre")

    # La ligne de l'administrateur n'a pas bougé : elle reste figée parce que
    # c'est la sienne, et non à cause du nombre d'administrateurs.
    expect(_ligne(page, "DevCoach").locator("kreek-select[disabled]")).to_have_count(1)


def test_retirer_un_membre_fait_disparaitre_sa_ligne(page: Page, espace_jetable: str):
    _ouvrir(page, espace_jetable)
    page.on("dialog", lambda d: d.accept())

    _ligne(page, MEMBRE_SIMPLE).locator(".sam-btn--danger").click()

    expect(page.locator(".sam-row")).to_have_count(1, timeout=10000)
    expect(_ligne(page, MEMBRE_SIMPLE)).to_have_count(0)

    # L'espace est remis en état : les tests de ce module le partagent, et rien
    # ne garantit l'ordre d'exécution. Un test qui laisse le terrain amputé n'est
    # pas un test isolé, c'est un test chanceux.
    _faire_rejoindre(espace_jetable)


def test_le_bouton_de_reinitialisation_bascule_apres_envoi(page: Page, espace_jetable: str):
    _ouvrir(page, espace_jetable)
    bouton = _ligne(page, "DevCoach").locator(".sam-btn--primary")

    bouton.click()

    # Le libellé ne bascule que si la requête a réussi : `event.detail.successful`
    # fait foi, pas le clic.
    expect(bouton).to_contain_text("Envoyé", timeout=10000)


# ── Les compteurs ─────────────────────────────────────────────────────────────

def _compteur(page: Page, nom: str):
    return page.locator(f'.sas-chip-valeur[data-compteur="{nom}"]')


def test_les_compteurs_suivent_une_promotion_sans_rechargement(page: Page, espace_jetable: str):
    """Le seul test qui vérifie que les compteurs se **rafraîchissent**.

    Le harnais vérifie ce qu'ils comptent, jamais qu'ils réagissent : cela dépend
    d'événements DOM. C'est exactement l'omission qui avait laissé la liste des
    membres périmée, et que seul un test de bout en bout pouvait dire.
    """
    _ouvrir(page, espace_jetable)
    expect(_compteur(page, "membres")).to_have_text("2", timeout=10000)
    expect(_compteur(page, "administrateurs")).to_have_text("1")

    _promouvoir(page, MEMBRE_SIMPLE)

    expect(_compteur(page, "administrateurs")).to_have_text("2", timeout=10000)
    # Le total de membres ne bouge pas : promouvoir n'ajoute personne.
    expect(_compteur(page, "membres")).to_have_text("2")

    _retrograder(page, MEMBRE_SIMPLE)
    expect(_compteur(page, "administrateurs")).to_have_text("1", timeout=10000)


def test_les_invitations_en_attente_valent_zero(page: Page, espace_jetable: str):
    _ouvrir(page, espace_jetable)
    expect(_compteur(page, "invitations")).to_have_text("0", timeout=10000)


# ── Autorisation ──────────────────────────────────────────────────────────────

def test_un_membre_simple_ne_peut_pas_ouvrir_la_page(espace_jetable: str):
    r = requests.get(
        f"{BASE_URL}/app/{espace_jetable}/admin",
        headers=ENTETE_MEMBRE_SIMPLE,
        timeout=15,
    )
    assert r.status_code == 403


# ── Helpers de mutation ───────────────────────────────────────────────────────

def _promouvoir(page: Page, pseudo: str) -> None:
    _choisir_role(page, pseudo, "Admin")


def _retrograder(page: Page, pseudo: str) -> None:
    _choisir_role(page, pseudo, "Membre")


def _choisir_role(page: Page, pseudo: str, libelle: str) -> None:
    """Ouvre le sélecteur de la ligne et choisit un rôle, puis **attend l'état
    résultant**.

    Le menu de `kreek-select` vit dans l'élément lui-même, jamais déporté sur
    `body` : les options se cherchent donc dans la ligne, sinon on cliquerait
    celle d'un autre membre.

    L'attente porte sur le rôle affiché, jamais sur une durée. Le serveur
    remplace la ligne entière, et Playwright rejouerait un clic sur un élément
    qui disparaît sous lui — c'est ce qui avait rendu `test_dismissals_phase`
    instable.
    """
    select = _ligne(page, pseudo).locator("kreek-select")
    select.click()
    select.locator(".ks-option", has_text=libelle).first.click()

    # Attendre **la fin de l'échange**, pas seulement l'affichage. Le composant
    # met son libellé à jour localement, donc le lire ne prouve rien : c'était
    # exactement ce qui masquait l'absence de requête avant que `kreek-select`
    # n'émette son `change`.
    #
    # Et la ligne entière est remplacée par le serveur : rendre la main trop tôt
    # fait tomber le clic suivant sur un élément en cours de remplacement, que
    # Playwright attend indéfiniment. C'est le défaut qui avait rendu
    # `test_dismissals_phase` instable.
    expect(_ligne(page, pseudo).locator(".htmx-request")).to_have_count(0, timeout=10000)
    expect(_ligne(page, pseudo).locator(".ks-display")).to_contain_text(
        libelle, timeout=10000
    )

"""Tests E2E — onglet Ajout direct de l'administration d'espace (cartes 376-383).

Ce que ces tests couvrent et que rien d'autre ne couvre : **le contrat entre deux
BCs qui franchit la frontière par le navigateur**. Le formulaire de création de
compte appartient au BC d'authentification ; il signale son succès par un
événement DOM, que l'onglet écoute pour poser l'appartenance.

Le harnais de handlers vérifie chaque bord séparément — que l'en-tête est posé
avec ses trois chaînes, et que le panneau les écoute. Les deux peuvent passer
sans que les bords s'accordent. Seul le scénario « créer un compte et ajouter »
ci-dessous ferme la boucle : si une clé est renommée d'un côté, il est le seul à
le dire.

**Chaque exécution crée son propre espace** : `bypass_auth` connecte toujours
DevCoach, et `Espace E2E` est partagé par toute la suite. Y ajouter ou retirer
des membres casserait les autres fichiers.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import time

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL
from db_helpers import query_db
from htmx_helpers import attendre_cablage

ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}
MEMBRE_SIMPLE = "E2E Coach 01"
NON_MEMBRE = "E2E Coach 02"
FORM_URLENCODED = "application/x-www-form-urlencoded"
LOGO = "https://res.cloudinary.com/demo/image/upload/sample.jpg"


@pytest.fixture(scope="module")
def espace_jetable() -> str:
    """Un espace à deux membres : DevCoach administrateur, E2E Coach 01 membre.

    Les dix autres coachs semés en sont absents : ce sont les candidats.
    """
    nom = f"E2E AjoutDirect {time.time_ns()}"
    r = requests.post(
        f"{BASE_URL}/app/space/create",
        data={"space_name": nom, "logo_url": LOGO},
        headers={"HX-Request": "true", "Content-Type": FORM_URLENCODED},
        timeout=15,
    )
    assert r.status_code < 400, f"création : {r.status_code} {r.text[:300]}"

    lignes = query_db(f"SELECT id FROM spaces WHERE space_name = '{nom}' LIMIT 1")
    assert lignes, f"l'espace « {nom} » doit exister"
    space_id = lignes[0][0] if isinstance(lignes[0], (list, tuple)) else lignes[0]

    r = requests.post(
        f"{BASE_URL}/app/space/join",
        json={"space_ids": [space_id]},
        headers={**ENTETE_MEMBRE_SIMPLE, "HX-Request": "true"},
        timeout=15,
    )
    assert r.status_code < 400, f"adhésion : {r.status_code} {r.text[:300]}"
    return space_id


def _ouvrir_ajout_direct(page: Page, space_id: str) -> None:
    page.goto(f"{BASE_URL}/app/{space_id}/admin", wait_until="load")
    expect(page.locator(".space-admin-members")).to_be_visible(timeout=10000)
    page.locator(".space-admin-tab", has_text="Ajout direct").click()
    # Le panneau de création arrive par un aller-retour : attendre sa présence,
    # pas une durée.
    expect(page.locator(".space-admin-creation")).to_be_visible(timeout=10000)


def _chercher(page: Page, terme: str) -> None:
    page.locator(".space-admin-recherche-champ").fill(terme)


def _candidat(page: Page, pseudo: str):
    return page.locator(".sac-row").filter(has_text=pseudo)


# ── La recherche et ses trois états ───────────────────────────────────────────

def test_un_seul_caractere_reste_sous_le_seuil(page: Page, espace_jetable: str):
    _ouvrir_ajout_direct(page, espace_jetable)

    _chercher(page, "E")

    expect(page.locator(".sac-etat")).to_contain_text(
        "au moins deux caractères", timeout=10000
    )
    expect(page.locator(".sac-row")).to_have_count(0)


def test_un_pseudo_inexistant_propose_de_creer_un_compte(page: Page, espace_jetable: str):
    _ouvrir_ajout_direct(page, espace_jetable)

    _chercher(page, "ZzzPersonneIci")

    expect(page.locator(".sac-etat--vide")).to_contain_text(
        "Créez-lui un compte", timeout=10000
    )


def test_un_coach_de_la_plateforme_est_proposé_avec_son_email_masqué(page: Page, espace_jetable: str):
    _ouvrir_ajout_direct(page, espace_jetable)

    _chercher(page, NON_MEMBRE)

    ligne = _candidat(page, NON_MEMBRE)
    expect(ligne).to_be_visible(timeout=10000)
    expect(ligne.locator(".sac-email")).to_contain_text("•••@")
    expect(ligne.locator(".sac-email")).not_to_contain_text("e2e-coach-")
    expect(ligne.locator(".sac-btn")).to_be_visible()


def test_un_membre_de_l_espace_porte_son_badge_et_aucun_bouton(page: Page, espace_jetable: str):
    _ouvrir_ajout_direct(page, espace_jetable)

    _chercher(page, MEMBRE_SIMPLE)

    ligne = _candidat(page, MEMBRE_SIMPLE)
    expect(ligne.locator(".sac-badge")).to_contain_text("Déjà membre", timeout=10000)
    expect(ligne.locator(".sac-btn")).to_have_count(0)


# ── L'ajout d'un coach déjà inscrit ───────────────────────────────────────────

def test_ajouter_un_coach_le_fait_passer_membre_et_apparaitre_au_journal(
    page: Page, espace_jetable: str
):
    """Le journal affiche **immédiatement**, sans relire.

    C'est ce qui masque le délai d'alimentation du cache d'utilisateurs. Si
    quelqu'un « simplifiait » plus tard en le faisant relire, ce test rougirait —
    et c'est tout ce qui protège la décision.
    """
    _ouvrir_ajout_direct(page, espace_jetable)
    _chercher(page, NON_MEMBRE)
    expect(_candidat(page, NON_MEMBRE).locator(".sac-btn")).to_be_visible(timeout=10000)

    _candidat(page, NON_MEMBRE).locator(".sac-btn").click()

    # La ligne est remplacée par le serveur : attendre l'état résultant.
    expect(_candidat(page, NON_MEMBRE).locator(".sac-badge")).to_contain_text(
        "Déjà membre", timeout=10000
    )
    expect(page.locator(".space-admin-journal-ligne")).to_contain_text(NON_MEMBRE)

    # Et il est bien membre : la liste des membres le contient.
    page.locator(".space-admin-tab", has_text="Membres").first.click()
    expect(page.locator(".sam-row").filter(has_text=NON_MEMBRE)).to_have_count(
        1, timeout=10000
    )

    _retirer_du_journal(page, NON_MEMBRE)


def _retirer_du_journal(page: Page, pseudo: str) -> None:
    """Remet l'espace en état — les tests de ce module le partagent."""
    page.locator(".space-admin-tab", has_text="Ajout direct").click()
    ligne = page.locator(".space-admin-journal-ligne").filter(has_text=pseudo)
    if ligne.count():
        ligne.locator(".space-admin-journal-retrait").click()
        expect(ligne).to_have_count(0, timeout=10000)


def test_retirer_depuis_le_journal_le_sort_de_l_espace(page: Page, espace_jetable: str):
    _ouvrir_ajout_direct(page, espace_jetable)
    _chercher(page, NON_MEMBRE)
    expect(_candidat(page, NON_MEMBRE).locator(".sac-btn")).to_be_visible(timeout=10000)
    _candidat(page, NON_MEMBRE).locator(".sac-btn").click()
    expect(page.locator(".space-admin-journal-ligne")).to_contain_text(
        NON_MEMBRE, timeout=10000
    )

    page.locator(".space-admin-journal-retrait").first.click()

    expect(page.locator(".space-admin-journal-ligne")).to_have_count(0, timeout=10000)
    page.locator(".space-admin-tab", has_text="Membres").first.click()
    expect(page.locator(".sam-row").filter(has_text=NON_MEMBRE)).to_have_count(
        0, timeout=10000
    )


# ── Le contrat entre les deux BCs ─────────────────────────────────────────────

def test_creer_un_compte_et_ajouter_de_bout_en_bout(page: Page, espace_jetable: str):
    """Le seul test qui vérifie que les deux bords du contrat s'accordent.

    Le nom de l'événement DOM et ses deux clés franchissent la frontière par le
    navigateur. Ni le compilateur, ni `cargo test`, ni `check-arch` ne les
    voient : renommer une clé d'un côté casse l'autre en silence, et **rien
    d'autre que ce test ne le dira**.

    Il vérifie donc la chaîne complète — compte créé *et* appartenance posée —
    pas seulement que le formulaire répond.
    """
    _ouvrir_ajout_direct(page, espace_jetable)
    # **L'horodatage entier, comme partout ailleurs dans la suite.**
    #
    # Un `% 1_000_000` tronquait ici, sans raison : `CoachName` accepte 50
    # caractères, et « E2ENouveau » plus dix-neuf chiffres en fait vingt-neuf.
    # La troncature ne gagnait rien et ramenait l'espace des noms de dix-neuf
    # chiffres à **mille valeurs** — dont l'horloge de la machine ne rend que
    # les multiples de mille, les trois derniers chiffres étant toujours nuls.
    #
    # Un pseudonyme est unique en base : chaque exécution en consommait une, et
    # la probabilité de collision montait à chaque `make e2e`. Constaté à 43
    # comptes posés, soit environ 4 % par course — et le test a fini par tomber
    # sur un nom déjà pris, en échouant sur un symptôme (« le journal n'affiche
    # pas le pseudonyme ») qui n'évoquait en rien sa cause.
    pseudo = f"E2ENouveau{time.time_ns()}"

    page.locator(".space-admin-creation input[name='coach_name']").fill(pseudo)
    page.locator(".space-admin-creation input[name='email']").fill(
        f"{pseudo.lower()}@bb.club"
    )
    # **Le formulaire arrive deux fois différé.** Le panneau de création est
    # chargé par `hx-get` au `load` de la page, puis htmx câble le `<form
    # hx-post>` qu'il contient. Un clic tombé dans cette fenêtre ne produit
    # rien — aucune requête, aucune erreur — et l'attente ci-dessous expire sur
    # un journal qui n'a jamais reçu de ligne. Le test échouait ainsi dans la
    # suite complète, où la machine est chargée, et passait seul.
    attendre_cablage(page, ".space-admin-creation form")
    page.locator(".space-admin-creation button[type='submit']").click()

    # Le compte existe.
    expect(page.locator(".space-admin-journal-ligne")).to_contain_text(
        pseudo, timeout=15000
    )
    assert query_db(
        f"SELECT id FROM auth__users WHERE coach_name = '{pseudo}'"
    ), "le compte doit avoir été créé"

    # Et l'appartenance a bien été posée — c'est le second maillon, celui que le
    # harnais ne peut pas vérifier.
    membres = query_db(
        f"""SELECT m.coach_id FROM spaces__user_space m
            JOIN auth__users u ON u.id = m.coach_id
            WHERE m.space_id = '{espace_jetable}' AND u.coach_name = '{pseudo}'"""
    )
    assert membres, "l'événement doit avoir déclenché l'ajout à l'espace"


def test_un_membre_simple_ne_peut_pas_ouvrir_l_onglet(espace_jetable: str):
    r = requests.get(
        f"{BASE_URL}/app/{espace_jetable}/admin/widgets/candidates?q=E2E",
        headers=ENTETE_MEMBRE_SIMPLE,
        timeout=15,
    )
    assert r.status_code == 403

"""Tests E2E — phase de recrutement (cartes 262 à 265).

Le test unitaire vérifie que le panier calcule juste ; celui-ci vérifie qu'il
**existe côté serveur**. C'est le seul niveau où la différence se voit : un
panier client passerait tous les tests unitaires du monde et perdrait ses
lignes au premier changement de page.

Deux scénarios portent l'essentiel :

- `test_09_…` quitte la page et y revient. Sans lui, la table de panier de la
  décision D1 aurait été payée pour rien.
- `test_04_…` vérifie qu'un panier intenable ne débite **rien** — ni la
  trésorerie projetée, ni le grand livre. C'est ce qui rend un clic malheureux
  sans conséquence tant que la phase n'est pas validée.

Ordre d'exécution : les tests d'un module s'exécutent dans l'ordre du fichier,
et ils partagent une équipe et son panier. La numérotation des noms n'est donc
pas décorative — elle s'écarte volontairement de celle de la carte, parce que
la validation (8) est **terminale** : elle fait passer l'équipe en phase de
renvois, où plus aucun des autres scénarios n'aurait de sens. Tout ce qui a
besoin de la phase de recrutement la précède ; la lecture du grand livre la
suit.

Le panier n'est pas remis à zéro entre les scénarios 5 et 8 : ce qu'ils y
déposent est précisément ce que 8 valide et ce que 10 relit. Un panier
d'un joueur et de trois relances éprouve mieux la validation en lot qu'un achat
isolé.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) et base seedée
(`make seed_e2e`), comme toute la suite.
"""

import re
import time

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import play_match

# Roster des Granitiers — l'équipe principale. Ces montants viennent de
# `assets/references.example/teams_fr.json` ; les asserts qui en dépendent le
# disent, pour qu'une évolution du corpus de démo produise un échec lisible
# plutôt qu'un timeout.
PIETAILLE_KPO = 50
COLOSSE_KPO = 140
RELANCE_BASE_KPO = 60
RELANCE_KPO = RELANCE_BASE_KPO * 2  # doublée après la création de l'équipe

# Gain du match, en kPo. Le minimum du formulaire (`step5.html`, min=5) : le
# scénario 4 doit pouvoir assécher la caisse par des achats réels, ce que les
# 50 000 kPo par défaut de `match_report_helpers` rendraient impossible.
GAIN_MATCH_KPO = 5

# Budget de création (1060) − 11 Piétailles, plus le gain du match.
CAISSE_ATTENDUE = 1060 - 11 * PIETAILLE_KPO + GAIN_MATCH_KPO


# ── Helpers ───────────────────────────────────────────────────────────────────


def _valider_phase_amelioration(space_id: str, team_id: str) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{team_id}/validate-improvement-phase",
        headers={"HX-Request": "true"},
        allow_redirects=False,
    )
    assert resp.status_code in (200, 302, 303), f"validate-improvement: {resp.status_code}"


def _attendre_phase(team_id: str, phase: str, timeout_s: int = 20) -> None:
    """La phase suit la publication du rapport par app event : elle est
    asynchrone, et un `goto` immédiat tomberait sur l'état d'avant."""
    deadline = time.time() + timeout_s
    vue = None
    while time.time() < deadline:
        rows = query_db(f"SELECT game_phase FROM team_proj WHERE team_id = '{team_id}'")
        vue = rows[0] if rows else None
        if vue == phase:
            return
        time.sleep(0.2)
    raise AssertionError(f"équipe {team_id} en phase {vue!r}, attendu {phase!r} après {timeout_s}s")


def _tresorerie(team_id: str) -> int:
    """Le solde vient du grand livre, jamais d'une somme SQL.

    `team_proj` ne porte pas la trésorerie — elle se dérive des événements, et
    `balance_after_kpo` est la valeur que le domaine a calculée au moment du
    mouvement. La recalculer ici serait une deuxième implémentation de la règle
    d'écrêtage à zéro (carte 255).
    """
    rows = query_db(
        "SELECT balance_after_kpo FROM teams__treasury_ledger "
        f"WHERE team_id = '{team_id}' ORDER BY id DESC LIMIT 1"
    )
    assert rows, f"aucun mouvement de trésorerie pour {team_id}"
    return int(rows[0])


def _tresorerie_affichee(page: Page) -> int:
    entete = page.locator(".rec-header .meta-item").filter(has_text="Trésorerie")
    return int(re.sub(r"[^0-9]", "", entete.locator(".meta-value").inner_text()))


def _lignes_grand_livre(team_id: str) -> list[str]:
    return query_db(
        "SELECT direction, amount_kpo, reason, balance_after_kpo "
        f"FROM teams__treasury_ledger WHERE team_id = '{team_id}' ORDER BY id"
    )


def _joueurs(team_id: str) -> list[str]:
    return query_db(
        f"SELECT jersey, position_name FROM players_proj WHERE team_id = '{team_id}' ORDER BY jersey"
    )


def _ouvrir_recrutement(page: Page, space_id: str, team_id: str) -> None:
    """Passe par la page, pas par le widget : c'est l'assemblage `hx-trigger=load`
    des deux colonnes qu'on veut voir fonctionner."""
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}/recruitment", wait_until="load")
    expect(page.locator(".rec-catalog")).to_be_visible()
    expect(page.locator(".rec-cart")).to_be_visible()


def _ligne_poste(page: Page, nom: str):
    return page.locator(".buy-table tbody tr").filter(has_text=nom).first


def _reste_apres_achats(page: Page) -> int:
    texte = page.locator(".rec-cart .purse-amount").inner_text()
    return int(re.sub(r"[^0-9]", "", texte))


def _acheter(page: Page, nom_ligne: str) -> None:
    """Clique et attend que le catalogue soit revenu — le bouton est remplacé
    par le swap, donc l'attente ne peut pas porter sur lui."""
    avant = page.locator(".rec-cart .purse-row").count()
    _ligne_poste(page, nom_ligne).locator(".act-btn").click()
    expect(page.locator(".rec-cart .purse-row")).not_to_have_count(avant, timeout=10000)


# ── Fixture : deux équipes amenées en phase de recrutement ────────────────────


@pytest.fixture(scope="module")
def recrutement_ctx(browser, space_id):
    """Un match entre les deux équipes les amène toutes deux en phase
    d'amélioration ; la valider les fait entrer en recrutement.

    L'équipe 1 est un roster **Lanterniers**, seul roster de démo sans
    apothicaire — c'est la seule façon de voir une ligne de staff refusée par le
    roster (scénario 6), et rien d'autre ne l'exige.
    """
    full = build_full_competition(
        browser, space_id, num_teams=2, num_rounds=2,
        roster_uids=["DEMO_GRANIT", "DEMO_LANTERNE"],
    )
    granit, lanterne = full["team_ids"][0], full["team_ids"][1]

    play_match(
        space_id, full, full["round_ids"][0], granit, lanterne,
        home_gain=GAIN_MATCH_KPO, away_gain=GAIN_MATCH_KPO,
    )

    for team_id in (granit, lanterne):
        _attendre_phase(team_id, "PlayerImprovement")
        _valider_phase_amelioration(space_id, team_id)
        _attendre_phase(team_id, "Recruitment")

    return {"granit": granit, "lanterne": lanterne, "space_id": space_id}


# ── 1 · La bannière ouvre la page, le catalogue est peuplé ────────────────────


def test_01_banniere_ouvre_le_catalogue(page: Page, recrutement_ctx):
    space_id, team_id = recrutement_ctx["space_id"], recrutement_ctx["granit"]
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")

    banniere = page.locator(".state-banner--phase")
    expect(banniere).to_contain_text("Phase de recrutement")

    # Le CTA est un `<a hx-get hx-push-url>` : HTMX échange `#app-content`, il
    # n'y a pas de navigation du navigateur à attendre. C'est l'apparition du
    # catalogue qui fait foi.
    banniere.locator(".state-banner-cta").click()
    expect(page.locator(".rec-catalog")).to_be_visible(timeout=10000)
    expect(page.locator(".rec-phase-badge")).to_contain_text("Phase de recrutement")
    assert "/recruitment" in page.url, f"l'URL n'a pas suivi le swap : {page.url}"

    # Les trois postes du roster, avec leur prix.
    for nom, prix in (
        ("Piétaille des Carrières", PIETAILLE_KPO),
        ("Percuteur", 90),
        ("Colosse de Granit", COLOSSE_KPO),
    ):
        ligne = _ligne_poste(page, nom)
        expect(ligne).to_be_visible()
        expect(ligne.locator(".price")).to_contain_text(f"{prix} kPo")

    # La trésorerie affichée est celle de l'équipe, gain du match compris —
    # c'est le solde que le grand livre a enregistré, pas un calcul de la page.
    assert _tresorerie_affichee(page) == _tresorerie(team_id)

    # Ancrage du corpus : tout le dimensionnement des scénarios suivants en
    # dépend, et un écart doit se lire ici plutôt que dans un blocage inattendu
    # trois tests plus loin.
    assert _tresorerie(team_id) == CAISSE_ATTENDUE, (
        f"caisse de {_tresorerie(team_id)} kPo, attendu {CAISSE_ATTENDUE} — "
        "budget de création, prix des Piétailles ou gain du match ont changé"
    )


# ── 2 · Ajouter un joueur : ligne au panier, reste ↓, quota +1 ────────────────


def test_02_ajouter_un_joueur(page: Page, recrutement_ctx):
    space_id, team_id = recrutement_ctx["space_id"], recrutement_ctx["granit"]
    _ouvrir_recrutement(page, space_id, team_id)

    caisse = _tresorerie(team_id)
    expect(page.locator(".rec-cart .purse-empty")).to_be_visible()
    assert _reste_apres_achats(page) == caisse

    _acheter(page, "Piétaille des Carrières")

    ligne_panier = page.locator(".rec-cart .purse-row").first
    expect(ligne_panier).to_contain_text("Piétaille des Carrières")
    expect(ligne_panier).to_contain_text(f"{PIETAILLE_KPO} kPo")
    assert _reste_apres_achats(page) == caisse - PIETAILLE_KPO

    # Le quota compte les possédés **et** les en-attente, séparément.
    quota = _ligne_poste(page, "Piétaille des Carrières").locator(".quota")
    expect(quota.locator(".pending")).to_have_text("+1")

    # Rien n'est débité : c'est la promesse du panier.
    assert _tresorerie(team_id) == caisse


# ── 3 · Retirer la ligne : retour à l'état initial ────────────────────────────


def test_03_retirer_la_ligne(page: Page, recrutement_ctx):
    space_id, team_id = recrutement_ctx["space_id"], recrutement_ctx["granit"]
    _ouvrir_recrutement(page, space_id, team_id)

    caisse = _tresorerie(team_id)
    expect(page.locator(".rec-cart .cart-remove")).to_have_count(1)
    page.locator(".rec-cart .cart-remove").first.click()

    expect(page.locator(".rec-cart .purse-empty")).to_be_visible(timeout=10000)
    assert _reste_apres_achats(page) == caisse
    expect(_ligne_poste(page, "Piétaille des Carrières").locator(".pending")).to_have_count(0)


# ── 9 · Le panier survit à un changement de page ──────────────────────────────


def test_09_le_panier_survit_a_la_navigation(page: Page, recrutement_ctx):
    """Le test qui distingue un panier serveur d'un panier client.

    La navigation est réelle — fiche d'équipe, puis retour. Un `reload()` ne
    prouverait rien : un panier client survit à certains rechargements.
    """
    space_id, team_id = recrutement_ctx["space_id"], recrutement_ctx["granit"]
    _ouvrir_recrutement(page, space_id, team_id)
    _acheter(page, "Piétaille des Carrières")
    reste_avant = _reste_apres_achats(page)

    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
    expect(page.locator(".team-status-badge")).to_be_visible()

    _ouvrir_recrutement(page, space_id, team_id)
    expect(page.locator(".rec-cart .purse-row").first).to_contain_text("Piétaille des Carrières")
    assert _reste_apres_achats(page) == reste_avant, "le panier a perdu sa ligne en changeant de page"

    # Remis à zéro pour la suite : les scénarios suivants comptent leurs lignes.
    page.locator(".rec-cart .cart-remove").first.click()
    expect(page.locator(".rec-cart .purse-empty")).to_be_visible(timeout=10000)


# ── 11 · Mobile 390px : panier replié, dépliable, « × » atteignable ───────────


def test_11_panier_repliable_en_mobile(page: Page, recrutement_ctx):
    """Le viewport est posé **avant** la navigation : le repliement est un
    `x-data` Alpine dont l'`init()` lit `window.innerWidth` au montage. Un
    redimensionnement après coup ne le rejouerait pas, et le test observerait
    l'état desktop en croyant tester le mobile.
    """
    space_id, team_id = recrutement_ctx["space_id"], recrutement_ctx["granit"]
    page.set_viewport_size({"width": 390, "height": 844})
    try:
        _ouvrir_recrutement(page, space_id, team_id)
        _acheter(page, "Piétaille des Carrières")

        actions = page.locator(".rec-cart .side-actions")
        expect(actions).not_to_have_class(re.compile(r"\bis-open\b"))

        # Barre du bas, et non panneau dans le flux.
        assert actions.evaluate("el => getComputedStyle(el).position") == "fixed"

        # Calée **au-dessus** de la tabbar globale : sans le décalage, le total
        # et le bouton de validation passeraient sous le menu du chrome mobile.
        tabbar = page.locator(".mobile-tabbar")
        expect(tabbar).to_be_visible()
        bas_panier = actions.bounding_box()
        haut_tabbar = tabbar.bounding_box()
        assert bas_panier["y"] + bas_panier["height"] <= haut_tabbar["y"] + 1, (
            "le panier fixe chevauche la tabbar mobile"
        )

        page.locator(".rec-cart .cart-head").click()
        expect(actions).to_have_class(re.compile(r"\bis-open\b"))

        retirer = page.locator(".rec-cart .cart-remove").first
        expect(retirer).to_be_visible()
        retirer.click()

        # Le panier se **replie** après la mutation : le swap remonte le
        # `x-data`, dont l'`init()` relit `innerWidth` et retrouve false. La
        # ligne a bien disparu, mais la vérifier à la visibilité testerait le
        # repliement au lieu du retrait.
        expect(page.locator(".rec-cart .cart-remove")).to_have_count(0, timeout=10000)
        expect(page.locator(".rec-cart .purse-empty")).to_have_count(1)
    finally:
        page.set_viewport_size({"width": 1280, "height": 900})


# ── 5 · Quota de poste atteint ────────────────────────────────────────────────


def test_05_quota_de_poste_atteint(page: Page, recrutement_ctx):
    """Le Colosse de Granit est plafonné à 1 : un seul achat suffit à fermer sa
    ligne. Il reste au panier — c'est lui que le scénario 8 validera."""
    space_id, team_id = recrutement_ctx["space_id"], recrutement_ctx["granit"]
    _ouvrir_recrutement(page, space_id, team_id)

    ligne = _ligne_poste(page, "Colosse de Granit")
    expect(ligne.locator(".act-btn")).to_have_text("Recruter")
    _acheter(page, "Colosse de Granit")

    bouton = _ligne_poste(page, "Colosse de Granit").locator(".act-btn")
    expect(bouton).to_have_text("Quota atteint")
    expect(bouton).to_be_disabled()


# ── 7 · La relance coûte le double, son prix de base est rappelé ─────────────


def test_07_relance_au_double_du_prix_de_base(page: Page, recrutement_ctx):
    space_id, team_id = recrutement_ctx["space_id"], recrutement_ctx["granit"]
    _ouvrir_recrutement(page, space_id, team_id)

    ligne = page.locator(".buy-table tbody tr").filter(has_text="Relance").first
    expect(ligne.locator(".price")).to_contain_text(f"{RELANCE_KPO} kPo")
    expect(ligne.locator(".price-note")).to_contain_text(f"base {RELANCE_BASE_KPO} kPo")


# ── 4 · Trésorerie insuffisante, et rien n'est débité ─────────────────────────


def test_04_tresorerie_insuffisante_ne_debite_rien(page: Page, recrutement_ctx):
    """On assèche le panier par des achats réels plutôt qu'en appauvrissant
    l'équipe en base : le blocage vient alors de l'application, pas d'un état
    fabriqué.

    Le Colosse du scénario 5 est déjà au panier ; trois relances suffisent à
    passer sous le prix d'une Piétaille.
    """
    space_id, team_id = recrutement_ctx["space_id"], recrutement_ctx["granit"]
    _ouvrir_recrutement(page, space_id, team_id)

    caisse = _tresorerie(team_id)
    lignes_avant = _lignes_grand_livre(team_id)

    for _ in range(3):
        _acheter(page, "Relance")

    reste = _reste_apres_achats(page)
    assert reste == caisse - COLOSSE_KPO - 3 * RELANCE_KPO
    assert reste < PIETAILLE_KPO, (
        f"reste {reste} kPo, il en faut moins de {PIETAILLE_KPO} pour que la "
        "Piétaille devienne intenable — le corpus de démo a-t-il changé ?"
    )

    bouton = _ligne_poste(page, "Piétaille des Carrières").locator(".act-btn")
    expect(bouton).to_have_text("Trésorerie")
    expect(bouton).to_be_disabled()

    # Le cœur du scénario : le panier promet un reste de quelques kPo, mais rien
    # n'a bougé — ni la caisse, ni le grand livre.
    assert _tresorerie(team_id) == caisse, "la trésorerie a été débitée avant validation"
    assert _lignes_grand_livre(team_id) == lignes_avant, "le grand livre a bougé avant validation"


# ── 6 · Roster sans apothicaire ───────────────────────────────────────────────


def test_06_roster_sans_apothicaire(page: Page, recrutement_ctx):
    """Les Lanterniers n'ont pas droit à l'apothicaire. Un interdit occupe la
    ligne entière et s'explique, là où un blocage passager tient dans le bouton.
    """
    space_id, team_id = recrutement_ctx["space_id"], recrutement_ctx["lanterne"]
    _ouvrir_recrutement(page, space_id, team_id)

    ligne = page.locator(".buy-table tbody tr").filter(has_text="Apothicaire").first
    expect(ligne).to_have_class(re.compile(r"\bis-blocked\b"))
    expect(ligne.locator(".reason")).to_have_text("Ce roster n'a pas droit à ce personnel.")
    expect(ligne.locator(".act-btn")).to_be_disabled()

    # Un staff que le roster autorise reste achetable : c'est bien le roster qui
    # ferme l'apothicaire, pas un blocage général de la page. Le libellé est
    # celui de `nom_staff()` dans view_models.rs, pas celui de staff_fr.json —
    # le catalogue ne lit pas ses noms dans le corpus.
    autorise = page.locator(".buy-table tbody tr").filter(has_text="Pom-pom girl").first
    expect(autorise.locator(".act-btn")).to_be_enabled()


# ── 8 · Valider : débit du total, joueurs créés, passage en renvois ───────────


def test_08_valider_debite_et_cree_les_joueurs(page: Page, recrutement_ctx):
    """La validation est terminale : elle clôt la phase de recrutement.

    Elle couvre aussi la traversée `teams → app event → players` (carte 265) —
    c'est le seul endroit de la suite où l'on vérifie qu'un joueur acheté dans
    un BC existe réellement dans l'autre, avec un maillot qui lui est propre.
    """
    space_id, team_id = recrutement_ctx["space_id"], recrutement_ctx["granit"]
    _ouvrir_recrutement(page, space_id, team_id)

    caisse_avant = _tresorerie(team_id)
    maillots_avant = {l.split("|")[0] for l in _joueurs(team_id)}
    total = COLOSSE_KPO + 3 * RELANCE_KPO

    cta = page.locator(".rec-cart .cta-primary")
    expect(cta).to_have_text("Valider 4 achats →")
    cta.click()

    _attendre_phase(team_id, "Dismissals")
    assert _tresorerie(team_id) == caisse_avant - total, "le débit ne vaut pas le total du panier"

    # Le joueur acheté existe dans `players`, avec un maillot inédit.
    deadline = time.time() + 20
    while time.time() < deadline:
        joueurs = _joueurs(team_id)
        if len(joueurs) == len(maillots_avant) + 1:
            break
        time.sleep(0.2)
    else:
        raise AssertionError(
            f"le joueur recruté n'est pas arrivé dans players_proj : {len(joueurs)} joueurs"
        )

    nouveaux = [l for l in joueurs if l.split("|")[0] not in maillots_avant]
    assert len(nouveaux) == 1, f"attendu un seul nouveau joueur, trouvé {nouveaux}"
    maillot, poste = nouveaux[0].split("|")
    assert poste == "Colosse de Granit", f"poste inattendu : {poste}"
    assert maillot not in maillots_avant, "le maillot attribué est déjà porté"


# ── 10 · Le grand livre porte une ligne par achat ─────────────────────────────


def test_10_grand_livre_une_ligne_par_achat(recrutement_ctx):
    """Aucun écran n'expose l'historique de trésorerie — une page de trésorerie
    n'est pas au périmètre de cette feature. La lecture passe donc par la base.
    """
    team_id = recrutement_ctx["granit"]
    lignes = [l.split("|") for l in _lignes_grand_livre(team_id)]

    achats = [l for l in lignes if l[0] == "Debit"]
    assert len(achats) >= 4, f"attendu au moins 4 débits (1 joueur + 3 relances), trouvé {achats}"

    quatre_derniers = achats[-4:]
    montants = sorted(int(l[1]) for l in quatre_derniers)
    assert montants == sorted([COLOSSE_KPO] + [RELANCE_KPO] * 3), f"montants inattendus : {montants}"

    # Le solde après chaque mouvement vient du domaine, jamais d'un calcul SQL :
    # il doit décroître du montant de la ligne, sans trou.
    for precedent, courant in zip(quatre_derniers, quatre_derniers[1:]):
        assert int(courant[3]) == int(precedent[3]) - int(courant[1]), (
            f"solde incohérent entre {precedent} et {courant}"
        )

    assert int(quatre_derniers[-1][3]) == _tresorerie(team_id), (
        "le dernier solde du grand livre doit être la trésorerie courante"
    )

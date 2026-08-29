"""Tests E2E — phase de renvois (cartes 267 à 270).

Deux propriétés de cette page ne sont vérifiables **qu'à ce niveau** :

- **le plancher des onze éligibles vu de l'interface** — le domaine sait le
  calculer, mais seul un navigateur montre que tous les boutons basculent
  ensemble au moment où l'avant-dernier renvoi passe ;
- **l'absence totale de mouvement de trésorerie de bout en bout**, du `match`
  exhaustif de `treasury_movement()` jusqu'au grand livre en base.

Un seul récit, une seule équipe. Le décor tient à une blessure : elle donne
**douze éligibles sur treize joueurs**, ce qui sert d'un coup les deux scénarios
qui comptent — marquer un disponible fait tomber à onze et bloque tous les
autres, pendant que l'absent, lui, reste renvoyable.

Ordre d'exécution : les tests d'un module s'exécutent dans l'ordre du fichier et
partagent l'équipe. La numérotation des noms s'écarte de celle de la carte parce
que la validation est **terminale** — elle fait passer l'équipe en « prête à
jouer », où plus aucun des autres scénarios n'aurait de sens.

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
from htmx_helpers import attendre_cablage_locator, cliquer_quand_cable
from team_phase_helpers import traverser_erreurs_couteuses
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    post_step5,
    publish,
)

# Roster des Granitiers, depuis `assets/references.example/teams_fr.json`. Les
# asserts qui en dépendent le disent, pour qu'une évolution du corpus produise un
# échec lisible plutôt qu'un timeout.
PIETAILLE = "DEMO_GRANIT__PIETAILLE"
PERCUTEUR = "DEMO_GRANIT__PERCUTEUR"
PIETAILLE_KPO = 50
PERCUTEUR_KPO = 90

# Gain minimal du formulaire de gains : la trésorerie n'est pas le sujet ici,
# mais une caisse démesurée rendrait le scénario 6 illisible.
GAIN_MATCH_KPO = 5

# Le maillot du blessé, et celui du disponible qu'on renverra. Choisis plutôt
# que tirés : le scénario du maillot réattribué a besoin de savoir exactement
# quels numéros se libèrent.
MAILLOT_RENVOYE = 1
MAILLOT_BLESSE = 2


# ── Helpers ───────────────────────────────────────────────────────────────────


def _attendre(predicat, quoi, timeout_s=20):
    """Attend une **condition**, jamais une durée.

    Le scénario de valeur d'équipe repose là-dessus : un `sleep` calibré
    masquerait la course que ce test existe pour surveiller.
    """
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if predicat():
            return
        time.sleep(0.2)
    raise AssertionError(f"{quoi} : jamais satisfait après {timeout_s}s")


def _phase(team_id: str) -> str | None:
    rows = query_db(f"SELECT game_phase FROM team_proj WHERE team_id = '{team_id}'")
    return rows[0] if rows else None


def _attendre_phase(team_id: str, phase: str) -> None:
    _attendre(lambda: _phase(team_id) == phase, f"équipe {team_id} en phase {phase}")


def _valider_phase(space_id: str, team_id: str, route: str) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{team_id}/{route}",
        headers={"HX-Request": "true"},
        allow_redirects=False,
    )
    assert resp.status_code in (200, 302, 303), f"{route} : {resp.status_code}"


def _actifs(team_id: str) -> list[str]:
    """`jersey|player_id|value_kpo|participation_status` des membres de l'effectif."""
    return query_db(
        "SELECT jersey, player_id, value_kpo, participation_status FROM players_proj "
        f"WHERE team_id = '{team_id}' AND membership = 'Active' "
        "ORDER BY jersey NULLS LAST, player_id"
    )


def _joueur_au_maillot(team_id: str, maillot: int) -> str:
    rows = query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{team_id}' AND jersey = {maillot}"
    )
    assert rows, f"aucun joueur au maillot {maillot} dans {team_id}"
    return rows[0]


def _appartenance(player_id: str) -> str | None:
    rows = query_db(f"SELECT membership FROM players_proj WHERE player_id = '{player_id}'")
    return rows[0] if rows else None


def _valeur_equipe(team_id: str) -> int:
    return int(query_db(f"SELECT team_value FROM team_proj WHERE team_id = '{team_id}'")[0])


def _grand_livre(team_id: str) -> list[str]:
    return query_db(
        "SELECT direction, amount_kpo, reason, balance_after_kpo "
        f"FROM teams__treasury_ledger WHERE team_id = '{team_id}' ORDER BY id"
    )


def _tresorerie(team_id: str) -> int:
    """Le solde vient du grand livre : `team_proj` ne porte pas la trésorerie."""
    rows = query_db(
        "SELECT balance_after_kpo FROM teams__treasury_ledger "
        f"WHERE team_id = '{team_id}' ORDER BY id DESC LIMIT 1"
    )
    assert rows, f"aucun mouvement de trésorerie pour {team_id}"
    return int(rows[0])


def _blesser(space_id: str, mr_id: str, victime: str) -> None:
    """Blessure sérieuse subie par un joueur de l'équipe **domicile**, infligée
    depuis le camp adverse (`step4`) — même chemin que
    `test_player_availability_after_injury`."""
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step4/actions",
        data={
            "turn": "3",
            "player_id": victime,
            "player_type": "regular",
            "action_type": "BLESSE",
            "injury_type": "BLESSURE_SERIEUSE",
        },
    )
    assert resp.status_code == 200, f"blessure : {resp.status_code}\n{resp.text[:200]}"


def _jouer_match(space_id, ctx, round_id, home, away, *, blesse=None) -> str:
    """`play_match` ne sait pas blesser : on compose les helpers publics."""
    mr_id = create_draft(space_id, ctx, round_id, home, away)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home, away)
    ensure_inducements(space_id, mr_id)
    if blesse:
        _blesser(space_id, mr_id, blesse)
    post_step5(space_id, mr_id, home_gain=GAIN_MATCH_KPO, away_gain=GAIN_MATCH_KPO)
    publish(space_id, mr_id)
    return mr_id


def _recruter(space_id: str, team_id: str, ligne: str, version: int) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{team_id}/recruitment/players/add",
        data={"roster_line_id": ligne, "version": version},
        headers={"HX-Request": "true"},
    )
    assert resp.status_code == 200, f"recrutement : {resp.status_code}"


def _ouvrir_renvois(page: Page, space_id: str, team_id: str) -> None:
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}/dismissals", wait_until="load")
    expect(page.locator(".dis-roster")).to_be_visible()
    expect(page.locator(".dis-cart")).to_be_visible()


def _ligne_du_maillot(page: Page, maillot: int):
    return page.locator(".dis-table tbody tr").filter(
        has=page.locator(f".col-num:text-is('{maillot}')")
    )


def _eligibles_affiches(page: Page) -> int:
    entete = page.locator(".dis-header-meta .meta-item").filter(has_text="Disponibles")
    return int(re.sub(r"[^0-9]", "", entete.locator(".meta-value").inner_text()))


def _marquer(page: Page, maillot: int) -> None:
    """Clique et attend que le panier ait grandi — le bouton est remplacé par le
    swap, l'attente ne peut donc pas porter sur lui.

    **Le clic attend le câblage.** Le tableau des renvois est lui-même du contenu
    inséré par htmx, et pendant quelques dizaines de millisecondes ses boutons
    sont peints, visibles et inertes : le clic s'y perd sans requête ni erreur.
    Le piège est documenté dans le `CLAUDE.md` ; il a fait tomber le scénario 3
    une fois sur une suite complète, en passant seul à chaque relance.
    """
    avant = page.locator(".dis-cart .recap-row").count()
    bouton = _ligne_du_maillot(page, maillot).locator(".fire-btn")
    attendre_cablage_locator(page, bouton)
    bouton.click()
    expect(page.locator(".dis-cart .recap-row")).to_have_count(avant + 1, timeout=10000)


# ── Fixture : une équipe de treize joueurs dont un absent ─────────────────────


@pytest.fixture(scope="module")
def renvois_ctx(browser, space_id):
    """Le décor complet, en un match et deux recrutements.

    La blessure est ce qui rend le plancher observable : sans elle, treize
    joueurs valides laisseraient deux renvois possibles au lieu d'un, et le
    scénario de l'absent n'aurait aucun sujet.
    """
    full = build_full_competition(browser, space_id, num_teams=2, num_rounds=3)
    home, away = full["team_ids"][0], full["team_ids"][1]

    blesse = _joueur_au_maillot(home, MAILLOT_BLESSE)
    _jouer_match(space_id, full, full["round_ids"][0], home, away, blesse=blesse)

    _attendre(
        lambda: query_db(
            f"SELECT participation_status FROM players_proj WHERE player_id = '{blesse}'"
        )
        == ["MissingNextGame"],
        "le blessé est absent au prochain match",
    )

    _attendre_phase(home, "PlayerImprovement")
    _valider_phase(space_id, home, "validate-improvement-phase")
    _attendre_phase(home, "Recruitment")

    # Deux Percuteurs : ils portent la valeur qui rendra le scénario 8
    # discriminant, en se distinguant des Piétailles.
    _recruter(space_id, home, PERCUTEUR, version=0)
    _recruter(space_id, home, PERCUTEUR, version=1)
    _valider_phase(space_id, home, "validate-recruitment-phase")
    _attendre_phase(home, "Dismissals")
    _attendre(lambda: len(_actifs(home)) == 13, "les deux recrues ont rejoint l'effectif")

    return {
        "space_id": space_id,
        "home": home,
        "away": away,
        "full": full,
        "blesse": blesse,
    }


# ── 1 · La bannière ouvre la page, l'effectif est listé ───────────────────────


def test_01_banniere_ouvre_l_effectif(page: Page, renvois_ctx):
    space_id, team_id = renvois_ctx["space_id"], renvois_ctx["home"]
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")

    banniere = page.locator(".state-banner--phase")
    expect(banniere).to_contain_text("Phase de renvois")

    # `<a hx-get hx-push-url>` : HTMX échange `#app-content`, il n'y a pas de
    # navigation du navigateur à attendre.
    banniere.locator(".state-banner-cta").click()
    expect(page.locator(".dis-roster")).to_be_visible(timeout=10000)
    assert "/dismissals" in page.url

    expect(page.locator(".dis-table tbody tr")).to_have_count(13)

    # Chaque ligne porte ce qui permet de décider : SPP, valeur, disponibilité.
    ligne = _ligne_du_maillot(page, MAILLOT_RENVOYE)
    expect(ligne.locator(".pl-value")).to_contain_text(f"{PIETAILLE_KPO} kPo")
    expect(ligne.locator(".pl-spp")).to_be_visible()
    expect(ligne.locator(".pl-status--ok")).to_have_text("Disponible")

    absent = _ligne_du_maillot(page, MAILLOT_BLESSE)
    expect(absent.locator(".pl-status--miss")).to_have_text("Absent")

    # Douze éligibles sur treize joueurs : c'est la blessure qui fait la
    # différence, et c'est elle qui rend le plancher observable.
    assert _eligibles_affiches(page) == 12


# ── 2 · Marquer un joueur ─────────────────────────────────────────────────────


def test_02_marquer_barre_la_ligne_et_remplit_le_panier(page: Page, renvois_ctx):
    space_id, team_id = renvois_ctx["space_id"], renvois_ctx["home"]
    _ouvrir_renvois(page, space_id, team_id)

    expect(page.locator(".dis-cart .recap-empty")).to_be_visible()
    _marquer(page, MAILLOT_RENVOYE)

    ligne = _ligne_du_maillot(page, MAILLOT_RENVOYE)
    expect(ligne).to_have_class(re.compile(r"\bis-gone\b"))
    expect(ligne.locator(".fire-btn")).to_have_text("Annuler")

    expect(page.locator(".dis-cart .recap-row")).to_have_count(1)
    expect(page.locator(".dis-cart .cta-primary")).to_contain_text("Valider 1 renvoi")
    expect(page.locator(".dis-cart .cta-primary")).to_have_class(
        re.compile(r"cta-primary--destructive")
    )

    # Rien n'est retiré tant que la phase n'est pas validée.
    assert len(_actifs(team_id)) == 13
    assert _eligibles_affiches(page) == 11, "le marqué ne compte plus parmi les éligibles"


# ── 3 · Annuler depuis la ligne, puis depuis le panier ────────────────────────


def test_03_les_deux_chemins_d_annulation_fonctionnent(page: Page, renvois_ctx):
    space_id, team_id = renvois_ctx["space_id"], renvois_ctx["home"]
    _ouvrir_renvois(page, space_id, team_id)

    # Depuis la ligne du joueur — le chemin que le recrutement n'a pas.
    _ligne_du_maillot(page, MAILLOT_RENVOYE).locator(".fire-btn").click()
    expect(page.locator(".dis-cart .recap-empty")).to_be_visible(timeout=10000)
    expect(page.locator(".dis-table tbody tr.is-gone")).to_have_count(0)

    # Depuis le panier.
    _marquer(page, MAILLOT_RENVOYE)
    page.locator(".dis-cart .cart-remove").first.click()
    expect(page.locator(".dis-cart .recap-empty")).to_be_visible(timeout=10000)
    expect(page.locator(".dis-table tbody tr.is-gone")).to_have_count(0)
    assert _eligibles_affiches(page) == 12, "démarquer rend l'éligible"


# ── 10 · Le panier survit à un changement de page ─────────────────────────────


def test_10_le_panier_survit_a_la_navigation(page: Page, renvois_ctx):
    """Le test qui distingue un panier serveur d'un panier client. La navigation
    est réelle — fiche d'équipe puis retour — parce qu'un panier client survit à
    certains rechargements."""
    space_id, team_id = renvois_ctx["space_id"], renvois_ctx["home"]
    _ouvrir_renvois(page, space_id, team_id)
    _marquer(page, MAILLOT_RENVOYE)

    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}", wait_until="load")
    expect(page.locator(".team-status-badge")).to_be_visible()

    _ouvrir_renvois(page, space_id, team_id)
    expect(page.locator(".dis-cart .recap-row")).to_have_count(1)
    expect(_ligne_du_maillot(page, MAILLOT_RENVOYE)).to_have_class(re.compile(r"\bis-gone\b"))

    page.locator(".dis-cart .cart-remove").first.click()
    expect(page.locator(".dis-cart .recap-empty")).to_be_visible(timeout=10000)


# ── 11 · Mobile 390px ─────────────────────────────────────────────────────────


def test_11_panier_repliable_et_avertissement_court_en_mobile(page: Page, renvois_ctx):
    """Le viewport est posé **avant** la navigation : le repliement est un
    `x-data` Alpine dont l'`init()` lit `window.innerWidth` au montage."""
    space_id, team_id = renvois_ctx["space_id"], renvois_ctx["home"]
    page.set_viewport_size({"width": 390, "height": 844})
    try:
        _ouvrir_renvois(page, space_id, team_id)

        # L'avertissement passe en version courte : la longue occupait un quart
        # du premier écran avant qu'on voie le moindre joueur.
        expect(page.locator(".dis-roster .warn-long")).to_be_hidden()
        expect(page.locator(".dis-roster .warn-short")).to_be_visible()

        actions = page.locator(".dis-cart .side-actions")
        expect(actions).not_to_have_class(re.compile(r"\bis-open\b"))
        assert actions.evaluate("el => getComputedStyle(el).position") == "fixed"

        # Calé au-dessus de la tabbar globale : sans le décalage, la validation
        # passerait sous le menu du chrome mobile.
        tabbar = page.locator(".mobile-tabbar")
        expect(tabbar).to_be_visible()
        bas = actions.bounding_box()
        haut = tabbar.bounding_box()
        assert bas["y"] + bas["height"] <= haut["y"] + 1, "le panier chevauche la tabbar"

        page.locator(".dis-cart .cart-head").click()
        expect(actions).to_have_class(re.compile(r"\bis-open\b"))

        # Le panier **reste ouvert** après une mutation : c'est la mémoire posée
        # sur `document.body` par la carte 269. Sans elle, le swap remontait le
        # composant et l'`init()` refermait ce que le coach venait d'ouvrir.
        _marquer(page, MAILLOT_RENVOYE)
        expect(page.locator(".dis-cart .side-actions")).to_have_class(
            re.compile(r"\bis-open\b")
        )

        retirer = page.locator(".dis-cart .cart-remove").first
        expect(retirer).to_be_visible()
        retirer.click()
        expect(page.locator(".dis-cart .cart-remove")).to_have_count(0, timeout=10000)
    finally:
        page.set_viewport_size({"width": 1280, "height": 900})


# ── 4 et 5 · Le plancher, et l'absent qui y échappe ───────────────────────────


def test_04_au_plancher_tous_les_disponibles_se_bloquent_ensemble(page: Page, renvois_ctx):
    """La seule vraie subtilité de la page, et la seule chose qu'un test unitaire
    ne montre pas : les boutons basculent **tous ensemble** au moment où
    l'avant-dernier renvoi passe."""
    space_id, team_id = renvois_ctx["space_id"], renvois_ctx["home"]
    _ouvrir_renvois(page, space_id, team_id)

    assert _eligibles_affiches(page) == 12
    disponibles = page.locator(".dis-table tbody tr").filter(has=page.locator(".pl-status--ok"))
    expect(disponibles.locator(".fire-btn")).to_have_count(12)
    for i in range(12):
        expect(disponibles.nth(i).locator(".fire-btn")).to_be_enabled()

    _marquer(page, MAILLOT_RENVOYE)

    assert _eligibles_affiches(page) == 11
    restants = page.locator(".dis-table tbody tr").filter(
        has=page.locator(".pl-status--ok")
    ).filter(has_not=page.locator(".fire-btn--undo"))
    expect(restants).to_have_count(11)
    for i in range(11):
        bouton = restants.nth(i).locator(".fire-btn")
        expect(bouton).to_be_disabled()
        expect(bouton).to_have_text("Minimum 11")


def test_05_l_absent_reste_renvoyable_sous_le_plancher(page: Page, renvois_ctx):
    """Un absent ne compte pas parmi les éligibles : le renvoyer n'entame pas le
    plancher. C'est la nuance que la page doit rendre, et elle seule empêche une
    équipe amoindrie par les blessures de se retrouver bloquée."""
    space_id, team_id = renvois_ctx["space_id"], renvois_ctx["home"]
    _ouvrir_renvois(page, space_id, team_id)

    assert _eligibles_affiches(page) == 11, "on est au plancher"

    absent = _ligne_du_maillot(page, MAILLOT_BLESSE)
    expect(absent.locator(".pl-status--miss")).to_have_text("Absent")
    expect(absent.locator(".fire-btn")).to_be_enabled()
    expect(absent.locator(".fire-btn")).to_have_text("Renvoyer")

    _marquer(page, MAILLOT_BLESSE)
    assert _eligibles_affiches(page) == 11, "renvoyer un absent n'entame pas le plancher"
    expect(page.locator(".dis-cart .cta-primary")).to_contain_text("Valider 2 renvois")


# ── 6, 7 et 8 · La validation ─────────────────────────────────────────────────


def test_06_valider_retire_les_joueurs_sans_toucher_la_tresorerie(page: Page, renvois_ctx):
    space_id, team_id = renvois_ctx["space_id"], renvois_ctx["home"]
    _ouvrir_renvois(page, space_id, team_id)

    caisse_avant = _tresorerie(team_id)
    renvois_ctx["grand_livre_avant"] = _grand_livre(team_id)
    assert renvois_ctx["grand_livre_avant"], "le grand livre n'est pas vide avant validation"
    renvoye = _joueur_au_maillot(team_id, MAILLOT_RENVOYE)
    blesse = renvois_ctx["blesse"]

    # Le panier est réinjecté à chaque ajout : ce bouton tombe dans la fenêtre
    # où htmx ne l'a pas encore câblé. Ce test n'a pas échoué, mais il portait la
    # même fragilité que celle mesurée dans `test_roster_edition`.
    cliquer_quand_cable(page, ".dis-cart .cta-primary")
    # **Plus `ReadyToPlay` depuis l'épic E13** : au-dessus de 100 kPo la
    # validation ouvre la phase des erreurs coûteuses, et cette équipe a encaissé
    # le gain de match par défaut. C'est ce qui rend l'assertion de trésorerie
    # ci-dessous plus forte qu'avant : le jet n'a pas encore eu lieu, donc si la
    # caisse a bougé, c'est bien le renvoi qui l'a fait.
    _attendre_phase(team_id, "CostlyMistakes")

    _attendre(
        lambda: _appartenance(renvoye) == "Dismissed" and _appartenance(blesse) == "Dismissed",
        "les deux renvoyés ont quitté l'effectif",
    )
    assert len(_actifs(team_id)) == 11, "treize moins deux"

    assert _tresorerie(team_id) == caisse_avant, "un renvoi ne rembourse rien"


def test_07_le_grand_livre_ne_gagne_aucune_ligne(renvois_ctx):
    """Vérifier une absence semble faible. C'est pourtant le seul test qui prouve
    que « un renvoi ne rembourse rien » tient de bout en bout — du `match`
    exhaustif de `treasury_movement()` jusqu'au grand livre en base.
    """
    team_id = renvois_ctx["home"]
    avant = renvois_ctx["grand_livre_avant"]
    apres = _grand_livre(team_id)

    # Comparer les lignes elles-mêmes, pas seulement leur nombre : une ligne
    # remplacée par une autre passerait un simple comptage.
    assert apres == avant, (
        f"le grand livre a bougé : {len(apres) - len(avant)} ligne(s) — "
        f"{[l for l in apres if l not in avant]}"
    )


def test_08_la_valeur_d_equipe_exclut_les_renvoyes(renvois_ctx):
    """Test de non-régression sur une course (carte 270).

    Le recalcul déclenché par `DismissalsPhaseValidated` part du bus interne de
    `teams` pendant que la sortie d'effectif traverse l'app event bus vers
    `players` : il lit systématiquement avant que `players` n'écrive. Seul le
    second recalcul, déclenché par l'annonce de `players`, donne la bonne valeur.

    Les trois issues donnent trois nombres **distincts**, et c'est voulu — avec
    des valeurs identiques entre recrue et renvoyé, « pas de recalcul » et
    « recalcul juste » se confondraient et le test ne prouverait rien.
    """
    team_id = renvois_ctx["home"]

    # Neuf Piétailles disponibles à 50, deux Percuteurs à 90, onze disponibles
    # donc aucun journalier à compter. Ni staff ni relance dans cette équipe.
    attendue = 9 * PIETAILLE_KPO + 2 * PERCUTEUR_KPO
    perimee = 11 * PIETAILLE_KPO  # la TV figée depuis l'inscription
    avec_renvoyes = 10 * PIETAILLE_KPO + 2 * PERCUTEUR_KPO
    assert len({attendue, perimee, avec_renvoyes}) == 3, "le test doit discriminer"

    _attendre(
        lambda: _valeur_equipe(team_id) == attendue,
        f"valeur d'équipe à {attendue} (périmée {perimee}, avec renvoyés {avec_renvoyes})",
    )

    # Stable : le premier recalcul, prématuré, ne doit pas reprendre la main.
    for i in range(5):
        time.sleep(0.3)
        assert _valeur_equipe(team_id) == attendue, f"relecture {i} instable"


# ── 9 · Le maillot libéré, réattribué à la séquence suivante ──────────────────


def test_09_le_maillot_libere_est_reattribue_a_la_sequence_suivante(renvois_ctx):
    """Un joueur `Dismissed` n'est plus lu par les recherches d'effectif : son
    numéro cesse d'être occupé. L'ordre des phases fait qu'il ne redevient
    utilisable qu'à la **séquence suivante** — recrutement puis renvois — ce qui
    est cohérent avec « on ne libère pas une place pour recruter dans la même
    séquence ».
    """
    ctx = renvois_ctx
    space_id, home, away = ctx["space_id"], ctx["home"], ctx["away"]

    maillots = {int(l.split("|")[0]) for l in _actifs(home)}
    assert MAILLOT_RENVOYE not in maillots, "le numéro est libre"

    # Le jet des erreurs coûteuses sépare désormais les renvois du match suivant.
    # Ce qu'il laisse en caisse est aléatoire — jusqu'à ne laisser que 20 kPo —
    # mais sans conséquence ici : le match qui suit rapporte 50 000 kPo, et le
    # recrutement n'a lieu qu'après.
    traverser_erreurs_couteuses(space_id, home)

    _jouer_match(space_id, ctx["full"], ctx["full"]["round_ids"][1], home, away)
    _attendre_phase(home, "PlayerImprovement")
    _valider_phase(space_id, home, "validate-improvement-phase")
    _attendre_phase(home, "Recruitment")

    _recruter(space_id, home, PIETAILLE, version=0)
    _valider_phase(space_id, home, "validate-recruitment-phase")

    _attendre(lambda: len(_actifs(home)) == 12, "la recrue a rejoint l'effectif")
    repris = {int(l.split("|")[0]) for l in _actifs(home)}
    assert MAILLOT_RENVOYE in repris, (
        f"le maillot {MAILLOT_RENVOYE} devait être réattribué, maillots portés : {sorted(repris)}"
    )

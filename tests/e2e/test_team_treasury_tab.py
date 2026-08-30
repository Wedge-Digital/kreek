"""Tests E2E — l'onglet Trésorerie de la fiche d'équipe (carte 437).

Ce que ces scénarios prouvent, et qu'aucun test unitaire ne voit : le câblage
des onglets, le rendu d'une URL collée, et surtout **la concordance des deux
chemins vers le solde**.

Le montage construit une seule compétition et lui fait porter les quatre formes
de ligne que l'écran sait rendre. Trois contraintes l'expliquent :

- **La paire jouée doit être une paire du calendrier.** Le contexte de match
  vient de `competition_match_display_proj`, que le listener n'alimente que si
  la paire correspond à un `competition_match_day_pairings`. Une paire arbitraire
  donnerait un relevé sans titre de journée, et les tests passeraient sans rien
  prouver de ce qui compte.
- **Les coups de pouce sont achetés par le camp le plus fort.** C'est la seule
  ligne du grand livre qui porte un identifiant de rapport, donc la seule qui
  puisse ouvrir une période. Et l'underdog paie sur sa petite monnaie, qui ne
  sort d'aucune caisse et n'écrit aucune ligne : acheter pour lui ne produirait
  rien. Les quatre équipes partageant le même roster, leurs valeurs sont égales
  et la petite monnaie du premier acheteur est nulle — il paie donc de sa poche.
- **Une équipe de la seconde paire n'est jamais jouée**, et sert l'état vide :
  sa dotation, et rien d'autre.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) et base seedée.
"""

import json
import re

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from htmx_helpers import cliquer_quand_cable
from match_report_helpers import create_draft, ensure_pre_match, post_step5, publish
from team_phase_helpers import attendre_une_phase

# Un seul roster pour les quatre équipes : la ligne de recrutement est alors
# connue d'avance, et les valeurs d'équipe sont égales.
ROSTER = "DEMO_GRANIT"
PIETAILLE = "DEMO_GRANIT__PIETAILLE"
PIETAILLE_POSTE = "Piétaille des Carrières"

# Corpus de démonstration (`assets/references.example/inducements_fr.json`).
MASSEUR = "DEMO_MASSEUR_DOUTEUX"  # 30 kPo
COUPS_DE_POUCE_KPO = 30

GAIN_MATCH_KPO = 200
GAIN_ADVERSE_KPO = 100


# ── Lectures ──────────────────────────────────────────────────────────────────


def _valeur_equipe(team_id: str) -> int:
    return int(query_db(f"SELECT team_value FROM team_proj WHERE team_id = '{team_id}'")[0])


def _kpo(texte: str) -> int:
    """Le nombre d'un libellé — « 565 kPo », « 565kPo », « −90 kPo »."""
    m = re.search(r"-?\d+", texte.replace("−", "-"))
    assert m, f"aucun nombre dans « {texte} »"
    return int(m.group())


# ── Écritures ─────────────────────────────────────────────────────────────────


def _valider_phase(space_id: str, team_id: str, route: str) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{team_id}/{route}",
        headers={"HX-Request": "true"},
    )
    assert resp.status_code == 200, f"{route} : {resp.status_code} — {resp.text[:200]}"


def _recruter(space_id: str, team_id: str, ligne: str, version: int) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{team_id}/recruitment/players/add",
        data={"roster_line_id": ligne, "version": version},
        headers={"HX-Request": "true"},
    )
    assert resp.status_code == 200, (
        f"recrutement de {ligne} : {resp.status_code} — {resp.text[:200]}"
    )


def _acheter_coups_de_pouce(space_id: str, mr_id: str, team_id: str) -> None:
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step2",
        data={"home_fan_roll": "2", "away_fan_roll": "3"},
        allow_redirects=False,
    )
    assert resp.status_code in (302, 303), f"facteur fans : {resp.status_code}"

    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/inducements/{team_id}",
        data={
            "intent": "buy",
            "selection": json.dumps([{"uid": MASSEUR, "qty": 1}]),
            "mercenaries": "[]",
        },
        allow_redirects=False,
    )
    assert resp.status_code in (200, 302, 303), f"achats : {resp.status_code}"


# ── Montage ───────────────────────────────────────────────────────────────────


@pytest.fixture(scope="module")
def tresorerie_ctx(browser, space_id):
    full = build_full_competition(
        browser, space_id, num_teams=4, num_rounds=1,
        roster_uids=[ROSTER] * 4,
    )
    round_id = full["round_ids"][0]
    paires = query_db(
        "SELECT home_team_id || '|' || away_team_id FROM competition_match_day_pairings "
        f"WHERE match_day_id = '{round_id}' ORDER BY id"
    )
    assert len(paires) >= 2, (
        f"deux appariements attendus pour disposer d'une équipe jamais jouée, {len(paires)} obtenus"
    )
    jouee, autre = paires[0].split("|"), paires[1].split("|")

    # L'acheteur est le camp le plus fort — le seul dont la petite monnaie est
    # nulle, donc le seul dont l'achat sort vraiment de la caisse.
    equipe, adverse = sorted(jouee, key=_valeur_equipe, reverse=True)
    equipe_neuve = autre[0]

    mr_id = create_draft(space_id, full, round_id, jouee[0], jouee[1])
    ensure_pre_match(space_id, mr_id, full, round_id, jouee[0], jouee[1])
    _acheter_coups_de_pouce(space_id, mr_id, equipe)
    post_step5(space_id, mr_id, home_gain=GAIN_MATCH_KPO, away_gain=GAIN_ADVERSE_KPO)
    publish(space_id, mr_id)

    # Un recrutement réel : c'est lui qui exerce la jointure vers l'événement,
    # dont le `player_id` est résolu en nom par le port effectif.
    attendre_une_phase(equipe, {"PlayerImprovement"})
    _valider_phase(space_id, equipe, "validate-improvement-phase")
    attendre_une_phase(equipe, {"Recruitment"})
    _recruter(space_id, equipe, PIETAILLE, 0)
    _valider_phase(space_id, equipe, "validate-recruitment-phase")
    attendre_une_phase(equipe, {"Dismissals"})

    return {
        "space_id": space_id,
        "equipe": equipe,
        "adverse": adverse,
        "equipe_neuve": equipe_neuve,
        "nom_adverse": query_db(
            f"SELECT team_name FROM team_proj WHERE team_id = '{adverse}'"
        )[0],
    }


def _url_equipe(ctx: dict, team_id: str) -> str:
    return f"{BASE_URL}/app/{ctx['space_id']}/teams/{team_id}"


# ── Scénarios ─────────────────────────────────────────────────────────────────


def test_l_onglet_tresorerie_s_ouvre_et_affiche_le_releve(page: Page, tresorerie_ctx):
    """Le câblage des onglets, qui n'existait pas avant la carte 436.

    Le relevé arrive par `hx-get` : l'onglet est peint avant d'être câblé, et un
    clic tombé dans cette fenêtre ne produirait rien du tout.
    """
    page.goto(_url_equipe(tresorerie_ctx, tresorerie_ctx["equipe"]), wait_until="load")
    cliquer_quand_cable(page, ".team-tabs a:has-text('Trésorerie')")

    expect(page.locator(".team-treasury")).to_be_visible(timeout=10000)
    expect(page.locator(".tr-table")).to_be_visible()
    expect(page.locator(".team-tabs .tab.active")).to_have_text("Trésorerie")

    # Le titre de période vient du contexte de match — c'est lui qui prouve que
    # la jointure vers `competition_match_display_proj` a abouti.
    expect(page.locator(".tr-row--sep")).to_contain_text(tresorerie_ctx["nom_adverse"])

    # L'URL suit le swap : un lien collé après coup doit mener au même écran.
    assert page.url.endswith("/tresorerie"), f"hx-push-url n'a pas suivi : {page.url}"


def test_l_url_de_tresorerie_se_charge_directement(page: Page, tresorerie_ctx):
    """Un lien collé rend la **page entière**, pas le fragment nu.

    C'est l'autre moitié de l'aiguillage `HX-Request` : sans en-tête htmx, la
    même route doit envelopper le relevé dans la fiche d'équipe.
    """
    page.goto(
        _url_equipe(tresorerie_ctx, tresorerie_ctx["equipe"]) + "/tresorerie",
        wait_until="load",
    )

    expect(page.locator(".team-treasury")).to_be_visible(timeout=10000)
    # L'enveloppe : en-tête d'équipe et bandeau d'onglets, que le fragment nu
    # n'aurait pas.
    expect(page.locator(".team-header-name")).to_be_visible()
    expect(page.locator(".team-tabs .tab.active")).to_have_text("Trésorerie")


def test_le_solde_du_releve_egale_celui_de_l_en_tete(page: Page, tresorerie_ctx):
    """**Le test qui vaut le prix de la suite.**

    L'en-tête affiche la trésorerie lue depuis l'agrégat ; le relevé affiche le
    solde de la dernière ligne du grand livre. Ce sont deux chemins vers la même
    vérité, et c'est le seul endroit de l'application où ils sont côte à côte à
    l'écran.

    Une divergence signifierait que le grand livre a décroché de l'agrégat — ce
    que la transaction commune de l'append est censée empêcher. Rien n'est relu
    en base ici : y ajouter une troisième source affaiblirait le test, puisque
    c'est la confrontation de ces deux-là qui a de la valeur.
    """
    page.goto(
        _url_equipe(tresorerie_ctx, tresorerie_ctx["equipe"]) + "/tresorerie",
        wait_until="load",
    )
    expect(page.locator(".team-treasury")).to_be_visible(timeout=10000)

    en_tete = page.locator(".meta-item").filter(has_text="Trésorerie").locator(".meta-value")
    expect(en_tete).to_have_count(1)

    assert _kpo(en_tete.inner_text()) == _kpo(page.locator(".tr-balance-value").inner_text()), (
        f"l'agrégat dit « {en_tete.inner_text()} », le grand livre "
        f"« {page.locator('.tr-balance-value').inner_text()} »"
    )

    # Et l'équation du bandeau tombe juste : dotation + encaissé − dépensé.
    termes = [_kpo(t) for t in page.locator(".tr-term-value").all_inner_texts()]
    dotation, encaisse, depense, solde = termes
    assert dotation + encaisse - depense == solde, f"équation fausse : {termes}"


def test_une_equipe_neuve_affiche_le_bloc_sans_mouvement(page: Page, tresorerie_ctx):
    """Une équipe qui vient d'être créée : sa dotation, et rien d'autre.

    Le bandeau reste rempli — masquer le solde avec le tableau reviendrait à
    dire qu'elle n'a rien.
    """
    page.goto(
        _url_equipe(tresorerie_ctx, tresorerie_ctx["equipe_neuve"]) + "/tresorerie",
        wait_until="load",
    )

    expect(page.locator(".tr-empty-title")).to_have_text("Aucun mouvement pour l'instant")
    expect(page.locator(".tr-table")).to_have_count(0)
    assert _kpo(page.locator(".tr-balance-value").inner_text()) > 0, "la dotation doit s'afficher"


def test_le_releve_montre_le_recrutement_qui_vient_d_etre_fait(page: Page, tresorerie_ctx):
    """La jointure vers l'événement, de bout en bout.

    Le détail de la ligne ne vient ni du grand livre ni de la projection : le
    `player_id` est lu dans le payload de l'événement, puis résolu en nom par le
    port de l'effectif. Une ligne muette signalerait une rupture de cette chaîne.
    """
    page.goto(
        _url_equipe(tresorerie_ctx, tresorerie_ctx["equipe"]) + "/tresorerie",
        wait_until="load",
    )
    expect(page.locator(".team-treasury")).to_be_visible(timeout=10000)

    ligne = page.locator(".tr-table tbody tr").filter(has_text="Recrutement")
    expect(ligne).to_have_count(1)
    expect(ligne.locator(".tr-detail")).to_contain_text(PIETAILLE_POSTE)
    assert _kpo(ligne.locator(".tr-amount").inner_text()) < 0, "un recrutement sort de la caisse"


def test_l_onglet_joueurs_reste_accessible_apres_un_aller_retour(page: Page, tresorerie_ctx):
    """La régression que le découpage en onglets peut créer.

    **L'onglet souligné est vérifié, pas seulement le contenu.** C'est le défaut
    trouvé à l'écran sur la carte 436 : le contenu basculait, le soulignement
    non, et l'effectif s'affichait sous un onglet « Trésorerie » actif. Un test
    qui ne regarderait que le tableau des joueurs passerait au vert sur cette
    application-là.
    """
    # `.players-widget` — la racine que le BC players rend — et non
    # `#players-widget`, qui est le conteneur d'attente : le widget arrive en
    # `hx-swap="outerHTML"` et l'`id` disparaît avec lui. Ni `.player-table`,
    # que l'effectif et le staff portent tous les deux.
    page.goto(_url_equipe(tresorerie_ctx, tresorerie_ctx["equipe"]), wait_until="load")
    expect(page.locator(".players-widget")).to_be_visible(timeout=10000)

    cliquer_quand_cable(page, ".team-tabs a:has-text('Trésorerie')")
    expect(page.locator(".team-treasury")).to_be_visible(timeout=10000)
    expect(page.locator(".players-widget")).to_have_count(0)

    # Le retour vise du contenu fraîchement injecté — la fenêtre où un élément
    # est peint, cliquable et inerte.
    cliquer_quand_cable(page, ".team-tabs a:has-text('Joueurs')")
    expect(page.locator(".players-widget")).to_be_visible(timeout=10000)
    expect(page.locator(".team-treasury")).to_have_count(0)
    expect(page.locator(".team-tabs .tab.active")).to_have_text("Joueurs & Staff")

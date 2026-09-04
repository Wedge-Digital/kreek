"""Tests E2E — disponibilité d'un joueur blessé en match (carte 225).

Reproduit le scénario complet en base réelle, à travers le vrai pipeline
d'app events :

1. Un joueur de l'équipe A subit une blessure sérieuse pendant le match N
2. Le rapport est publié
3. Le joueur doit être **absent au prochain match**
4. L'équipe A joue et publie le match N+1
5. Le joueur redevient **disponible** — après ce match-là, pas avant

Le bug corrigé par cette carte faisait échouer l'étape 3 : la conclusion du
match N restaurait la disponibilité du joueur qu'il venait lui-même de
blesser, annulant l'effet « absent au prochain match » pour toutes les
blessures subies en jeu.

Le statut est lu directement dans `players_proj` : c'est la projection que
consomment les écrans, et une désynchronisation entre agrégat et projection
passerait inaperçue si on interrogeait l'event store.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) — cf. README.
"""

import time

import pytest
import requests

from competition_lifecycle import BASE_URL
from db_helpers import query_db
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    first_player_id,
    post_step5,
    publish,
)


def _record_injury(space_id: str, mr_id: str, victim_id: str) -> None:
    """Enregistre une blessure sérieuse subie par un joueur de l'équipe
    **domicile**, infligée depuis le camp adverse (`step4`)."""
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step4/actions",
        data={
            "turn": "3",
            "player_id": victim_id,
            "player_type": "regular",
            "action_type": "BLESSE",
            "injury_type": "BLESSURE_SERIEUSE",
        },
    )
    assert resp.status_code == 200, f"blessure : {resp.status_code}\n{resp.text[:200]}"


def _participation_status(player_id: str) -> str | None:
    rows = query_db(
        f"SELECT participation_status FROM players_proj WHERE player_id = '{player_id}'"
    )
    return rows[0] if rows else None


def _wait_status(player_id: str, expected: str, timeout_s: int = 25) -> None:
    """Les impacts joueur transitent par l'app event bus : le statut n'est pas
    à jour au retour de la requête de publication."""
    deadline = time.time() + timeout_s
    last = None
    while time.time() < deadline:
        last = _participation_status(player_id)
        if last == expected:
            return
        time.sleep(0.3)
    raise AssertionError(
        f"statut attendu « {expected} » pour {player_id}, obtenu « {last} » après {timeout_s}s"
    )


@pytest.fixture(scope="module")
def availability_ctx(browser, space_id):
    from competition_lifecycle import build_full_competition

    full = build_full_competition(browser, space_id, num_teams=12)
    return {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_ids": full["round_ids"],
        "teams": full["team_ids"],
    }


def _play_and_publish(space_id, ctx, round_id, home_idx, away_idx, *, injure_home_player=None):
    home_team_id = ctx["teams"][home_idx]
    away_team_id = ctx["teams"][away_idx]
    mr_id = create_draft(space_id, ctx, round_id, home_team_id, away_team_id)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home_team_id, away_team_id)
    ensure_inducements(space_id, mr_id)

    if injure_home_player:
        _record_injury(space_id, mr_id, injure_home_player)

    post_step5(space_id, mr_id)
    publish(space_id, mr_id)
    return mr_id


def test_un_joueur_blesse_en_match_manque_le_match_suivant(space_id, availability_ctx):
    ctx = availability_ctx
    # Indices 6/7 puis 6/8 : l'équipe 6 joue les deux matchs, les autres paires
    # sont réservées aux autres modules de test.
    premier_round = ctx["round_ids"][0]
    second_round = ctx["round_ids"][1]

    mr1 = create_draft(space_id, ctx, premier_round, ctx["teams"][6], ctx["teams"][7])
    victime = first_player_id(mr1, "home")

    # ── Match N : le joueur est blessé ────────────────────────────────────────
    ensure_pre_match(space_id, mr1, ctx, premier_round, ctx["teams"][6], ctx["teams"][7])
    ensure_inducements(space_id, mr1)
    _record_injury(space_id, mr1, victime)
    post_step5(space_id, mr1)
    publish(space_id, mr1)

    # C'est l'assertion qui échouait avant le correctif de la carte 225 : la
    # conclusion du match N remettait le joueur disponible aussitôt blessé.
    _wait_status(victime, "MissingNextGame")

    # ── Match N+1 : le joueur redevient disponible ────────────────────────────
    _play_and_publish(space_id, ctx, second_round, home_idx=6, away_idx=8)

    _wait_status(victime, "Available")


# ── L'écran le dit-il ? (carte 489) ──────────────────────────────────────────


@pytest.fixture(scope="module")
def equipe_avec_un_blesse(space_id, availability_ctx):
    """Une équipe dont un joueur manque le prochain match, et lui seul.

    Indices 9/10 : les paires 6/7 et 6/8 servent au test de disponibilité de ce
    module, et rejouer sur elles remettrait le blessé disponible — c'est
    justement ce que ce module vérifie par ailleurs.
    """
    ctx = availability_ctx
    mr = create_draft(space_id, ctx, ctx["round_ids"][2], ctx["teams"][9], ctx["teams"][10])
    victime = first_player_id(mr, "home")
    ensure_pre_match(space_id, mr, ctx, ctx["round_ids"][2], ctx["teams"][9], ctx["teams"][10])
    ensure_inducements(space_id, mr)
    _record_injury(space_id, mr, victime)
    post_step5(space_id, mr)
    publish(space_id, mr)
    _wait_status(victime, "MissingNextGame")
    return {"team_id": ctx["teams"][9], "player_id": victime}


def _feuille(space_id: str, team_id: str) -> str:
    return f"{BASE_URL}/app/{space_id}/teams/{team_id}"


def test_le_joueur_indisponible_est_barre_dans_la_liste(page, space_id, equipe_avec_un_blesse):
    """**La donnée existait, l'écran la taisait.**

    `participation_status` vit dans la projection depuis toujours et le dépôt le
    lit ; il s'arrêtait au view model. Un joueur qui manquera le prochain match
    se lisait donc exactement comme un joueur disponible.

    L'assertion porte sur le **style calculé** : vérifier la présence de la
    classe passerait alors même qu'aucune règle ne l'atteint — c'est exactement
    ce qui est arrivé au bouton des points manuels (carte 487).
    """
    page.set_viewport_size({"width": 1440, "height": 900})
    page.goto(_feuille(space_id, equipe_avec_un_blesse["team_id"]), wait_until="load")

    lignes = page.locator("tr.player-absent")
    lignes.first.wait_for(state="attached", timeout=15000)
    assert lignes.count() == 1, f"{lignes.count()} lignes barrées, une seule attendue"

    mesures = lignes.first.evaluate(
        """tr => {
             const g = s => { const e = tr.querySelector(s);
                              return e ? getComputedStyle(e) : null; };
             const nom = g('.display-value'), poste = g('.player-position');
             const rep = tr.querySelector('.player-absence');
             const tag = tr.querySelector('.skill-tag');
             return { nom: nom.textDecorationLine, poste: poste.textDecorationLine,
                      cellule: getComputedStyle(tr.querySelector('td')).color,
                      repere: rep ? rep.textContent.trim() : null,
                      repereBarre: rep ? getComputedStyle(rep).textDecorationLine : null,
                      titre: rep ? rep.getAttribute('title') : null,
                      tagBarre: tag ? getComputedStyle(tag).textDecorationLine : 'aucune',
                      tagOpacite: tag ? getComputedStyle(tag).opacity : null };
           }"""
    )

    assert mesures["nom"] == "line-through", "le nom n'est pas barré"
    assert mesures["poste"] == "line-through", "le poste n'est pas barré"
    assert mesures["cellule"] != "rgb(38, 38, 61)", "la ligne n'a pas grisé"
    assert mesures["titre"] == "Manque le prochain match", f"repère : {mesures['titre']}"
    # Le repère explique la barre : le barrer le rendrait aussi douteux que le reste.
    assert mesures["repereBarre"] == "none", "le repère ne doit pas être barré"
    if mesures["tagBarre"] != "aucune":
        assert mesures["tagBarre"] == "none", "les pastilles pâlissent, elles ne se barrent pas"
        assert float(mesures["tagOpacite"]) < 1, "les pastilles n'ont pas pâli"


def test_les_autres_joueurs_ne_sont_pas_barres(page, space_id, equipe_avec_un_blesse):
    """La contre-épreuve. Sans elle, une règle qui barrerait *toute* la table
    passerait le test précédent — et le tableau entier serait rayé."""
    page.set_viewport_size({"width": 1440, "height": 900})
    page.goto(_feuille(space_id, equipe_avec_un_blesse["team_id"]), wait_until="load")

    page.locator("tr.player-table-row").first.wait_for(state="attached", timeout=15000)
    total = page.locator("tr.player-table-row").count()
    barrees = page.locator("tr.player-absent").count()
    assert total > 1, f"{total} joueur(s) — la contre-épreuve n'a rien à prouver"
    assert barrees == 1, f"{barrees} lignes barrées sur {total}"

    saine = page.locator("tr.player-table-row:not(.player-absent)").first
    decoration = saine.evaluate(
        "tr => getComputedStyle(tr.querySelector('.display-value')).textDecorationLine"
    )
    assert decoration == "none", "un joueur disponible ne doit pas être barré"


def test_le_repere_se_reduit_a_son_icone_en_mobile(page, space_id, equipe_avec_un_blesse):
    """Sous 768 px, le repère garde son sens et perd ses mots.

    **Ce que ce test ne prouve pas.** La cellule du nom déborde déjà en mobile,
    sans rapport avec cette carte : mesuré à 390 px, le nom occupe 158 px dans
    une cellule de 78 — le même dépassement de 88 px sur une ligne barrée et sur
    une ligne saine. Le repère s'ajoute derrière un contenu qui débordait avant
    lui, et le masquage du libellé **limite** cet ajout sans corriger le défaut
    de fond, qui mérite sa propre carte.

    Une première version de ce test asseyait « la page ne déborde pas
    latéralement ». C'était creux : la table vit dans un `overflow-x: auto`, donc
    l'assertion est vraie que le libellé soit affiché ou non — elle passait aussi
    bien avec le défaut qu'avec la correction.
    """
    page.set_viewport_size({"width": 390, "height": 844})
    page.goto(_feuille(space_id, equipe_avec_un_blesse["team_id"]), wait_until="load")

    page.locator("tr.player-absent").first.wait_for(state="attached", timeout=15000)
    mesures = page.evaluate(
        """() => {
             const rep = document.querySelector('.player-absence');
             const mot = document.querySelector('.player-absence-mot');
             return { largeur: window.innerWidth,
                      libelleMasque: getComputedStyle(mot).display === 'none',
                      largeurRepere: Math.round(rep.getBoundingClientRect().width),
                      titre: rep.getAttribute('title') };
           }"""
    )

    assert mesures["largeur"] <= 768, f"viewport à {mesures['largeur']} px"
    assert mesures["libelleMasque"], "le libellé doit s'effacer sous 768 px"
    # 160 px avec le libellé, 24 sans — mesuré. Le seuil laisse la marge d'une
    # icône plus large sans laisser repasser une phrase.
    assert mesures["largeurRepere"] < 45, (
        f"le repère fait {mesures['largeurRepere']} px : ce n'est plus une icône"
    )
    assert mesures["titre"] == "Manque le prochain match", (
        "le sens doit rester accessible — c'est ce qui rend le masquage acceptable"
    )

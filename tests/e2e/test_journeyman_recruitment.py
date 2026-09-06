"""Tests E2E — recruter un journalier (épic E15).

Ce module prouve une chaîne qui traverse **trois BCs et deux bus d'événements**,
et qu'aucun test unitaire ne voit d'un bout à l'autre :

    match_report  TempPlayersInitialized  →  JourneymenFielded   (app event)
    teams         JourneymanFielded       →  JourneymanFielded   (app event)
    players       PlayerCreated { starting_membership: Journeyman }

Puis, à la phase de recrutement : le panneau rendu par `players`, le panier de
`teams`, et la clôture qui perd ceux qu'on n'a pas gardés.

## Le préalable : une équipe à effectif incomplet

`collect_journeymen` ne crée un journalier que si l'équipe a **moins de onze
joueurs alignables**. Or une équipe ne peut pas naître incomplète —
`MIN_PLAYERS_FOR_SUBMISSION = 11` l'interdit à la soumission.

Le seul chemin est celui du jeu réel : un premier match où un joueur prend une
blessure sérieuse, publié. Il devient `MissingNextGame`, l'équipe n'a plus que
dix alignables au match suivant, et le journalier apparaît.

C'est pourquoi le fixture joue **deux** matchs. C'est aussi ce qui explique que
la suite n'ait jamais exercé un journalier avant cette carte : toutes ses
équipes ont un effectif complet.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) — cf. README.
"""

import time

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from htmx_helpers import cliquer_quand_cable_locator
from match_report_helpers import (
    create_draft,
    ensure_inducements,
    ensure_pre_match,
    post_step5,
    publish,
)
from playwright.sync_api import Page, expect
from team_phase_helpers import attendre_une_phase


def _blesser(space_id: str, mr_id: str, victime: str) -> None:
    """Une blessure sérieuse infligée au joueur domicile, depuis le camp adverse.

    Repris de `test_player_availability_after_injury`, qui l'a éprouvée : c'est
    la seule façon de rendre un joueur indisponible sans toucher la base.
    """
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


def _jouer(space_id, ctx, round_id, home_idx, away_idx, *, blesser=None) -> str:
    home = ctx["teams"][home_idx]
    away = ctx["teams"][away_idx]
    mr_id = create_draft(space_id, ctx, round_id, home, away)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home, away)
    ensure_inducements(space_id, mr_id)
    if blesser:
        _blesser(space_id, mr_id, blesser)
    post_step5(space_id, mr_id)
    publish(space_id, mr_id)
    return mr_id


def _un_joueur_de(team_id: str) -> str:
    lignes = query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{team_id}' "
        f"AND membership = 'Active' ORDER BY player_id LIMIT 1"
    )
    assert lignes, f"aucun joueur dans l'équipe {team_id}"
    return lignes[0]


def _journaliers_de(team_id: str) -> list[str]:
    return [
        l
        for l in query_db(
            f"SELECT player_id FROM players_proj WHERE team_id = '{team_id}' "
            f"AND membership = 'Journeyman' ORDER BY player_id"
        )
        if l
    ]


def _attendre(condition, quoi: str, timeout_s: int = 25):
    """Les app events traversent trois BCs de façon asynchrone : on attend le
    fait, jamais une durée. Rend la valeur dès qu'elle est vraie."""
    limite = time.time() + timeout_s
    while time.time() < limite:
        valeur = condition()
        if valeur:
            return valeur
        time.sleep(0.2)
    pytest.fail(f"jamais obtenu : {quoi}")


@pytest.fixture(scope="module")
def journalier_ctx(browser, space_id):
    """Une équipe qui a joué un match avec un journalier.

    Deux matchs : le premier blesse un titulaire, le second appelle donc un
    journalier pour compléter les onze. L'équipe 2 est le sujet ; l'équipe 3,
    complète, sert de contre-épreuve au panneau absent.
    """
    full = build_full_competition(browser, space_id, num_teams=4)
    ctx = {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_ids": full["round_ids"],
        "teams": full["team_ids"],
    }
    sujet = ctx["teams"][2]

    victime = _un_joueur_de(sujet)
    _jouer(space_id, ctx, ctx["round_ids"][0], 2, 3, blesser=victime)

    # Le second match : l'équipe n'a plus que dix alignables.
    mr2 = _jouer(space_id, ctx, ctx["round_ids"][1], 2, 3)

    journaliers = _attendre(
        lambda: _journaliers_de(sujet),
        "un journalier créé dans players pour l'équipe sujette",
    )
    return {
        "ctx": ctx,
        "space_id": space_id,
        "equipe": sujet,
        "equipe_complete": ctx["teams"][3],
        "victime": victime,
        "journaliers": journaliers,
        "mr2": mr2,
    }


# ── La chaîne, de bout en bout ───────────────────────────────────────────────


def test_un_journalier_apparait_apres_un_match(journalier_ctx):
    """La dette laissée par la carte 455, enfin payée.

    Elle n'avait pas pu écrire ce test : aucun événement ne produisait alors de
    journalier. C'est le premier passage réel de `match_report → teams →
    players`, et il prouve les trois maillons d'un coup.
    """
    journaliers = journalier_ctx["journaliers"]
    assert len(journaliers) == 1, (
        "un blessé sur onze appelle exactement un journalier — "
        f"obtenu {len(journaliers)}"
    )

    # Il est un joueur à part entière : il porte un maillot, pris parmi ceux
    # qui restaient libres.
    ligne = query_db(
        f"SELECT jersey, membership FROM players_proj WHERE player_id = '{journaliers[0]}'"
    )[0]
    jersey, membership = ligne.split("|")
    assert membership == "Journeyman"
    assert jersey and jersey != "", "un journalier reçoit un maillot"


def test_le_journalier_ne_prend_pas_le_maillot_d_un_coequipier(journalier_ctx):
    """`premier_libre` lit `jerseys_by_team_id`, la requête que la carte 454 a
    élargie. Sans elle, le journalier aurait repris un numéro déjà porté."""
    equipe = journalier_ctx["equipe"]
    numeros = [
        l.split("|")[0]
        for l in query_db(
            f"SELECT jersey FROM players_proj WHERE team_id = '{equipe}' "
            f"AND membership <> 'Dismissed' AND jersey IS NOT NULL"
        )
        if l
    ]
    assert len(numeros) == len(set(numeros)), f"maillots en double : {numeros}"


# ── L'écran de recrutement ───────────────────────────────────────────────────


def _amener_en_recrutement(space_id: str, team_id: str) -> None:
    """La publication du second match ouvre la phase d'amélioration ; on la
    valide pour atteindre le recrutement. Asynchrone, donc on attend le fait."""
    attendre_une_phase(team_id, {"PlayerImprovement", "Recruitment"})
    if _phase(team_id) == "PlayerImprovement":
        resp = requests.post(
            f"{BASE_URL}/app/{space_id}/teams/{team_id}/validate-improvement-phase",
            headers={"HX-Request": "true"},
            allow_redirects=False,
        )
        assert resp.status_code in (200, 302, 303), f"validate-improvement: {resp.status_code}"
    attendre_une_phase(team_id, {"Recruitment"})


def _phase(team_id: str) -> str | None:
    rows = query_db(f"SELECT game_phase FROM team_proj WHERE team_id = '{team_id}'")
    return rows[0] if rows else None


def _ouvrir_recrutement(page: Page, space_id: str, team_id: str) -> None:
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{team_id}/recruitment", wait_until="load")
    expect(page.locator(".rec-catalog")).to_be_visible()
    expect(page.locator(".rec-cart")).to_be_visible()


@pytest.fixture(scope="module")
def en_recrutement(journalier_ctx):
    """Les deux équipes du second match, amenées en phase de recrutement.

    L'équipe complète sert de contre-épreuve : sans elle, un panneau qui ne
    s'afficherait jamais passerait le test de l'absence.
    """
    space_id = journalier_ctx["space_id"]
    for equipe in (journalier_ctx["equipe"], journalier_ctx["equipe_complete"]):
        _amener_en_recrutement(space_id, equipe)
    return journalier_ctx


def test_le_panneau_montre_le_journalier(page: Page, en_recrutement):
    ctx = en_recrutement
    _ouvrir_recrutement(page, ctx["space_id"], ctx["equipe"])

    panneau = page.locator(".rec-journeymen")
    expect(panneau).to_be_visible(timeout=10000)
    expect(panneau.locator(".rec-journeyman-row")).to_have_count(1)
    # L'avertissement doit être là : c'est la seule différence de nature entre
    # ce panneau et le catalogue.
    expect(panneau).to_contain_text("perdu")


def test_le_panneau_est_absent_sans_journalier(page: Page, en_recrutement):
    """Le cas le plus fréquent — et la contre-épreuve du test précédent.

    L'équipe adverse n'a perdu personne : elle n'a appelé aucun journalier, et
    le panneau ne doit pas s'afficher. Un panneau vide poserait la question
    « qu'est-ce que j'ai raté ? » à chaque phase de recrutement.
    """
    ctx = en_recrutement
    _ouvrir_recrutement(page, ctx["space_id"], ctx["equipe_complete"])
    expect(page.locator(".rec-catalog")).to_be_visible()
    expect(page.locator(".rec-journeymen")).to_have_count(0)


def test_le_prix_du_journalier_est_affiche(page: Page, en_recrutement):
    """Le prix est celui du joueur, décomposé s'il a progressé.

    Le journalier de ce parcours n'a rien gagné — le rapport ne lui donne
    aucune action —, donc le prix est nu et l'amélioration dit « aucune ».
    C'est le cas majoritaire, et le seul que ce parcours produit sans forcer
    une action de match sur un joueur qui n'existait pas encore côté rapport.
    """
    ctx = en_recrutement
    _ouvrir_recrutement(page, ctx["space_id"], ctx["equipe"])

    ligne = page.locator(".rec-journeyman-row").first
    expect(ligne.locator(".rec-journeyman-prix-total")).to_contain_text("kPo")
    expect(ligne.locator(".rec-journeyman-amelioration")).to_contain_text("aucune")


def test_le_journalier_recrute_reste_dans_l_effectif(page: Page, en_recrutement):
    """**Le test qui vaut le prix de la suite.**

    Il traverse tout : la naissance à l'ouverture du rapport, le match, la
    publication, le recrutement au panier, la validation de phase — et vérifie
    qu'il est **toujours là** quand la phase est close.

    C'est le seul qui prouve que l'ordre du lot tient : le basculement en
    `Active` précède `RecruitmentPhaseValidated` dans le même lot. Si quelqu'un
    déplace un jour le ménage avant la validation, ce test échoue — alors que
    le code compilerait parfaitement et perdrait un joueur qu'on vient de payer.
    """
    ctx = en_recrutement
    equipe = ctx["equipe"]
    journalier = ctx["journaliers"][0]
    _ouvrir_recrutement(page, ctx["space_id"], equipe)

    cliquer_quand_cable_locator(page, page.locator(".rec-journeyman-btn").first)
    # Le panneau se recharge avec le catalogue : le journalier en sort.
    expect(page.locator(".rec-journeyman-row")).to_have_count(0, timeout=10000)

    cliquer_quand_cable_locator(page, page.locator(".rec-cart .cta-primary"))
    _attendre(
        lambda: _phase(equipe) in ("Dismissals", "CostlyMistakes", "ReadyToPlay"),
        "la phase de recrutement close",
    )

    membership = _attendre(
        lambda: (
            query_db(f"SELECT membership FROM players_proj WHERE player_id = '{journalier}'")
            or [None]
        )[0],
        "l'appartenance du journalier recruté",
    )
    assert membership == "Active", (
        "le journalier payé doit être devenu permanent, pas perdu par le ménage"
    )


def test_le_journalier_non_recrute_disparait(browser, space_id):
    """La décision 13, de bout en bout.

    Un second parcours complet, sur des équipes à part : celui du coach qui ne
    garde pas son journalier. À la clôture de la phase, il quitte l'effectif.
    """
    full = build_full_competition(browser, space_id, num_teams=4)
    ctx = {
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "round_ids": full["round_ids"],
        "teams": full["team_ids"],
    }
    equipe = ctx["teams"][0]

    victime = _un_joueur_de(equipe)
    _jouer(space_id, ctx, ctx["round_ids"][0], 0, 1, blesser=victime)
    _jouer(space_id, ctx, ctx["round_ids"][1], 0, 1)

    journaliers = _attendre(lambda: _journaliers_de(equipe), "un journalier créé")
    _amener_en_recrutement(space_id, equipe)

    # On ne le recrute pas : on clôt la phase telle quelle.
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/teams/{equipe}/validate-recruitment-phase",
        headers={"HX-Request": "true"},
        allow_redirects=False,
    )
    assert resp.status_code in (200, 302, 303), f"validate-recruitment: {resp.status_code}"

    membership = _attendre(
        lambda: (
            query_db(
                f"SELECT membership FROM players_proj WHERE player_id = '{journaliers[0]}'"
            )
            or [None]
        )[0]
        == "Dismissed",
        "le journalier non recruté sorti de l'effectif",
    )
    assert membership

    # Et il ne figure plus dans l'effectif lu par l'écran.
    restants = query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{equipe}' "
        f"AND membership <> 'Dismissed'"
    )
    assert journaliers[0] not in restants

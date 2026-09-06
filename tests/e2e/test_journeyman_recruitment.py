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


def _marquer(space_id: str, mr_id: str, player_id: str, *, temporaire: bool) -> None:
    """Un touchdown, sur un joueur régulier ou sur un remplaçant.

    `player_type` décide de la nature — et c'est lui qui, pour un journalier,
    fait la différence entre des SPP gagnés et des SPP perdus jusqu'à la
    carte 502.
    """
    resp = requests.post(
        f"{BASE_URL}/app/{space_id}/match-report/{mr_id}/step3/actions",
        data={
            "turn": "2",
            "player_id": player_id,
            "player_type": "temp" if temporaire else "regular",
            "action_type": "TOUCHDOWN",
        },
    )
    assert resp.status_code == 200, f"touchdown : {resp.status_code}\n{resp.text[:200]}"


def _jouer(
    space_id, ctx, round_id, home_idx, away_idx, *, blesser=None, td_du_journalier=False
) -> str:
    home = ctx["teams"][home_idx]
    away = ctx["teams"][away_idx]
    mr_id = create_draft(space_id, ctx, round_id, home, away)
    ensure_pre_match(space_id, mr_id, ctx, round_id, home, away)
    ensure_inducements(space_id, mr_id)
    if blesser:
        _blesser(space_id, mr_id, blesser)
    if td_du_journalier:
        # Les journaliers sont nés à l'enregistrement des coups de pouce : on
        # les retrouve dans l'effectif, et leur identifiant temporaire **est**
        # leur identifiant de joueur.
        journaliers = _attendre(
            lambda: _journaliers_de(home), "un journalier avant la saisie des actions"
        )
        _marquer(space_id, mr_id, journaliers[0], temporaire=True)
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

    panneau = page.locator(".panel--jm")
    expect(panneau).to_be_visible(timeout=10000)
    expect(panneau.locator("tbody tr")).to_have_count(1)
    # L'avertissement doit être là : c'est la seule différence de nature entre
    # ce panneau et le catalogue.
    expect(panneau).to_contain_text("perdu")

    # **Carte 503 — il porte les classes de la maison, pas les siennes.**
    # Vérifié ici et non dans un test à part : c'est le même écran au même
    # moment, et un second parcours de deux matchs pour l'observer coûterait
    # plus que ce qu'il prouve.
    expect(panneau.locator("table.buy-table")).to_have_count(1)
    expect(panneau.locator(".act-btn")).to_have_count(1)
    expect(panneau.locator(".price")).to_have_count(1)
    expect(page.locator("[class*='rec-journeyman']")).to_have_count(0)


def test_le_panneau_est_absent_sans_journalier(page: Page, en_recrutement):
    """Le cas le plus fréquent — et la contre-épreuve du test précédent.

    L'équipe adverse n'a perdu personne : elle n'a appelé aucun journalier, et
    le panneau ne doit pas s'afficher. Un panneau vide poserait la question
    « qu'est-ce que j'ai raté ? » à chaque phase de recrutement.
    """
    ctx = en_recrutement
    _ouvrir_recrutement(page, ctx["space_id"], ctx["equipe_complete"])
    expect(page.locator(".rec-catalog")).to_be_visible()
    expect(page.locator(".panel--jm")).to_have_count(0)


def test_le_prix_du_journalier_est_affiche(page: Page, en_recrutement):
    """Le prix est celui du joueur, décomposé s'il a progressé.

    Le journalier de ce parcours n'a gagné aucune action au match, donc son
    prix est nu — il vaut le tarif de son poste.

    **Sa colonne « Amélioration » affiche Solitaire (4+)**, et c'est l'attendu :
    le règlement le lui donne en naissant, et le coach doit voir ce qu'il porte
    avant de décider. Ce n'est pas un gain de match, mais c'est une compétence
    qu'il a.
    """
    ctx = en_recrutement
    _ouvrir_recrutement(page, ctx["space_id"], ctx["equipe"])

    ligne = page.locator(".panel--jm tbody tr").first
    expect(ligne.locator(".price")).to_contain_text("kPo")
    expect(ligne).to_contain_text("Solitaire")


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

    cliquer_quand_cable_locator(page, page.locator(".panel--jm .act-btn").first)
    # Le panneau se recharge avec le catalogue : le journalier en sort.
    expect(page.locator(".panel--jm")).to_have_count(0, timeout=10000)

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


# ── Le journalier gagne ses SPP (carte 502) ──────────────────────────────────


@pytest.fixture(scope="module")
def journalier_neuf(browser, space_id):
    """Un journalier que personne n'a encore embauché.

    Les scénarios de recrutement consomment celui de `journalier_ctx` — et
    l'embauche retire Solitaire, ce qui est précisément ce qu'on veut observer
    avant qu'elle n'ait lieu.
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
    return _attendre(lambda: _journaliers_de(equipe), "un journalier neuf")[0]


def test_le_journalier_marque_et_gagne_des_spp(browser, space_id):
    """**La dette de la carte 455, enfin payée.**

    Elle promettait que « les actions du rapport pointent le joueur réel ».
    C'était faux : le publisher écartait tout remplaçant, journaliers compris,
    par une règle antérieure qu'un test protégeait. Un journalier marquait deux
    touchdowns pour rien.

    Le test a été légué de la 455 à la 459, puis déclaré couvert par un scénario
    qui ne vérifiait que l'existence du joueur. Le voici.
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
    _jouer(space_id, ctx, ctx["round_ids"][1], 0, 1, td_du_journalier=True)

    journaliers = _attendre(lambda: _journaliers_de(equipe), "un journalier créé")

    # **Le barème n'est pas codé en dur** : il appartient à la compétition, et
    # varie de 2 à 6 SPP par touchdown selon celui qu'elle a choisi. Ce que ce
    # test prouve, c'est que le touchdown **atteint le joueur** — pas combien il
    # vaut, que les tests du barème couvrent ailleurs.
    spp = _attendre(
        lambda: int(
            (query_db(f"SELECT spp FROM players_proj WHERE player_id = '{journaliers[0]}'") or ["0"])[0]
        )
        or None,
        "les SPP du journalier après son touchdown",
    )
    assert spp > 0, "son touchdown doit lui rapporter des SPP"

    # Et l'événement porte son nom : c'est lui qui a marqué, pas un remplaçant
    # anonyme dont l'action serait restée dans le rapport.
    evenements = query_db(
        f"SELECT event_type FROM players_events WHERE player_id = '{journaliers[0]}' "
        f"ORDER BY version"
    )
    assert "TouchdownScored" in evenements, f"obtenu {evenements}"


def test_le_journalier_nait_avec_solitaire_et_un_nom(journalier_neuf):
    """Deux choses qu'un journalier doit porter dès sa naissance.

    **Solitaire (4+)** parce que le LRB le lui donne — et son absence est ce qui
    a fait douter du bon fonctionnement. **Un nom** parce que sans lui, deux
    journaliers d'un même poste sont indiscernables : l'affichage retombe sur
    le nom du poste.

    **Un journalier à part**, et non celui de `journalier_ctx` : les scénarios
    de recrutement l'embauchent, et l'embauche lui retire justement Solitaire.
    Le test aurait lu l'état d'après.
    """
    journalier = journalier_neuf
    ligne = query_db(
        f"SELECT personal_name, acquired_skills, jersey FROM players_proj "
        f"WHERE player_id = '{journalier}'"
    )[0]
    nom, competences, maillot = ligne.split("|", 2)
    maillot = maillot.strip()

    # **Une compétence acquise, pas de base** : le tableau d'effectif affiche
    # les compétences du poste, et un trait rangé dans `base_skills` n'aurait
    # eu aucun écran pour le lire (carte 504).
    assert "LONER_4" in competences, f"Solitaire (4+) manquant : {competences}"
    assert nom == f"Journalier #{maillot}", f"nommage attendu, obtenu {nom!r}"


def test_solitaire_s_affiche_sur_la_ligne_du_journalier(page: Page, journalier_neuf, space_id):
    """**Le test qui manquait à la carte 502.**

    Le sien vérifiait la présence du trait **en base**, jamais **à l'écran** —
    et il passait alors que personne ne le voyait, parce que le tableau
    d'effectif affiche les compétences du poste et non celles du joueur.

    C'est la même erreur que la carte 458 avait faite avec le style : vérifier
    ce qu'on a écrit plutôt que ce que le coach lit.
    """
    equipe = query_db(
        f"SELECT team_id FROM players_proj WHERE player_id = '{journalier_neuf}'"
    )[0]
    page.goto(f"{BASE_URL}/app/{space_id}/teams/{equipe}", wait_until="load")

    ligne = page.locator(f'.player-table-row[data-player-detail*="{journalier_neuf}"]')
    ligne.wait_for(timeout=10000)
    expect(ligne).to_contain_text("Solitaire")


def test_solitaire_porte_la_pastille_de_customisation(
    page: Page, journalier_neuf, space_id
):
    """**Le mode n'est pas qu'un libellé** — c'est le prix de la suivante.

    La carte 504 avait posé Solitaire en `Chosen`, et `est_une_amelioration()`
    s'en sert pour compter le niveau du joueur : le journalier recruté payait
    sa première vraie compétence un niveau plus cher, pour un trait que le
    règlement lui a donné (carte 505).

    La pastille est la seule trace **visible** de ce mode. Le test unitaire
    couvre le prix ; celui-ci couvre ce que le coach lit — et les deux se
    trompent ensemble si le mode redevient `Chosen`.
    """
    page.goto(
        f"{BASE_URL}/app/{space_id}/players/{journalier_neuf}/detail",
        wait_until="load",
    )

    ligne = page.locator(".spp-summary-table tbody tr", has_text="Solitaire")
    ligne.wait_for(timeout=10000)
    expect(ligne.locator(".mode-chip")).to_have_text("Customisation")


# ── L'affichage (carte 503) ──────────────────────────────────────────────────


@pytest.fixture(scope="module")
def saisie_en_cours(browser, space_id):
    """Un rapport **non publié**, arrêté juste après la naissance du journalier.

    Les deux sélecteurs de l'écran de saisie n'existent qu'en avant-match : sur
    un rapport publié, la page ne les rend pas. Le fixture s'arrête donc aux
    coups de pouce — le moment exact où le journalier vient de naître.
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

    # Le second match, laissé en avant-match.
    mr = create_draft(space_id, ctx, ctx["round_ids"][1], equipe, ctx["teams"][1])
    ensure_pre_match(space_id, mr, ctx, ctx["round_ids"][1], equipe, ctx["teams"][1])
    ensure_inducements(space_id, mr)

    journaliers = _attendre(lambda: _journaliers_de(equipe), "un journalier créé")
    return {"space_id": space_id, "mr": mr, "journalier": journaliers[0]}


def test_le_journalier_n_apparait_qu_une_fois_a_la_saisie(page: Page, saisie_en_cours):
    """Il est dans les joueurs réguliers ; il ne doit plus être aussi dans les
    remplaçants.

    Le sélecteur régulier lit l'effectif, que la carte 454 a ouvert aux
    journaliers. La section temporaire venait du rapport. Le coach voyait deux
    entrées pour un seul homme.
    """
    ctx = saisie_en_cours
    page.goto(
        f"{BASE_URL}/app/{ctx['space_id']}/match-report/{ctx['mr']}/step3",
        wait_until="load",
    )
    puce = page.locator(f'.mr-player-chip[data-player-id="{ctx["journalier"]}"]')
    expect(puce).to_have_count(1, timeout=10000)

    # Et la section des remplaçants ne le nomme plus.
    section = page.locator(".mr-temp-players")
    if section.count() > 0:
        expect(section).not_to_contain_text("Journalier #")



"""Ce que le découpage par panneau ne pouvait pas prouver (carte 426).

Les cartes 421 à 425 ont livré leur e2e chacune de son côté : renommage,
recalcul du barème, cascade des poules, collecte JS des coups de pouce,
préservation des invités. Ce fichier ne les rejoue pas — il porte les **trois
choses transverses** qu'aucun fichier de panneau ne pouvait porter seul.

1. **Les onze routes de l'onglet, paramétrées.** Un test de garde par fichier
   couvre son panneau ; aucun ne couvre la douzième route que quelqu'un
   ajoutera. Ici la liste est explicite, et un ajout non gardé s'y voit.

2. **Les deux chemins d'autorisation, séparés.** `require_admin_access` accepte
   par `SpaceProfile::SpaceAdmin` **ou** par l'appartenance à la compétition.
   Dans l'espace e2e, `DevCoach` est les deux à la fois : tous les tests
   positifs existants franchissent les deux portes ensemble, et une régression
   sur l'une seule ne se verrait nulle part.

3. **L'assemblage qui se remplit.** Le test de la carte 420 vérifie que les cinq
   conteneurs existent — il date de l'époque où ils étaient vides. Que les cinq
   `hx-get` soient câblés *ensemble* n'est vérifié nulle part.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import json

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import execute_db, query_db

HX = {"HX-Request": "true"}
MEMBRE_SIMPLE = {**HX, "X-Bypass-Auth-Profile": "simple"}

#: Le coach que `bypass_auth` connecte sur `X-Bypass-Auth-Profile: simple`.
#: `SpaceUser` dans l'espace e2e — c'est ce qui permet d'isoler le chemin
#: « admin de compétition sans être admin d'espace ».
COACH_SIMPLE = "E2E Coach 01"
COACH_DEV = "DevCoach"

PANNEAUX = ("general", "ranking", "pools", "tiers", "visibility")


def _base(space_id: str, ctx: dict) -> str:
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/admin/settings"
    )


def _coach_id(nom: str) -> str:
    return query_db(f"SELECT id FROM auth__users WHERE coach_name = '{nom}'")[0]


@pytest.fixture(scope="module")
def onglet(browser, space_id):
    ctx = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    return {"space_id": space_id, "ctx": ctx, "base": _base(space_id, ctx)}


# ── 1. Les onze routes ────────────────────────────────────────────────────────


def _corps_analysable(season_id: str) -> dict:
    """Un corps que l'extracteur accepte, pour chaque route.

    **Sans cela le test ne prouve rien.** Les extracteurs de corps d'axum
    s'exécutent *avant* le corps du handler, donc avant `require_admin_access` :
    un POST au corps vide rend `415` — pas `403` — et l'autorisation n'est
    jamais atteinte. Mesuré : les cinq POST rendaient `415` sur un `data={}`.

    C'est aussi pourquoi les assertions ci-dessous exigent **exactement** `403`.
    Un `415` signerait un corps non analysé, donc un test creux.
    """
    regles = json.loads(
        query_db(f"SELECT rules->'ranking_rules' FROM competition_seasons WHERE id = '{season_id}'")[0]
    )
    return {
        "general": ("data", {"name": "X", "season_name": "Y", "logo_url": ""}),
        "ranking": ("json", regles),
        "pools": ("data", {"use_pools": "false"}),
        "tiers": ("json", {"tiers": []}),
        "visibility": ("data", {"access_mode": "open", "requires_validation": "manual"}),
    }


@pytest.mark.parametrize("panneau", ("",) + PANNEAUX)
def test_un_membre_simple_est_refuse_en_lecture(onglet, panneau):
    """Les six `GET` : l'onglet et les cinq panneaux.

    Masquer l'onglet dans le menu est du confort — les URL restent devinables,
    et ce sont elles qui doivent refuser.
    """
    url = onglet["base"] + (f"/{panneau}" if panneau else "")

    refus = requests.get(url, headers=MEMBRE_SIMPLE, timeout=15)

    assert refus.status_code == 403, f"{panneau or 'onglet'} : {refus.status_code}"
    # Contre-épreuve : sans l'en-tête, la même URL passe. Sans elle, un 403 dû à
    # une route mal orthographiée se lirait comme un refus d'autorisation.
    admis = requests.get(url, headers=HX, timeout=15)
    assert admis.status_code == 200, f"{panneau or 'onglet'} (DevCoach) : {admis.status_code}"


@pytest.mark.parametrize("panneau", PANNEAUX)
def test_un_membre_simple_est_refuse_en_ecriture(onglet, panneau):
    """Les cinq `POST`. La lecture gardée ne dit rien de l'écriture."""
    genre, corps = _corps_analysable(onglet["ctx"]["season_id"])[panneau]

    refus = requests.post(
        f"{onglet['base']}/{panneau}",
        headers=MEMBRE_SIMPLE,
        timeout=30,
        **{genre: corps},
    )

    assert refus.status_code == 403, (
        f"{panneau} : {refus.status_code} — un 415 signerait un corps non "
        f"analysé, donc une autorisation jamais atteinte"
    )


def test_les_onze_routes_sont_bien_onze(onglet):
    """Le compte, écrit noir sur blanc.

    Les deux tests ci-dessus se paramètrent sur des listes ; si quelqu'un ajoute
    un sixième panneau sans l'y inscrire, ils resteront verts en couvrant moins.
    Ce test échoue à la place — c'est le seul endroit qui relie la liste des
    tests au nombre de routes réellement servies.
    """
    servies = [
        p
        for p in PANNEAUX
        if requests.get(f"{onglet['base']}/{p}", headers=HX, timeout=15).status_code == 200
    ]

    assert len(servies) == len(PANNEAUX), f"panneaux servis : {servies}"
    assert 1 + 2 * len(PANNEAUX) == 11


# ── 2. Les deux chemins d'autorisation, séparés ───────────────────────────────


def _ouvre_les_cinq_panneaux(base: str, entetes: dict, qui: str) -> None:
    for panneau in PANNEAUX:
        r = requests.get(f"{base}/{panneau}", headers=entetes, timeout=15)
        assert r.status_code == 200, f"{qui} sur {panneau} : {r.status_code}"


def test_un_admin_de_competition_qui_n_est_pas_admin_d_espace_ouvre_les_panneaux(onglet):
    """Le premier chemin, **isolé**.

    `E2E Coach 01` est `SpaceUser` dans l'espace : `is_space_admin` vaut faux.
    Inscrit comme membre de la compétition, il ne passe donc que par
    `is_comp_admin`. Sans cette isolation, la suppression de cette branche
    resterait invisible — `DevCoach` franchirait toujours l'autre porte.
    """
    comp_id = onglet["ctx"]["competition_id"]
    coach = _coach_id(COACH_SIMPLE)
    execute_db(
        "INSERT INTO competitions_members (competition_id, coach_id, competition_profile, created_at) "
        f"VALUES ('{comp_id}', '{coach}', 'CompetitionAdmin', now()) ON CONFLICT DO NOTHING"
    )
    try:
        profil = query_db(
            "SELECT profile FROM spaces__user_space "
            f"WHERE space_id = '{onglet['space_id']}' AND coach_id = '{coach}'"
        )
        assert profil == ["SpaceUser"], f"le montage suppose un membre simple : {profil}"

        _ouvre_les_cinq_panneaux(onglet["base"], MEMBRE_SIMPLE, "admin de compétition seul")
    finally:
        execute_db(
            "DELETE FROM competitions_members "
            f"WHERE competition_id = '{comp_id}' AND coach_id = '{coach}'"
        )


def test_un_admin_d_espace_qui_n_est_pas_membre_de_la_competition_ouvre_les_panneaux(onglet):
    """Le second chemin, **isolé**.

    On retire `DevCoach` des membres de la compétition : `is_comp_admin` tombe,
    et seul son `SpaceAdmin` le fait entrer.
    """
    comp_id = onglet["ctx"]["competition_id"]
    coach = _coach_id(COACH_DEV)
    execute_db(
        "DELETE FROM competitions_members "
        f"WHERE competition_id = '{comp_id}' AND coach_id = '{coach}'"
    )
    try:
        restants = query_db(
            f"SELECT count(*) FROM competitions_members WHERE competition_id = '{comp_id}'"
        )
        assert restants == ["0"], f"le montage suppose une compétition sans membre : {restants}"

        _ouvre_les_cinq_panneaux(onglet["base"], HX, "admin d'espace seul")
    finally:
        execute_db(
            "INSERT INTO competitions_members (competition_id, coach_id, competition_profile, created_at) "
            f"VALUES ('{comp_id}', '{coach}', 'CompetitionAdmin', now()) ON CONFLICT DO NOTHING"
        )


# ── 3. L'assemblage qui se remplit ────────────────────────────────────────────


def test_l_onglet_remplit_ses_cinq_panneaux(page: Page, onglet):
    """Les cinq `hx-get` câblés **ensemble**.

    Le test de la carte 420 vérifie que les cinq conteneurs existent ; il a été
    écrit quand ils étaient vides, et resterait vert si aucun ne se remplissait.
    Ici on attend le panneau *rendu* — `#settings-<nom>-panel`, produit par le
    widget, jamais par la page d'assemblage.
    """
    page.goto(onglet["base"], wait_until="load")

    for panneau in PANNEAUX:
        expect(page.locator(f"#settings-{panneau}-panel")).to_be_visible(timeout=15000)

    # Un panneau qui aurait remplacé son conteneur au lieu de le remplir ferait
    # disparaître les autres : on vérifie que les cinq coexistent.
    expect(page.locator(".competition-admin-settings .settings-panel")).to_have_count(5)


# ── 4. Aucun panneau ne fait régresser la saison ─────────────────────────────


def _statut(season_id: str) -> str:
    return query_db(f"SELECT status FROM competition_seasons WHERE id = '{season_id}'")[0]


def _un(sql: str) -> str:
    lignes = query_db(sql)
    return lignes[0] if lignes else ""


def _cas_ecrivant(season_id: str, competition_id: str) -> dict:
    """Pour chaque panneau : un corps que **le domaine accepte**, et le témoin
    qui prouve que l'écriture a bien eu lieu.

    **Pourquoi pas `_corps_analysable`.** Son contrat est de passer les
    extracteurs d'axum, pas le domaine — c'est ce qu'il fallait pour tester des
    `403`. Réutilisé ici, il rendait le test creux sur deux panneaux : `tiers`
    recevait `{"tiers": []}`, que le use case ignore, et `general` un
    `logo_url` vide que le value object refuse avant même d'atteindre le use
    case. Les deux répondaient `200` sans rien écrire, donc sans jamais toucher
    au statut : le test était vert alors que le défaut était en place. Mesuré.

    **Pourquoi des bascules et non des constantes.** Un témoin qui vaut déjà la
    valeur attendue passe sans qu'aucune écriture ait eu lieu — au second
    passage du test, ou sur une base déjà dans cet état. Chaque valeur est donc
    calculée pour **différer de la courante**.
    """
    nom_compet = _un(f"SELECT name FROM competitions WHERE id = '{competition_id}'")
    logo = _un(f"SELECT COALESCE(logo, '') FROM competitions WHERE id = '{competition_id}'")
    lire = lambda col: _un(f"SELECT {col} FROM competition_seasons WHERE id = '{season_id}'")

    # Le nom de la compétition est réémis tel quel : le use case ne contrôle
    # l'unicité que si le nom change, et se heurterait sinon à elle-même.
    saison = "Saison Témoin A" if lire("name") != "Saison Témoin A" else "Saison Témoin B"

    regles = json.loads(lire("rules->'ranking_rules'"))
    victoire = 3 if regles["win_points"] != 3 else 5
    regles["win_points"] = victoire

    poules = lire("structure->'ranking_group'->>'use_ranking_groups'") == "true"
    corps_poules = [("use_pools", "false")] if poules else [
        ("use_pools", "true"), ("pool_id", ""), ("pool_name", "Poule Témoin")
    ]

    # Les tiers n'ont pas de bascule symétrique : `name` et `budget` sont des
    # champs figés que le domaine refuse de modifier, et seuls les coups de
    # pouce sont ouverts. L'état de départ est donc posé en base — un `[]`
    # posté sur des coups de pouce déjà vides ne prouverait rien.
    tiers = json.loads(lire("rules->'tiers'"))
    execute_db(
        "UPDATE competition_seasons SET rules = jsonb_set(rules, "
        f"'{{tiers,0,inducements}}', '[\"BABE\"]') WHERE id = '{season_id}'"
    )
    tiers[0]["inducements"] = []

    ouverte = lire("invitations->>'access_mode'") == "open"
    acces = "invitation" if ouverte else "open"

    return {
        "general": (
            dict(data={"name": nom_compet, "season_name": saison, "logo_url": logo}),
            f"SELECT name FROM competition_seasons WHERE id = '{season_id}'",
            saison,
        ),
        "ranking": (
            dict(json=regles),
            f"SELECT rules->'ranking_rules'->>'win_points' FROM competition_seasons WHERE id = '{season_id}'",
            str(victoire),
        ),
        "pools": (
            dict(data=corps_poules),
            f"SELECT structure->'ranking_group'->>'use_ranking_groups' FROM competition_seasons WHERE id = '{season_id}'",
            "false" if poules else "true",
        ),
        "tiers": (
            dict(json={"tiers": tiers}),
            f"SELECT COALESCE(rules->'tiers'->0->>'inducements', 'absent') FROM competition_seasons WHERE id = '{season_id}'",
            "[]",
        ),
        "visibility": (
            dict(data={"access_mode": acces, "requires_validation": "manual"}),
            f"SELECT invitations->>'access_mode' FROM competition_seasons WHERE id = '{season_id}'",
            acces,
        ),
    }


@pytest.mark.parametrize("panneau", PANNEAUX)
def test_aucun_panneau_ne_fait_regresser_la_saison(onglet, panneau):
    """**Le défaut de la carte 485, sur les cinq panneaux à la fois.**

    Une saison monte une échelle : `draft` → `rules_selected` →
    `structure_selected` → `invitations_configured` → `ready`. Les méthodes du
    magicien *posent* le barreau qu'elles viennent de franchir. Appelées depuis
    un panneau de réglages, elles font **redescendre** une saison en cours sous
    `ready` — et la carte 407 interdit la création d'équipe sur une saison qui
    ne l'est pas : modifier un réglage casse l'inscription de la compétition
    entière. Rien ne le signale : l'enregistrement réussit, le panneau affiche
    son succès, et le défaut ne se voit qu'à la carte de la compétition, qui
    renvoie soudain vers une étape du magicien.

    **Pourquoi ici et paramétré.** Structure l'a eu (carte 423), Visibilité l'a
    eu (426), puis Général, Classement et Tiers l'ont eu ensemble (485). Chaque
    correction avait posé son assertion dans le fichier de son panneau, et
    aucune n'a empêché la suivante. C'est le fichier des choses transverses ;
    la liste `PANNEAUX` fait qu'un sixième panneau hérite du garde-fou sans que
    personne y pense. Le verrou statique correspondant est l'axe 16 de
    `check-arch.sh`.
    """
    season_id = onglet["ctx"]["season_id"]
    kwargs, sql_temoin, attendu = _cas_ecrivant(season_id, onglet["ctx"]["competition_id"])[panneau]
    # Les cinq cas partagent la saison de la fixture : sans cette remise à zéro,
    # le premier panneau fautif empoisonne les suivants, qui échouent sur la
    # garde d'entrée. Le test accusait alors quatre panneaux pour trois défauts —
    # dont deux innocents — et le rapport devenait illisible. Mesuré.
    execute_db(f"UPDATE competition_seasons SET status = 'ready' WHERE id = '{season_id}'")

    reponse = requests.post(f"{onglet['base']}/{panneau}", headers=HX, timeout=30, **kwargs)

    assert reponse.status_code == 200, f"{panneau} : {reponse.status_code}"
    assert _un(sql_temoin) == attendu, (
        f"{panneau} : l'enregistrement n'a pas eu lieu — le test ne prouverait "
        f"rien sur le statut"
    )
    assert _statut(season_id) == "ready", (
        f"{panneau} : la saison a régressé en « {_statut(season_id)} » — "
        f"plus aucune équipe ne peut s'inscrire"
    )


# ── 5. Les boutons d'enregistrement (carte 487) ──────────────────────────────


def test_les_boutons_d_enregistrement_ne_remplissent_pas_leur_panneau(page: Page, onglet):
    """**Cinq boutons de 71 px de haut, larges de 833 à 1012 px.**

    Ils portaient `.btn .btn-primary` de `common.css` : `padding: var(--p2)
    var(--p3)` en fait 24 px sur 36, et `.btn-primary` ajoute `width: 100%`. Le
    bouton pesait plus lourd que le champ qu'il valide, et sa largeur changeait
    d'un panneau à l'autre selon ce qu'il y avait à côté.

    Quatre des cinq vivaient en plus dans l'en-tête du panneau ; seul « Général »
    était en pied. Ils y sont désormais tous.

    L'assertion est **relative au panneau**, pas une valeur en dur : un bouton
    dont la largeur suit son libellé est correct, un bouton qui remplit son
    conteneur ne l'est pas.
    """
    page.set_viewport_size({"width": 1440, "height": 900})
    page.goto(onglet["base"], wait_until="load")
    expect(page.locator("#settings-general-panel")).to_be_visible(timeout=10000)
    for panneau in PANNEAUX[1:]:
        expect(page.locator(f"#settings-{panneau}-panel")).to_be_visible(timeout=10000)

    mesures = page.evaluate(
        """() => [...document.querySelectorAll('.settings-panel')].map(p => {
             const b = p.querySelector('.btn-primary');
             if (!b) return {panneau: p.id, absent: true};
             const r = b.getBoundingClientRect(), pr = p.getBoundingClientRect();
             return {panneau: p.id, largeur: Math.round(r.width),
                     hauteur: Math.round(r.height),
                     panneauLargeur: Math.round(pr.width),
                     enPied: !!b.closest('.settings-panel-foot'),
                     margeDroite: Math.round(pr.right - r.right)};
           })"""
    )

    assert len(mesures) == len(PANNEAUX), f"{len(mesures)} panneaux rendus : {mesures}"
    for m in mesures:
        assert not m.get("absent"), f"{m['panneau']} : aucun bouton d'enregistrement"
        assert m["enPied"], f"{m['panneau']} : le bouton n'est pas en pied de panneau"
        assert m["hauteur"] < 56, f"{m['panneau']} : bouton de {m['hauteur']} px de haut"
        assert m["largeur"] < m["panneauLargeur"] / 2, (
            f"{m['panneau']} : le bouton occupe {m['largeur']} px "
            f"sur {m['panneauLargeur']} — il remplit son panneau"
        )
    marges = {m["margeDroite"] for m in mesures}
    assert len(marges) == 1, f"les cinq boutons ne s'alignent pas à droite : {marges}"

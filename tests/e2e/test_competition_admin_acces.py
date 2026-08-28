"""Les treize routes de mutation de l'administration de compétition (carte 416).

Aucune n'acceptait `AuthSession`, aucune n'appelait `require_admin_access` :
n'importe quel membre connecté pouvait régénérer un calendrier, vider une
journée, supprimer un match ou réinitialiser des poules — sur une compétition
qu'il ne gère pas.

**Ces tests sont le seul filet.** Rien dans le compilateur ne signale un handler
qui ne contrôle rien, et le projet n'a pas de harnais au niveau handler (carte
311). Une régression ne se verrait qu'ici.

Trois familles de scénarios :

- **le droit** — un membre simple reçoit `403` sur chacune des treize ;
- **la portée** — un administrateur légitime ne peut pas viser la journée,
  l'appariement ou le groupe d'une **autre** saison. `space_scope` n'a de
  résolveur ni pour `round_id` ni pour `pairing_id` : ils passent librement,
  dans le chemin comme dans le corps ;
- **la cohérence du chemin** — la saison doit appartenir à la compétition du
  chemin, sans quoi l'administrateur de l'une agit sur l'autre.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import pytest
import requests

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db

# Coach seedé sans droit d'administration (`seed_e2e.rs::SIMPLE_COACH_NAME`).
# Sans cet en-tête c'est DevCoach — admin de l'espace — qui répond, et aucun
# refus n'est observable.
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}
JSON = {"Content-Type": "application/json", "HX-Request": "true"}


# ── Fixtures ──────────────────────────────────────────────────────────────────


@pytest.fixture(scope="module")
def deux_competitions(browser, space_id):
    """**Deux** compétitions du même espace, et c'est le sujet.

    Les scénarios de portée ont besoin d'une cible légitime *ailleurs* : une
    journée qui existe, dans le même espace, sous une autre saison. Une cible
    inventée ne prouverait rien — elle rendrait `404` pour la seule raison
    qu'elle n'existe pas.
    """
    a = build_full_competition(browser, space_id, num_teams=2, num_rounds=2)
    b = build_full_competition(browser, space_id, num_teams=2, num_rounds=2)
    return {"space_id": space_id, "a": a, "b": b}


def _url(space_id: str, ctx: dict, suffixe: str) -> str:
    return (
        f"{BASE_URL}/app/{space_id}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/admin/{suffixe}"
    )


def _un_appariement(season_id: str) -> str | None:
    lignes = query_db(
        "SELECT p.id FROM competition_match_day_pairings p "
        "JOIN competition_match_days d ON d.id = p.match_day_id "
        f"WHERE d.season_id = '{season_id}' LIMIT 1"
    )
    return lignes[0] if lignes else None


def _un_groupe(space_id: str, ctx: dict) -> str:
    """Un groupe de cette saison, **matérialisé si besoin**.

    Les poules sont configurées par le magicien mais n'existent en base qu'au
    premier affichage de leur widget (`ensure_groups_from_structure`). Sans cet
    appel, la table est vide et le scénario de portée se sautait — un test sauté
    n'est pas une couverture.
    """
    requests.get(_url(space_id, ctx, "groups/cards"), headers={"HX-Request": "true"}, timeout=10)
    lignes = query_db(
        f"SELECT id FROM competition_groups WHERE season_id = '{ctx['season_id']}' LIMIT 1"
    )
    assert lignes, f"aucune poule pour la saison {ctx['season_id']}"
    return lignes[0]


# ── Les treize routes, et leur forme d'appel ──────────────────────────────────
#
# Le corps est celui que le vrai client envoie : un corps absent ou mal formé
# ferait échouer l'extracteur **avant** le contrôle d'accès, et le test
# vérifierait un rejet de format au lieu d'un refus de droit.


def _routes(ctx: dict, round_id: str, pairing_id: str, group_id: str, team_id: str):
    return [
        ("post", "groups/random-draw", None),
        ("post", "groups/reset", None),
        ("post", "groups/assign", {"team_id": team_id, "group_id": group_id}),
        ("post", "schedule/generate-all", None),
        ("post", "schedule/clear-all", None),
        ("post", "schedule/add-round", {"name": "Journée test"}),
        ("post", "schedule/add-rest", {"name": "Repos test"}),
        ("put", f"schedule/rounds/{round_id}", {"name": "Renommée"}),
        ("delete", f"schedule/rounds/{round_id}", None),
        ("post", "schedule/generate-round", {"round_id": round_id}),
        ("post", "schedule/clear-round", {"round_id": round_id}),
        (
            "post",
            "schedule/add-match",
            {"round_id": round_id, "home_team_id": team_id, "away_team_id": team_id},
        ),
        ("delete", "schedule/delete-match", {"pairing_id": pairing_id}),
    ]


def _appeler(methode: str, url: str, corps, entetes: dict) -> requests.Response:
    return requests.request(
        methode.upper(), url, json=corps, headers={**JSON, **entetes}, timeout=10
    )


@pytest.fixture(scope="module")
def cibles(deux_competitions):
    """Les identifiants réels de la compétition A — cibles légitimes."""
    a = deux_competitions["a"]
    return {
        "round_id": a["round_ids"][0],
        "pairing_id": _un_appariement(a["season_id"]),
        "group_id": _un_groupe(deux_competitions["space_id"], a),
        "team_id": a["team_ids"][0],
    }


# ── 1 · Le droit ──────────────────────────────────────────────────────────────


def test_les_treize_routes_refusent_un_membre_simple(deux_competitions, cibles):
    """Une seule fonction pour treize routes : la liste **est** l'assertion.

    Un test par route les aurait dispersées, et rien n'aurait dit qu'il en
    manquait une. Ici l'échec nomme la route fautive, et le décompte final
    interdit d'en retirer une sans s'en apercevoir.
    """
    space_id, a = deux_competitions["space_id"], deux_competitions["a"]
    routes = _routes(a, **cibles)
    assert len(routes) == 13, f"la carte en dénombre treize, pas {len(routes)}"

    refusees = []
    for methode, suffixe, corps in routes:
        reponse = _appeler(
            methode, _url(space_id, a, suffixe), corps, ENTETE_MEMBRE_SIMPLE
        )
        if reponse.status_code == 403:
            refusees.append(suffixe)
        else:
            refusees.append(f"!! {methode} {suffixe} → {reponse.status_code}")

    fautives = [r for r in refusees if r.startswith("!!")]
    assert not fautives, f"routes non gardées : {fautives}"


def test_un_administrateur_n_est_pas_refuse(deux_competitions, cibles):
    """Contre-épreuve. Sans elle, un `403` dû à une URL fautive, à un corps mal
    formé ou à une garde antérieure se lirait comme un refus d'autorisation —
    et treize tests verts ne prouveraient rien.

    Seule la lecture est exercée ici : `random-draw` répartit les équipes en
    poules, geste réversible, alors qu'une génération ou une suppression
    laisserait la compétition A dans un état dont les autres scénarios
    dépendent.
    """
    space_id, a = deux_competitions["space_id"], deux_competitions["a"]

    reponse = _appeler("post", _url(space_id, a, "groups/random-draw"), None, {})

    assert reponse.status_code != 403, "DevCoach administre cet espace"


# ── 2 · La portée : une cible d'une autre saison ──────────────────────────────


def test_une_journee_d_une_autre_saison_est_hors_de_portee(deux_competitions):
    """`space_scope` n'a **pas** de résolveur pour `round_id`, et sa docstring en
    tire la conclusion inverse de la vérité : « ils sont toujours accompagnés
    d'un parent qui, lui, est contrôlé ». Le parent l'est ; rien ne rattache
    l'enfant au parent.

    La journée visée existe, dans le même espace, et l'appelant administre bien
    la compétition du chemin. Seule son appartenance à **cette** saison manque.
    """
    space_id, a, b = (
        deux_competitions["space_id"],
        deux_competitions["a"],
        deux_competitions["b"],
    )
    etrangere = b["round_ids"][0]

    for methode, suffixe, corps in [
        ("put", f"schedule/rounds/{etrangere}", {"name": "Détournée"}),
        ("delete", f"schedule/rounds/{etrangere}", None),
        ("post", "schedule/generate-round", {"round_id": etrangere}),
        ("post", "schedule/clear-round", {"round_id": etrangere}),
        ("post", "schedule/add-match", {
            "round_id": etrangere,
            "home_team_id": a["team_ids"][0],
            "away_team_id": a["team_ids"][1],
        }),
    ]:
        reponse = _appeler(methode, _url(space_id, a, suffixe), corps, {})
        assert reponse.status_code == 404, (
            f"{methode} {suffixe} a rendu {reponse.status_code} sur une journée "
            "d'une autre saison"
        )

    # La journée étrangère n'a pas bougé.
    restante = query_db(
        f"SELECT season_id FROM competition_match_days WHERE id = '{etrangere}'"
    )
    assert restante == [b["season_id"]], "la journée visée a été touchée"


def test_un_appariement_d_une_autre_saison_est_hors_de_portee(deux_competitions):
    """L'exemple que la carte cite : la cible vivait dans le corps, et le chemin
    entier était ignoré."""
    space_id, a, b = (
        deux_competitions["space_id"],
        deux_competitions["a"],
        deux_competitions["b"],
    )
    etranger = _un_appariement(b["season_id"])
    assert etranger, "la compétition B doit avoir au moins un appariement"

    reponse = _appeler(
        "delete",
        _url(space_id, a, "schedule/delete-match"),
        {"pairing_id": etranger},
        {},
    )

    assert reponse.status_code == 404, f"obtenu {reponse.status_code}"
    assert query_db(
        f"SELECT id FROM competition_match_day_pairings WHERE id = '{etranger}'"
    ) == [etranger], "l'appariement visé a été supprimé"


def test_un_groupe_d_une_autre_saison_est_hors_de_portee(deux_competitions):
    space_id, a, b = (
        deux_competitions["space_id"],
        deux_competitions["a"],
        deux_competitions["b"],
    )
    etranger = _un_groupe(space_id, b)

    reponse = _appeler(
        "post",
        _url(space_id, a, "groups/assign"),
        {"team_id": a["team_ids"][0], "group_id": etranger},
        {},
    )

    assert reponse.status_code == 404, f"obtenu {reponse.status_code}"


# ── 3 · La cohérence du chemin ────────────────────────────────────────────────


def test_la_saison_doit_appartenir_a_la_competition_du_chemin(deux_competitions):
    """Le droit est accordé **par compétition**. Sans ce contrôle,
    l'administrateur de A pose son propre `competition_id` et le `season_id` de
    B : la garde l'accepte, puis le handler agit sur B.

    `space_scope` ne le rattrape pas — il vérifie que la saison appartient à
    l'espace, jamais à la compétition.
    """
    space_id, a, b = (
        deux_competitions["space_id"],
        deux_competitions["a"],
        deux_competitions["b"],
    )
    chemin_mixte = {"competition_id": a["competition_id"], "season_id": b["season_id"]}

    reponse = _appeler(
        "post", _url(space_id, chemin_mixte, "schedule/clear-all"), None, {}
    )

    assert reponse.status_code == 404, f"obtenu {reponse.status_code}"
    # Contre-épreuve : le chemin cohérent de B, lui, passe.
    assert (
        _appeler("post", _url(space_id, b, "groups/random-draw"), None, {}).status_code
        != 404
    )

"""Tests E2E — mode customisation (cartes 302 à 315).

Ce que ces tests couvrent et qu'aucun test unitaire ne voit :

- **la table des directions de bout en bout** — améliorer l'agilité fait
  *descendre* le chiffre affiché. Une inversion de cette table passerait tous
  les tests unitaires, qui la vérifient chacun de leur côté ;
- **l'asymétrie de la valeur d'équipe** — le prix la déplace, la compétence
  non. C'est la règle la plus contre-intuitive de la fonctionnalité, la seule
  qu'un lecteur de bonne foi prendrait pour un bug ;
- **le cloisonnement des espaces** (carte 315), dont le correctif n'a pas de
  test unitaire possible : il porte sur un `AppState`, et le projet n'a pas de
  harnais au niveau handler (carte 311).

Le panneau étant rendu par le serveur, la plupart des scénarios s'exercent en
HTTP direct — même arbitrage que `competition_lifecycle` et
`match_report_helpers`. Le navigateur sert là où le rendu **fait partie de ce
qu'on affirme** : présence du bouton, valeur affichée, bouton grisé.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import re

import pytest
import requests
from playwright.sync_api import Page, expect

from htmx_helpers import cliquer_quand_cable

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import execute_db, query_db

# Coach seedé sans droit d'administration (`seed_e2e.rs::SIMPLE_COACH_NAME`).
# Sans cet en-tête c'est DevCoach — admin de l'espace — qui répond, et aucun
# refus n'est observable.
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}
HX = {"HX-Request": "true"}


# ── Fixture ───────────────────────────────────────────────────────────────────


@pytest.fixture(scope="module")
def custo_ctx(browser, space_id):
    """Une compétition dédiée, ses équipes, et les joueurs de la première.

    Dédiée au fichier : chaque test customise **son** joueur. Les
    customisations écrivent des événements domaine, et deux tests qui se
    partageraient un joueur se marcheraient dessus sans que la cause soit
    lisible dans l'échec.
    """
    ctx = build_full_competition(browser, space_id, num_teams=2)
    team_id = ctx["team_ids"][0]
    joueurs = query_db(
        f"SELECT player_id FROM players_proj WHERE team_id = '{team_id}' "
        "AND membership = 'Active' ORDER BY player_id"
    )
    assert len(joueurs) >= 9, f"{len(joueurs)} joueurs seulement dans {team_id}"
    return {"space_id": space_id, "team_id": team_id, "joueurs": joueurs}


# ── Helpers ───────────────────────────────────────────────────────────────────


def _url(ctx, player_id: str, suffixe: str) -> str:
    return f"{BASE_URL}/app/{ctx['space_id']}/players/{player_id}/{suffixe}"


def _panneau(ctx, player_id: str, entete: dict | None = None) -> requests.Response:
    """Ouvre le mode : le `GET` crée le panier s'il n'existe pas."""
    return requests.get(
        _url(ctx, player_id, "widgets/customisation"),
        headers=entete or {},
        timeout=10,
    )


def _version(html: str) -> int:
    """La version du panier, telle que le panneau vient de la rendre.

    Relue à chaque fois plutôt que comptée : c'est exactement ce que fait le
    navigateur, et compter à côté ferait passer les tests là où un vrai clic
    tomberait en écriture concurrente.
    """
    m = re.search(r'name="expected_version" value="(\d+)"', html)
    assert m, "le panneau ne porte pas de version — a-t-il seulement été rendu ?"
    return int(m.group(1))


def _muter(ctx, player_id: str, route: str, data: dict, entete: dict | None = None):
    reponse = _panneau(ctx, player_id)
    data = {**data, "expected_version": _version(reponse.text)}
    return requests.post(
        _url(ctx, player_id, f"customisation/{route}"),
        data=data,
        headers={**HX, **(entete or {})},
        timeout=10,
    )


def _lignes_du_panier(player_id: str) -> str:
    lignes = query_db(
        f"SELECT state FROM players__customisation_baskets WHERE player_id = '{player_id}'"
    )
    return lignes[0] if lignes else ""


def _valeur(player_id: str) -> int:
    return int(query_db(f"SELECT value_kpo FROM players_proj WHERE player_id = '{player_id}'")[0])


def _tv(team_id: str) -> int:
    return int(query_db(f"SELECT team_value FROM team_proj WHERE team_id = '{team_id}'")[0])


def _agilite_affichee(html: str) -> str:
    """La valeur d'agilité que le panneau affiche, lue et non devinée : la
    coder en dur lierait le test au roster de la fixture."""
    bloc = re.search(
        r'<div class="custo-stat-name">AG</div>\s*<div class="custo-stat-current">([^<]+)</div>',
        html,
    )
    assert bloc, "carte AG introuvable dans le panneau"
    return bloc.group(1).strip()


def _attendre(predicat, quoi: str, timeout_s: int = 20) -> None:
    """Sonde jusqu'à satisfaction. La valeur d'équipe est recalculée par app
    event cross-BC, donc **après** que le `POST` a rendu la main : une
    assertion sèche serait instable, et l'instabilité serait mise sur le compte
    de la suite plutôt que du test."""
    import time

    for _ in range(timeout_s * 5):
        if predicat():
            return
        time.sleep(0.2)
    raise AssertionError(f"délai dépassé : {quoi}")


def _slot_droit(fiche_html: str) -> str:
    """L'URL que la fiche installe dans `#pd-right-panel`.

    Ciblée précisément, et non cherchée dans toute la page : le bouton
    « Customiser » porte lui aussi l'URL du panneau dans son `hx-get`. Chercher
    la chaîne n'importe où ferait passer « le mode s'est rouvert » sur une page
    où il ne s'est rien passé — le test aurait été vert pour rien.
    """
    m = re.search(r'id="pd-right-panel"\s+hx-get="([^"]+)"', fiche_html)
    assert m, "conteneur #pd-right-panel introuvable dans la fiche"
    return m.group(1)


def _premiere_competence_ajoutable(html: str) -> str:
    m = re.search(r'"skill_id": "([A-Z_]+)"', html)
    assert m, "aucune compétence ajoutable dans le panneau"
    return m.group(1)


# ── Scénario 1 — autorisation ─────────────────────────────────────────────────


def test_un_membre_simple_ne_voit_pas_le_mode_et_ne_peut_pas_le_forcer(custo_ctx):
    """Masquer un bouton n'est pas un contrôle d'accès : le refus doit tenir
    aussi quand on tape l'URL.

    En HTTP et non au navigateur : `set_extra_http_headers` poserait
    `X-Bypass-Auth-Profile` sur **toutes** les requêtes de la page, polices
    Google comprises, dont le préflight CORS échouerait alors — un échec qui ne
    dirait rien de la fonctionnalité. L'absence du bouton est de toute façon
    décidée par le serveur, le navigateur n'ajoute rien ici.
    """
    joueur = custo_ctx["joueurs"][0]

    fiche = requests.get(
        _url(custo_ctx, joueur, "detail"), headers=ENTETE_MEMBRE_SIMPLE, timeout=10
    ).text
    assert "btn-customise" not in fiche, "un coach ne doit pas voir le bouton"

    # Le panneau atteint directement retombe sur le journal, jamais sur le mode.
    panneau = _panneau(custo_ctx, joueur, ENTETE_MEMBRE_SIMPLE)
    assert panneau.status_code == 200
    assert "Mode customisation" not in panneau.text
    assert "Journal des évolutions" in panneau.text

    # Et les mutations sont refusées net.
    refus = requests.post(
        _url(custo_ctx, joueur, "customisation/spp/add"),
        data={"amount": 5, "expected_version": 1},
        headers={**HX, **ENTETE_MEMBRE_SIMPLE},
        timeout=10,
    )
    assert refus.status_code == 403, f"obtenu {refus.status_code}"


def test_un_commissaire_voit_le_bouton(page: Page, custo_ctx):
    page.goto(_url(custo_ctx, custo_ctx["joueurs"][0], "detail"))
    expect(page.locator(".btn-customise")).to_have_count(1)


# ── Scénario 2 — la compétence est appliquée et journalisée ───────────────────


def test_une_competence_customisee_est_appliquee_et_journalisee(custo_ctx):
    joueur = custo_ctx["joueurs"][1]
    competence = _premiere_competence_ajoutable(_panneau(custo_ctx, joueur).text)

    assert _muter(custo_ctx, joueur, "skills/add", {"skill_id": competence}).status_code == 200
    version = _version(_panneau(custo_ctx, joueur).text)
    validation = requests.post(
        _url(custo_ctx, joueur, "customisation/validate"),
        data={"expected_version": version},
        headers=HX,
        timeout=10,
    )
    assert validation.status_code == 200
    assert validation.headers.get("HX-Refresh") == "true"

    # Le panier disparaît : son existence commande l'affichage du mode.
    assert _lignes_du_panier(joueur) == ""

    journal = requests.get(
        _url(custo_ctx, joueur, "widgets/evolution-journal"), timeout=10
    ).text
    assert "Customisation par" in journal, "le journal doit nommer le commissaire"
    assert "mode-chip-custom" in journal


# ── Scénario 2 bis — une compétence dont le nom porte une apostrophe ──────────


def test_une_competence_a_apostrophe_traverse_toute_la_chaine(custo_ctx):
    """La régression qui a coûté une enquête.

    « Capitaine d'équipe » s'affichait dans le panneau, s'ajoutait au panier,
    et n'échouait qu'à la validation : le nom traverse un `SkillName` dont le
    charset refusait l'apostrophe, et l'échec était écrasé en `UnknownSkill`
    — une erreur qui accusait le catalogue alors que seul son nom était en
    cause. Aucun test unitaire ne pouvait le voir : le panier acceptait la
    ligne, seule l'application de l'événement la refusait.

    Le test vise donc nommément cette compétence-là, et non « la première
    ajoutable » : elle est troisième dans l'ordre alphabétique du panneau, et
    un test qui prend la première ne la rencontrerait jamais.
    """
    joueur = custo_ctx["joueurs"][8]
    panneau = _panneau(custo_ctx, joueur).text
    assert "CAPITAINE_EQUIPE" in panneau, "la compétence à apostrophe doit être proposée"

    assert (
        _muter(custo_ctx, joueur, "skills/add", {"skill_id": "CAPITAINE_EQUIPE"}).status_code
        == 200
    )
    version = _version(_panneau(custo_ctx, joueur).text)
    validation = requests.post(
        _url(custo_ctx, joueur, "customisation/validate"),
        data={"expected_version": version},
        headers=HX,
        timeout=10,
    )
    assert validation.status_code == 200, "la validation refusait le nom, pas la compétence"
    assert _lignes_du_panier(joueur) == ""

    journal = requests.get(
        _url(custo_ctx, joueur, "widgets/evolution-journal"), timeout=10
    ).text
    # Askama échappe l'apostrophe en `&#x27;` — les deux formes valent preuve.
    assert "Capitaine d&#x27;équipe" in journal or "Capitaine d'équipe" in journal


# ── Scénarios 3 et 4 — direction des seuils de dé, puis borne ─────────────────


def test_ameliorer_l_agilite_fait_descendre_le_seuil_puis_bute_sur_la_borne(custo_ctx):
    """Le pilier du fichier. AG et PA sont des nombres cibles à atteindre au
    dé : **améliorer les fait descendre**. Une inversion de la table des
    directions passerait tous les tests unitaires."""
    joueur = custo_ctx["joueurs"][2]
    avant = _agilite_affichee(_panneau(custo_ctx, joueur).text)
    assert avant.endswith("+"), f"l'agilité s'affiche avec un « + », obtenu {avant!r}"

    assert _muter(custo_ctx, joueur, "stats/add", {"stat": "ag", "crans": 1}).status_code == 200
    apres = _agilite_affichee(_panneau(custo_ctx, joueur).text)
    assert int(apres.rstrip("+")) == int(avant.rstrip("+")) - 1, (
        f"améliorer l'agilité doit faire descendre le seuil : {avant} → {apres}"
    )

    # Jusqu'à la borne : 1+ est le plafond de qualité.
    for _ in range(6):
        html = _panneau(custo_ctx, joueur).text
        if _agilite_affichee(html) == "1+":
            break
        _muter(custo_ctx, joueur, "stats/add", {"stat": "ag", "crans": 1})

    html = _panneau(custo_ctx, joueur).text
    assert _agilite_affichee(html) == "1+"
    assert 'disabled title="Borne atteinte"' in html, "le bouton doit être grisé"

    # Et le `POST` forcé est refusé, avec son motif : le grisage n'est pas la
    # vérité, le serveur l'est.
    forcé = _muter(custo_ctx, joueur, "stats/add", {"stat": "ag", "crans": 1})
    assert forcé.status_code == 200
    assert "custo-refusal" in forcé.text, "le refus doit s'afficher dans le panneau"


# ── Scénario 5 — doublon de compétence ───────────────────────────────────────


def test_une_competence_deja_possedee_est_refusee(custo_ctx):
    """La compétence est d'abord **acquise par customisation**, puis re-demandée.

    Plutôt que de lire une compétence de base en projection : celle-ci y est
    stockée en JSON et sous une forme dont rien ne garantit que ce soit
    l'identifiant attendu par l'endpoint. Passer par le parcours réel donne un
    identifiant dont on est sûr, et exerce la règle sur une compétence acquise
    — le cas que la phase 1 visait explicitement.
    """
    joueur = custo_ctx["joueurs"][3]
    competence = _premiere_competence_ajoutable(_panneau(custo_ctx, joueur).text)

    _muter(custo_ctx, joueur, "skills/add", {"skill_id": competence})
    version = _version(_panneau(custo_ctx, joueur).text)
    requests.post(
        _url(custo_ctx, joueur, "customisation/validate"),
        data={"expected_version": version}, headers=HX, timeout=10,
    )
    _attendre(lambda: _lignes_du_panier(joueur) == "", "la validation vide le panier")

    # Elle a disparu de la liste — mais le grisage n'est pas la vérité, le
    # serveur l'est : on force le `POST`.
    assert competence not in _panneau(custo_ctx, joueur).text

    refus = _muter(custo_ctx, joueur, "skills/add", {"skill_id": competence})
    assert refus.status_code == 200
    assert "custo-refusal" in refus.text
    assert _lignes_du_panier(joueur) in ("", "[]"), "un refus ne doit rien écrire"


# ── Scénario 6 — la TV suit le prix, pas la compétence ───────────────────────


def test_le_prix_deplace_la_valeur_d_equipe_mais_pas_la_competence(custo_ctx):
    """La règle la plus contre-intuitive : dans la progression normale une
    compétence achetée augmente la valeur du joueur, mais une compétence
    **customisée** ne déplace pas la TV — la customisation pose une valeur au
    lieu de la dériver d'un barème."""
    joueur = custo_ctx["joueurs"][4]
    equipe = custo_ctx["team_id"]

    valeur_avant, tv_avant = _valeur(joueur), _tv(equipe)

    _muter(custo_ctx, joueur, "price/adjust", {"delta_kpo": 20})
    version = _version(_panneau(custo_ctx, joueur).text)
    requests.post(
        _url(custo_ctx, joueur, "customisation/validate"),
        data={"expected_version": version}, headers=HX, timeout=10,
    )

    _attendre(lambda: _valeur(joueur) == valeur_avant + 20, "la valeur du joueur suit le prix")
    _attendre(lambda: _tv(equipe) > tv_avant, "la valeur d'équipe suit le prix")

    # Puis une compétence : le joueur change, la TV non.
    tv_apres_prix = _tv(equipe)
    competence = _premiere_competence_ajoutable(_panneau(custo_ctx, joueur).text)
    _muter(custo_ctx, joueur, "skills/add", {"skill_id": competence})
    version = _version(_panneau(custo_ctx, joueur).text)
    requests.post(
        _url(custo_ctx, joueur, "customisation/validate"),
        data={"expected_version": version}, headers=HX, timeout=10,
    )
    _attendre(lambda: _lignes_du_panier(joueur) == "", "la validation vide le panier")

    import time

    time.sleep(2)  # laisser le temps à un recalcul indu de se produire
    assert _tv(equipe) == tv_apres_prix, (
        "une compétence customisée ne doit pas déplacer la valeur d'équipe"
    )


# ── Scénarios 7 et 8 — annulation et persistance ─────────────────────────────


def test_annuler_vide_le_panier_et_ne_rouvre_pas_le_mode(custo_ctx):
    joueur = custo_ctx["joueurs"][5]
    _muter(custo_ctx, joueur, "spp/add", {"amount": 3})
    assert "Spp" in _lignes_du_panier(joueur)

    annulation = requests.post(
        _url(custo_ctx, joueur, "customisation/cancel"), headers=HX, timeout=10
    )
    assert annulation.status_code == 200
    assert "Journal des évolutions" in annulation.text
    assert _lignes_du_panier(joueur) == "", "le panier doit disparaître, pas se vider"

    # Un rechargement complet ne rouvre pas le mode : c'est l'existence du
    # panier qui commande l'occupant du slot.
    fiche = requests.get(_url(custo_ctx, joueur, "detail"), timeout=10).text
    assert "widgets/customisation" not in _slot_droit(fiche)


def test_une_saisie_en_cours_est_retrouvee_apres_rechargement(custo_ctx):
    joueur = custo_ctx["joueurs"][6]
    _muter(custo_ctx, joueur, "spp/add", {"amount": 4})

    fiche = requests.get(_url(custo_ctx, joueur, "detail"), timeout=10).text
    assert "widgets/customisation" in _slot_droit(fiche), "le mode doit se rouvrir tout seul"
    assert "SPP +4" in _panneau(custo_ctx, joueur).text

    requests.post(_url(custo_ctx, joueur, "customisation/cancel"), headers=HX, timeout=10)


# ── Scénario 9 — plancher de prix ────────────────────────────────────────────


def test_un_prix_sous_zero_est_refuse(custo_ctx):
    joueur = custo_ctx["joueurs"][7]
    valeur = _valeur(joueur)

    refus = _muter(custo_ctx, joueur, "price/adjust", {"delta_kpo": -(valeur + 10)})
    assert refus.status_code == 200
    assert "custo-refusal" in refus.text
    assert _lignes_du_panier(joueur) in ("", "[]")
    assert _valeur(joueur) == valeur


# ── Scénario 10 — péremption ─────────────────────────────────────────────────


def test_un_panier_de_plus_de_vingt_quatre_heures_est_abandonne(custo_ctx):
    """Vieillir le panier en base est le seul moyen : attendre une journée n'en
    est pas un, et reculer l'horloge du serveur en toucherait bien d'autres."""
    joueur = custo_ctx["joueurs"][0]
    _muter(custo_ctx, joueur, "spp/add", {"amount": 2})
    assert _lignes_du_panier(joueur) != ""

    execute_db(
        "UPDATE players__customisation_baskets SET updated_at = now() - interval '25 hours' "
        f"WHERE player_id = '{joueur}'"
    )

    fiche = requests.get(_url(custo_ctx, joueur, "detail"), timeout=10).text
    assert "abandoned=true" in _slot_droit(fiche), (
        "la fiche doit demander le journal en mode abandon"
    )
    assert _lignes_du_panier(joueur) == "", "le panier périmé doit être supprimé"

    journal = requests.get(
        _url(custo_ctx, joueur, "widgets/evolution-journal") + "?abandoned=true", timeout=10
    ).text
    assert "de plus de 24 h a été abandonnée" in journal


# ── Scénario 11 — cloisonnement des espaces (carte 315) ──────────────────────


def test_un_joueur_n_est_atteignable_que_depuis_son_espace(custo_ctx):
    """`404` et non `403` : rien ne doit confirmer l'existence d'un joueur d'un
    autre espace à qui l'énumère."""
    joueur = custo_ctx["joueurs"][0]
    autre_espace = query_db(
        f"SELECT id FROM spaces WHERE id <> '{custo_ctx['space_id']}' LIMIT 1"
    )
    if not autre_espace:
        pytest.skip("un seul espace en base")
    etranger = f"{BASE_URL}/app/{autre_espace[0]}/players/{joueur}"

    for route in ("detail", "debug", "widgets/customisation", "widgets/evolution-journal"):
        reponse = requests.get(f"{etranger}/{route}", timeout=10)
        assert reponse.status_code == 404, f"{route} : {reponse.status_code}"

    ecriture = requests.post(
        f"{etranger}/customisation/spp/add",
        data={"amount": 5, "expected_version": 1},
        headers=HX,
        timeout=10,
    )
    assert ecriture.status_code == 404, f"écriture : {ecriture.status_code}"

# ── Scénario — l'onglet actif survit à l'enregistrement ───────────────────────


SPP_AJOUTER = ".custo-action-panel:has(input[name='amount']) .custo-action-btn"


def test_l_onglet_actif_survit_a_l_enregistrement(page: Page, custo_ctx):
    """L'enregistrement renvoie `HX-Refresh: true` — la page entière se
    recharge, et c'est voulu : c'est ce qui garde justes les quatre endroits
    qui affichent les SPP. Mais un rechargement efface l'état Alpine, donc
    l'onglet, et l'utilisateur repart de « Compétences » à chaque validation.

    Seul un test de navigateur voit ce défaut : il n'y a ni requête ni réponse
    à observer, juste un état client perdu (carte 398).
    """
    joueur = custo_ctx["joueurs"][-1]
    page.goto(_url(custo_ctx, joueur, "detail"), wait_until="load")

    cliquer_quand_cable(page, ".btn-customise")
    expect(page.locator(".custo-zone .tabs")).to_be_visible(timeout=10000)

    page.locator(".custo-zone .tab", has_text="SPP").click()
    expect(page.locator(".custo-zone .tab.active")).to_have_text("SPP")

    # Repéré par son contenu, pas par sa visibilité : le champ `amount`
    # n'existe que dans le panneau SPP. Juste après un remplacement, Alpine
    # n'a pas encore appliqué `x-show` et les quatre panneaux sont visibles —
    # un sélecteur `:visible` désignerait alors celui des compétences.
    panneau_spp = page.locator(".custo-action-panel:has(input[name='amount'])")
    panneau_spp.locator("input[name='amount']").fill("5")
    cliquer_quand_cable(page, SPP_AJOUTER)

    # Attendre que le remplacement du panneau ait eu lieu **avant** de lire
    # l'onglet : lu trop tôt, on lirait l'ancien DOM, où « SPP » est encore
    # actif, et l'assertion passerait sans rien vérifier.
    expect(page.locator(".custo-zone")).not_to_contain_text(
        "Aucune modification en attente", timeout=10000
    )
    expect(page.locator(".custo-zone .tab.active")).to_have_text("SPP", timeout=10000)

    # L'annulation d'une ligne re-rend le panneau elle aussi.
    cliquer_quand_cable(page, ".btn-cancel-entry")
    expect(page.locator(".custo-zone")).to_contain_text(
        "Aucune modification en attente", timeout=10000
    )
    expect(page.locator(".custo-zone .tab.active")).to_have_text("SPP", timeout=10000)

    # Le vidage du panier n'est pas couvert ici : il **ferme** le mode
    # customisation — comportement voulu, tenu par
    # `test_annuler_vide_le_panier_et_ne_rouvre_pas_le_mode`. Il n'y a donc
    # aucun onglet à restaurer dans ce cas, et la case de la carte 398 qui le
    # demandait n'a pas d'objet.

    # Enfin l'enregistrement, qui recharge la page entière.
    panneau_spp.locator("input[name='amount']").fill("5")
    cliquer_quand_cable(page, SPP_AJOUTER)
    expect(page.locator(".custo-zone")).not_to_contain_text(
        "Aucune modification en attente", timeout=10000
    )
    cliquer_quand_cable(page, "button.btn-submit")

    expect(page.locator(".custo-zone .tabs")).to_be_visible(timeout=10000)
    expect(page.locator(".custo-zone .tab.active")).to_have_text("SPP", timeout=10000)


def test_arriver_sur_la_fiche_ouvre_l_onglet_par_defaut(page: Page, custo_ctx):
    """Le fragment ne doit rien laisser derrière lui : une fiche ouverte
    directement, sans fragment, part de « Compétences » — sinon l'onglet d'un
    joueur suivrait sur le suivant."""
    joueur = custo_ctx["joueurs"][0]
    page.goto(_url(custo_ctx, joueur, "detail"), wait_until="load")
    cliquer_quand_cable(page, ".btn-customise")

    expect(page.locator(".custo-zone .tab.active")).to_have_text(
        "Compétences", timeout=10000
    )

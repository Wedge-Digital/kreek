"""Les points de classement manuels, de l'attribution au rang affiché (carte 453).

**Ce que ce fichier prouve et qu'aucun test unitaire ne peut voir.** La carte 451
l'a mesuré : faire afficher les points de match au lieu du total laissait 1503
tests unitaires verts, alors que le classement aurait contredit son propre ordre.
Le lien entre le rang et le nombre affiché ne se vérifie qu'à l'écran.

**Le réordonnancement sans redémarrage** est le scénario central. Le classement
n'est stocké nulle part : `build_ordered_standings` recalcule l'ordre à chaque
affichage, et aucune propagation n'est due. Si ce test échouait, c'est toute la
conception de la série qui serait fausse.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import pytest
import requests
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from htmx_helpers import cliquer_quand_cable
from match_report_helpers import play_match, wait_ranking_lines

HX = {"HX-Request": "true"}
MEMBRE_SIMPLE = {**HX, "X-Bypass-Auth-Profile": "simple"}


# ── Adresses ──────────────────────────────────────────────────────────────────


def _page_gestion(ctx: dict) -> str:
    return (
        f"{BASE_URL}/app/{ctx['space_id']}/ranking/"
        f"{ctx['competition_id']}/{ctx['season_id']}/manual-points"
    )


def _onglet_classement(ctx: dict) -> str:
    return (
        f"{BASE_URL}/app/{ctx['space_id']}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/standings"
    )


def _onglet_detaille(ctx: dict) -> str:
    return (
        f"{BASE_URL}/app/{ctx['space_id']}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/detailed-standings"
    )


# ── Fixture ───────────────────────────────────────────────────────────────────


@pytest.fixture(scope="module")
def saison_jouee(browser, space_id):
    """Deux équipes, une journée, un match joué — **et aucun point manuel**.

    Une compétition dédiée, et non celle qui traîne en base de développement :
    un test qui dépend d'un état posé à la main rougit chez quelqu'un d'autre.

    Le match donne un vainqueur, donc un ordre connu : c'est de cet ordre-là que
    les tests mesurent l'écart.
    """
    ctx = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    ctx["space_id"] = space_id
    play_match(
        space_id,
        ctx,
        ctx["round_ids"][0],
        ctx["team_ids"][0],
        ctx["team_ids"][1],
        home_td=1,
        away_td=0,
    )
    wait_ranking_lines(ctx["season_id"], expected_lines=2)
    return ctx


@pytest.fixture(autouse=True)
def sans_point_manuel(saison_jouee):
    """Chaque test part d'une saison sans point manuel et la laisse ainsi.

    Sans ce nettoyage, l'ordre des tests deviendrait significatif — et un échec
    porterait alors sur sa prémisse plutôt que sur son assertion, ce qui coûte
    cher à diagnostiquer.
    """
    from db_helpers import execute_db

    sql = f"DELETE FROM ranking__manual_points WHERE season_id = '{saison_jouee['season_id']}'"
    execute_db(sql)
    yield
    execute_db(sql)


# ── Lecture de l'écran ────────────────────────────────────────────────────────


def _lire_compact(page: Page, ctx: dict) -> list[dict]:
    page.goto(_onglet_classement(ctx), wait_until="load")
    page.wait_for_selector(".ranking-classement-widget .standings-row", timeout=15000)
    lignes = page.locator(".ranking-classement-widget .standings-row")
    sortie = []
    for i in range(lignes.count()):
        r = lignes.nth(i)
        sortie.append(
            {
                "rang": r.locator(".standings-rank").inner_text().strip().lstrip("🏆"),
                "equipe": r.locator(".standings-team").inner_text().strip(),
                "manuel": r.locator(".standings-manual").inner_text().strip(),
                "total": r.locator(".standings-pts").inner_text().strip(),
            }
        )
    return sortie


def _lire_detaille(page: Page, ctx: dict) -> list[dict]:
    page.goto(_onglet_detaille(ctx), wait_until="load")
    page.wait_for_selector(".ranking-detailed-standings-widget .sd-table tbody tr", timeout=15000)
    lignes = page.locator(".ranking-detailed-standings-widget .sd-table tbody tr")
    sortie = []
    for i in range(lignes.count()):
        r = lignes.nth(i)
        sortie.append(
            {
                "rang": r.locator("td.sd-rank").inner_text().strip(),
                "equipe": r.locator("td.sd-team").inner_text().strip().lstrip("🏆 "),
                "manuel": r.locator("td.sd-manual").inner_text().strip(),
                "total": r.locator("td.sd-total").inner_text().strip(),
            }
        )
    return sortie


def _est_le_json_des_equipes(reponse) -> bool:
    return "manual-points/teams.json" in reponse.url


def _ouvrir_gestion(page: Page, ctx: dict) -> None:
    """Ouvre la page de gestion et attend que le sélecteur ait ses options.

    **L'attente porte sur la réponse réseau, pas sur l'état du DOM.** Une
    première version guettait la présence d'options : après un enregistrement,
    le formulaire est ré-échangé et le sélecteur se remonte, mais les options du
    montage précédent sont encore là — la condition était donc satisfaite par des
    options périmées pendant que la nouvelle requête volait encore. Le test
    passait huit fois sur dix, et journalisait « Failed to fetch » les deux
    autres, quand la navigation suivante abandonnait la requête.

    Ce n'est pas un défaut du produit : aucun humain ne quitte la page trente
    millisecondes après l'avoir ouverte. C'est le même genre d'artefact que la
    fenêtre non câblée d'htmx, et il se règle de la même façon — en attendant
    une condition précise, jamais une durée.
    """
    with page.expect_response(_est_le_json_des_equipes, timeout=15000):
        page.goto(_page_gestion(ctx), wait_until="load")
    page.wait_for_selector(".mp-form", timeout=15000)


def _attribuer(page: Page, ctx: dict, index_equipe: int, points: int, sens: str, motif: str = ""):
    """Attribue par l'écran, jamais par la base : c'est le formulaire qu'on teste."""
    _ouvrir_gestion(page, ctx)

    page.locator("kreek-select").first.click()
    page.locator("kreek-select .ks-option, kreek-select [role=option]").nth(index_equipe).click()

    page.locator(f'.mp-sign button.{sens}').click()
    page.fill("#mp-points", str(points))
    if motif:
        page.fill("#mp-reason", motif)
    # Le formulaire est ré-échangé, donc le sélecteur se remonte et refait sa
    # requête : on attend **cette** réponse-là, pas la présence d'options.
    with page.expect_response(_est_le_json_des_equipes, timeout=15000):
        cliquer_quand_cable(page, ".mp-btn--primary")
    page.wait_for_selector(".mp-form", timeout=15000)


# ── Scénarios ─────────────────────────────────────────────────────────────────


def test_attribuer_des_points_a_une_equipe(page: Page, saison_jouee, console_errors):
    """Le chemin heureux, par l'écran de bout en bout."""
    _attribuer(page, saison_jouee, 0, 3, "plus", "forfait adverse")

    lignes = query_db(
        "SELECT points || '|' || coalesce(reason, 'NULL') FROM ranking__manual_points "
        f"WHERE season_id = '{saison_jouee['season_id']}'"
    )
    assert lignes == ["3|forfait adverse"], lignes


def test_la_liste_se_recharge_apres_attribution(page: Page, saison_jouee, console_errors):
    """`HX-Trigger` — sans lui, la ligne n'apparaîtrait qu'au rechargement suivant."""
    _ouvrir_gestion(page, saison_jouee)
    assert page.locator(".mp-row--team").count() == 0, "la saison doit partir vide"

    _attribuer(page, saison_jouee, 0, 2, "plus", "bonus d'organisation")

    # Aucun rechargement de page : c'est l'événement qui doit avoir rempli la liste.
    expect(page.locator(".mp-row--team")).to_have_count(1, timeout=10000)
    assert "1 ligne" in page.locator(".mp-row--team").first.inner_text()


def test_le_classement_affiche_les_points_manuels(page: Page, saison_jouee, console_errors):
    """La colonne, **dans les deux vues** — elles calculent séparément."""
    _attribuer(page, saison_jouee, 0, 4, "plus", "forfait adverse")

    compact = _lire_compact(page, saison_jouee)
    detaille = _lire_detaille(page, saison_jouee)

    avec = [l for l in compact if l["manuel"] not in ("—", "")]
    assert len(avec) == 1, compact
    assert avec[0]["manuel"] == "+4"

    avec_d = [l for l in detaille if l["manuel"] not in ("—", "")]
    assert len(avec_d) == 1, detaille
    assert avec_d[0]["manuel"] == "+4"
    assert avec_d[0]["equipe"] == avec[0]["equipe"], "les deux vues doivent viser la même équipe"


def test_le_total_affiche_est_celui_qui_ordonne(page: Page, saison_jouee, console_errors):
    """**La limite que la carte 451 a mesurée.**

    Faire afficher les points de match au lieu du total laissait 1503 tests
    unitaires verts, alors que le classement aurait contredit son propre ordre.
    Ce test lie le rang au nombre affiché : les totaux doivent décroître avec le
    rang, sinon la colonne ment sur l'ordre.
    """
    _attribuer(page, saison_jouee, 1, 5, "plus", "rattrapage")

    for lignes in (_lire_compact(page, saison_jouee), _lire_detaille(page, saison_jouee)):
        totaux = [int(l["total"].replace("−", "-")) for l in lignes]
        assert totaux == sorted(totaux, reverse=True), (
            f"le total affiché contredit l'ordre : {lignes}"
        )


def test_le_classement_est_reordonne_sans_rechargement_du_serveur(
    page: Page, saison_jouee, console_errors
):
    """**Le test qui compte.**

    Le classement n'est stocké nulle part : `build_ordered_standings` recalcule
    l'ordre à chaque affichage. Aucune propagation n'est due, aucun redémarrage
    n'est nécessaire — et c'est ce que ce test constate.
    """
    avant = _lire_compact(page, saison_jouee)
    perdante = avant[-1]["equipe"]
    assert avant[0]["equipe"] != perdante

    # De quoi renverser l'ordre : la perdante est à 0, la gagnante à 3.
    _attribuer(page, saison_jouee, 1, 5, "plus", "forfait adverse")

    apres = _lire_compact(page, saison_jouee)
    assert apres[0]["equipe"] == perdante, f"l'ordre n'a pas suivi : {apres}"
    assert apres[0]["rang"] == "1"

    # Et les deux vues s'accordent : elles calculent séparément, et c'est
    # exactement ce désaccord que la carte 451 a trouvé.
    detaille = _lire_detaille(page, saison_jouee)
    assert [l["equipe"] for l in detaille] == [l["equipe"] for l in apres]


def test_une_penalite_fait_descendre_l_equipe(page: Page, saison_jouee, console_errors):
    """Le sens négatif, bout en bout."""
    avant = _lire_compact(page, saison_jouee)
    gagnante = avant[0]["equipe"]

    _attribuer(page, saison_jouee, 0, 5, "minus", "sanction")

    apres = _lire_compact(page, saison_jouee)
    assert apres[-1]["equipe"] == gagnante, f"la sanctionnée doit descendre : {apres}"
    assert apres[-1]["manuel"] == "−5"
    assert int(apres[-1]["total"].replace("−", "-")) < 0, "3 − 5 = −2, un rang valide"


def test_supprimer_une_ligne_la_retire_du_classement(page: Page, saison_jouee, console_errors):
    """Le retour en arrière — le classement redevient ce qu'il était."""
    ordre_initial = [l["equipe"] for l in _lire_compact(page, saison_jouee)]

    _attribuer(page, saison_jouee, 1, 5, "plus", "à retirer")
    assert [l["equipe"] for l in _lire_compact(page, saison_jouee)] != ordre_initial

    _ouvrir_gestion(page, saison_jouee)
    page.wait_for_selector(".mp-row--team", timeout=15000)
    # **La ligne de groupe est un élément Alpine, pas htmx** : `cliquer_quand_cable`
    # y attendrait un câblage qui n'arrive jamais. Le ✕, lui, porte `hx-delete`,
    # et c'est là que la fenêtre non câblée existe pour de bon.
    page.locator(".mp-row--team").first.click()
    page.wait_for_selector(".mp-icon-btn", state="visible", timeout=10000)
    cliquer_quand_cable(page, ".mp-icon-btn")
    expect(page.locator(".mp-row--team")).to_have_count(0, timeout=10000)

    assert [l["equipe"] for l in _lire_compact(page, saison_jouee)] == ordre_initial


def test_un_non_admin_voit_la_page_sans_les_actions(saison_jouee):
    """Public en lecture, réservé en écriture.

    Les points manuels s'affichent déjà dans le classement : réserver la page
    donnerait à croire qu'ils se cachent. Ce sont les gestes qui sont réservés,
    et le gabarit ne doit pas en proposer qu'on refusera.
    """
    url = _page_gestion(saison_jouee)

    lecture = requests.get(url, headers=MEMBRE_SIMPLE, timeout=15)
    assert lecture.status_code == 200, "la page est publique"

    formulaire = requests.get(f"{url}/form", headers=MEMBRE_SIMPLE, timeout=15)
    assert formulaire.status_code == 200
    assert "Lecture seule" in formulaire.text
    assert "mp-btn--primary" not in formulaire.text, "aucun bouton d'attribution"

    ecriture = requests.post(
        url,
        data={"team_id": saison_jouee["team_ids"][0], "direction": "bonus", "points": "1", "reason": "x"},
        headers=MEMBRE_SIMPLE,
        timeout=20,
    )
    assert ecriture.status_code == 403, f"écriture : {ecriture.status_code}"


# ── Le CSS rencontre-t-il son markup ? (carte 487) ────────────────────────────


@pytest.mark.parametrize("onglet", ("classement", "detaille"))
def test_le_bouton_de_gestion_est_habille(page: Page, saison_jouee, onglet):
    """**Six règles écrites, aucun markup pour les recevoir.**

    Le bloc du bouton vivait sous le `</div>` de fermeture du widget, alors que
    les règles de la feuille sont scopées sous la racine. Mesuré avant
    correction : `padding: 0px`, `border: none`, fond transparent — un lien de
    corps de texte, dans les deux onglets de classement.

    L'assertion porte sur le **style calculé**, pas sur la présence du bloc : un
    test qui vérifie que le bouton existe passait déjà quand il était nu.
    """
    ctx = saison_jouee
    url = _onglet_classement(ctx) if onglet == "classement" else _onglet_detaille(ctx)
    page.goto(url, wait_until="load")

    lien = page.locator("[class$='-manage-link']").first
    expect(lien).to_be_visible(timeout=10000)

    style = lien.evaluate(
        """e => { const c = getComputedStyle(e); return {
             pad: c.paddingTop, bord: c.borderTopWidth, rayon: c.borderTopLeftRadius,
             fond: c.backgroundColor, dansLaRacine: !!e.closest("[class^='ranking-']") };
           }"""
    )
    assert style["dansLaRacine"], f"{onglet} : le bouton est hors de la racine du widget"
    assert style["pad"] != "0px", f"{onglet} : aucun padding — les règles ne s'appliquent pas"
    assert style["bord"] != "0px", f"{onglet} : aucune bordure"
    assert style["rayon"] != "0px", f"{onglet} : aucun rayon"
    assert "0, 0, 0, 0" not in style["fond"], f"{onglet} : fond transparent"


@pytest.mark.parametrize("onglet", ("classement", "detaille"))
def test_le_bouton_de_gestion_precede_le_classement(page: Page, saison_jouee, onglet):
    """Il est en tête du widget, à droite — décision prise sur maquette.

    Pas accroché à un titre : les seuls titres du widget sont ceux des poules,
    et ils sont optionnels. Les points manuels s'attribuent par saison, pas par
    poule.
    """
    ctx = saison_jouee
    url = _onglet_classement(ctx) if onglet == "classement" else _onglet_detaille(ctx)
    page.goto(url, wait_until="load")

    lien = page.locator("[class$='-manage-link']").first
    expect(lien).to_be_visible(timeout=10000)
    boite = lien.bounding_box()
    tableau = page.locator(".standings-table, .sd-scroll, .tab-empty-state").first
    expect(tableau).to_be_visible(timeout=10000)
    tb = tableau.bounding_box()

    assert boite["y"] < tb["y"], f"{onglet} : le bouton devrait précéder le tableau"
    racine = page.locator("[class^='ranking-'][hx-disinherit]").first.bounding_box()
    ecart = racine["x"] + racine["width"] - (boite["x"] + boite["width"])
    assert ecart < 4, f"{onglet} : le bouton n'est pas collé à droite ({ecart:.0f} px)"


def test_la_rangee_d_attribution_partage_une_ligne_de_base(page: Page, saison_jouee):
    """**Quatre champs, un bouton, un seul bas.**

    `.mp-form` aligne par `align-items: flex-end`, donc par le bas du
    *conteneur* et non du champ. Le motif était seul à porter une ligne d'aide —
    14 px, plus 5 px de gouttière : les 19 px de décrochage mesurés.

    Le bouton compte autant que les champs : sans place d'aide à réserver, lui
    seul serait descendu sous la rangée, l'inverse exact du défaut corrigé.
    """
    # Une fenêtre large : `.mp-form` porte `flex-wrap: wrap`, et sous ~1100 px le
    # bouton passe à la ligne — mesuré à 82 px plus bas, ce qui n'est pas un
    # désalignement mais un enroulement voulu. Sous 768 px la rangée devient même
    # une colonne, par media query.
    page.set_viewport_size({"width": 1440, "height": 900})
    page.goto(_page_gestion(saison_jouee), wait_until="load")
    expect(page.locator("kreek-select")).to_be_visible(timeout=10000)

    bas = page.evaluate(
        """() => {
             const b = s => { const e = document.querySelector(s);
                              return e ? Math.round(e.getBoundingClientRect().bottom) : null; };
             return { equipe: b('kreek-select'), sens: b('.mp-sign'),
                      points: b('input[name=points]'), motif: b('input[name=reason]'),
                      bouton: b('.mp-btn--primary') };
           }"""
    )
    assert None not in bas.values(), f"un élément de la rangée est absent : {bas}"
    assert len(set(bas.values())) == 1, f"la rangée n'a pas une seule ligne de base : {bas}"


def test_le_menu_du_selecteur_deborde_du_panneau(page: Page, saison_jouee):
    """**Le menu d'équipes était rogné par le panneau qui l'entoure.**

    `.mp-panel` porte `overflow: hidden`, qui sert à ses coins arrondis — sans
    lui, le fond gris de l'en-tête déborde de la courbe. Mais un `overflow`
    coupe aussi les enfants en `position: absolute`, et le menu du
    `kreek-select` en est un : le commissaire n'en voyait qu'une tranche.

    **`getBoundingClientRect` ne voit pas ce défaut** : le rectangle du menu est
    le même, coupé ou non. Seul `elementFromPoint` dit ce qui est réellement
    peint à un endroit donné — c'est pourquoi ce test tâte un point du menu
    situé sous la limite du panneau, plutôt que de comparer des coordonnées.
    """
    page.set_viewport_size({"width": 1440, "height": 900})
    page.goto(_page_gestion(saison_jouee), wait_until="load")
    page.locator("kreek-select").first.wait_for(state="attached", timeout=15000)

    # Pas de `cliquer_quand_cable` ici : le sélecteur est un Web Component, pas
    # un élément câblé par htmx. Il charge ses options par `fetch` ; c'est leur
    # apparition qu'on attend, pas un câblage.
    page.click("kreek-select .ks-control")
    page.locator(".ks-dropdown .ks-option").first.wait_for(state="visible", timeout=10000)

    verdict = page.evaluate(
        """() => {
             const menu = document.querySelector('.ks-dropdown');
             const panneau = document.querySelector('.mp-panel--form');
             const m = menu.getBoundingClientRect();
             const p = panneau.getBoundingClientRect();
             const y = m.bottom - 6;
             const touche = document.elementFromPoint(m.left + m.width / 2, y);
             return { depasse: Math.round(m.bottom - p.bottom),
                      visible: !!(touche && menu.contains(touche)),
                      touche: touche ? (touche.className || touche.tagName) : 'rien' };
           }"""
    )

    assert verdict["depasse"] > 0, (
        "le menu ne dépasse pas du panneau : le test ne prouve rien — "
        "la fixture doit avoir assez d'équipes pour une liste plus haute"
    )
    assert verdict["visible"], (
        f"le bas du menu est coupé par le panneau : on y touche "
        f"« {verdict['touche']} » au lieu d'une option"
    )

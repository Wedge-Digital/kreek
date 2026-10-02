"""L'identité d'équipe dans le classement (cartes 566 et 567).

Chaque ligne du classement charge le widget d'identité de `teams` : logo ou
initiales, nom, badge de statut, roster · coach. `ranking` n'en connaît que
l'adresse, fournie par un port.

**Ce que ce fichier prouve et qu'aucun test unitaire ne peut voir** : que le
widget se charge *dans sa case* — la ligne porte un `hx-target="#app-content"`,
et sans `hx-disinherit` le widget remplacerait la page entière — et que la
ligne repliée tient dans un écran de téléphone sans défilement horizontal.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true), `make seed_e2e`.
"""

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import query_db
from match_report_helpers import play_match, wait_ranking_lines


def _onglet_classement(ctx: dict) -> str:
    return (
        f"{BASE_URL}/app/{ctx['space_id']}/competitions/"
        f"{ctx['competition_id']}/{ctx['season_id']}/standings"
    )


def _fiche(ctx: dict, team_id: str) -> str:
    return f"{BASE_URL}/app/{ctx['space_id']}/teams/{team_id}"


@pytest.fixture(scope="module")
def saison_jouee(browser, space_id):
    """Deux équipes, un match joué : le classement a deux lignes."""
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


def _ouvrir_classement(page: Page, ctx: dict):
    page.goto(_onglet_classement(ctx), wait_until="load")
    lignes = page.locator(".ranking-classement-widget .standings-row")
    expect(lignes).to_have_count(2, timeout=15000)
    # Le widget remplace le nom d'attente : on attend qu'il soit là partout.
    expect(lignes.locator(".team-identity")).to_have_count(2, timeout=10000)
    return lignes


def _nom_en_base(team_id: str) -> str:
    return query_db(f"SELECT team_name FROM team_proj WHERE team_id = '{team_id}'")[0]


def _coachs_en_base(ctx: dict) -> dict[str, str]:
    """Nom d'équipe → coach, tels que `teams` les connaît.

    Le coach est un texte libre que la fixture laisse **vide**
    (`competition_lifecycle.py` envoie `"coach_name": ""`) : le test compare
    donc à la base au lieu de supposer qu'un coach existe toujours.
    """
    ids = ", ".join(f"'{t}'" for t in ctx["team_ids"])
    lignes = query_db(f"SELECT team_name, coach_name FROM team_proj WHERE team_id IN ({ids})")
    return dict(ligne.split("|", 1) for ligne in lignes)


def test_chaque_ligne_affiche_l_identite_de_son_equipe(page: Page, saison_jouee, console_errors):
    lignes = _ouvrir_classement(page, saison_jouee)
    coachs = _coachs_en_base(saison_jouee)

    noms = set()
    for i in range(2):
        identite = lignes.nth(i).locator(".standings-team .team-identity")
        nom = identite.locator(".team-identity-name-text").inner_text().strip()
        noms.add(nom)
        expect(identite.locator(".team-identity-logo")).to_be_visible()
        expect(identite.locator(".team-identity-roster")).not_to_be_empty()
        expect(identite.locator(".team-status-badge")).not_to_be_empty()
        coach = identite.locator(".team-identity-coach")
        if coachs[nom]:
            expect(coach).to_have_text(coachs[nom])
        else:
            expect(coach).to_have_count(0)

    assert noms == set(coachs)


def test_le_widget_reste_dans_sa_case(page: Page, saison_jouee, console_errors):
    """La ligne cible `#app-content` : un héritage non coupé ferait remplacer
    la page par le widget. La page de compétition doit survivre au chargement."""
    _ouvrir_classement(page, saison_jouee)
    expect(page.locator(".ranking-classement-widget .standings-header")).to_be_visible()
    assert page.url == _onglet_classement(saison_jouee), "l'adresse ne doit pas changer"


def test_le_badge_dit_la_meme_chose_que_la_fiche(page: Page, saison_jouee, console_errors):
    lignes = _ouvrir_classement(page, saison_jouee)
    nom = lignes.nth(0).locator(".team-identity-name-text").inner_text().strip()
    badge = lignes.nth(0).locator(".team-status-badge").inner_text().strip()
    team_id = next(t for t in saison_jouee["team_ids"] if _nom_en_base(t) == nom)

    page.goto(_fiche(saison_jouee, team_id), wait_until="load")
    expect(page.locator(".team-header .team-status-badge")).to_have_text(badge)


def test_sur_mobile_la_ligne_se_replie_sans_rien_perdre(page: Page, saison_jouee, console_errors):
    page.set_viewport_size({"width": 375, "height": 812})
    lignes = _ouvrir_classement(page, saison_jouee)

    largeur = page.evaluate("document.documentElement.scrollWidth")
    assert largeur <= 375, f"la page défile horizontalement : {largeur}px"

    ligne = lignes.nth(0)
    identite = ligne.locator(".team-identity").bounding_box()
    chiffres = ligne.locator(".standings-num, .standings-pts")
    expect(chiffres).to_have_count(6)
    expect(ligne.locator(".team-identity-roster")).to_be_hidden()
    nom = ligne.locator(".team-identity-name-text").inner_text().strip()
    if _coachs_en_base(saison_jouee)[nom]:
        expect(ligne.locator(".team-identity-coach")).to_be_visible()
    for i in range(6):
        boite = chiffres.nth(i).bounding_box()
        assert boite["x"] + boite["width"] <= 375, f"colonne {i} hors de l'écran"
        assert boite["y"] >= identite["y"] + identite["height"] - 1, (
            f"colonne {i} : les chiffres passent sous l'identité"
        )

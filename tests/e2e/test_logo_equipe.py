"""Tests E2E — changer le logo d'une équipe depuis sa fiche (cartes 508 à 512, 581).

Le défaut qui a motivé ces scénarios : annuler une fois le sélecteur d'image
retirait l'écouteur `cloudinaryUploadClosed`. L'envoi suivant mettait l'aperçu
à jour, mais plus rien ne soumettait le formulaire — le widget n'a pas de
bouton « Enregistrer » — et le logo disparaissait au rechargement.

# Pourquoi aucun test unitaire ne pouvait le voir

Le défaut vivait dans le JavaScript du widget, entre la fermeture du sélecteur
et la soumission du formulaire. Côté serveur, tout était juste : il ne recevait
simplement jamais rien.

# Le sélecteur Cloudinary est simulé

Le vrai widget charge `all.js` depuis Internet et ouvre une iframe tierce. Le
test sert à sa place un faux `all.js` qui retient le rappel que la macro
`upload_widget.html` lui confie : le test joue lui-même « image envoyée » et
« fenêtre fermée », exactement comme Cloudinary les signale.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true) et base seedée.
"""

import re
import uuid

import pytest
from playwright.sync_api import Page, expect

from competition_lifecycle import BASE_URL, build_full_competition
from db_helpers import attendre_que, query_db

# Le faux `all.js` : `createUploadWidget` garde le rappel, `open()` ne fait rien.
FAUX_CLOUDINARY = """
window.cloudinary = {
    createUploadWidget: function (options, rappel) {
        window.__rappelsCloudinary = window.__rappelsCloudinary || [];
        window.__rappelsCloudinary.push(rappel);
        return { open: function () {} };
    }
};
"""


@pytest.fixture(scope="module")
def logo_ctx(browser, space_id):
    full = build_full_competition(browser, space_id, num_teams=2, num_rounds=1)
    return {
        "space_id": space_id,
        "competition_id": full["competition_id"],
        "season_id": full["season_id"],
        "equipes": full["team_ids"],
    }


def nouveau_logo() -> tuple[str, str]:
    """Une URL Cloudinary propre à ce passage, et l'identifiant qui la signe.

    Les équipes de `build_full_competition` portent déjà un logo : une URL
    neuve est la seule façon de voir que c'est **le nôtre** qui s'affiche. La
    fiche transforme l'URL (`c_fill,w_120…`), on cherche donc l'identifiant.
    """
    identifiant = f"logo-e2e-{uuid.uuid4().hex[:8]}"
    return f"https://res.cloudinary.com/demo/image/upload/v1/{identifiant}.jpg", identifiant


def ouvrir_la_fiche(page: Page, ctx, equipe: str):
    page.route(
        "**/widget.cloudinary.com/**",
        lambda route: route.fulfill(content_type="application/javascript", body=FAUX_CLOUDINARY),
    )
    page.goto(f"{BASE_URL}/app/{ctx['space_id']}/teams/{equipe}", wait_until="load")
    expect(page.locator(".team-logo .team-logo-edit")).to_be_visible(timeout=10000)
    page.locator(".team-logo .team-logo-edit").click()
    page.locator(".team-logo #zone-logo_url").click()
    page.wait_for_function("(window.__rappelsCloudinary || []).length > 0")


def fermer_le_selecteur(page: Page):
    page.evaluate("window.__rappelsCloudinary.at(-1)(null, { event: 'close' })")


def envoyer_une_image(page: Page, url: str):
    """Ce que Cloudinary signale après un envoi réussi : `success`, puis `close`."""
    with page.expect_response(lambda r: r.request.method == "POST" and "/widgets/logo" in r.url):
        page.evaluate(
            "url => { const rappel = window.__rappelsCloudinary.at(-1);"
            " rappel(null, { event: 'success', info: { secure_url: url } });"
            " rappel(null, { event: 'close' }); }",
            url,
        )


def logo_en_base(equipe: str) -> str:
    lignes = query_db(f"SELECT COALESCE(logo_url, '') FROM team_proj WHERE team_id = '{equipe}'")
    return lignes[0] if lignes else ""


def test_changer_le_logo_survit_au_rechargement(page: Page, logo_ctx):
    """Le scénario de base : envoyer une image, recharger, la voir."""
    equipe = logo_ctx["equipes"][0]
    url, identifiant = nouveau_logo()

    ouvrir_la_fiche(page, logo_ctx, equipe)
    envoyer_une_image(page, url)
    page.reload(wait_until="load")

    expect(page.locator(".team-logo .team-header-logo-img")).to_have_attribute(
        "src", re.compile(identifiant), timeout=10000
    )
    assert logo_en_base(equipe) == url


def test_annuler_une_fois_puis_envoyer_enregistre_le_logo(page: Page, logo_ctx):
    """**Le défaut de la revue.** Une annulation d'abord, puis un envoi : le
    second doit encore soumettre le formulaire."""
    equipe = logo_ctx["equipes"][1]
    url, identifiant = nouveau_logo()

    ouvrir_la_fiche(page, logo_ctx, equipe)
    fermer_le_selecteur(page)
    envoyer_une_image(page, url)
    page.reload(wait_until="load")

    expect(page.locator(".team-logo .team-header-logo-img")).to_have_attribute(
        "src", re.compile(identifiant), timeout=10000
    )
    assert logo_en_base(equipe) == url


def test_le_nouveau_logo_atteint_le_calendrier(page: Page, logo_ctx):
    """Carte 581 : `competitions` garde sa propre copie du logo, faite à
    l'appariement. Le nouveau logo doit y arriver sans réappariement."""
    equipe = logo_ctx["equipes"][0]
    url, identifiant = nouveau_logo()

    ouvrir_la_fiche(page, logo_ctx, equipe)
    envoyer_une_image(page, url)

    # L'app event est traité dans une tâche séparée : la projection de
    # `competitions` n'est pas à jour au retour du POST.
    attendre_que(
        lambda: query_db(
            "SELECT count(*) FROM competition_match_display_proj "
            f"WHERE (home_team_id = '{equipe}' AND home_logo_url = '{url}') "
            f"OR (away_team_id = '{equipe}' AND away_logo_url = '{url}')"
        )[0] != "0",
        quoi="le logo reporté dans competition_match_display_proj",
    )
    page.goto(
        f"{BASE_URL}/app/{logo_ctx['space_id']}/competitions/"
        f"{logo_ctx['competition_id']}/{logo_ctx['season_id']}/calendrier",
        wait_until="load",
    )
    expect(page.locator(f"img.cal-logo[src*='{identifiant}']").first).to_be_visible(timeout=10000)

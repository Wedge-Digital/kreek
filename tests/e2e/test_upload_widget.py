"""Tests E2E du champ d'upload d'image (carte 17).

Le widget maison de Cloudinary a été retiré : c'était un script tiers qui allait
chercher sa propre feuille de style sur son domaine au moment d'ouvrir sa boîte
de dialogue, et cette requête tombait pendant une navigation HTMX.

Ce qui le remplace est du code à nous, donc à couvrir. **La requête vers
Cloudinary est interceptée** : un vrai envoi déposerait un fichier dans le
compte à chaque exécution de la suite, et ferait dépendre la CI d'un service
tiers pour tester du code qui est entièrement local.

Prérequis : serveur kreek lancé en dev.
"""

import json

from playwright.sync_api import Page, expect

BASE_URL = "http://localhost:3210"
CLOUDINARY = "**/api.cloudinary.com/**"

# Un PNG 1×1 valide — le contenu n'a aucune importance, seuls le type MIME et la
# taille sont examinés par le widget.
PNG = bytes.fromhex(
    "89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c489"
    "0000000a49444154789c6360000002000100ffff03000006000557bfabd4000000"
    "0049454e44ae426082"
)
URL_RENVOYEE = "https://res.cloudinary.com/bloodbowlclub-com/image/upload/v1/logo-de-test.png"


def _ouvrir(page: Page, space_id: str) -> None:
    page.goto(f"{BASE_URL}/app/{space_id}/team/create", wait_until="load")
    page.wait_for_selector("#draft-team-form", timeout=5000)
    page.wait_for_selector("#zone-logo_url", timeout=5000)


def _intercepter(page: Page, appels: list[str]):
    """Répond à la place de Cloudinary, et note ce qui lui aurait été envoyé."""

    def repondre(route):
        appels.append(route.request.url)
        route.fulfill(
            status=200,
            content_type="application/json",
            body=json.dumps({"secure_url": URL_RENVOYEE}),
        )

    page.route(CLOUDINARY, repondre)


def _deposer(page: Page, nom: str, mime: str, contenu: bytes) -> None:
    page.set_input_files(
        "#fichier-logo_url", files=[{"name": nom, "mimeType": mime, "buffer": contenu}]
    )


# ── Scénario 1 — un envoi réussi renseigne le champ soumis ───────────────────


def test_envoi_reussi_renseigne_le_champ_et_l_apercu(page: Page, space_id):
    _ouvrir(page, space_id)
    appels: list[str] = []
    _intercepter(page, appels)

    _deposer(page, "logo.png", "image/png", PNG)

    # C'est ce champ-là qui part avec le formulaire : sans lui, l'envoi n'a
    # servi à rien, quelle que soit l'apparence de l'aperçu.
    expect(page.locator("#logo_url")).to_have_value(URL_RENVOYEE, timeout=10000)
    expect(page.locator("#preview-logo_url")).to_be_visible()
    expect(page.locator("#placeholder-logo_url")).to_be_hidden()
    expect(page.locator("#statut-logo_url")).to_be_hidden()
    assert len(appels) == 1, f"un seul envoi attendu, reçu {appels}"


# ── Scénario 2 — un format refusé ne part pas sur le réseau ──────────────────


def test_format_refuse_ne_declenche_aucun_envoi(page: Page, space_id):
    _ouvrir(page, space_id)
    appels: list[str] = []
    _intercepter(page, appels)

    _deposer(page, "roster.pdf", "application/pdf", b"%PDF-1.4 pas une image")

    statut = page.locator("#statut-logo_url")
    expect(statut).to_be_visible(timeout=5000)
    expect(statut).to_contain_text("JPG, PNG ou WebP")
    expect(statut).to_have_class("upload-statut upload-statut--erreur")
    expect(page.locator("#logo_url")).to_have_value("")
    page.wait_for_timeout(500)
    assert appels == [], f"aucun envoi ne doit partir sur un format refusé : {appels}"


# ── Scénario 3 — au-delà de 2 Mo, refus local également ──────────────────────


def test_image_trop_lourde_est_refusee_localement(page: Page, space_id):
    _ouvrir(page, space_id)
    appels: list[str] = []
    _intercepter(page, appels)

    _deposer(page, "enorme.png", "image/png", PNG + b"\x00" * 2_000_001)

    statut = page.locator("#statut-logo_url")
    expect(statut).to_be_visible(timeout=5000)
    expect(statut).to_contain_text("2 Mo maximum")
    expect(page.locator("#logo_url")).to_have_value("")
    page.wait_for_timeout(500)
    assert appels == [], f"aucun envoi ne doit partir sur un fichier trop lourd : {appels}"


# ── Scénario 4 — un refus de Cloudinary est dit, pas avalé ───────────────────


def test_refus_du_serveur_d_images_est_affiche(page: Page, space_id):
    _ouvrir(page, space_id)
    page.route(
        CLOUDINARY,
        lambda route: route.fulfill(status=401, content_type="text/plain", body="nope"),
    )

    _deposer(page, "logo.png", "image/png", PNG)

    statut = page.locator("#statut-logo_url")
    expect(statut).to_be_visible(timeout=10000)
    expect(statut).to_contain_text("401")
    expect(statut).to_have_class("upload-statut upload-statut--erreur")
    # Le champ garde sa valeur précédente — ici vide : un envoi refusé ne doit
    # pas laisser croire qu'une image est attachée.
    expect(page.locator("#logo_url")).to_have_value("")


# ── Scénario 5 — plus aucune ressource tierce dans la page ───────────────────


def test_la_page_ne_charge_plus_aucun_script_tiers(page: Page, space_id):
    tiers: list[str] = []
    page.on(
        "request",
        lambda r: tiers.append(r.url)
        if r.resource_type in ("script", "stylesheet", "font")
        and "localhost" not in r.url
        else None,
    )
    _ouvrir(page, space_id)
    page.wait_for_timeout(1500)

    assert tiers == [], f"scripts, feuilles et polices doivent être servis localement : {tiers}"

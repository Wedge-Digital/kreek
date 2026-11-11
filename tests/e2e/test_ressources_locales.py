"""Ce qui sort de nos domaines, et depuis quelles pages (carte 17).

Trois origines externes ont été supprimées : `cdn.jsdelivr.net` (Tom Select et
Alpine) et `fonts.googleapis.com` (l'`@import` qui ouvrait `common.css`). Elles
ne se voient dans aucun test de comportement — une page fonctionne exactement
pareil que la ressource vienne d'un CDN ou de `/static` — et un
`<script src="https://…">` réintroduit se lit comme n'importe quelle autre ligne
dans une revue. D'où ces tests, qui regardent le réseau et non le DOM.

La quatrième, `widget.cloudinary.com`, est **conservée** : la remplacer par un
champ maison faisait perdre la source URL, le glisser-déposer et le recadrage.
Mais son chargeur tire de lui-même Google Tag Manager, GA4 et Rollbar, donc il
n'est plus dans le layout : la macro du champ d'upload le pose elle-même, et
seulement au premier clic sur la zone (carte 592). C'est **cette frontière** que
le second test tient — sans lui, un retour du script dans le layout, ou un
chargement à l'ouverture de la page, ne casserait rien de visible, et les trois
traqueurs suivraient à nouveau le coach partout.

Prérequis : serveur kreek lancé en dev.
"""

from playwright.sync_api import Page

from e2e_env import BASE_URL

# Tiré par `widget.cloudinary.com/v2.0/global/all.js`, pas par nous. Listé
# nommément plutôt que sous un `cloudinary.com` large : le jour où le widget
# ajoute une quatrième origine, ce test le dira.
DANS_LE_PAQUET_CLOUDINARY = ("cloudinary.com", "googletagmanager.com", "cdnjs.cloudflare.com")

RESSOURCES = ("script", "stylesheet", "font")


def _ecouter(page: Page) -> list[str]:
    vues: list[str] = []
    page.on(
        "request",
        lambda r: vues.append(f"{r.resource_type} {r.url.split('/')[2]}")
        if "localhost" not in r.url and r.resource_type in RESSOURCES
        else None,
    )
    return vues


def _tierces(page: Page, chemin: str, space_id: str) -> list[str]:
    vues = _ecouter(page)
    page.goto(f"{BASE_URL}/app/{space_id}/{chemin}", wait_until="load")
    page.wait_for_timeout(2500)
    return vues


def test_une_page_sans_upload_ne_sort_pas_de_nos_domaines(page: Page, space_id):
    """L'accueil n'a pas de champ d'upload : rien n'a à en sortir. C'est le test
    qui couvre le vendoring de Tom Select, d'Alpine et des polices."""
    assert _tierces(page, "home", space_id) == []


def test_le_chargeur_cloudinary_ne_part_qu_au_clic_sur_le_champ(page: Page, space_id):
    """Sur la page qui porte le champ, rien ne sort tant qu'on n'y touche pas.
    Au clic, le chargeur part — et n'amène que ce qu'on lui connaît."""
    vues = _tierces(page, "team/create", space_id)
    assert vues == [], "Cloudinary est chargé sans clic :\n  " + "\n  ".join(vues)

    page.locator("#zone-logo_url").click()
    page.wait_for_function("() => !!window.cloudinary", timeout=15000)
    page.wait_for_timeout(2500)
    inconnues = [v for v in vues if not any(h in v for h in DANS_LE_PAQUET_CLOUDINARY)]

    assert any("widget.cloudinary.com" in v for v in vues), (
        "le clic ne charge pas Cloudinary : le champ d'upload est inerte"
    )
    assert not inconnues, "origines inattendues :\n  " + "\n  ".join(inconnues)


def test_le_premier_clic_ouvre_le_champ_d_upload(page: Page, space_id):
    """Le point fragile du chargement au clic : `all.js` n'est pas là quand la
    zone est cliquée. Le clic doit le télécharger, créer le widget **et**
    l'ouvrir — si l'un des trois casse, la zone devient muette, et **aucun
    test de réseau ne le verrait**."""
    page.goto(f"{BASE_URL}/app/{space_id}/team/create", wait_until="load")
    page.wait_for_selector("#zone-logo_url", timeout=5000)

    page.locator("#zone-logo_url").click()

    # Le widget s'affiche dans son propre iframe : sa présence atteste que
    # `createUploadWidget` a bien été appelé et que `open()` est branché.
    # Ciblé par son `src` : Cloudinary ne pose sur cet iframe ni classe ni id.
    page.wait_for_selector("iframe[src*='upload-widget.cloudinary.com']", timeout=15000)

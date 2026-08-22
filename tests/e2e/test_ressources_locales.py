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
n'est plus dans le layout : la macro du champ d'upload le pose elle-même, sur
les deux seules pages concernées. C'est **cette frontière** que le second test
tient — sans lui, un retour du script dans le layout ne casserait rien de
visible, et les trois traqueurs suivraient à nouveau le coach partout.

Prérequis : serveur kreek lancé en dev.
"""

from playwright.sync_api import Page

BASE_URL = "http://localhost:3210"

# Tiré par `widget.cloudinary.com/v2.0/global/all.js`, pas par nous. Listé
# nommément plutôt que sous un `cloudinary.com` large : le jour où le widget
# ajoute une quatrième origine, ce test le dira.
DANS_LE_PAQUET_CLOUDINARY = ("cloudinary.com", "googletagmanager.com", "cdnjs.cloudflare.com")

RESSOURCES = ("script", "stylesheet", "font")


def _tierces(page: Page, chemin: str, space_id: str) -> list[str]:
    vues: list[str] = []
    page.on(
        "request",
        lambda r: vues.append(f"{r.resource_type} {r.url.split('/')[2]}")
        if "localhost" not in r.url and r.resource_type in RESSOURCES
        else None,
    )
    page.goto(f"{BASE_URL}/app/{space_id}/{chemin}", wait_until="load")
    page.wait_for_timeout(2500)
    return vues


def test_une_page_sans_upload_ne_sort_pas_de_nos_domaines(page: Page, space_id):
    """L'accueil n'a pas de champ d'upload : rien n'a à en sortir. C'est le test
    qui couvre le vendoring de Tom Select, d'Alpine et des polices."""
    assert _tierces(page, "home", space_id) == []


def test_le_chargeur_cloudinary_reste_cantonne_a_la_page_qui_l_utilise(page: Page, space_id):
    """Sur la page qui porte le champ, le widget se charge — et n'amène que ce
    qu'on lui connaît."""
    vues = _tierces(page, "team/create", space_id)
    inconnues = [v for v in vues if not any(h in v for h in DANS_LE_PAQUET_CLOUDINARY)]

    assert any("widget.cloudinary.com" in v for v in vues), (
        "le chargeur Cloudinary ne se charge plus : le champ d'upload est inerte"
    )
    assert not inconnues, "origines inattendues :\n  " + "\n  ".join(inconnues)


def test_le_champ_d_upload_s_ouvre_malgre_le_chargement_differe(page: Page, space_id):
    """Le point fragile du chargement à la demande : `all.js` n'est plus là
    quand le script en ligne du champ s'exécute. Son corps est donc différé
    jusqu'à ce que `cloudinary` existe — si ce report casse, la zone devient
    muette au clic, et **aucun test de réseau ne le verrait**."""
    page.goto(f"{BASE_URL}/app/{space_id}/team/create", wait_until="load")
    page.wait_for_selector("#zone-logo_url", timeout=5000)
    page.wait_for_function("() => !!window.cloudinary", timeout=15000)

    page.locator("#zone-logo_url").click()

    # Le widget s'affiche dans son propre iframe : sa présence atteste que
    # `createUploadWidget` a bien été appelé et que `open()` est branché.
    # Ciblé par son `src` : Cloudinary ne pose sur cet iframe ni classe ni id.
    page.wait_for_selector("iframe[src*='upload-widget.cloudinary.com']", timeout=15000)

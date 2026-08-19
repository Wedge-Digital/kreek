"""Relève les styles calculés de chaque élément, sous un libellé.

    uv run python visual/releve.py avant
    …  modification du CSS  …
    uv run python visual/releve.py apres
    uv run python visual/comparer.py avant apres

# Pourquoi les styles calculés et non des captures d'écran

La carte 341 n'énonce pas sa contrainte en pixels :

    « Aucune valeur calculée ne change. On ne modifie que la portée des
      règles, jamais leur contenu. »

« Valeur calculée » est le terme du navigateur — `getComputedStyle`. Comparer
ces valeurs, c'est mesurer littéralement ce que la carte exige, et un écart
nomme l'élément **et** la propriété au lieu de dire « cette page a changé ».

Un harnais par captures d'écran a été écrit d'abord, puis abandonné. Après trois
corrections successives — réseau externe neutralisé, polices attendues,
animations gelées — il variait encore de 5 à 12 % d'un passage à l'autre **sans
qu'aucun CSS n'ait changé**. Un harnais de non-régression qui bouge tout seul
produit un bruit qui masque ce qu'il devrait montrer.

Vérification faite avant de basculer : les 43 pages rendent un DOM **identique**
d'un passage à l'autre, sur 13 280 éléments. L'instabilité était dans la
peinture, jamais dans la structure — donc les styles calculés sont
déterministes ici, et les captures ne pouvaient pas l'être.

# Ce que ce relevé ne voit pas

- Les **états d'interaction** (`:hover`, `:focus`) et ce qui n'apparaît
  qu'après un clic. Les captures d'écran ne les voyaient pas non plus.
- Ce qui n'est pas une valeur calculée d'un élément présent : une
  `background-image` dont seule l'URL change reste vue, mais un écart de
  z-order entre deux calques identiques par ailleurs, non.

Les **pseudo-éléments** `::before` et `::after`, eux, sont relevés
explicitement : ils portent dans ce projet des puces, des chevrons et des
badges, et une règle scopée qui meurt sur un `::after` ne se verrait nulle part
ailleurs.
"""

import base64
import gzip
import time
import urllib.error
import urllib.request
import json
import sys
from pathlib import Path

from playwright.sync_api import sync_playwright

from urls import FEUILLE_ATTENDUE, collecter

LARGEURS = {"desktop": 1440, "mobile": 390}

PIXEL = base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk"
    "+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
)

# Une animation en cours donne une valeur calculée de milieu de course : le
# `transform` d'un spinner y est une matrice de rotation qui dépend de la
# milliseconde. Deux relevés successifs différaient sur exactement ces deux
# valeurs-là — le harnais les a nommées, ce qu'une capture d'écran n'aurait pas
# su faire. On les fige donc à leur état final, stable par définition.
GELER_LES_ANIMATIONS = """
*, *::before, *::after {
  animation-duration: 0s !important;
  animation-delay: 0s !important;
  animation-iteration-count: 1 !important;
  transition-duration: 0s !important;
  transition-delay: 0s !important;
}
"""

PAGE_STABLE = (
    "() => document.fonts.status === 'loaded'"
    " && !document.querySelector('.htmx-request')"
    " && Array.from(document.images).every(i => i.complete)"
)

# La clé d'un élément est son **chemin dans l'arbre** — `0/3/1/2` — et non un
# identifiant tiré de son contenu. Le DOM étant identique d'un passage à
# l'autre, ce chemin est stable ; une clé tirée du texte aurait bougé au moindre
# changement de donnée.
RELEVE = """
() => {
  const sortie = {};
  const lire = (el, chemin, pseudo) => {
    const s = getComputedStyle(el, pseudo);
    const vals = [];
    for (let i = 0; i < s.length; i++) vals.push(s[i] + ':' + s.getPropertyValue(s[i]));
    sortie[chemin + (pseudo || '')] = el.tagName + '§' + (el.className || '') + '§' + vals.join(';');
  };
  // Les éléments qui ne rendent rien sont **ignorés, et ne consomment pas
  // d'indice** : la clé d'un élément est son chemin dans l'arbre, donc retirer
  // un `<link>` décalerait tous ses frères suivants et produirait des milliers
  // d'écarts qui ne sont que du désalignement.
  //
  // Ce n'est pas un détail de confort : la carte 342 supprime 146 `<link>` d'un
  // coup, et sans cette exclusion le harnais serait inutilisable là où il sert
  // le plus.
  const INVISIBLES = new Set(['LINK', 'SCRIPT', 'STYLE', 'META', 'TITLE', 'TEMPLATE']);
  const parcours = (el, chemin) => {
    lire(el, chemin, null);
    lire(el, chemin, '::before');
    lire(el, chemin, '::after');
    let i = 0;
    for (const enfant of el.children) {
      if (INVISIBLES.has(enfant.tagName)) continue;
      parcours(enfant, chemin + '/' + (i++));
    }
  };
  parcours(document.body, '0');
  return sortie;
}
"""


def _neutraliser_le_reseau_externe(page):
    def router(route):
        url = route.request.url
        if "localhost" in url or "127.0.0.1" in url:
            return route.continue_()
        if route.request.resource_type == "image":
            return route.fulfill(status=200, content_type="image/png", body=PIXEL)
        return route.abort()

    page.route("**/*", router)


def attendre_le_serveur(base: str, limite: int = 180) -> None:
    """Attend que le serveur réponde avant de relever.

    `make dev` tourne sous `cargo watch -w src -w assets/static/css` : **chaque
    modification de CSS reconstruit le binaire et redémarre le serveur**. Un
    relevé lancé dans cette fenêtre échoue en bloc, ou pire, à moitié — ce qui
    s'est produit trois fois avant qu'on en trouve la cause, et donnait
    l'apparence d'un harnais instable.
    """
    debut = time.monotonic()
    while time.monotonic() - debut < limite:
        try:
            urllib.request.urlopen(base, timeout=3)
            return
        except (urllib.error.URLError, TimeoutError, OSError):
            time.sleep(2)
    raise SystemExit(
        f"serveur injoignable sur {base} après {limite}s — reconstruction en cours ?"
    )


def main(libelle: str) -> int:
    from urls import BASE

    attendre_le_serveur(BASE)
    pages, manquantes = collecter()
    sortie = Path(__file__).parent / "releves"
    sortie.mkdir(parents=True, exist_ok=True)

    tout: dict[str, dict[str, str]] = {}
    couverture: dict[str, list[str]] = {}
    echecs: list[str] = []

    with sync_playwright() as p:
        navigateur = p.chromium.launch()
        for nom, url in pages.items():
            for etiquette, largeur in LARGEURS.items():
                # Deux tentatives. Sur 86 vues enchaînées, une poignée échouait
                # au hasard — la page suivante passait, la même vue passait au
                # relevé d'après. Un dépassement de délai isolé sous charge, pas
                # un défaut de la page. Un harnais qui rate au hasard finit
                # ignoré, et c'est un harnais qui ne sert plus à rien.
                for tentative in (1, 2):
                    page = navigateur.new_page(
                        viewport={"width": largeur, "height": 900})
                    _neutraliser_le_reseau_externe(page)
                    try:
                        page.goto(url, wait_until="load", timeout=20000)
                        page.wait_for_timeout(1500)
                        page.wait_for_function(PAGE_STABLE, timeout=10000,
                                               polling=200)
                        page.add_style_tag(content=GELER_LES_ANIMATIONS)
                        page.wait_for_timeout(100)
                        tout[f"{nom}.{etiquette}"] = page.evaluate(RELEVE)
                        if etiquette == "desktop":
                            couverture[nom] = page.eval_on_selector_all(
                                "link[rel=stylesheet]",
                                "els => els.map(e => e.getAttribute('href'))",
                            )
                        break
                    except Exception as exc:  # noqa: BLE001 — on veut la liste
                        if tentative == 2:
                            echecs.append(
                                f"{nom}.{etiquette} — {type(exc).__name__}")
                    finally:
                        page.close()
            print(f"  {nom}")
        navigateur.close()

    trompeuses = [
        f"{nom} — attendait {FEUILLE_ATTENDUE[nom]}"
        for nom, liens in couverture.items()
        if nom in FEUILLE_ATTENDUE
        and not any(FEUILLE_ATTENDUE[nom] in (h or "") for h in liens or [])
    ]

    fichier = sortie / f"{libelle}.json.gz"
    with gzip.open(fichier, "wt", encoding="utf-8") as f:
        json.dump({"styles": tout, "couverture": couverture, "echecs": echecs,
                   "sans_entite": manquantes, "trompeuses": trompeuses}, f)

    elements = sum(len(v) for v in tout.values())
    print(f"\n{len(tout)} vues, {elements} relevés (éléments et pseudo-éléments)")
    print(f"→ {fichier} ({fichier.stat().st_size // 1024} Ko)")
    if manquantes:
        print(f"  · {len(manquantes)} pages sans entité : {', '.join(manquantes)}")
    if echecs:
        print(f"  · {len(echecs)} vues en échec :")
        for e in echecs:
            print(f"       {e}")
    if trompeuses:
        print(f"  · {len(trompeuses)} pages n'ont pas chargé la feuille attendue :")
        for t in trompeuses:
            print(f"       {t}")
    return 1 if (echecs or trompeuses) else 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage : releve.py <libellé>")
    raise SystemExit(main(sys.argv[1]))

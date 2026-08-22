"""Contrôle C — quelles feuilles débordent sur des pages qui ne les chargeaient pas.

    uv run python visual/debordements.py

# Pourquoi ce contrôle existe

La carte 341 a donné une portée à chaque feuille de `pages/` et `widgets/`.
Restaient **globales** : `common.css`, les layouts, `components/`, et deux
feuilles de `pages/` déclarées telles. Leur isolation ne venait pas de leurs
sélecteurs — elle venait du fait qu'on ne les chargeait pas.

La carte 342 réunit tout en un fichier unique. Elle supprime donc exactement
cette isolation-là, et un sélecteur qui ne collisionnait avec rien se met à
s'appliquer partout où il trouve du markup. Mesuré à la première fusion :
13 853 valeurs calculées modifiées sur 32 pages.

Le cas d'école : `pages/app-home.css` style `.home-grid`, qui est déclarée dans
`app-layout.html` — donc présente sur **toutes** les pages, alors que trois
seulement chargeaient la feuille.

# Pourquoi le verrou de la 341 ne pouvait pas le voir

Son contrôle B compare les feuilles **deux à deux** et signale les sélecteurs
*divergents*. Un sélecteur défini dans une seule feuille n'est pas une
collision — et c'est pourtant lui qui déborde.

Ce contrôle-ci pose la bonne question : **ce sélecteur trouve-t-il du markup sur
une page qui ne chargeait pas sa feuille ?**
"""

import gzip
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

from playwright.sync_api import sync_playwright

from urls import collecter

RACINE = Path(__file__).resolve().parents[3] / "assets" / "static" / "css"
# Chargées partout depuis toujours : les embarquer ne change rien.
DEJA_PARTOUT = {"common.css", "layout-app.css"}


def regles_par_feuille() -> dict[str, list[str]]:
    """Les sélecteurs de chaque feuille non scopée, hors pseudo-éléments.

    `querySelector` rejette `::before` ; on ne teste donc que la partie qui
    désigne un élément réel — ce qui suffit, un pseudo-élément n'existant que
    porté par lui.
    """
    script = (RACINE.parents[2] / "scripts" / "check-css-collisions.sh").read_text()
    code = script.split("PYTHON'\n", 1)[1].rsplit("\nPYTHON", 1)[0]
    ns: dict = {}
    exec(code.split("# ── Contrôle A")[0], ns)  # noqa: S102 — on réutilise le parseur du verrou

    sortie = {}
    for f in sorted(RACINE.rglob("*.css")):
        rel = f.relative_to(RACINE).as_posix()
        if rel in DEJA_PARTOUT:
            continue
        sels = [s for s, _ in ns["regles"](f)]
        if not sels:
            continue
        if all(ns["porte_le_scope"](s, "." + f.stem) for s in sels):
            continue  # scopée : elle ne peut pas déborder
        propres = sorted({re.sub(r"::[a-z-]+", "", s).strip() for s in sels})
        sortie[rel] = [s for s in propres if s and not s.startswith("@")]
    return sortie


def feuilles_par_page(releve: str) -> dict[str, set[str]]:
    """Ce que chaque page chargeait **avant** la fusion."""
    chemin = Path(__file__).parent / "releves" / f"{releve}.json.gz"
    with gzip.open(chemin, "rt", encoding="utf-8") as f:
        cov = json.load(f)["couverture"]
    return {
        page: {h.split("/static/css/")[1] for h in (liens or []) if h and "/static/css/" in h}
        for page, liens in cov.items()
    }


def main(releve: str = "ctrl") -> int:
    feuilles = regles_par_feuille()
    avant = feuilles_par_page(releve)
    pages, _ = collecter()

    debordements: dict[str, dict[str, list[str]]] = defaultdict(lambda: defaultdict(list))

    with sync_playwright() as p:
        nav = p.chromium.launch()
        for nom, url in pages.items():
            chargees = avant.get(nom)
            if chargees is None:
                continue
            page = nav.new_page(viewport={"width": 1440, "height": 900})
            try:
                page.goto(url, wait_until="load", timeout=20000)
                page.wait_for_timeout(2500)
                for rel, sels in feuilles.items():
                    if rel in chargees:
                        continue
                    touches = page.evaluate(
                        """sels => sels.filter(s => {
                             try { return !!document.querySelector(s); } catch (e) { return false; }
                           })""",
                        sels,
                    )
                    if touches:
                        debordements[rel][nom] = touches
            finally:
                page.close()
            print(f"  {nom}")
        nav.close()

    print()
    if not debordements:
        print("  ✓ Aucune feuille ne déborde")
        return 0

    total = sum(len(s) for f in debordements.values() for s in f.values())
    print(f"  ✗ {len(debordements)} feuilles débordent, {total} correspondances\n")
    for rel in sorted(debordements, key=lambda r: -sum(len(v) for v in debordements[r].values())):
        pages_touchees = debordements[rel]
        sels = sorted({s for v in pages_touchees.values() for s in v})
        print(f"── {rel} — {len(pages_touchees)} pages")
        print(f"     sélecteurs : {', '.join(sels[:8])}" + (" …" if len(sels) > 8 else ""))
        print(f"     pages      : {', '.join(sorted(pages_touchees)[:6])}"
              + (" …" if len(pages_touchees) > 6 else ""))
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1] if len(sys.argv) > 1 else "ctrl"))

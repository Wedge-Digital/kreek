"""Compare deux relevés de styles calculés.

    uv run python visual/comparer.py avant apres

Un écart nomme la vue, l'élément et la propriété — pas seulement « ça a
changé ». C'est ce qui permet de trancher immédiatement entre une régression et
un effet voulu, sans rouvrir le navigateur.

La carte 341 ne tolère aucun écart : le lot ne touche que la **portée** des
règles, jamais leur **contenu**. Toute différence est donc à expliquer avant
d'aller plus loin.
"""

import gzip
import json
import sys
from pathlib import Path

RACINE = Path(__file__).parent / "releves"
PLAFOND = 25  # écarts détaillés affichés ; le reste est compté


def charger(libelle: str) -> dict:
    fichier = RACINE / f"{libelle}.json.gz"
    if not fichier.exists():
        raise SystemExit(f"relevé introuvable : {fichier}")
    with gzip.open(fichier, "rt", encoding="utf-8") as f:
        return json.load(f)


def proprietes(brut: str) -> dict[str, str]:
    _, _, styles = brut.split("§", 2)
    return dict(p.split(":", 1) for p in styles.split(";") if ":" in p)


def main(a: str, b: str) -> int:
    avant, apres = charger(a)["styles"], charger(b)["styles"]

    vues = sorted(set(avant) & set(apres))
    perdues = sorted(set(avant) - set(apres))
    gagnees = sorted(set(apres) - set(avant))

    ecarts: list[str] = []
    elements_disparus = 0
    for vue in vues:
        av, ap = avant[vue], apres[vue]
        for cle in sorted(set(av) & set(ap)):
            if av[cle] == ap[cle]:
                continue
            pa, pb = proprietes(av[cle]), proprietes(ap[cle])
            balise, classes, _ = av[cle].split("§", 2)
            ou = f"{vue}  {balise.lower()}"
            if classes:
                ou += f".{classes.split()[0]}"
            for prop in sorted(set(pa) | set(pb)):
                if pa.get(prop) != pb.get(prop):
                    ecarts.append(f"{ou}  {prop} : {pa.get(prop)} → {pb.get(prop)}")
        elements_disparus += len(set(av) ^ set(ap))

    total = sum(len(avant[v]) for v in vues)
    print(f"\n{len(vues)} vues comparées, {total} relevés")
    if perdues:
        print(f"  · {len(perdues)} vues absentes de « {b} » : {', '.join(perdues)}")
    if gagnees:
        print(f"  · {len(gagnees)} vues nouvelles dans « {b} » : {', '.join(gagnees)}")
    if elements_disparus:
        print(f"  · {elements_disparus} éléments présents d'un côté seulement — "
              "la structure du DOM a bougé, ce que ce lot ne devait pas faire")

    if ecarts:
        print(f"\n  ✗ {len(ecarts)} valeurs calculées ont changé :")
        for e in ecarts[:PLAFOND]:
            print(f"       {e}")
        if len(ecarts) > PLAFOND:
            print(f"       … et {len(ecarts) - PLAFOND} autres")
        return 1

    if elements_disparus:
        return 1
    print("\n  ✓ Aucune valeur calculée n'a changé")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage : comparer.py <avant> <après>")
    raise SystemExit(main(sys.argv[1], sys.argv[2]))

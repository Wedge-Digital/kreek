#!/usr/bin/env python3
"""Du CSS qui ne rencontrera jamais le markup qu'il vise (carte 487).

Deux contrôles, tous deux **statiques** : ils lisent les gabarits, jamais le DOM.
C'est ce qui les rend utilisables — voir « Ce qui a été écarté » plus bas.

    A · une classe stylée sous une racine, qui vit hors de cette racine
    B · deux attributs `class` sur un même élément

Sortie : une ligne par constat, code 1 s'il y en a.

# Ce qui a été écarté, et pourquoi

Le contrôle évident serait « une règle qui ne trouve aucun markup », mesuré au
navigateur sur les pages du harnais visuel. Il a été prototypé : **2 442
sélecteurs sur 3 588 ne rencontrent rien, soit 68 %**.

Ce chiffre ne dit pas que le dépôt porte 2 442 règles mortes. Il dit que le
harnais ne visite pas assez d'états — listes vides, formulaires en erreur, états
posés par Alpine, modales — pour que « ne trouve rien » veuille dire « est
mort ». Un verrou à ce bruit-là ne serait jamais lu, donc jamais utile.

Les deux contrôles ci-dessous répondent à une question plus étroite et **exacte**,
sans dépendre des données ni des écrans visités.

# Contrôle A

Une feuille scopée `.nom` style `.nom .cible`. Si, dans un gabarit qui porte
`.nom`, tous les éléments portant `.cible` sont **hors** de `.nom`, la règle ne
rencontrera jamais rien.

C'est le défaut du bouton « Gérer les points manuels » : ses six règles étaient
écrites, et le bloc vivait sous le `</div>` de fermeture du widget. Le bouton
était un lien nu, mesuré à `padding: 0`, sans bordure ni fond — dans les deux
widgets de classement, la duplication ayant dupliqué l'erreur.

**Même gabarit seulement.** Une classe définie dans un fragment et enveloppée par
l'hôte est parfaitement légitime ; le contrôle ne dit donc rien des compositions
inter-gabarits. C'est étroit, et c'est le prix de zéro faux positif.

# Contrôle B

Le navigateur ne retient que le **premier** attribut `class` d'un élément ; un
second est perdu, avec toutes les règles qui le visent. Un cas dans le dépôt.

# Deux pièges rencontrés en l'écrivant

**Les commentaires Askama citent du markup.** « Le bloc vivait sous le `</div>` »
suffit à ce qu'un parseur HTML y voie une vraie fermeture, dépile la racine, et
déclare hors scope tout ce qui suit : 36 faux positifs mesurés. Ils sont retirés
avant l'analyse.

**Askama ne rend qu'une branche.** Deux `class` séparés par un `{% when %}` sont
exclusifs, pas doublés. Sans ce filtre, le contrôle B accuse le relevé de
trésorerie, qui est correct.
"""

import re
import sys
from html.parser import HTMLParser
from pathlib import Path

RACINE = Path(__file__).resolve().parents[2]
CSS = RACINE / "assets" / "static" / "css"
GABARITS = RACINE / "src"

# Le parseur de sélecteurs et le test de portée vivent dans le verrou CSS ; les
# recopier ici les ferait diverger. `debordements.py` emploie déjà ce montage.
_ns = {}
# Un seul espace de noms — globals et locals confondus : séparés, les fonctions
# définies par le `exec` capturent les globals et n'y trouvent pas leurs propres
# imports (`NameError: name 're' is not defined`).
exec(  # noqa: S102
    (RACINE / "scripts" / "check-css-collisions.sh")
    .read_text()
    .split("PYTHON'\n", 1)[1]
    .rsplit("\nPYTHON", 1)[0]
    .split("# ── Contrôle A")[0],
    _ns,
)

DOUBLONS = []


class Arbre(HTMLParser):
    """Pour chaque classe : les jeux de classes de ses ancêtres, à chaque occurrence."""

    def __init__(self, fichier: str):
        self.fichier = fichier
        super().__init__(convert_charrefs=True)
        self.pile = []
        self.vus = {}
        self.toutes = set()

    def handle_starttag(self, tag, attrs):
        cls, vus_class = set(), 0
        for k, v in attrs:
            if k != "class":
                continue
            vus_class += 1
            if vus_class == 1 and v:
                cls = {c for c in re.split(r"\s+", v) if c and "{" not in c}
        if vus_class > 1:
            brut = self.get_starttag_text() or ""
            if "{%" not in brut[brut.find("class") : brut.rfind("class")]:
                DOUBLONS.append((self.fichier, tag, [v for k, v in attrs if k == "class"]))
        ancetres = set().union(*self.pile) if self.pile else set()
        for c in cls:
            self.vus.setdefault(c, []).append(ancetres | cls)
            self.toutes.add(c)
        if tag not in ("br", "img", "input", "hr", "meta", "link", "source"):
            self.pile.append(cls)

    def handle_endtag(self, tag):
        if self.pile:
            self.pile.pop()


def lire_les_gabarits():
    arbres = {}
    for f in GABARITS.rglob("*.html"):
        a = Arbre(f.relative_to(RACINE).as_posix())
        try:
            a.feed(re.sub(r"\{#.*?#\}", "", f.read_text(), flags=re.S))
        except Exception:  # noqa: BLE001 — un gabarit illisible n'est pas notre sujet
            continue
        arbres[f] = a
    return arbres


def classes_stylees(feuille):
    """Les classes qu'une feuille **scopée** style sous sa racine, ou None.

    Sans annotation de retour : `set[str] | None` demande Python 3.10, et le
    `python3` du système est en 3.9 — c'est le piège de la carte 480, où un
    verrou entier s'est tu pour la même raison.
    """
    sels = [s for s, _ in _ns["regles"](feuille)]
    scope = feuille.stem
    if not sels or not all(_ns["porte_le_scope"](s, "." + scope) for s in sels):
        return None
    return {
        c
        for s in sels
        for c in re.findall(r"\.([A-Za-z][A-Za-z0-9_-]*)", s)
        if c != scope
    }


def hors_de_leur_racine(arbres):
    constats = []
    for feuille in sorted(CSS.rglob("*.css")):
        cibles = classes_stylees(feuille)
        if not cibles:
            continue
        scope = feuille.stem
        for chemin, a in arbres.items():
            if scope not in a.toutes:
                continue
            for cls in sorted(cibles & a.toutes):
                occurrences = a.vus[cls]
                if all(scope not in o for o in occurrences):
                    constats.append(
                        f".{cls} — stylée par {feuille.relative_to(CSS).as_posix()} "
                        f"sous .{scope}, mais vit hors de .{scope} dans "
                        f"{chemin.relative_to(RACINE).as_posix()}"
                    )
    return constats


def main():
    arbres = lire_les_gabarits()
    if not arbres:
        print("aucun gabarit lu — l'analyse a échoué", file=sys.stderr)
        return 1
    constats = hors_de_leur_racine(arbres)
    constats += [
        f"deux attributs class sur <{tag}> dans {f} — le second est perdu : {vals}"
        for f, tag, vals in DOUBLONS
    ]
    for c in constats:
        print(c)
    return 1 if constats else 0


if __name__ == "__main__":
    sys.exit(main())

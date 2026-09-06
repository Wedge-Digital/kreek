#!/usr/bin/env python3
"""Axe 18 — les domain events que personne n'émet.

Un variant d'événement peut être déclaré, porter sa constante de type, avoir
son bras dans le publisher — et n'être **construit nulle part**. La chaîne a
alors l'air complète et ne transporte rien.

Le compilateur ne peut pas le dire : les variants d'un enum public échappent à
`dead_code`. C'est pourquoi ce contrôle existe, là où la forme voisine — un
variant émis sans bras dans `to_app_event` — est verrouillée par le
compilateur depuis que les jokers `_ => None` ont disparu (carte 506).

**Le piège à éviter, et la raison de la moitié de ce fichier** : chercher
`Enum::Variant` dans le dépôt ne distingue pas une émission d'une réception.
Un listener qui filtre sur un événement l'écrit exactement comme celui qui le
produit. Compter les occurrences aurait donc déclaré « émis » tout événement
seulement consommé — c'est-à-dire précisément un fantôme de plus.

La distinction est syntaxique et sûre : un motif de `match` est suivi de `=>`
une fois son bloc refermé, une construction ne l'est pas.

Exception : `// arch:pas-emis <motif>` sur la déclaration du variant ou juste
au-dessus. Le motif est obligatoire. Le cas légitime qu'il couvre est celui
d'un événement dont le code émetteur a été retiré : il doit rester dans l'enum
pour rejouer l'historique, sans être construit nulle part.
"""

import re
import sys
from pathlib import Path

RACINE = Path(__file__).resolve().parents[2] / "src"


def fichiers_rust():
    for p in RACINE.rglob("*.rs"):
        s = str(p)
        if "/tests/" in s or p.name.startswith("test_"):
            continue
        yield p


def enums_d_evenements():
    """Les enums `*DomainEvent`, avec leurs variants et leur marqueur éventuel."""
    for chemin in fichiers_rust():
        texte = chemin.read_text(encoding="utf-8")
        for m in re.finditer(r"pub enum (\w*DomainEvent) \{", texte):
            nom = m.group(1)
            variants = []
            # Le marqueur peut tenir sur plusieurs lignes : on garde le bloc de
            # commentaire contigu qui précède, pas la seule ligne d'avant.
            # Ne lire que celle-ci avait laissé passer trois des six déclarés.
            commentaire = []
            for ligne in texte[m.end():].splitlines():
                if ligne.startswith("}"):
                    break
                mv = re.match(r"    ([A-Z]\w*)\s*[,{(]", ligne)
                if mv:
                    bloc = "\n".join(commentaire)
                    marque = "arch:pas-emis" in ligne or "arch:pas-emis" in bloc
                    variants.append((mv.group(1), marque))
                    commentaire = []
                elif ligne.strip().startswith(("//", "///")):
                    commentaire.append(ligne)
                elif not ligne.strip():
                    pass
                else:
                    commentaire = []
            yield chemin, nom, variants


def fin_du_bloc(texte, i):
    """Index juste après le motif ouvert en `i` (`{`, `(` ou rien)."""
    if i >= len(texte) or texte[i] not in "{(":
        return i
    ouvrant, fermant = ("{", "}") if texte[i] == "{" else ("(", ")")
    profondeur = 0
    for j in range(i, len(texte)):
        if texte[j] == ouvrant:
            profondeur += 1
        elif texte[j] == fermant:
            profondeur -= 1
            if profondeur == 0:
                return j + 1
    return len(texte)


def est_construit(variant, enum, definition):
    """Le variant est-il construit ailleurs que dans son fichier de définition ?

    Une occurrence suivie de `=>` est un motif de `match` — on la passe.
    """
    motif = re.compile(r"(?:%s|Self)::%s\b" % (enum, variant))
    for chemin in fichiers_rust():
        if chemin == definition:
            continue
        texte = chemin.read_text(encoding="utf-8")
        # Les modules de test inline ne comptent pas : un variant construit
        # seulement en test est un fantôme en production.
        coupe = texte.find("#[cfg(test)]")
        if coupe != -1:
            texte = texte[:coupe]
        for m in motif.finditer(texte):
            suite = texte[fin_du_bloc(texte, m.end()):].lstrip()
            if not suite.startswith("=>"):
                return True
    return False


def main():
    fantomes = []
    for definition, enum, variants in enums_d_evenements():
        for variant, marque in variants:
            if marque:
                continue
            if not est_construit(variant, enum, definition):
                rel = definition.relative_to(RACINE.parent)
                fantomes.append(f"{rel} : {enum}::{variant} n'est construit nulle part")

    if fantomes:
        print("\n".join(sorted(fantomes)))
        print("")
        print("Un événement déclaré et jamais émis a l'air d'une chaîne complète.")
        print("Le câbler, ou le déclarer par `// arch:pas-emis <motif>`.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Préfixe tous les sélecteurs d'une feuille par sa classe de portée.

    python3 scripts/scoper-css.py assets/static/css/pages/team-page.css .team-page

La convention de l'épic E03 : le **nom du fichier est le sélecteur de portée**.
`pages/team-page.css` ⇒ toute règle sous `.team-page`. Aucune table de
correspondance à tenir, et `scripts/check-css-collisions.sh` vérifie la règle en
comparant un nom de fichier à un préfixe de sélecteur.

# Ce que l'outil ne touche pas

- Les **étapes de `@keyframes`** (`from`, `to`, `50%`) : ce sont des positions
  dans une animation, pas des sélecteurs.
- Un sélecteur **déjà sous la portée** — le scoping est idempotent, on peut
  repasser sur un fichier sans le doubler.
- Les sélecteurs qui **ne peuvent pas être scopés** (`html`, `body`, `:root`,
  `*`) : ils sont **signalés et laissés en place**, jamais réécrits. Les
  scoper silencieusement changerait le rendu, ce que la carte 341 interdit.

# Ce qu'il ne décide pas

La classe de portée. Elle se lit dans le template — et un template qui n'en a
pas doit en recevoir une, ce qui est une décision, pas une transformation.
"""

import re
import sys
from pathlib import Path

INSCOPABLES = {"html", "body", ":root", "*", "html body"}

COMBINATEURS = re.compile(r"[\s>+~]")


def forme_composee(selecteur: str, portee: str) -> str:
    """Rend la variante où la portée est **collée** au premier compound.

    `.mr-container` ⇒ `.scope.mr-container`, et non `.scope .mr-container`.

    Sans elle, une feuille qui style son propre élément racine cesse de
    s'appliquer dès que la portée est une classe **ajoutée à cet élément** :
    `<div class="mr-container match-report-inducements">` n'est pas *à
    l'intérieur* de `.match-report-inducements`, il l'est. Le sélecteur
    descendant l'exclut, et la feuille globale reprend la main — mesuré, un
    `padding-bottom` de 120 px retombé à 24 px.

    Le cas n'apparaît pas tant que la portée **est** la classe racine ; il
    surgit dès qu'une racine porte deux classes dont une seule est la portée.
    """
    m = COMBINATEURS.search(selecteur)
    premier, reste = (selecteur[:m.start()], selecteur[m.start():]) if m else (selecteur, "")
    nom = re.match(r"^[a-zA-Z][a-zA-Z0-9-]*", premier)
    coupe = nom.end() if nom else 0
    return premier[:coupe] + portee + premier[coupe:] + reste


def masquer_les_commentaires(texte: str) -> str:
    """Rend le texte avec le **contenu** des commentaires remplacé par des
    espaces, longueur préservée.

    Sans ce masquage, un `;`, un `{` ou un `}` écrit dans un commentaire est lu
    comme de la syntaxe. Vécu : un commentaire contenant « défile dans son
    conteneur ; » faisait redémarrer le prélude au milieu de la phrase, et la
    portée s'insérait dans le texte du commentaire — produisant un fichier qui
    se parse encore, dont une règle a disparu, et que rien ne signale.

    Les positions restant identiques, on repère sur le texte masqué et on
    découpe dans l'original.
    """
    return re.sub(r"/\*.*?\*/",
                  lambda m: " " * len(m.group(0)),
                  texte, flags=re.S)


def decouper(texte: str):
    """Rend la liste des (début, fin) des préludes de règle, hors at-rules."""
    texte = masquer_les_commentaires(texte)
    positions = []
    pile = []
    debut = 0
    i = 0
    while i < len(texte):
        c = texte[i]
        if c == "{":
            tete = texte[debut:i]
            # Les commentaires sont retirés **avant** de reconnaître une
            # at-rule : un `/* … */` posé devant un `@media` faisait commencer
            # le prélude par `/`, l'at-rule passait pour un sélecteur, et la
            # portée s'insérait devant elle — `.team-page @media (…)`, qui ne
            # s'applique à rien et fait disparaître tout le bloc mobile.
            nu = tete.strip()
            if nu.startswith("@"):
                pile.append(nu.split()[0].lower())
            else:
                pile.append(None)
                if not any(p == "@keyframes" for p in pile if p):
                    positions.append((debut, i))
            debut = i + 1
        elif c == "}":
            if pile:
                pile.pop()
            debut = i + 1
        elif c == ";":
            debut = i + 1
        i += 1
    return positions


def scoper(texte: str, portee: str) -> tuple[str, list[str]]:
    ignores: list[str] = []
    sortie = []
    precedent = 0
    for a, b in decouper(texte):
        sortie.append(texte[precedent:a])
        tete = texte[a:b]
        # Un commentaire qui précède un sélecteur fait partie du prélude. Sans
        # cette extraction, la portée s'insérait *avant* lui et produisait
        # `.team-page /* En-tête */ .team-header`, qui est un sélecteur
        # descendant valide et faux.
        avant = ""
        reste = tete
        while True:
            m = re.match(r"^(\s*(?:/\*.*?\*/)?\s*)", reste, re.S)
            if not m or not m.group(1):
                break
            avant += m.group(1)
            reste = reste[m.end():]
            if "/*" not in m.group(1):
                break
        m = re.match(r"^(.*?)(\s*)$", reste, re.S)
        corps, apres = m.group(1), m.group(2)
        nouveaux = []
        for sel in corps.split(","):
            nu = " ".join(sel.split())
            if not nu:
                continue
            if nu in INSCOPABLES:
                ignores.append(nu)
                nouveaux.append(nu)
            elif nu == portee or nu.startswith(portee + " ") or nu.startswith(portee + ":") \
                    or nu.startswith(portee + ".") or nu.startswith(portee + ">"):
                nouveaux.append(nu)
            else:
                # Deux formes : descendante pour ce qui est sous la racine,
                # composée pour la racine elle-même.
                nouveaux.append(f"{portee} {nu}")
                nouveaux.append(forme_composee(nu, portee))
        sortie.append(avant + ",\n".join(nouveaux) + apres)
        precedent = b
    sortie.append(texte[precedent:])
    return "".join(sortie), ignores


def main() -> int:
    if len(sys.argv) != 3:
        raise SystemExit("usage : scoper-css.py <fichier.css> <.portee>")
    fichier, portee = Path(sys.argv[1]), sys.argv[2]
    texte = fichier.read_text()
    nouveau, ignores = scoper(texte, portee)
    # Garde-fou : une at-rule préfixée ne s'applique à rien, et le bloc entier
    # disparaît sans erreur. C'est arrivé une fois, sur un `@media` précédé
    # d'un commentaire.
    # `[^\S\n]` et non `\s` : ce dernier englobe le saut de ligne, et un
    # `@media` légitime en début de ligne déclenchait le garde-fou. Un garde-fou
    # qui crie au loup finit désarmé.
    fautives = re.findall(
        r"^[^\n{]*\S[^\S\n]+@(?:media|supports|container|layer|keyframes)\b",
        nouveau, re.M)
    if fautives:
        raise SystemExit(
            f"REFUS d'écrire {fichier} : {len(fautives)} at-rules ont été "
            f"préfixées — {fautives[0].strip()}"
        )
    fichier.write_text(nouveau)
    print(f"  {fichier.name} scopé sous {portee}")
    if ignores:
        print(f"    · {len(ignores)} sélecteurs non scopables laissés en place : "
              + ", ".join(sorted(set(ignores))))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

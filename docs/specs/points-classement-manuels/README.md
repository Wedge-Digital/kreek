# Points de classement manuels

**Maquettes :** `assets/rawpages/html/app-manual-ranking-points.html` (saisie)
et `app-competition-detail.html` (le classement qui les intègre)

## La fonction

Attribuer à une équipe des points de classement qui ne viennent d'aucun match —
un forfait subi, une sanction, un rattrapage — et rendre chacun de ces
ajustements **visible et motivé**.

Aujourd'hui le classement ne tient que des matchs joués. Une décision d'arbitre
n'a nulle part où s'écrire, et se traduit par un score de match falsifié ou par
rien du tout.

## Les pages

| Page | État |
|---|---|
| `page-de-gestion/` | phases 1 et 2 faites |

## Règles tranchées en phase 1

1. **Un point manuel survit au recalcul du classement.** C'est une troisième
   composante du total, à côté des points de match et des bonus.
2. **Il entre dans le total *avant* les départages.** 3 points + 2 manuels
   égalent 5 points sans manuel : les deux équipes sont à égalité, et ce sont
   les départages qui tranchent.
3. **Il est public**, consultable par tous, et le classement l'affiche.
4. **Le total d'une équipe peut devenir négatif.**
5. **Une ligne se supprime, elle ne se modifie pas.**
6. **Le motif est facultatif, mais recommandé.**

# Le menu du sélecteur était rogné par son panneau

**Priorité : moyenne** — le commissaire ne voyait qu'une tranche de la liste
**Dépend de :** rien · **Sans épic**
**Signalée par :** l'utilisateur

## Le défaut

Sur « Points de classement manuels », le menu déroulant du `kreek-select` était
coupé net par le panneau qui l'entoure. Choisir une équipe demandait de deviner
ce qui restait sous la coupure.

## La cause

```css
.ranking-manual-points .mp-panel { …; overflow: hidden; }
```

`overflow: hidden` sert aux coins arrondis : sans lui, le fond gris de l'en-tête
déborde de la courbe. Mais un `overflow` coupe aussi les descendants en
`position: absolute` — et le menu du composant en est un, posé sous le champ
avec un `z-index: 100` qui ne lui sert alors à rien.

**Un `z-index` ne franchit pas un `overflow`.** C'est le piège : le composant
avait tout ce qu'il fallait pour passer au-dessus, et se faisait rogner par un
ancêtre à trois niveaux de là.

## La correction

`overflow: visible` sur le seul panneau de formulaire, et les coins découpés là
où ils se voient — sur l'en-tête lui-même, par `border-radius: 9px 9px 0 0`.

Neuf et non dix : le panneau porte une bordure d'un pixel, et l'en-tête s'arrondit
à l'intérieur.

Le panneau de la liste garde son `overflow: hidden` : il ne contient aucun menu,
et son tableau comme sa bordure supérieure de trois pixels en dépendent.

## Ce que la mesure a appris

**`getBoundingClientRect` ne voit pas ce défaut.** Le rectangle du menu est
exactement le même, coupé ou non — 166 px de haut, dépassant de 92 px sous le
panneau, dans les deux cas. Une première version du test comparait ces
coordonnées et passait avec le défaut en place.

Seul `elementFromPoint` dit ce qui est **réellement peint** à un endroit :

```
overflow: hidden   → au bas du menu, on touche « page-content »   → COUPÉ
overflow: visible  → au bas du menu, on touche « ks-option »      → VISIBLE
```

Le test tâte donc un point du menu situé sous la limite du panneau.

## Un défaut de test révélé au passage

`test_pending_enrollment_banner_is_informational` cherchait une équipe
`PendingEnrollment` **dans toute la base**, puis l'ouvrait sous l'espace e2e.
Tant que la base ne contenait que cet espace, les deux coïncidaient. Une base
chargée depuis la production — ce que `make import_prod_db` permet depuis
aujourd'hui — rend une équipe d'un autre espace : la page ne la trouve pas, et le
test échoue en accusant le bandeau.

La requête filtre désormais par espace. Les autres requêtes de la suite ciblent
toutes un identifiant précis ; ce test était le seul à piocher au hasard.

C'est un bénéfice inattendu de l'import : **une base réaliste trouve les tests
qui supposaient une base vide.**

## Ce que la carte ne fait pas

**Elle ne touche pas au composant.** `kreek-select` pose correctement son menu ;
c'est son environnement qui le rognait. D'autres pages peuvent porter le même
piège — un `overflow: hidden` sur un ancêtre d'un `kreek-select` — et rien ne les
signale aujourd'hui. Ça mériterait un contrôle, pas une chasse manuelle.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `test_le_menu_du_selecteur_deborde_du_panneau` | le bas du menu est peint, pas rogné |
| `test_pending_enrollment_banner_is_informational` | corrigé — il vise l'espace du test |

Falsifié : `overflow: hidden` rétabli, le test rougit ; le point tâté rend
`page-content` au lieu d'une option.

## Checklist

- [x] `overflow: visible` sur le panneau de formulaire
- [x] Coins arrondis portés par l'en-tête
- [x] Le panneau de liste garde son `overflow: hidden`
- [x] Le test e2e, falsifié
- [x] La requête du bandeau filtrée par espace
- [x] `make check-arch` (17 axes), `check-css-collisions`, `make e2e` (355, 0 échec)

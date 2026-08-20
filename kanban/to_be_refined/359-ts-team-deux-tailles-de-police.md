# `components` — `.ts-team-name` et `.ts-team-meta` ont deux tailles de police

**Priorité : basse** — écart invisible à l'œil nu, mais il bloque le dernier
verrou du CSS
**Trouvée par :** le lot 3 de la carte 341
**État : à raffiner** — c'est un arbitrage visuel, pas une opération de portée
**Fichiers :** `assets/static/css/components/team-selection.css`,
`assets/static/css/pages/match-report-shared.css`

## Le problème

Deux feuilles définissent les mêmes classes avec des valeurs différentes :

| Sélecteur | `components/team-selection.css` | `pages/match-report-shared.css` |
|---|---|---|
| `.ts-team-name` | `font-size: var(--text-small)` | `font-size: var(--text-tiny)` |
| `.ts-team-meta` | `font-size: var(--text-tiny)` | `font-size: 11px` |

Elles sont **co-chargées sur cinq pages** de rapport de match, donc la cascade
tranche aujourd'hui par l'ordre de chargement. Ce sont les deux derniers
sélecteurs divergents du dépôt : tous les autres ont été soldés par la portée.

## Pourquoi la carte 341 ne peut pas le faire

Elle pose une contrainte qu'aucune des deux issues ne respecte :

> On ne modifie que la **portée** des règles, jamais leur **contenu**.

**Scoper le composant est impossible.** `schedule-round-detail.html` emploie
`.ts-team-name` et `.ts-team-meta` **sans porter `.team-selection`** : le
composant déborde de sa racine, et lui donner une portée ferait mourir ses
règles sur cette page.

**Choisir un gagnant est un changement de rendu.** C'est exactement ce que la
carte 341 renvoie à une autre carte : *« harmoniser un `letter-spacing` à 0,4 px
et 0,5 px … c'est une autre carte, et elle s'assume comme un changement
visuel »*.

## Les questions à trancher

1. **Quelle valeur garde-t-on ?** `--text-small` ou `--text-tiny` pour le nom,
   `--text-tiny` ou `11px` pour la ligne secondaire. Il faut regarder les deux
   rendus, pas seulement les valeurs.
2. **Le `11px` en dur est-il voulu ?** Le projet a des tokens de typographie
   (`--text-tiny`, `--text-small`) ; une valeur en dur au milieu est soit un
   oubli, soit un ajustement délibéré qui mérite son commentaire.
3. **Faut-il traiter le débordement du composant ?** `team-selection.css` style
   des éléments hors de sa racine. Tant que c'est vrai, il ne peut pas recevoir
   de portée, et le dépôt garde un composant qui échappe à la règle générale.
   C'est peut-être une carte à part.

## Ce que ça débloque

Le contrôle B de `scripts/check-css-collisions.sh` — « aucun sélecteur divergent
entre feuilles » — passe de 2 à 0. Le verrou peut alors être branché sur
`make lint` et la CI, ce que l'épic E03 prévoit à sa fin.

## Checklist — à compléter au raffinage

- [ ] La valeur retenue pour chacune des deux classes est décidée, en regardant
      le rendu
- [ ] Le sort du `11px` en dur est tranché
- [ ] Le débordement de `team-selection.css` hors de sa racine est traité, ou
      renvoyé à sa propre carte avec son motif
- [ ] `check-css-collisions.sh` passe ses deux contrôles
- [ ] Vérifié au harnais visuel : les seuls écarts sont ceux qu'on a décidés

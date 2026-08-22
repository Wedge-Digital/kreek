# `components` — `.ts-team-name` et `.ts-team-meta` ont deux tailles de police

**Priorité : basse** — écart invisible à l'œil nu, mais il bloque le dernier
verrou du CSS
**Trouvée par :** le lot 3 de la carte 341
**État : faite** — l'arbitrage visuel n'a pas eu lieu : il avait déjà été tranché
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

## Checklist

- [x] La valeur retenue pour chacune des deux classes est décidée, en regardant
      le rendu
- [x] Le sort du `11px` en dur est tranché — **gardé**
- [x] Le débordement de `team-selection.css` hors de sa racine est traité — il
      cesse d'être un problème, cf. ci-dessous
- [x] `check-css-collisions.sh` : contrôle B de 6 à 4 sélecteurs, les quatre
      restants venant de la feuille amont de Tom Select (carte 363)
- [x] Vérifié au harnais visuel : **0 écart sur 86 vues**

## Ce qui a été fait

**La prémisse de la carte n'était plus vraie.** Elle posait qu'un choix de
valeur serait un changement de rendu. Ce n'était vrai qu'avant la carte 342 :
`match-report-shared.css` est déclarée `css:global`, elle est donc dans le
bundle servi à **toutes** les pages, et elle y vient après
`components/team-selection.css` à spécificité égale. Elle gagnait déjà partout.

Mesuré avant toute modification, sur la page de sélection de match :

```
.ts-team-name → 12px        .ts-team-meta → 11px
```

L'arbitrage avait donc déjà eu lieu, silencieusement, à la fusion des feuilles.
Il ne restait qu'à faire dire à la source ce que l'écran montrait : les deux
règles sont consolidées dans `components/team-selection.css`, aux valeurs de
`match-report-shared`, et supprimées de cette dernière.

**`.ts-team-option` a suivi**, bien qu'il ne fût en collision avec rien : les
trois règles forment un bloc, et laisser un tiers du rendu TomSelect dans une
feuille de rapport de match reproduirait la scission qui a créé cette carte.

**Le `11px` est gardé**, avec son motif écrit sur place : au token
`--text-tiny` (12px), la ligne secondaire prendrait la taille du nom et la
hiérarchie ne tiendrait plus que par la graisse et la couleur.

**Le débordement du composant ne demande pas de carte.**
`schedule-round-detail.html` emploie ces classes sans porter `.team-selection`,
ce qui interdisait de scoper le composant — mais `components/` est **global par
conception**, et le contrôle A ne vérifie la portée que de `pages/` et
`widgets/`. Un composant global n'a pas à être scopé ; la question tombe.

**Preuve du rendu inchangé** : relevé complet avant et après, 86 vues, 47 040
éléments et pseudo-éléments — **0 différence**. Ce n'est pas un raisonnement sur
la cascade, c'est une mesure.

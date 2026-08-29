# Deux tokens de gris indiscernables

**Priorité : moyenne** — le défaut ne casse rien, il rend un état invisible
**Périmètre : les tokens de `common.css` et les feuilles qui les opposent**
**Dépend de :** rien
**Trouvée par :** un passage `/impeccable colorize` sur la maquette des points
de classement manuels, le 2026-08-27

## Le constat

```css
--dark-6: #F0F2F4;   /* (240, 242, 244) */
--dark-7: #EFF2F5;   /* (239, 242, 245) */
```

**Un écart de ±1 par canal.** Rapport de contraste entre les deux :
**1,0012**. À titre de comparaison, blanc contre `--dark-7` vaut 1,124 — cent
fois plus, et c'est déjà un pas discret.

Ces deux tokens ne sont pas deux nuances : c'est la même couleur écrite deux
fois.

## Ce que ça produit

Partout où une feuille les **oppose** pour distinguer deux états, l'un des deux
est invisible.

Le cas le plus visible est le classement, sous les yeux de tous les coachs :

```css
/* widgets/ranking-detailed-standings-widget.css */
:40  .sd-table tbody tr:nth-child(even) { background: var(--dark-7); }
:44  .sd-table tbody tr:hover           { background: var(--dark-6); }
```

**Le survol du classement détaillé ne se voit pas une ligne sur deux.** Sur les
lignes impaires il fonctionne — blanc vers `--dark-6` — et sur les paires il ne
change rien.

Même forme dans `pages/competition-detail.css` :

```css
:73  .standings-row:nth-child(even) { background: var(--dark-7); }
:75  .standings-row:hover           { background: var(--dark-6); }
```

## L'étendue

**Vingt et une feuilles emploient les deux**, dont celles-ci. Toutes ne les
opposent pas dans la même zone — beaucoup s'en servent à des endroits sans
rapport — mais chacune est un endroit où la confusion peut naître au prochain
état ajouté.

```
--dark-6 : 136 usages
--dark-7 :  35 usages
```

Le déséquilibre est lui-même une indication : `--dark-7` a l'air d'avoir été
ajouté sans qu'on sache ce qu'il apportait de plus.

## Ce que la carte fait

**Deux voies, et il faut trancher.**

### A — Fusionner

`--dark-7` disparaît, ses 35 usages deviennent `--dark-6`. Un token de moins,
plus aucune confusion possible, et les endroits qui les opposaient **cessent
d'avoir un état** — ce qui les rend visibles comme défauts à traiter un par un.

C'est la voie honnête : elle ne cache pas le problème, elle le fait remonter.

### B — Écarter

`--dark-7` prend une valeur qui se distingue vraiment, autour de `#E9ECF0`, et
devient officiellement **le fond alterné**, tandis que `--dark-6` reste
**l'état de survol**. Deux rôles nommés, une échelle qui fonctionne.

Elle coûte une relecture des 35 usages de `--dark-7` pour vérifier qu'aucun ne
servait de survol.

**Recommandation : B.** Le zébrage d'un tableau et le survol d'une ligne sont
deux besoins réels ; les fondre en une seule valeur obligerait à réinventer le
zébrage ailleurs. Mais A est défendable si l'on juge que le zébrage n'a pas sa
place.

## Ce que la carte ne fait pas

- **Elle ne redéfinit pas la palette.** Les autres tokens ne sont pas en cause :
  `--dark-5` (#E5E6EF) et `--dark-6` sont séparés de 1,10, ce qui se voit.
- **Elle ne corrige pas les feuilles qui n'opposent pas les deux.** Un
  `--dark-7` employé seul comme fond n'a aucun problème.

## Le verrou

Sans lui, deux tokens rapprochés se reproduiront.

```bash
# scripts/check-css-collisions.sh — un axe de plus
# Deux tokens dont le rapport de contraste est inférieur à 1,05 ne peuvent pas
# servir à distinguer deux états.
```

Un contrôle sur les **valeurs** des tokens, calculé une fois au démarrage du
script, et non sur leurs usages : il attrape la cause plutôt que ses effets.

## Tests

- **Visuel** : le survol d'une ligne paire du classement détaillé se voit.
- **Automatique** : le contrôle de proximité des tokens échoue si l'on
  rapproche deux valeurs à moins de 1,05.

## Checklist

- [x] Trancher A ou B — **A, plus l'idiome que le projet employait déjà**
- [x] Appliquer, en relisant les usages de `--dark-7`
- [x] Vérifier les feuilles qui portent les deux
- [x] Le contrôle de proximité dans `check-css-collisions.sh`
- [x] `make lint && make check-arch`

## Pourquoi B était impossible

Sa valeur recommandée, `#E9ECF0`, **échoue au verrou que la carte demande dans
la même page** : 1,0488 contre `--dark-5`. Et ce n'est pas un mauvais choix
isolé, c'est structurel — `--dark-5` et `--dark-6` ne sont séparés que de
1,1074, or loger une valeur entre eux à ≥1,05 des deux exige 1,1025. Il n'y a
pas la place.

**Et la carte ignorait que `--dark-7` est le fond de l'application** —
`common.css:90` (`body`) et `layout-app.css:6`. L'assombrir vers `#E9ECF0`
aurait changé le fond de tous les écrans, ce que la carte ne mentionne nulle
part.

## La voie retenue

`--dark-7` disparaît ; ses usages deviennent `--dark-6` (25 feuilles réécrites).
Le fond de page passe de `#EFF2F5` à `#F0F2F4` — 1,0012, invisible à l'œil,
c'est tout l'objet de la carte.

Les quatre feuilles qui opposaient les deux passent au couple **zébrage
`--dark-6`, survol `--dark-5`** (1,1074) : l'idiome que `team-page.css:172-173`
et `players-widget.css:14-15` employaient **déjà**. Aucune couleur inventée.

## Le défaut touchait quatre feuilles, pas deux

La carte citait le classement détaillé et `competition-detail`. Le recensement
en a trouvé deux de plus :

| Feuille | zébrage / survol |
|---|---|
| `widgets/ranking-detailed-standings-widget.css:40/44` | `--dark-7` / `--dark-6` |
| `widgets/ranking-classement-widget.css:34/36` | `--dark-7` / `--dark-6` |
| `pages/competition-detail.css:73/75` | `--dark-7` / `--dark-6` |
| `pages/match-report-shared.css:366/367` | `--dark-7` / `--dark-6` |

Le survol était donc invisible une ligne sur deux sur **les deux** classements —
le compact comme le détaillé — et dans le journal de match.

## Le verrou a trouvé deux paires que la carte ne voyait pas

Dès sa première exécution :

```
--white  (#FFFFFF) / --white-1 (#FDFDFD) = 1,0172
--dark-1 (#26263D) / --black-1 (#292929) = 1,0115
```

Même défaut, même cause. `--white-1` et `--black-1` avaient **un usage chacun**,
contre 345 et 217 pour leurs jumeaux — le même déséquilibre qui trahissait
`--dark-7` (35 contre 136). Aucune des deux paires n'était opposée nulle part.

Les deux ont été fusionnées : l'unique usage de `--white-1` vit dans une feuille
qu'aucun template ne charge, et celui de `--black-1` colorait un texte de survol
à 1,0115 de la couleur de texte courante — un survol qui ne changeait rien.

Un verrou qui échoue n'est pas livrable ; les fusionner était le minimum pour
que le contrôle soit vert, et le coût s'est révélé de deux lignes.

## Vérification à l'écran

Le survol du classement détaillé, mesuré au `getComputedStyle` dans un
navigateur, sur la page où un coach le rencontre — pas sur le fragment nu, qui
se rend sans le bundle et donc sans aucun style.

**Avant** (contre-épreuve, ancien couple remis) :

```
ligne 1  repos transparent    survol rgb(240,242,244)   VISIBLE
ligne 2  repos rgb(240,242,244)  survol rgb(240,242,244)   *** INVISIBLE ***
ligne 3  repos transparent    survol rgb(240,242,244)   VISIBLE
ligne 4  repos rgb(240,242,244)  survol rgb(240,242,244)   *** INVISIBLE ***
```

**Après** : les quatre lignes passent à `rgb(229,230,239)` — `--dark-5`.

Une ligne sur deux, exactement comme la carte l'annonçait.

## Falsification

| Mutation | Constaté |
|---|---|
| Réintroduire un token à 1,0012 d'un autre | contrôle C rouge, la paire nommée avec son rapport |
| Reculer le survol du classement sur `--dark-6` | recensement : `--dark-6/--dark-6 = 1,0000 INVISIBLE` |
| La même, mesurée au navigateur | une ligne sur deux redevient invisible |

## Ce que le verrou ne couvre pas

Il porte sur les **valeurs des tokens**, donc il n'attrape pas deux états qui
emploieraient le **même** token — `--dark-6` en zébrage et `--dark-6` en survol
se voit à 1,0000 et passe le contrôle. C'est le choix de la carte, assumé :
attraper la cause plutôt que ses effets, une fois plutôt qu'en chaque endroit.
Le script de recensement écrit pour cette carte couvre l'autre moitié, mais il
demande de connaître les neutralisations légitimes (`no-hover`) et n'a pas été
retenu comme contrôle.

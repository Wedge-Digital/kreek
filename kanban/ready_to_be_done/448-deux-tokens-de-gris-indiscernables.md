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

- [ ] Trancher A ou B
- [ ] Appliquer, en relisant les 35 usages de `--dark-7`
- [ ] Vérifier les vingt et une feuilles qui portent les deux
- [ ] Le contrôle de proximité dans `check-css-collisions.sh`
- [ ] `make lint && make check-arch`

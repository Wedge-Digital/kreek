# Le verrou de portée du CSS n'est exécuté par personne

**Priorité : moyenne** — le verrou existe, il est rouge, et rien ne le dit
**Fichiers :** `scripts/check-css-collisions.sh`, `scripts/check-arch.sh`
**Trouvée par :** la carte 332, en lançant le script à la main
**Dépend de :** 359

## Problème

`scripts/check-css-collisions.sh` est le verrou de la carte 341. Il **n'est
branché nulle part** : ni dans `make lint`, ni dans `make check-arch`, ni dans
un job de CI. Aucune des cinq commandes que la CI exécute ne l'appelle.

Il est aujourd'hui **rouge**, et personne ne l'a su :

```
Contrôle A · Portée      46/46 feuilles conformes            ✓ PASS
Contrôle B · Collisions  6 sélecteurs divergents             ✗ FAIL
     2×  :root
     2×  .ts-dropdown     2×  .ts-dropdown .option     2×  .ts-dropdown-content
     2×  .ts-team-name    2×  .ts-team-meta
```

C'est mot pour mot ce que le `CLAUDE.md` décrit — *« une cible que personne
n'exécute finit rouge sans que personne ne le sache, c'est exactement ce qui est
arrivé au formatage »*. La carte 341 a écrit la règle **et** l'outil qui la
vérifie, puis a oublié de le brancher. Le verrou de son épic n'a donc jamais
rien gardé.

## Deux causes distinctes derrière les six sélecteurs

**Quatre viennent de `vendor/tom-select.min.css`**, entrée dans le bundle à la
carte 17. Une feuille amont n'est pas scopable : la scoper serait la réécrire,
et la prochaine montée de version écraserait le travail. Le contrôle B n'a pas
de raison de la regarder.

**Deux sont la paire de la carte 359** — `.ts-team-name` et `.ts-team-meta`,
deux tailles de police entre `components/team-selection.css` et
`pages/match-report-shared.css`. Arbitrage visuel, traité là-bas.

## Conception

### `vendor/` est hors périmètre, et c'est le dossier qui le dit

Le script écarte déjà les feuilles globales par un marqueur `css:global` **en
tête de fichier**, en préférant un marqueur adjacent à une liste tenue dans le
script — une liste dérive.

Une feuille minifiée sur une ligne n'a pas d'en-tête commode, et lui en ajouter
un serait modifier un fichier amont qu'on veut pouvoir remplacer à l'identique à
la prochaine montée de version. **C'est donc le dossier `vendor/` qui porte
l'information.** Ce n'est pas une entorse au principe : un dossier est une
information adjacente au fichier, pas une liste tenue ailleurs.

Le contrôle A l'ignore déjà — il ne regarde que `pages/` et `widgets/`. Seul le
contrôle B est à corriger.

### Où brancher, et pourquoi pas une sixième cible

Dans **`make check-arch`**, qui est déjà bloquant, déjà exécuté par le job
`qualite` de la CI, et où vivent les quatorze autres axes. Une sixième cible
`make` serait une cible de plus à penser à appeler — le défaut même que cette
carte corrige.

Un appel au script depuis `check-arch.sh`, et non un axe 15 qui absorberait ses
200 lignes de Python : le script a sa propre documentation en tête, ses deux
contrôles et ses messages, et rien à gagner à être recopié.

## Ce qui commande l'ordre

**359 d'abord.** Tant que la paire `.ts-team-*` n'est pas tranchée, le contrôle
B reste rouge de deux sélecteurs. On pourrait brancher en excluant nommément
cette paire, mais une exception que personne ne lève finit par s'installer — et
c'est le genre de tolérance que l'épic E04 passe son temps à retirer ailleurs.

## Checklist

- [ ] 359 close et le contrôle B vert sur `.ts-team-*`
- [ ] `vendor/` exclu du contrôle B, déduit du chemin, avec son motif en
      commentaire dans le script
- [ ] `bash scripts/check-css-collisions.sh` vert, contrôles A **et** B
- [ ] Appel depuis `scripts/check-arch.sh`, bloquant comme les autres axes
- [ ] Vérifié en le faisant échouer exprès : une règle non scopée ajoutée à une
      feuille de `widgets/` doit faire rougir `make check-arch`
- [ ] `make check-arch` vert sur l'ensemble du projet

## Ce que la carte ne couvre pas

L'arbitrage visuel de `.ts-team-name` / `.ts-team-meta` — c'est la 359, et elle
a sa propre discussion.

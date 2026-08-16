# CSS — solder les 79 collisions de sélecteurs, à rendu constant

**Priorité : haute** — rien n'est cassé aujourd'hui, mais tant que ces
collisions existent la carte 331 est impossible, et c'est elle qui supprime le
clignotement au chargement des pages
**Dépend de :** rien
**Bloque :** carte 331 (fusion du CSS en fichier unique chargé dans le `<head>`)
**Fichiers :** `assets/static/css/{pages,widgets,components}/*.css`,
`src/app/competitions/io/web/templates/admin/schedule.html`,
`scripts/check-css-collisions.sh` (nouveau), `Makefile`,
`.github/workflows/ci.yml`, `tests/e2e/` (harnais de captures)

## Le problème

Les 62 feuilles CSS chargées par l'application ont été écrites en **isolation
totale** : chaque page charge la sienne via un `<link>` placé dans son
fragment, et deux feuilles de page ne cohabitent jamais dans un même document.

Cette isolation a autorisé, sans que rien ne le signale, la réutilisation des
mêmes noms de classe avec des valeurs différentes. Mesure sur les 62 fichiers :

| | |
|---|---|
| Sélecteurs distincts | 2 390 |
| Paires en collision **déjà co-chargées** (la cascade tranche aujourd'hui) | 90 |
| Paires nouvelles à déclarations **identiques** (dédoublonnables, sans effet) | 137 |
| Sélecteurs à déclarations **divergentes** | **79**, en 55 familles |

Tant que les feuilles restent isolées, les 79 sont invisibles. Dès qu'on les
réunit dans un fichier unique, la dernière chargée gagne — **pour toute
l'application**. Le bundle ne neutralise pas les collisions, il les active.

Un cas n'attend même pas la fusion :

```css
/* widgets/dismissals.css */   :root { --role-risk: #A61B1B;  --tint-caution: … }
/* widgets/recruitment.css */  :root { --role-pending: #1B6B4A; --tint-cost: … }
```

Deux widgets redéfinissent les **variables globales** avec des sémantiques
différentes. `.side-actions` est bordé de rouge chez l'un, de vert chez
l'autre. Ils ne cohabitent pas aujourd'hui ; rien ne l'empêche demain, et le
premier qui les réunira n'aura aucun message d'erreur.

## Pourquoi cette carte existe séparément de la 331

Le clignotement mesuré sur la démo vient du `<link>` transporté par chaque
fragment : au premier passage sur une page, le contenu reste **50 à 200 ms
sans ses styles** (0,7 ms une fois la feuille en cache, d'où l'intermittence).
Le corriger définitivement suppose de tout charger une fois dans le `<head>`,
donc de réunir les feuilles, donc de régler les 79 collisions d'abord.

Séparer les deux cartes permet de vérifier ce lot-ci **page par page, à rendu
constant**, sans mêler la question du chargement à celle du nommage. Cette
carte ne touche pas à la mécanique de chargement : à la fin, le clignotement
est toujours là.

## La contrainte : aucune valeur calculée ne change

C'est l'exigence qui commande tout le reste.

> On ne modifie que la **portée** des règles, jamais leur **contenu**.

Trois opérations sont autorisées, par ordre de sûreté.

**1. Scoper sous la classe racine de la page ou du widget** — l'opération par
défaut.

```css
/* pages/app-team-detail.css */   .team-page     .team-header-logo { height: 120px }
/* pages/team-finalize.css   */   .finalize-page .team-header-logo { height:  72px }
```

Les déclarations sont intactes, et **les noms de classe dans le HTML aussi** —
donc les templates ne bougent pas et les tests e2e non plus. Les pages
disposent déjà de l'ancre nécessaire : `.team-page`, `.finalize-page`,
`.player-page`, et les widgets de leur classe racine (`.players-widget`,
`.coaches-search-panel`…).

**2. Renommer des variables CSS** — pour le seul cas non scopable, `:root`.
Renommer `--role-risk` en `--dismissals-role-risk` et ses références à
l'intérieur du même fichier ne touche ni le HTML, ni les tests, ni une valeur.

**3. Renommer une classe** — en dernier recours seulement, si le scoping est
impossible. Là seulement il faut corriger les templates *et* les tests e2e.

**Ce qui est interdit dans cette carte :** unifier deux définitions
divergentes, dédupliquer en choisissant un gagnant, « harmoniser » un
`letter-spacing` à 0,4 px et 0,5 px. Ces écarts sont peut-être des scories,
mais les corriger change le rendu. Si on les veut, c'est une autre carte, et
elle s'assume comme un changement visuel.

## Piège n°1 — la spécificité peut changer le gagnant

Ajouter un scope fait passer un sélecteur de `(0,1,0)` à `(0,2,0)`. Ça ne peut
altérer le rendu **que** là où la cascade se résout aujourd'hui par ordre de
chargement, c'est-à-dire entre deux fichiers effectivement co-chargés.

Vérification faite : sur les 79 sélecteurs divergents, **5 seulement** sont
dans ce cas, et l'examen les ramène à deux situations (cf. lots 2 et 3). Les
74 autres ne sont jamais arbitrés par la cascade — leurs fichiers ne se
rencontrent pas.

## Piège n°2 — le scoping doit englober le markup, sinon la règle meurt

Une règle scopée sous `.team-page` ne s'applique que si l'élément visé est
**à l'intérieur** de cet élément. Si un fragment de widget est injecté ailleurs
dans le DOM (hors de la racine de page), ses règles scopées ne le touchent
plus, et le style disparaît silencieusement — sans erreur, sans avertissement.

D'où l'ordre imposé plus bas : le harnais de captures **avant** le premier
scoping. C'est le seul dispositif qui attrape ce défaut.

## Le harnais de captures — à écrire en premier

Les tests e2e vérifient des comportements, pas des pixels. Or l'exigence ici
est « le rendu ne bouge pas », et les écarts en jeu sont géométriques —
hauteurs de logo, tailles de police, interlignes. Aucun test fonctionnel ne les
voit, et un œil ne les rattrape pas sur 40 pages.

Le harnais capture une image de référence par page touchée, avant tout
changement, et compare après chaque lot. Playwright est déjà en place ; il
manque le pas de comparaison et la liste des pages.

Il s'écrit **avant** le lot 1, et les références sont capturées sur `main`
avant la première modification de CSS. Une référence prise après coup ne prouve
rien.

## Lot 1 — le `:root` des widgets

Un sélecteur, mais c'est le blocage dur : `:root` ne peut pas être scopé.

Les `--role-*` / `--tint-*` de `widgets/dismissals.css` et
`widgets/recruitment.css` sont renommés avec un préfixe propre à chaque widget,
ou remontés dans `common.css` sous des noms distincts par intention. Aucun
fichier de widget ne redéfinit plus `:root`.

Zéro impact HTML, zéro impact test, zéro changement de valeur.

## Lot 2 — 77 sélecteurs divergents, scopés

Le gros du travail, sans risque mesuré. Les grappes principales :

| Grappe | Fichiers en cause | Nature de l'écart |
|---|---|---|
| `.team-header-*` (6) | `pages/app-team-detail` / `pages/team-finalize` | logo 120 px vs 72 px, titre `--h2` vs `--h3` |
| `.spp-budget-*`, `.spp-summary*` (10) | `pages/player-detail` / `pages/team-finalize` / `widgets/player-customisation` | même bloc SPP, trois copies |
| `.poule-*`, `.bracket-*` (12) | `components/competition-card` / `pages/new-competition-phase-3` | version vignette vs pleine page |
| `.team-card-*` (5) | `components/team-card` / `pages/competition-detail` | la page a recopié le composant |
| `.player-table*` (3) | `pages/team-build` / `widgets/team-player-widget` | |
| `.cart-*`, `.side-actions`, `.alert--warn` (5) | `widgets/dismissals` / `widgets/recruitment` | |
| Noms génériques (~40) | `.meta-label` (5 fichiers), `.section-title` (5), `.info-note` (4), `.skill-tag` (4), `.tab-empty-state` (4), `.type-mutation` (4), `.empty-state` (3), `.panel-title` (3), `.sub-section*` (3), + ~20 paires | |

Y compris `.match-date`, `.match-score` et `.match-team-name` : ils opposent
`app-home`/`app-news-feed` à un troisième fichier (`competition-detail`,
`player-detail`) qui ne cohabite jamais avec eux. Entre `app-home` et
`app-news-feed`, seuls fichiers réellement co-chargés (`news-feed.html`), leurs
déclarations sont **identiques** — la cascade n'arbitre rien.

### La règle qui simplifie tout

**Une collision oppose deux fichiers ; en scoper un seul suffit à la rompre.**
On scope donc systématiquement **celui qui a une racine unique et sans
ambiguïté**, et on laisse l'autre tel quel.

Corollaire : restent **globaux, jamais scopés** — `common.css`,
`layout-app.css`, tout `components/` (partagés par construction), et
`pages/match-report-shared.css`, qui est un fichier partagé malgré son dossier.
Face à eux, c'est toujours la page qui se scope. Ça résout à soi seul les
grappes `.team-card-*`, `.poule-*`/`.bracket-*`, `.ts-team-*`, `.two-col`,
`.btn-outline-sm`/`.btn-primary-sm` et `.mr-tab`.

### Inventaire des ancres

Les 35 fichiers porteurs d'au moins un sélecteur divergent, et la racine du
template qui les charge :

| Racine disponible | Fichiers CSS |
|---|---|
| `.team-page` | `pages/app-team-detail` *(deux templates, même racine)* |
| `.finalize-page` | `pages/team-finalize` |
| `.player-page` | `pages/player-detail` |
| `.competition-container` | `pages/all-competition` |
| `.article-container` | `pages/app-article-detail` |
| `.editor-container` | `pages/app-create-article` |
| `.my-teams-container` | `pages/app-my-teams` |
| `.admin-container` | `pages/competition-admin` |
| `.admin-panel` | `pages/competition-admin-groups`, `pages/competition-admin-schedule` |
| `.mr-container` | `pages/match-report-inducements` |
| `.pd-right` | `widgets/player-customisation` |
| `.players-widget` | `widgets/team-player-widget` |
| `.inducement-selector` | `widgets/inducement-selector` |
| `.ranking-classement-widget` | `widgets/classement-widget` |
| `.ranking-detailed-standings-widget` | `widgets/detailed-standings-widget` |

### Piège n°3 — quatre templates n'ont aucune racine

Leur premier élément ne porte pas de classe. Il faut lui en ajouter une, sans
style attaché :

| Template | Élément | Fichier concerné |
|---|---|---|
| `teams/…/widgets/my-teams-widget.html` | `<div>` | `widgets/my-teams-widget` |
| `team_creation/…/draft-team.html` | `<form>` | `pages/app-new-team` |
| `references/…/skill-picker-fragment.html` | `<div>` | `widgets/skill-picker` |
| `competitions/…/admin/schedule.html` | `<div>` | cf. lot 3 (attribut dupliqué) |

### Piège n°4 — `.create-card` est partagée par quatre pages

C'est l'angle mort que le scoping ne voit pas. Les phases 2, 3, 4 et 5 de
création de compétition ouvrent **toutes** sur `div.create-card
create-card--wide`. Or elles entrent en collision **entre elles** :

| Fichiers | Sélecteurs |
|---|---|
| `new-competition-phase-3` ↔ `phase-4` | `.info-note`, `.section`, `.section-title`, `.sub-section`, `.sub-section-title` |
| `new-competition-phase-2` ↔ `phase-3` | `.info-note`, `.sub-section`, `.sub-section-title` |

Scoper sous `.create-card` ne les distingue pas — les huit collisions
survivent intactes. Il faut un **modificateur par phase**
(`create-card--phase-2`, `-3`, `-4`, `-5`), ajouté à côté de
`create-card--wide` et sans style propre, qui devient l'ancre réelle.

C'est le seul endroit de la carte où le markup doit gagner une classe pour une
raison purement technique. À vérifier en démarrant : aucune autre famille de
pages ne partage ainsi sa racine.

### Piège n°5 — cinq fichiers sont chargés sous des racines différentes

Un même fichier CSS servi à plusieurs templates dont les racines diffèrent ne
peut pas être couvert par une ancre unique :

| Fichier | Racines rencontrées | Résolution |
|---|---|---|
| `pages/app-home` | `home-grid`, `home-main`, `allspace-home-grid` | ne pas le scoper — scoper l'autre côté (`competition-detail`, `player-detail`) |
| `pages/competition-detail` | `page-content-wide`, `admin-panel` | racine dédiée à ajouter aux deux templates |
| `pages/new-competition-phase-5` | `create-card`, `admin-summary` | idem, ou modificateur de phase (cf. piège n°4) |
| `widgets/dismissals` | `dis-page`, `dis-roster`, `dis-cart` | classe de famille `dis-scope` sur les trois racines |
| `widgets/recruitment` | `rec-page`, `rec-cart`, `rec-catalog`, `alert` | classe de famille `rec-scope` sur les quatre racines |

Pour `dismissals` et `recruitment`, la classe de famille est de toute façon
souhaitable : leur collision mutuelle porte sur `.cart-*`, `.side-actions` et
`.alert--warn`, qui vivent dans les fragments de panier des deux widgets.

Enfin, `page-content-wide` — racine de `competition-detail.html` — a tout d'une
classe partagée de mise en page. Elle ne peut pas servir d'ancre ; il faut une
classe propre à la page.

## Lot 3 — `.ts-team-meta` et `.ts-team-name`, un override délibéré

```
.ts-team-name
  components/team-selection.css         font-size: var(--text-small)
  pages/competition-admin-schedule.css  font-size: var(--text-tiny)
.ts-team-meta
  components/team-selection.css         font-size: var(--text-tiny)
  pages/competition-admin-schedule.css  font-size: 11px
```

`components/team-selection.css` est chargé dans le `<head>` du layout
(`app-layout.html:27`), donc présent sur **toutes** les pages.
`competition-admin-schedule.css` arrive d'un fragment dans le `<body>`, donc
après, et gagne par ordre de source. Ce n'est pas un accident de nommage : la
page de planning **compacte volontairement** le composant.

Scoper le fichier de page augmente sa spécificité, donc il continue de gagner :
le rendu est préservé et l'intention devient explicite au lieu de reposer sur
l'ordre de chargement.

Reste l'ancre. La racine du fragment est `.admin-panel`, partagée avec les
autres fragments d'admin, donc inutilisable. La classe qui conviendrait existe
déjà dans le markup — mais elle est inerte :

```html
<!-- src/app/competitions/io/web/templates/admin/schedule.html:3 -->
<div class="admin-panel" class="schedule-actions-panel">
```

**Deux attributs `class` sur le même élément.** HTML ne retient que le premier ;
`schedule-actions-panel` n'est jamais appliquée. Elle n'est stylée nulle part,
donc la réparer en `class="admin-panel schedule-actions-panel"` ne change rien
au rendu et fournit exactement l'ancre attendue. C'est la seule occurrence de
ce défaut dans le projet.

## Le verrou — pour que la dette ne se reforme pas

Sans lui, un copier-coller suffit à recréer une collision, et la carte 331
redevient impossible dans six mois sans que personne ne l'ait vu venir.

`scripts/check-css-collisions.sh` échoue si un même sélecteur est défini avec
des déclarations divergentes dans deux fichiers non scopés destinés au même
bundle. Cible `make check-css`, branchée sur le job `qualite` de la CI.

Le `CLAUDE.md` est explicite : une cible que personne n'exécute finit rouge
sans que personne ne le sache. Le branchement CI fait partie de la carte, pas
d'un suivi.

## Hors périmètre

- **La fusion en fichier unique** — carte 331. Elle devra garantir que
  `components/` est concaténé **avant** `pages/`, sans quoi l'override du lot 3
  s'inverse.
- **Les 137 paires à déclarations identiques** — dédoublonnables sans aucun
  risque, mais ça ne se justifie que dans le bundle. Carte 331.
- **Les 13 feuilles mortes** (`pages/index.css`, `widgets/turn-selector.css`,
  `components/match-report.css`…) référencées seulement par les maquettes
  `assets/templates/` — carte de nettoyage séparée.
- **Toute harmonisation visuelle.** Cf. « la contrainte » ci-dessus.

## Checklist

- [ ] Harnais de captures écrit, liste des pages touchées arrêtée
- [ ] Références capturées **sur `main`, avant toute modification de CSS**
- [ ] Lot 1 — `:root` de `dismissals` et `recruitment` : variables renommées,
      plus aucun `:root` dans `widgets/`
- [ ] Comparaison de captures après lot 1 : aucun écart
- [ ] Lot 2 — 77 sélecteurs scopés, en ne scopant **qu'un côté** de chaque
      collision : celui dont la racine est unique
- [ ] `common.css`, `layout-app.css`, `components/*` et
      `pages/match-report-shared.css` laissés globaux, non scopés
- [ ] Piège n°3 — racine ajoutée aux quatre templates qui n'en ont pas
      (`my-teams-widget`, `draft-team`, `skill-picker-fragment`, `admin/schedule`)
- [ ] Piège n°4 — modificateurs `create-card--phase-{2,3,4,5}` ajoutés, et
      vérification qu'aucune **autre** famille de pages ne partage sa racine
- [ ] Piège n°5 — racine dédiée pour `competition-detail` (ses deux templates)
      et `new-competition-phase-5` ; classes de famille `dis-scope` et
      `rec-scope` posées sur toutes les racines de `dismissals` et `recruitment`
- [ ] `pages/app-home.css` laissé non scopé — c'est l'autre côté qui se scope
- [ ] Comparaison de captures après lot 2 : aucun écart
- [ ] Lot 3 — attribut `class` dupliqué réparé dans `admin/schedule.html`
- [ ] Lot 3 — `.ts-team-*` scopés sous `.schedule-actions-panel`, override
      vérifié à l'écran sur la page de planning
- [ ] Comparaison de captures après lot 3 : aucun écart
- [ ] `scripts/check-css-collisions.sh` écrit, retourne 0 divergence
- [ ] Cible `make check-css` créée et branchée sur le job `qualite` de la CI
- [ ] `make lint` passe
- [ ] `make check-arch` passe sur l'ensemble du projet
- [ ] `make test` passe
- [ ] `make e2e` passe — aucun sélecteur de test n'a dû être modifié (si l'un
      l'a été, c'est qu'un renommage a eu lieu là où un scoping suffisait)

# Chrome et widgets différés — réserver la place avant le remplissage

**Priorité : moyenne** — rien n'est cassé, mais c'est le décalage visuel le
plus visible de l'application, et il survivra aux cartes 341 et 342
**Dépend de :** rien — indépendant du chantier CSS
**Fichiers :** `assets/static/css/layout-app.css`, `assets/static/css/common.css`,
les templates portant un conteneur `hx-trigger="load"` (37 occurrences)

## Le problème

Au chargement complet d'une page, le contenu est peint avant que les zones
remplies en différé n'aient reçu quoi que ce soit. Ces zones ont alors une
hauteur nulle. Quand leur fragment arrive, elles prennent leur taille réelle et
**poussent tout ce qui est en dessous**.

Mesuré sur la page d'accueil, en comparant la géométrie finale à celle des
zones vides — c'est-à-dire à l'état exact du premier rendu :

| Zone | Vide → remplie | Déplacement du contenu | Arrivée (à chaud → à froid) |
|---|---|---|---|
| `.menu-zone` | 0 → **120 px** | tout le contenu descend de **120 px** | 28 ms → 85 ms |
| `#latest-results-widget` | 0 → **160 px** | ce qui est dessous descend de **160 px** | 73 ms → 118 ms |
| `.sidebar` | 0 → 90 px de large | **aucun** | 28 ms → 45 ms |

Le contenu est peint vers 21 ms, puis subit jusqu'à **280 px de saut vertical
cumulé** pendant les 100 ms qui suivent. C'est ça qu'on voit bouger.

Deux précisions qui bornent le sujet. Le décalage ne se produit qu'aux
**chargements complets** — F5, URL directe, arrivée sur l'application. En
navigation HTMX interne, la `menu-zone` est bien réinterrogée mais son contenu
est remplacé **en une seule mutation** : il n'y a jamais d'état vide
intermédiaire, donc pas de saut. Et l'amplitude ne dépend pas de la tiédeur du
serveur — seule la durée d'exposition varie.

## Le contre-exemple donne la solution

La sidebar est remplie exactement comme les autres, au même moment, et ne
décale rien. La raison tient en deux lignes :

```css
/* layout-app.css:10 */
.sidebar { width: var(--sidebar-width); min-width: var(--sidebar-width); }
```

Elle **réserve sa dimension en CSS**, indépendamment de son contenu. Le
fragment se remplit dedans sans rien pousser.

C'est le principe à généraliser : *toute zone remplie de façon asynchrone doit
réserver sa dimension finale avant d'être remplie.*

## Cause n°1 — `.menu-zone` n'a aucune règle CSS

Vérifié : le sélecteur `.menu-zone` n'apparaît dans **aucun** fichier CSS du
projet. Sa hauteur vient entièrement du contenu injecté, donc elle vaut zéro
jusqu'à l'arrivée du fragment.

Or cette hauteur est parfaitement connue à l'avance, et déjà exprimée en token :

```css
common.css:69       --menubar-height: 60px;
layout-app.css:78   .menu-bar            { height: var(--menubar-height) }
layout-app.css:119  .sub-menu-placeholder{ height: var(--menubar-height) }
```

Les deux barres sont rendues **inconditionnellement** dans `app-menu.html` —
le `{% if %}` ne gouverne que les boutons *à l'intérieur* de
`.sub-menu-placeholder`, pas la barre elle-même. La hauteur desktop est donc
toujours de 120 px, et la réservation s'écrit sans valeur en dur :

```css
.menu-zone { min-height: calc(var(--menubar-height) * 2); }
```

C'est la correction la plus rentable de la carte : 120 px des 280, sur
**toutes** les pages de l'application.

## Cause n°2 — `.loading-placeholder` n'est stylé nulle part

Cinq templates affichent un texte d'attente en attendant leur fragment :

| Template | Texte |
|---|---|
| `teams/…/teams-team-detail.html:158` | « Chargement des joueurs… » |
| `players/…/player-detail.html:159` | « Chargement… » |
| `players/…/spp-spending-panel.html:21` | « Chargement… » |
| `competitions/…/competition-tab-standings.html:6` | « Chargement du classement… » |
| `competitions/…/competition-tab-detailed-standings.html:6` | « Chargement du classement détaillé… » |

La classe `.loading-placeholder` **n'existe dans aucun fichier CSS**. Chacun de
ces placeholders occupe donc la hauteur naturelle d'une ligne de texte — une
vingtaine de pixels — là où le contenu qui le remplace en fait plusieurs
centaines. Le placeholder donne l'illusion d'avoir traité le sujet ; il ne
réserve rien.

Lui donner une hauteur minimale règle les cinq d'un coup. La valeur ne peut pas
être unique — un tableau de joueurs et un classement n'ont pas la même taille —
donc soit une hauteur par usage via une classe modificatrice, soit une valeur
de base raisonnable complétée au cas par cas. À trancher au démarrage, sur
mesure des hauteurs réelles.

## Cause n°3 — 32 conteneurs différés sans aucun placeholder

L'application compte **37 conteneurs** `hx-trigger="load"`. Cinq ont un
placeholder (cf. ci-dessus), les autres sont des `div` vides — comme celui qui
cause le second saut de l'accueil :

```html
<!-- news/…/news-feed.html:137 -->
<div class="home-side">
  <div id="latest-results-widget" hx-get="…" hx-trigger="load" hx-swap="innerHTML">
  </div>
</div>
```

Aucune classe, aucune règle CSS, aucune hauteur. Il passe de 0 à 160 px.

Tous ne posent pas problème : un conteneur en bas de page, ou seul dans sa
colonne, ne pousse rien de visible. **L'inventaire des 37, avec la hauteur
réservée nécessaire ou la mention « sans effet », est le premier travail de la
carte.** Corriger sans cet inventaire, c'est réserver de la place au hasard.

## Ce que la carte ne fait pas

Elle ne supprime pas les requêtes, elle supprime leur effet visuel. Le menu
continue d'être chargé par un aller-retour HTMX au chargement de chaque page.

Le **rendre côté serveur** dans le layout — ce qui supprimerait la requête au
lieu d'en masquer l'effet — est une autre carte, plus lourde : il faut alors
que chaque page dispose du contexte du menu. À ouvrir séparément si le gain de
latence le justifie ; la présente carte n'en dépend pas et ne le bloque pas.

Elle ne traite pas non plus l'appel **dupliqué** à `/app/spaces` : la sidebar
figure deux fois dans `app-layout.html`, en desktop (`:32`) et en drawer mobile
(`:67`), chacune en `hx-trigger="load"`. Deux allers-retours pour le même
endpoint et les mêmes données, à chaque chargement. Réel, mesuré, mais c'est un
problème de requêtes, pas de mise en page.

## Piège n°1 — réserver juste, pas généreusement

Une réservation trop grande est un défaut symétrique : elle laisse un blanc
visible sous une zone dont le contenu est plus court que prévu, en permanence
cette fois plutôt que pendant 100 ms.

D'où `min-height` et non `height` : la zone peut grandir si son contenu
l'exige, elle ne peut simplement pas être plus petite que sa taille attendue.
Et d'où l'inventaire préalable : chaque valeur doit venir d'une mesure de la
hauteur réelle, pas d'une estimation.

## Piège n°2 — le breakpoint mobile change les hauteurs

Sous 768 px, `layout-app.css:157` masque `.sidebar` et `.menu-bar`
(`display: none !important`) au profit du header et de la tabbar mobiles. La
`menu-zone` n'a donc pas la même hauteur qu'en desktop, et une réservation de
120 px y créerait un blanc.

Chaque `min-height` posée doit être vérifiée — et probablement ajustée — dans
le `@media (max-width: 768px)` existant. Le `CLAUDE.md` impose ce breakpoint
unique ; ne pas en introduire d'autre.

## Vérification

Le protocole qui a servi au diagnostic est rejouable tel quel : comparer la
géométrie des zones vides à la géométrie finale, en vidant les conteneurs et en
forçant un recalcul de layout.

| Attendu après correction | |
|---|---|
| `.menu-zone` vide vs remplie | même hauteur |
| `#app-content` vide vs rempli | même `y` |
| Déplacement cumulé sur l'accueil | **0 px** |

À mesurer en desktop **et** sous 768 px, sur au moins l'accueil, la fiche
d'équipe et une page de classement — les trois familles de conteneurs différés.

Attention à un piège de mesure rencontré pendant le diagnostic : restaurer un
`innerHTML` brut après l'avoir vidé **détruit les liaisons HTMX** des éléments
concernés, qui cessent de réagir. Recharger la page après chaque mesure plutôt
que de restaurer.

## Checklist

- [ ] Inventaire des 37 conteneurs `hx-trigger="load"` : hauteur réelle mesurée,
      ou mention « sans effet visuel » justifiée
- [ ] `.menu-zone` : `min-height: calc(var(--menubar-height) * 2)`
- [ ] `.menu-zone` : valeur vérifiée et ajustée sous `@media (max-width: 768px)`
- [ ] `.loading-placeholder` : hauteur minimale, déclinée par usage si les cinq
      contenus diffèrent trop
- [ ] Conteneurs sans placeholder retenus par l'inventaire : `min-height` posée
- [ ] Aucune valeur en dur là où un token existe (`--menubar-height`,
      `--sidebar-width`)
- [ ] `min-height` partout, jamais `height` (cf. piège n°1)
- [ ] Mesure de non-régression : déplacement cumulé nul sur l'accueil, la fiche
      d'équipe et une page de classement, en desktop et sous 768 px
- [ ] Vérification visuelle qu'aucun blanc permanent n'est apparu sous une zone
      au contenu plus court que prévu
- [ ] `make lint` passe
- [ ] `make check-arch` passe sur l'ensemble du projet
- [ ] `make e2e` passe

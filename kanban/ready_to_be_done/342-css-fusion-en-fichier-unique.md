# CSS — fusionner les feuilles en un fichier unique chargé dans le `<head>`

**Priorité : haute** — c'est cette carte qui supprime le clignotement au
chargement des pages, mesuré à 50–200 ms sur la démo
**Dépend de :** carte 341, **terminée et vérifiée par les captures**. Fusionner
avant d'avoir soldé les collisions produit des régressions visuelles silencieuses.
**Fichiers :** `src/web/templates/app-layout.html`, les ~45 templates portant un
`<link>`, `src/main.rs` (ou `src/web/assets.rs`, nouveau), `Cargo.toml`
(`lightningcss`), `CLAUDE.md` (règle 5 des conventions widgets), `tests/e2e/`

## Le problème

Chaque page et chaque widget transporte sa feuille de style dans son propre
fragment :

```html
{% block content %}
<link rel="stylesheet" href="/static/css/pages/app-team-detail.css">
<div class="team-page">
```

Un `<link>` rencontré pendant le parsing HTML bloque le rendu de ce qui suit.
Mais un `<link>` **inséré dans un DOM déjà vivant** — ce que fait chaque swap
HTMX — ne bloque rien : le markup est peint immédiatement, sans ses styles,
puis re-stylé quand la feuille arrive.

Mesure sur la démo, transition `team/list` → `teams/<id>` :

| | |
|---|---|
| `htmx:afterSwap` — contenu dans le DOM, non stylé | t = 303,6 ms |
| `<link>` inséré | t = 304,9 ms |
| feuille appliquée | t = 313,2 ms |

Soit 9,6 ms **avec la feuille déjà en cache**. Au premier passage, cache vide,
la même feuille coûte **92 à 198 ms** (mesuré trois fois, CDN contourné ;
48–68 ms en HIT sur connexion chaude). C'est 3 à 12 frames à 60 Hz — le
clignotement.

Et sur cette page il est payé **deux fois** : le widget joueurs, chargé en
différé, porte sa propre feuille et arrive 260 ms plus tard.

## Ce que le cache ne peut pas faire

Piste écartée après mesure, pour qu'elle ne soit pas rouverte : ajouter un
`Cache-Control` sur `/static` ne change rien.

Cloudflare **pose déjà** `cache-control: max-age=14400` devant la démo
(`cf-cache-status: HIT`, gzip). Le réglage est en production, et le
clignotement existe quand même — parce qu'aucun en-tête de cache n'accélère un
fichier qui n'a **jamais** été téléchargé. Le cas froid est irréductible par le
cache ; il ne l'est que par l'absence de requête.

## Ce que fait la carte

Toutes les feuilles sont réunies en un fichier unique, chargé une seule fois
dans le `<head>` du layout. Plus aucun `<link>` dans un fragment, donc plus
aucune feuille à attendre après un swap. Le clignotement disparaît par
construction, pas par optimisation.

## Le poids — ce qu'on échange

| | Fichiers | Brut | Gzip |
|---|---|---|---|
| `<head>` aujourd'hui | 5 | 32 Ko | **6,8 Ko** |
| Bundle, concaténé | 62 | 319 Ko | 52 Ko |
| Bundle, minifié | 62 | 287 Ko | **~42 Ko** |

Le `<head>` passe donc de 6,8 à ~42 Ko gzippés, **bloquants au rendu**, sur le
premier chargement d'une session. En échange, plus une seule requête CSS sur
aucune navigation, jamais.

L'arbitrage est favorable et il faut le dire explicitement : on paie une fois
~35 Ko gzip de plus, mis en cache 4 h par Cloudflare, contre 50–200 ms
économisés à chaque première visite de chaque page. Mais c'est un arbitrage, et
si le premier rendu se dégradait au-delà du raisonnable, la mesure d'après-coup
doit pouvoir le dire — d'où le protocole de vérification plus bas.

## Les mesures — faites, pas supposées

**Le swap.** Navigation HTMX `team/list` → `teams/<id>`, observée au
`MutationObserver` et au journal réseau :

| Attendu | Mesuré |
|---|---|
| `<link>` insérés | **0** |
| Requêtes CSS | **0** |
| Contenu stylé à l'arrivée | oui — `.team-page` rend `max-width: 1400px` immédiatement |

**Le premier rendu**, cinq chargements à cache vide par page, médiane :

| Page | Avant | Après |
|---|---|---|
| Accueil | 488 ms, 8 requêtes CSS, 56 Ko | 500 ms, 2 requêtes, 308 Ko |
| Fiche d'équipe | 364 ms, 8 requêtes, 56 Ko | 356 ms, 2 requêtes, 308 Ko |

**La contrepartie annoncée ne se matérialise pas** : +12 ms sur une page, −8 ms
sur l'autre, dans le bruit de mesure. Et les 308 Ko sont **non compressés** —
le serveur de développement ne gzippe pas, là où Cloudflare le fait déjà en
production (`cf-cache-status: HIT`, gzip). Le bundle y pèsera ~40 Ko.

À noter : les 56 Ko de l'avant ne valent que pour la **première** page. Chaque
navigation en ajoutait, et c'est précisément ce que la carte supprime.

**Le démarrage du serveur** : construction du bundle en **70 ms**, 403 Ko lus,
313 Ko produits. Négligeable devant la compilation.

**Le dédoublonnage est sans objet.** La carte annonçait 137 paires à
déclarations identiques. Elles n'existent plus : le scoping de la carte 341 a
rendu ces sélecteurs distincts — `.team-page .x` et `.players-widget .x` ne sont
plus le même sélecteur. Il reste 2 sélecteurs présents dans deux feuilles, tous
deux divergents, et c'est la carte 359.

## Ce que la carte n'avait pas prévu

**La carte 341 était nécessaire et pas suffisante.** La première fusion a
modifié **13 853 valeurs calculées sur 32 pages**. L'isolation des feuilles
**globales** ne venait pas de leurs sélecteurs mais du fait qu'on ne les
chargeait pas, et le bundle supprime exactement cela.

Le cas d'école : `pages/app-home.css` style `.home-grid`, déclarée dans
`app-layout.html` — donc présente sur **toutes** les pages, alors que trois
seulement chargeaient la feuille.

Le verrou de la 341 ne pouvait pas le voir : son contrôle B compare les feuilles
deux à deux et signale les sélecteurs *divergents*. Un sélecteur défini une
seule fois n'est pas une collision, et c'est pourtant lui qui déborde.

D'où un **contrôle C** — `tests/e2e/visual/debordements.py` — qui pose la
question manquante : *ce sélecteur trouve-t-il du markup sur une page qui ne
chargeait pas sa feuille ?* Quatre cas dans le bundle, tous traités :

| Feuille | Sélecteur | Traitement |
|---|---|---|
| `pages/app-home.css` | `.home-grid`, 28 pages | conditionnée à son contenu par `:has()` — un marqueur de template ne marcherait pas, 46 templates ciblant `#app-content` en swap |
| `components/competition-card.css` | `.create-header-text .title/.sub`, 7 pages | doublon exact de `create-card.css` supprimé |
| `components/team-card.css` | `.tab-empty-state`, 3 pages | règle morte supprimée |
| `pages/match-report-shared.css` | `.btn-*-sm`, 2 pages | scopés sous `.mr-container` — seules 2 de ses 87 classes n'étaient pas préfixées |

Il reste **261 écarts** sur les 31 pages qui portent le bundle, tous identiques
et sans effet visuel : `lightningcss` normalise `background-position: 0% 0%` en
`0px 0px`.

**Le `<link>` est posé par un appel Rust direct**, pas par un filtre Askama.
La carte annonçait « trois modules à équiper » ; en réalité Askama compile un
filtre **dans le module de la struct**, et le `<link>` vit dans
`app-layout.html`, étendu par une soixantaine de templates. `{{
crate::web::css_bundle::chemin_app() }}` ne demande rien.

**Deux ruptures d'outillage.** Le verrou de la 341 s'est retrouvé à surveiller
une feuille sur quarante-six : son périmètre se lisait dans les `<link>`, que
cette carte supprime. Il le lit désormais dans la liste du bundle. Et le harnais
visuel vérifiait qu'une page charge sa feuille — contrôle devenu impossible ; il
vérifie maintenant qu'elle porte sa **classe de portée**.

**Une conséquence assumée.** Huit endpoints de fragment — les `widget-*`, plus
le tableau de bord et le résumé d'administration — perdent leurs styles quand on
visite leur URL directement. Ils ne sont jamais atteints autrement que par swap
HTMX, mais un signet sur l'un d'eux donnerait du contenu nu au lieu de contenu
stylé sans layout.

**Les tests e2e attendaient des `<link>`.** Deux d'entre eux utilisaient
l'apparition de la feuille d'un widget comme signal d'arrivée du fragment :

```python
page.wait_for_selector('link[href*="my-teams-widget.css"]')
```

Quatorze tests échouaient de ce fait. Ils attendent désormais la **racine du
widget** — plus juste, puisqu'elle atteste que le fragment est là, et non
qu'une feuille l'accompagnait.

Un quinzième échec est **antérieur à cette carte** et le reste :
`test_pending_enrollment_banner_is_informational` cherche `.state-banner--pending`,
une classe qui n'a **jamais** existé dans un template — aucun commit ne l'y a
ajoutée, et le CSS qui la style n'a donc jamais trouvé de markup. Le test ne peut
pas passer, et ce n'est pas le sujet ici.

## Le mécanisme recommandé — construire au démarrage, pas au build

Le projet n'a **ni `build.rs`, ni `package.json`, ni node**. Plutôt que d'y
introduire une étape de build, le bundle se construit **au démarrage du
serveur** :

1. lecture des feuilles sources depuis `assets/static/css/`, dans l'ordre imposé
   (cf. piège n°1) ;
2. concaténation puis minification via `lightningcss` — une crate Rust, aucun
   écosystème JS à tirer ;
3. empreinte du contenu → `/static/css/kreek.<hash>.css`, servi depuis la
   mémoire ;
4. l'URL est exposée aux templates par un `OnceLock`.

Les avantages tiennent en trois points. **Aucune cible de build à brancher** —
donc rien qui puisse être oublié dans `make dev` ou dans la CI, ce que le
`CLAUDE.md` identifie comme le mode de défaillance classique. **La boucle de
dev fonctionne telle quelle** : `make dev` surveille déjà `assets/static/css`,
et `cargo watch` relance le process, qui reconstruit le bundle. **Le
cache-busting est gratuit** : l'empreinte change exactement quand le contenu
change, et l'URL avec elle, ce qui autorise enfin un `Cache-Control:
immutable`.

Le coût est un temps de démarrage légèrement allongé — 319 Ko à parser et
minifier — à mesurer et à noter dans la carte au moment de l'implémentation.

L'alternative, une cible `make build-css` produisant un fichier versionné, est
possible mais introduit une étape que rien n'oblige à exécuter. À n'envisager
que si le coût de démarrage se révèle gênant.

## Piège n°1 — l'ordre de concaténation n'est pas libre

Héritage direct du lot 3 de la carte 341. La page de planning surcharge
volontairement le composant de sélection d'équipe :

```
components/team-selection.css         .ts-team-name { font-size: var(--text-small) }
pages/competition-admin-schedule.css  .ts-team-name { font-size: var(--text-tiny) }
```

Aujourd'hui la page gagne parce que son `<link>` est dans le `<body>`, après le
`<head>`. Dans un fichier unique, **c'est l'ordre de concaténation qui décide**.
L'ordre imposé est donc :

```
common.css → layout-app.css → components/* → pages/* → widgets/*
```

Toute inversion casse silencieusement les surcharges de page. L'ordre est à
inscrire en dur dans le code de construction, avec un commentaire qui dit
pourquoi — pas dans un `glob` dont le tri dépendrait du système de fichiers.

## Piège n°2 — les templates doivent perdre leurs `<link>`, et le `CLAUDE.md` avec

Il ne suffit pas d'ajouter le bundle au `<head>` : si un fragment conserve son
`<link>`, il continue de déclencher une requête au swap et le clignotement
persiste sur cette page-là. Les ~45 templates concernés doivent être vidés de
leur `<link>`, sans exception.

Conséquence documentaire : la **règle 5 des conventions widgets** du
`CLAUDE.md` — « chaque widget embarque son propre `<link rel="stylesheet">` » —
devient fausse. Elle doit être réécrite dans le même commit, sans quoi la
prochaine session la réappliquera de bonne foi.

## Piège n°3 — les BCs extractibles, et pourquoi ce n'est pas un problème

`auth` et `spaces` sont maintenus copiables tels quels. En apparence, mettre
leur CSS dans le bundle de l'hôte crée une adhérence.

En réalité non, et la distinction est nette : **les fichiers sources restent
rangés par BC** dans `assets/static/css/`, le bundle n'est qu'un artefact de
construction. Extraire un BC, c'est copier ses sources CSS comme on copie ses
templates. Ce qui change, c'est que ses templates cessent de déclarer un
`<link>` — donc le BC dépend **moins** de son hôte, pas plus : la feuille lui
est fournie par l'hôte, exactement comme le layout.

Cas particulier `auth` : ses pages passent par `auth-layout.html`, pas par
`app-layout.html`, et ce sont des chargements complets sans swap HTMX — donc
sans clignotement. Elles peuvent garder leur chargement actuel. À trancher au
démarrage : un second bundle pour `auth`, ou statu quo.

## Piège n°4 — l'URL du bundle doit atteindre trois layouts, pas quarante

Le `<link>` du bundle vit dans `app-layout.html`, `auth-layout.html` et
`widget-tester-layout.html`. C'est là, et seulement là, qu'il faut lire
l'empreinte.

Un filtre Askama lisant le `OnceLock` évite d'ajouter un champ aux ~60 structs
de template :

```html
<link rel="stylesheet" href="{{ "app"|css_bundle }}">
```

Trois modules à équiper d'un `use crate::filters;`, contre soixante structs à
modifier. Aucun filtre personnalisé n'existe encore dans le projet — ce serait
le premier, et Askama 0.12 les résout depuis un module `filters` visible dans
la portée de la struct.

## Piège n°5 — n'embarquer que les feuilles vivantes

Treize fichiers ne sont référencés que par les maquettes statiques
`assets/templates/` : `pages/index.css`, `pages/app-match-report-{1,2}.css`,
`widgets/turn-selector.css`, `components/match-report.css`, et neuf autres. Ils
ne doivent **pas** entrer dans le bundle — ce serait 13 fichiers de règles
mortes servies à chaque visiteur, et autant de collisions potentielles
réintroduites après le travail de la 341.

La liste des feuilles à concaténer se dérive des `<link>` présents dans `src/`,
pas d'un parcours de dossier.

## Le dédoublonnage

Les **137 paires de sélecteurs à déclarations identiques** repérées lors de
l'analyse deviennent, dans un fichier unique, de la duplication pure. Les
fusionner est sans risque **par définition** — les déclarations sont
rigoureusement les mêmes. C'est le bon moment, et c'est autant de poids en
moins dans un fichier désormais bloquant au rendu.

À faire **après** la première mesure de gain, pour ne pas mêler deux effets.

## Vérification — remesurer, pas supposer

Le critère d'acceptation est un chiffre, pas une impression.

**Sur la démo**, rejouer la trace de la transition `team/list` → `teams/<id>`
avec l'instrumentation qui a servi au diagnostic : écouter `htmx:afterSwap`,
observer les `<link>` insérés, corréler avec `performance.getEntriesByType('resource')`.

| Attendu | Valeur |
|---|---|
| `<link>` insérés lors d'un swap | **0** |
| Requêtes CSS lors d'un swap | **0** |
| Fenêtre `afterSwap` → styles appliqués | **néant** — il n'y a plus rien à attendre |

**Sur le premier chargement**, mesurer la contrepartie : temps jusqu'au premier
rendu, avant et après. C'est le seul risque de régression de cette carte, et il
doit être chiffré, pas supposé.

**Les captures de la carte 341** sont rejouées : le bundle ne doit modifier
aucun pixel. Si un écart apparaît, c'est que l'ordre de concaténation est faux
(piège n°1) ou qu'une collision a échappé à la 341.

## Hors périmètre

- **Les 13 feuilles mortes** — les exclure du bundle suffit ici ; les supprimer
  du dépôt est une carte de nettoyage séparée.
- **Toute harmonisation visuelle.** Comme dans la 341 : le rendu ne bouge pas.
- **Le décalage de mise en page au chargement complet** — la `menu-zone` qui
  passe de 0 à 120 px et pousse le contenu, le widget interne qui fait de même
  sur 160 px. Mesuré, réel, mais c'est un autre phénomène que le clignotement
  CSS, et il appelle sa propre carte (`min-height` réservant la place, ou rendu
  serveur du menu).

## Checklist

- [ ] Carte 341 terminée, captures comparées, aucun écart
- [ ] `lightningcss` ajouté au `Cargo.toml`
- [ ] Construction du bundle au démarrage : lecture, ordre imposé,
      minification, empreinte, service depuis la mémoire
- [ ] L'ordre `common → layout → components → pages → widgets` est en dur, et
      commenté — pas un `glob`
- [ ] Liste des feuilles dérivée des `<link>` de `src/`, pas d'un parcours de
      dossier (les 13 mortes restent dehors)
- [ ] Filtre Askama `css_bundle`, câblé dans les trois layouts
- [ ] Sort de `auth` tranché : bundle séparé ou statu quo
- [ ] Les ~45 templates vidés de leur `<link>` — vérifier qu'il n'en reste
      aucun : `grep -r 'rel="stylesheet"' src/`
- [ ] `Cache-Control: immutable` posé sur le bundle empreinté (et sur lui seul)
- [ ] `CLAUDE.md` — règle 5 des conventions widgets réécrite
- [ ] Mesure sur la démo : 0 `<link>` inséré, 0 requête CSS au swap
- [ ] Mesure du premier rendu, avant/après, chiffrée dans la carte
- [ ] Captures de la 341 rejouées : aucun pixel modifié
- [ ] Temps de démarrage du serveur mesuré et noté
- [ ] Dédoublonnage des 137 paires identiques, **après** la première mesure
- [ ] `make lint` passe
- [ ] `make check-arch` passe sur l'ensemble du projet
- [ ] `make test` passe
- [ ] `make e2e` passe

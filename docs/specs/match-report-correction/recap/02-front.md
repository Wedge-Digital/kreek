# Phase 2 — Architecture front · page Recap

Maquette de référence : `assets/rawpages/html/app-match-report-recap-correction.html`

## Widgets

**Aucun widget.** La zone de correction est rendue **inline** dans `recap.html`.

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| — | — | — | — | — | — |

Justification :

- `recap.html` est aujourd'hui un template monolithique rendu par `get_recap`,
  qui agrège déjà des données de plusieurs BCs (`teams`, `competitions`,
  `spp_calculator`) **via ports, pas via widgets**. Introduire un premier widget
  ici ferait cohabiter deux modèles de composition sur la même page.
- Le critère CLAUDE.md du pattern « page d'assemblage à widgets » (3+ sections
  interactives indépendantes) n'est pas rempli : la zone porte **une seule
  action**, sans interaction locale.
- Aucun swap partiel n'est nécessaire : la correction change l'état du rapport
  entier, donc toute la page change.

## Événements DOM

**Aucun.** Pas de widget, donc aucune communication inter-widgets, rien à publier
sur `body`.

## Actions

| Action | Méthode | Route | Déclencheur | Réponse |
|---|---|---|---|---|
| Corriger le rapport | POST | `/app/{space_id}/match-report/{match_report_id}/recap/unpublish` | `hx-post` + `hx-confirm` | `HX-Refresh: true` |

### Pourquoi `hx-post` et non `<form method="post">`

La publication actuelle utilise un `<form method="post">` nu. Deux raisons de ne
pas reproduire ce choix :

1. **`hx-confirm` ne se déclenche que sur une requête pilotée par HTMX.** Avec un
   form nu, aucune confirmation n'est possible sans JS maison. Le précédent du
   projet pour les actions destructives est `hx-confirm`
   (`admin/schedule.html`, `admin/widgets/schedule-round-detail.html`).
2. **Le middleware CSRF décrit dans CLAUDE.md n'est pas implémenté** —
   `src/web/middleware/` ne contient aucun module CSRF. C'est de l'architecture
   cible. Le jour où il arrive, il rejettera les POST sans `HX-Request: true`, et
   la publication actuelle cassera. Inutile d'ajouter une seconde dette du même
   type.

### Pourquoi `HX-Refresh` et non `HX-Redirect`

L'URL ne change pas — seul l'état du rapport change. La page se recharge sur le
même recap, désormais en `ReadyToPublish`, avec le bandeau « en cours de
correction » (état 5 de la maquette).

## Répartition front / back

| Traitement | Côté | Détail |
|---|---|---|
| Décider si la correction est possible | **Back** | ports vers `teams` (phase de jeu) et `players` (SPP dépensés) |
| Formuler la raison du blocage | **Back** | VM construit par le handler — inclut le nom de l'équipe qui bloque (règle 3) |
| Confirmation avant envoi | **Front** | `hx-confirm`, natif HTMX |
| Rechargement après correction | **Back** | `HX-Refresh: true` |

**Zéro JS inline, zéro Alpine.** La règle CLAUDE.md « la page hôte ne porte pas
de logique » est respectée sans effort.

## Modes d'interaction

La zone n'a que des états d'affichage, aucun mode d'édition :

| État | Rendu | Condition |
|---|---|---|
| Corrigeable | bouton actif + phrase de conséquence | rapport `Published`, garde-fou passant |
| Bloqué | bouton `disabled` + encart avec la raison nommant l'équipe | rapport `Published`, garde-fou bloquant |
| En cours de correction | bandeau orange + CTA de re-publication existants | rapport `ReadyToPublish` après une correction |

La zone n'est pas rendue du tout pour un rapport `Draft`, `PreMatch` ou
`Cancelled` — ces états ne sont pas accessibles depuis le recap.

## Le garde-fou affiché n'est qu'un indice

Le blocage est **volatile** : l'adversaire peut le déclencher à tout instant, y
compris pendant que le coach a la page ouverte. Un bouton actif affiché n'est
donc jamais une garantie.

Conséquence, actée en règle 9 : **le POST revérifie le garde-fou côté serveur**
et le domaine refuse la transition si les conditions ne tiennent plus. C'est ce
qui rend acceptable de ne pas rafraîchir l'état côté client — donc de ne pas
faire de widget.

Comportement en cas d'échec de cette revérification : la page se recharge en
affichant l'état bloqué avec la raison à jour, **pas** une page d'erreur.

## Impact sur les fichiers existants

| Fichier | Modification |
|---|---|
| `src/app/match_report/io/web/templates/recap.html` | bandeau conditionnel avant `ms-cta-row` ; zone de correction après, dans la branche `is_published` |
| `assets/static/css/pages/match-report-recap.css` | nouvelles classes `.ms-correct-*` et `.ms-unpublished-banner`, telles que maquettées |
| `src/app/match_report/routes.rs` | route `recap_unpublish` |

## Règles métier identifiées en phase 2

Reportées dans le README (règles 9 et 10) :

- Le garde-fou est revérifié côté serveur au POST ; l'affichage n'est qu'un
  indice.
- Le bandeau « en cours de correction » est **visible par tous** ceux qui
  accèdent au recap, y compris un admin d'espace spectateur : c'est un fait sur
  le rapport, pas une information personnelle.

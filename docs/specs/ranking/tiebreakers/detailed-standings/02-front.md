# Phase 2 — Architecture front (`detailed-standings`)

Onglet « Classement détaillé » de la page compétition. Onglet de **vérification** : il
expose chaque nombre composant le total, puis les compteurs de départage dans l'ordre de
priorité configuré, pour répondre à « pourquoi cette équipe est-elle devant celle-là ? ».

Maquette de référence : `assets/rawpages/html/app-competition-detail.html`, bloc
`#tab-classement-detaille` (tableau nominal, variante à 7 critères, deux états vides et
un état d'erreur).

## Composition

Montage identique à celui de l'onglet « Classement » déjà en place : `competitions`
possède la page et la barre d'onglets, son template pose un conteneur qui charge la
widget de `ranking` via `AppRoutes`.

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| Onglet hôte `competition-tab-detailed-standings.html` | `competitions` | `competition_tab_detailed_standings(space_id, competition_id, season_id)` | clic sur l'onglet (`hx-get` → `#tab-content`) | — | assemblage pur |
| `detailed_standings_widget` | `ranking` | `GET /app/{space_id}/ranking/{competition_id}/{season_id}/detailed-widget` | `load` | — | **lecture seule** |

Fichiers correspondants :

| Rôle | Chemin |
|---|---|
| Handler widget | `src/app/ranking/io/web/widgets/detailed_standings_widget.rs` |
| Template widget | `src/app/ranking/io/web/templates/widgets/detailed-standings-widget.html` |
| CSS widget | `assets/static/css/widgets/detailed-standings-widget.css` |
| Route widget | `src/app/ranking/routes.rs` — `DETAILED_STANDINGS_WIDGET` |
| Template onglet | `src/app/competitions/io/web/templates/competition-tab-detailed-standings.html` |
| Route onglet | `src/app/competitions/routes.rs` — `competition_tab_detailed_standings` |

## Événements DOM

**Aucun.**

L'onglet est en lecture seule : aucune mutation, aucune widget voisine à notifier,
aucune donnée à recevoir d'un autre widget. Les tableaux « Événements » et « Actions »
du gabarit de phase 2 sont donc vides — écrit explicitement pour éviter qu'on invente
plus tard un événement dont personne n'a besoin.

## Actions

**Aucune.** Pas d'endpoint POST/PUT/DELETE dans cette unité.

## Front vs back

Tout est **rendu serveur**. Zéro JS, zéro Alpine dans la widget.

| Besoin | Où c'est traité |
|---|---|
| Ordre des lignes, rangs, ex æquo | Back — `standings_service::build_ordered_standings` (déjà livré) |
| Colonnes de départage à afficher et leur ordre | Back — le `TiebreakOrder` de la compétition |
| Mise en évidence du critère décisif | Back — règles R21/R22 ci-dessous, une classe CSS par cellule |
| Défilement horizontal du tableau large | **Front, CSS pur** — conteneur `.sd-scroll` en `overflow-x: auto` |

Le défilement horizontal est indispensable : de 1 à 7 colonnes de départage s'ajoutent
aux 8 colonnes fixes. Conformément au CLAUDE.md, c'est le conteneur du tableau qui
défile, jamais le `body`.

## Widgets existantes réutilisables

- **`classement_widget` n'est pas étendu** — deux onglets distincts, deux endpoints, deux
  VMs. Un mode « détaillé » sur la widget existante mêlerait deux responsabilités
  d'affichage dans un même handler.
- **La réutilisation se fait au niveau du service** : les deux widgets appellent
  `standings_service::build_ordered_standings`. L'ordre affiché par les deux onglets est
  ainsi le même par construction, et non par coïncidence.
- **Aucun port nouveau** : `IRankingCompetitionPort` fournit déjà les noms d'équipes
  (`find_enrolled_teams`), les poules (`find_groups`) et les règles avec la configuration
  de départage (`find_ranking_rules`).

## Décisions de maquette confirmées

| # | Décision | Motif |
|---|---|---|
| 1 | **Découpage par poule** repris à l'identique du classement simple (un tableau par poule, plus la section « Non assignées ») | Deux onglets affichant le même classement ne peuvent pas diverger sur le périmètre d'un classement |
| 2 | **Colonne `nb_wins` conservée** dans le bloc départages, malgré la redondance avec la colonne `G` | Sa position dans l'ordre de priorité est une information que `G` ne porte pas — c'est la lecture de gauche à droite qui fait l'intérêt de l'onglet |
| 3 | **Colonnes = critères actifs uniquement**, dans l'ordre de priorité configuré, numérotées (`1 · Δ TD`, `2 · TD+`, …) | La lecture de gauche à droite suit l'algorithme de résolution |

## Règles métier identifiées à cette étape

La feature en comptait 20 à l'issue de `tiebreak-calc`. Deux s'ajoutent, propres à cet
onglet :

| # | Règle |
|---|---|
| **21** | Le critère mis en évidence est le **premier**, dans l'ordre de priorité, dont les valeurs ne sont pas toutes égales au sein d'un groupe d'équipes à égalité de points. Les critères qui le précèdent sont marqués « égaux », ceux qui le suivent sont neutres. |
| **22** | Lorsque tous les critères actifs sont égaux au sein du groupe, **aucun** n'est mis en évidence et l'ex æquo est signalé comme tel (règle 19 rendue visible). |

Ces deux règles ne touchent **pas** au calcul du classement. Les équipes à égalité de
points sont déjà consécutives dans la liste ordonnée (les points sont la clé de tri
primaire) : il s'agit d'un parcours de groupes sur des données déjà produites, appuyé sur
`TiebreakCriterion::value_of`, sans aucune évolution de `standings::compare`.

## Points reportés

- **Sous-groupes imbriqués (phase 6)** — dans un groupe de trois équipes à égalité de
  points dont les différences de TD valent +5, +2, +2, le critère 1 « n'est pas
  constant » et devient donc décisif au sens de R21 ; mais entre la 2ᵉ et la 3ᵉ, c'est le
  critère 2 qui a tranché. Faut-il marquer par sous-groupes successifs, ou s'en tenir au
  premier niveau ? Sans effet sur l'architecture front.
- **Découpage en cartes (phase 8)** — R21 et R22 iront dans une **carte distincte**,
  posée après celle qui affiche le tableau. Si leur implémentation se révèle plus retorse
  que prévu, elle se reporte sans rien bloquer : l'onglet reste exploitable sans la mise
  en évidence, il perd son commentaire visuel.

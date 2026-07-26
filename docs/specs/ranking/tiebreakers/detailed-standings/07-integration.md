# Phase 7 — Effets de bord (`detailed-standings`)

Persistance, événements, handlers, templates et tests E2E. Conception, pas
implémentation.

## Persistance

**Aucun impact base de données.** Ni migration, ni colonne, ni méthode de repository
nouvelle.

| Besoin | Couvert par |
|---|---|
| Lignes de classement de la saison | `IRankingRepository::find_latest_lines_for_season` |
| Part bonus du total | `ranking_lines.bonus_points` (carte 213) |
| Compteurs de départage | `ranking_lines.{td_for, td_against, casualties, fouls, completions}` (carte 216) |

C'est la première unité de la feature à ne rien toucher en base : les cartes 213 et 216
ont créé ces colonnes précisément pour cet onglet.

## Événements

**Aucun.** Pas d'événement domaine, pas d'app event, pas de listener. L'onglet lit une
projection existante — rien n'y mute.

## Handlers

### Coquille d'onglet — `competitions`

```rust
// competitions/io/web/competition_detail.rs

pub async fn get_tab_detailed_standings(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse
```

Calqué sur `get_tab_standings` (`competition_detail.rs:476`) :

- en-tête `HX-Request` présent ⇒ rend `DetailedStandingsTabTemplate` seul (fragment) ;
- absent ⇒ rend la page complète avec `active_tab = "detailed-standings"`, pour que
  l'accès direct à l'URL fonctionne comme la navigation HTMX ;
- identifiants invalides ⇒ `400`.

### Widget — `ranking`

```rust
// ranking/io/web/widgets/detailed_standings_widget.rs

pub async fn detailed_standings_widget(
    auth_session: AuthSession,
    Path((space_id, _competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse
```

Calqué sur `classement_widget` :

- `auth_session.user` absent ⇒ `401` ;
- `competition_id` ignoré — la saison suffit à identifier le classement ;
- un `build_vm` privé charge les quatre sources en `tokio::join!` (règles, équipes
  inscrites, lignes, poules), puis délègue la construction du VM à `builders.rs`.

`build_vm` doit rester **sous 20 lignes** (règle du CLAUDE.md) : la construction des
colonnes et celle des groupes sont extraites en fonctions nommées, comme l'a été
`tiebreak_order_of` dans le widget existant.

### Déplacement nécessaire

`tiebreak_order_of` est aujourd'hui **privé** dans `classement_widget.rs`, et le nouveau
widget en a besoin à l'identique. Il est déplacé dans `standings_service` : c'est le même
mappage port → domaine que `to_tiebreak_order`, avec la gestion de l'`Option` en plus.
Ses trois tests suivent le déplacement.

Sans ce déplacement, les deux widgets construiraient l'ordre de départage chacun de leur
côté et pourraient diverger — l'onglet détaillé afficherait des colonnes qui ne
correspondraient pas à l'ordre appliqué par l'onglet simple.

## Templates

| Template | Consomme | Nature |
|---|---|---|
| `ranking/io/web/templates/widgets/detailed-standings-widget.html` | `DetailedStandingsVm` | fragment de widget |
| `competitions/io/web/templates/competition-tab-detailed-standings.html` | — (conteneur `hx-get`) | fragment d'onglet |
| `competitions/io/web/templates/competition-detail.html` | — | + un onglet dans la barre, + une branche `active_tab` |

### Structure du widget

Alignée sur `classement-widget.html` :

1. `<link rel="stylesheet">` embarqué (règle 5 des widgets) ;
2. racine en `hx-disinherit="*"` (règle 3) ;
3. branche `rules_missing` ⇒ zone d'erreur ;
4. sinon, pour chaque groupe : titre optionnel, puis l'un des trois états ci-dessous ;
5. légende sous le tableau (elle explique la colonne Bonus, la mise en évidence et
   l'ex æquo — sans elle, les couleurs ne se comprennent pas).

### États

Les trois états existent déjà dans le widget Classement et sont repris à l'identique, à
un libellé près :

| Condition | Rendu |
|---|---|
| `vm.rules_missing` | « Impossible d'afficher le classement détaillé : les règles de classement ne sont pas configurées pour cette saison. » |
| `!group.has_enrolled_teams` | « Aucune équipe dans la compétition. » |
| `group.rows.is_empty()` | « Aucun match n'a encore été joué — tous les compteurs sont à zéro. » |

La mention « tous les compteurs sont à zéro » est propre à cet onglet : la maquette
l'ajoute parce qu'un tableau de compteurs vide, ici, se lirait comme une anomalie.

### Divergence assumée sur le trophée

`classement-widget.html` place le trophée dans la cellule du rang (`🏆1`). La maquette du
détaillé le place dans la cellule de l'équipe (`🏆 Les Korrigans FC`) et laisse le rang
nu. Les deux onglets afficheront donc le trophée dans une colonne différente.

**La maquette fait foi** — elle a été validée visuellement et la colonne de rang du
tableau détaillé est étroite. Écrit ici pour que ce soit un choix et non un oubli.

### Défilement horizontal

Le tableau est enveloppé dans `.sd-scroll` en `overflow-x: auto` : de 1 à 7 colonnes de
départage s'ajoutent aux 8 colonnes fixes. Conformément au CLAUDE.md, c'est le conteneur
du tableau qui défile, jamais le `body`.

## Tests E2E

Nouveau fichier `tests/e2e/test_detailed_standings.py`.

| Scénario | Vérifie |
|---|---|
| **Critère décisif mis en évidence** | Deux équipes à égalité de points que seule la différence de TD sépare ⇒ la cellule correspondante porte la classe décisive sur les deux lignes, les autres colonnes ne la portent pas |
| **Ex æquo total** | Deux équipes égales sur tous les critères actifs ⇒ même rang affiché, aucune cellule décisive, toutes marquées égales |
| **Colonnes = critères actifs, dans l'ordre** | Deux critères décochés en phase 2 ⇒ le tableau n'affiche que les restants, numérotés 1..n |

### Deux pièges à ne pas rejouer

**1. Les bonus sont cochés par défaut.** C'est ce qui a rendu vert et inutile le premier
test de la carte 219 : avec les bonus offensif et défensif actifs, un vainqueur 3-0 ne
totalise pas comme un vainqueur 1-0, aucune équipe n'est jamais à égalité de points, et le
classement est décidé par les seuls points. Les scénarios 1 et 2 exigent
`with_default_bonuses=False` (paramètre ajouté à `create_full_competition` par la carte
219).

**2. Le scénario 3 doit décocher des critères sans glisser-déposer.** Le drag & drop
HTML5 est la partie fragile de `test_competition_rules_tiebreakers.py`, qui doit déjà
gérer un repli en dispatchant les événements à la main. Décocher suffit : les critères
restants gardent l'ordre canonique, et l'ordre configuré est déjà couvert unitairement par
`to_tiebreak_order`. `create_full_competition` ne sait pas décocher de critère — l'ajout
d'un paramètre relève de la carte E2E (phase 8).

### Ce que les tests E2E doivent viser

La mise en évidence est le seul comportement de cette unité qu'aucun test unitaire ne
peut voir : `tiebreak_outcomes` est couvert dans le domaine, mais rien ne garantit que la
classe CSS atterrisse sur la bonne cellule du bon tableau. C'est là qu'est la valeur du
scénario 1 — et il doit être vérifié par mutation avant d'être commité, comme l'a exigé la
carte 219.

## Règles métier

**Aucune règle nouvelle.** La phase 7 place des effets de bord, et cette unité n'en a
presque aucun.

# Phase 3 — Architecture back — team-detail

## Mapping widget → BC

Un seul widget touché, un seul BC concerné : `players-widget` → BC `players`.
Le bandeau d'état (déclencheur) reste dans la page hôte du BC `teams`, sans
aucun fichier back nouveau côté `teams` — toute la coordination passe par des
événements DOM déjà spécifiés en Phase 2, aucune route `teams` n'est
impliquée.

## Widgets existantes vérifiées

`player_table_widget` (GET, déjà existant sous `io/web/player_table.rs`) est
la seule widget concernée. Aucune widget de rename/renumber/reorder
n'existe ailleurs dans le projet (vérifié Phase 1 : `personal_name` toujours
écrit `""` en base, `jersey` figé à la création, aucun event
`PlayerRenamed`/`JerseyChanged`).

## Plan de fichiers

| Fichier | Statut | Rôle |
|---|---|---|
| `players/io/web/widgets/player_table_widget.rs` | **Renommé** depuis `io/web/player_table.rs` | Dette de convention comblée (fichier fortement modifié → doit suivre la convention `_widget.rs` sous `widgets/`, cf. CLAUDE.md). Copié-collé intégral du contenu actuel (règle 5 CLAUDE.md), puis extension : le template struct gagne un champ `team_id: String` (nécessaire pour construire l'URL du POST batch depuis le fragment) |
| `players/io/web/templates/player-table-fragment.html` | Modifié en place | Ajoute le `<form>` autour de `#roster-tbody`, les listeners `hx-trigger` sur `rosterEditRequested`/`rosterEditCancelRequested`/`rosterEditSaveRequested` (cf. 02-front.md), la zone d'erreur inline |
| `players/io/web/roster_edition_controller.rs` | **Nouveau** | `post_update_roster` — reçoit le batch, appelle le use case, retourne le fragment à jour + `HX-Trigger` |
| `players/use_cases/update_roster_use_case.rs` | **Nouveau** | Orchestration : charge les joueurs actifs de l'équipe, vérifie l'unicité des numéros sur le batch soumis, appelle les méthodes de domaine par joueur (Phase 6), persiste, agrège succès/erreurs |
| `players/routes.rs` | Modifié | Nouvelle constante `PLAYERS_ROSTER_UPDATE` + méthode `Routes::update_roster()` |
| `players/router.rs` | Modifié | Câblage de la nouvelle route, import mis à jour vers le nouveau chemin du widget |
| `migrations/<timestamp>_add_display_order_to_players_proj.sql` | **Nouveau** | Colonne `display_order` (nullable) sur `players_proj` — l'ordre libre (glisser-déposer) est indépendant du numéro de maillot (cf. 02-front.md), il lui faut son propre champ persisté. Nullable : les joueurs jamais réordonnés n'ont pas de valeur |

*Oubli corrigé après coup : cette colonne et la migration associée n'avaient pas été listées à l'écriture initiale de cette phase — nécessaire pour que la Phase 4 (DTOs) sache où lire/écrire l'ordre.*

`find_by_team_id` (`projection_repository.rs:30`) change d'ordre de tri :
`ORDER BY jersey NULLS LAST, player_id` → `ORDER BY display_order NULLS LAST, jersey NULLS LAST, player_id`. Rétrocompatible : un joueur jamais réordonné (colonne `NULL`) garde le tri par numéro de maillot actuel.

## Routes

```rust
pub const PLAYERS_ROSTER_UPDATE: &str = "/app/{space_id}/players/by-team/{team_id}/roster";
```

POST, même préfixe que `PLAYERS_BY_TEAM_WIDGET` (déjà `/app/{space_id}/players/by-team/{team_id}/widget`). Référencée depuis le template via
`app_routes.players.update_roster(space_id, team_id)` — même pattern que
`player_detail` déjà utilisé dans ce même fragment, y compris pour une route
appartenant à son propre BC (convention déjà en place dans ce fichier : tout
passe par `AppRoutes`, même en intra-BC).

## Ports

Aucun nouveau port : tout est intra-BC `players`, aucune donnée d'un autre BC
n'est nécessaire pour renommer, renuméroter ou réordonner.

## Domain services

Aucun : pas de DTO de port à transformer, la commande s'appuie directement sur
le repository `players` existant.

## Règles métier confirmées à cette étape

- **Numéro de maillot : 1 à 99** — correction explicite de l'utilisateur sur
  cette phase : ne pas reprendre la borne haute de `JerseyNumber`
  (`team_creation/domain/roster.rs:61`, limitée à 16, propre au contexte de
  recrutement initial) — la seule contrainte réelle ici est `< 100`. Le value
  object `players` (Phase 6) aura donc sa propre borne `1..99`, distincte de
  celle de `team_creation`.
- **Nom du joueur : 50 caractères maximum** — même limite que
  `set_player_identity()` (`team_roster_selected.rs:287`,
  `DomainError::PlayerNameTooLong`).
- **Unicité du numéro vérifiée au niveau use case, pas domaine** — écart
  documenté à la grille de décision générale du CLAUDE.md (« Ce jersey
  est-il libre ? » → Domaine), motivé par la frontière d'agrégat déjà en
  place dans ce BC : `Player` est individuellement event-sourcé (pas
  d'agrégat `Roster` englobant), un agrégat `Player` n'a donc pas de
  visibilité sur ses voisins. Le use case charge l'ensemble des joueurs
  actifs de l'équipe (déjà nécessaire pour construire la réponse) et
  rejette le batch entier si deux numéros entrent en collision, avant
  d'appeler la moindre méthode de domaine.

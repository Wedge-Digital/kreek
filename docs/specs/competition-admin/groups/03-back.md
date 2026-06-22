# Poules — Phase 3 : Architecture back ✅

## BC responsable

Le BC `competitions` possède les poules et leur structure. Les infos d'équipe pour l'affichage viennent du BC `teams` via un port.

## Fragment onglet Poules — BC `competitions`

### Route

```
GET /app/{space_id}/competitions/{competition_id}/{season_id}/admin/groups
    → fragment onglet poules
```

### Fichiers

```
src/app/competitions/io/web/admin/
├── groups_tab.rs                       ← handler GET fragment (assemblage)
├── groups_widgets.rs                   ← widgets unassigned pool + group cards
├── groups_actions.rs                   ← handlers POST (random-draw, reset, assign)
└── templates/admin/
    ├── groups.html                     ← fragment onglet (actions bar + conteneurs widgets)
    └── widgets/
        ├── unassigned-pool.html        ← chips équipes non assignées
        └── group-cards.html            ← grille de poules avec équipes

assets/static/css/pages/
└── competition-admin-groups.css        ← styles poules (group cards, chips, drop zones)
```

## Widgets — BC `competitions`

### Routes

```
GET .../admin/groups/unassigned     → widget pool d'équipes non assignées
GET .../admin/groups/cards          → widget grille de poules
```

Les deux widgets se rechargent sur `groupsChanged from:body`.

## Actions — BC `competitions`

### Routes

```
POST .../admin/groups/random-draw   → tirage aléatoire  → HX-Trigger: groupsChanged
POST .../admin/groups/reset         → vider les poules   → HX-Trigger: groupsChanged
POST .../admin/groups/assign        → assigner une équipe → HX-Trigger: groupsChanged
    body: { "team_id": "...", "group_id": "..." }
```

## Port inter-BC

Le BC `competitions` définit un trait pour récupérer les infos d'équipe depuis le BC `teams`.

```
src/app/competitions/ports.rs
    → trait ITeamInfoPort { find_enrolled_teams(season_id) -> Vec<TeamInfoDto> }

src/infrastructure/competitions/
    └── team_info_adapter.rs
        → implémente ITeamInfoPort en appelant ITeamRepository du BC teams
```

### DTO du port

```rust
pub struct TeamInfoDto {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
}
```

## Persistance

### Nouvelles tables

```sql
CREATE TABLE competition_groups (
    id          TEXT PRIMARY KEY,
    season_id   TEXT NOT NULL,
    name        TEXT NOT NULL,
    position    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE competition_group_teams (
    group_id    TEXT NOT NULL REFERENCES competition_groups(id),
    team_id     TEXT NOT NULL,
    PRIMARY KEY (group_id, team_id)
);
```

### Méthodes repository (ISeasonRepository ou nouveau port dédié)

- `find_groups(season_id) -> Vec<GroupWithTeams>`
- `save_groups(season_id, groups: Vec<GroupAssignment>)`
- `reset_groups(season_id)`
- `assign_team(group_id, team_id)`
- `unassign_team(team_id)`

## Middleware d'autorisation

Même guard admin que les autres onglets (admin espace OU admin compétition).

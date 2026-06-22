# Poules — Phase 7 : Intégration ✅

## Persistance

### Migration

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

CREATE INDEX idx_competition_groups_season ON competition_groups (season_id);
```

### Repository

Nouveau trait `IGroupRepository` dans le BC `competitions` :

```rust
#[async_trait]
pub trait IGroupRepository: Send + Sync {
    async fn find_groups(&self, season_id: &str) -> Result<Vec<GroupWithTeams>, RepositoryError>;
    async fn save_assignments(&self, assignments: &[(String, String)]) -> Result<(), RepositoryError>;
    async fn reset_assignments(&self, season_id: &str) -> Result<(), RepositoryError>;
    async fn assign_team(&self, group_id: &str, team_id: &str) -> Result<(), RepositoryError>;
    async fn unassign_team(&self, team_id: &str) -> Result<(), RepositoryError>;
}
```

### Port inter-BC

Adapter dans `src/infrastructure/competitions/team_info_adapter.rs` :
- Implémente `ITeamInfoPort`
- Appelle `ITeamRepository::find_enrolled_for_season()` du BC teams
- Instancié dans `main.rs`, injecté dans le context competitions

## Handlers

### `groups_tab.rs`

- Route : `GET .../admin/groups`
- Guard admin
- HTMX → fragment seul / accès direct → page complète via `render_admin_page`

### `groups_widgets.rs`

- `GET .../admin/groups/unassigned` : charge enrolled teams (port) - assigned teams (repo) = unassigned
- `GET .../admin/groups/cards` : charge groups with teams (repo), enrichit avec infos équipe (port)

### `groups_actions.rs`

- `POST .../admin/groups/random-draw` → appelle use case → `HX-Trigger: groupsChanged`
- `POST .../admin/groups/reset` → appelle use case → `HX-Trigger: groupsChanged`
- `POST .../admin/groups/assign` → appelle use case → `HX-Trigger: groupsChanged`

## Templates

### `admin/groups.html`

Fragment assemblage :
- Actions bar (info note + boutons vider/tirage)
- Conteneur `#unassigned-pool` avec `hx-get` + `hx-trigger="load, groupsChanged from:body"`
- Conteneur `#group-cards` avec `hx-get` + `hx-trigger="load, groupsChanged from:body"`

### `admin/widgets/unassigned-pool.html`

- Chips draggables des équipes non assignées
- Compteur "X restantes"

### `admin/widgets/group-cards.html`

- Grille de group cards (auto-fill)
- Chaque card : header (nom poule + count) + body (équipes draggables + drop zone)

## CSS

`competition-admin-groups.css` : group cards, chips, drop zones, drag states.

## Tests E2E

Fichier : `tests/e2e/test_competition_admin_groups.py`

1. Accéder à l'onglet Poules → les widgets se chargent
2. Tirage aléatoire → les équipes sont distribuées dans les poules
3. Vider les poules → toutes les équipes reviennent dans le pool non assigné

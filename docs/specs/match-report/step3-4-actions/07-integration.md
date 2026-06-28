# Step 3 & 4 — Actions match — Intégration / Persistance

---

## Migration SQL

### Nouvelles colonnes sur `match_report_pre_match`

```sql
ALTER TABLE match_report_pre_match
    ADD COLUMN home_temp_players JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN away_temp_players JSONB NOT NULL DEFAULT '[]'::jsonb;
```

- `'[]'::jsonb` = aucun joueur temporaire (état initial et après reset)
- Remplis après l'enregistrement des inducements de chaque équipe

### Nouvelle table `match_report_actions`

```sql
CREATE TABLE match_report_actions (
    action_id           TEXT        PRIMARY KEY,
    match_report_id     TEXT        NOT NULL,
    team_side           TEXT        NOT NULL,   -- 'home' | 'away'
    turn_number         SMALLINT    NOT NULL,
    player_id           TEXT        NOT NULL,   -- PlayerId ou TempPlayerId
    player_type         TEXT        NOT NULL,   -- 'regular' | 'temp'
    action_json         JSONB       NOT NULL,   -- MatchActionType sérialisé
    player_display_name TEXT        NOT NULL,
    recorded_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_deleted          BOOLEAN     NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_mr_actions_mr_side
    ON match_report_actions (match_report_id, team_side)
    WHERE NOT is_deleted;
```

La suppression est un **soft delete** (`is_deleted = true`). Le widget action-log filtre `WHERE NOT is_deleted`.

---

## Projection repository — nouveaux event handlers

Tous les handlers s'exécutent dans la même transaction que le `INSERT` de l'event (règle CLAUDE.md).

### `TempPlayersInitialized`

```sql
UPDATE match_report_pre_match
SET home_temp_players = $1   -- si team_id == home_team_id
 -- ou away_temp_players = $1
WHERE match_report_id = $2
```

`$1` = liste JSON des `TempPlayer` sérialisés.

### `TempPlayersReset`

```sql
UPDATE match_report_pre_match
SET home_temp_players = '[]'::jsonb   -- si team_id == home_team_id
 -- ou away_temp_players = '[]'::jsonb
WHERE match_report_id = $1
```

### `ActionRecorded`

```sql
INSERT INTO match_report_actions
    (action_id, match_report_id, team_side, turn_number,
     player_id, player_type, action_json, player_display_name)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
```

### `ActionDeleted`

```sql
UPDATE match_report_actions
SET is_deleted = TRUE
WHERE action_id = $1
```

---

## Rehydratation — nouveaux bras dans `rehydrate()`

```rust
(Some(MatchReportState::PreMatch(pm)), MatchReportDomainEvent::TempPlayersInitialized { team_id, players }) => {
    let mut updated = pm;
    if team_id == &updated.home_team_id {
        updated.home_temp_players = players.clone();
    } else {
        updated.away_temp_players = players.clone();
    }
    updated.version += 1;
    MatchReportState::PreMatch(updated)
}

(Some(MatchReportState::PreMatch(pm)), MatchReportDomainEvent::TempPlayersReset { team_id }) => {
    let mut updated = pm;
    if team_id == &updated.home_team_id {
        updated.home_temp_players = vec![];
    } else {
        updated.away_temp_players = vec![];
    }
    updated.version += 1;
    MatchReportState::PreMatch(updated)
}

(Some(MatchReportState::PreMatch(pm)), MatchReportDomainEvent::ActionRecorded { action_id, team_side, turn, player, action, player_display_name, .. }) => {
    let mut updated = pm;
    let entry = MatchAction { id: action_id.clone(), turn: *turn, player: player.clone(), action: action.clone(), player_display_name: player_display_name.clone() };
    match team_side {
        TeamSide::Home => updated.home_actions.push(entry),
        TeamSide::Away => updated.away_actions.push(entry),
    }
    updated.version += 1;
    MatchReportState::PreMatch(updated)
}

(Some(MatchReportState::PreMatch(pm)), MatchReportDomainEvent::ActionDeleted { action_id, team_side, .. }) => {
    let mut updated = pm;
    match team_side {
        TeamSide::Home => updated.home_actions.retain(|a| &a.id != action_id),
        TeamSide::Away => updated.away_actions.retain(|a| &a.id != action_id),
    }
    updated.version += 1;
    MatchReportState::PreMatch(updated)
}
```

---

## Infrastructure adapters

### `player_data_adapter.rs` (nouveau — `src/infrastructure/match_report/`)

```rust
pub struct PlayerDataAdapter {
    player_projection_repo: Arc<dyn IPlayerProjectionRepository>,
}
```

Implémente `IPlayerDataPort` :

| Méthode | Implémentation |
|---|---|
| `count_available_players(team_id)` | `player_projection_repo.find_by_team_id(team_id)` → `len()` (tous disponibles en V1) |
| `find_player_display(player_id)` | `player_projection_repo.find_by_id(player_id)` → `"{personal_name} (#{jersey})"` ; `None` si joueur introuvable |

**Prérequis** : `IPlayerProjectionRepository` doit exposer une méthode `find_by_id` — à ajouter dans `src/app/players/ports.rs` :

```rust
async fn find_by_id(
    &self,
    player_id: &PlayerId,
) -> Result<Option<PlayerProjection>, RepositoryError>;
```

### `team_data_adapter.rs` (modifié)

Ajout de `reference_repo: Arc<dyn IReferenceRepository>` dans `TeamDataAdapter`.

Nouvelle méthode `find_journalier_position` :

```rust
async fn find_journalier_position(&self, team_id: &str) -> Option<JournalierPositionDto> {
    let team = self.team_repo.find_by_id(team_id).await.ok()??;
    let roster_id = team.roster_id.to_string();
    let ref_team = self.reference_repo.find_team_by_uid(&roster_id)?;
    let pos = ref_team.available_players.iter().find(|p| p.is_journalier)?;
    Some(JournalierPositionDto {
        position_uid:  pos.uid.clone(),
        position_name: pos.position_name.clone(),
    })
}
```

---

## Contexte BC MatchReport (`context.rs`)

Ajout du champ `player_data: Arc<dyn IPlayerDataPort>`.

Instancié dans `main.rs` :

```rust
let player_data_adapter = Arc::new(PlayerDataAdapter::new(
    Arc::clone(&players_context.projection_repo),
));
```

---

## Ports BC Players — extension

Dans `src/app/players/ports.rs`, ajouter sur `IPlayerProjectionRepository` :

```rust
async fn find_by_id(
    &self,
    player_id: &PlayerId,
) -> Result<Option<PlayerProjection>, RepositoryError>;
```

Implémentation dans `src/app/players/io/repository/projection_repository.rs`.

---

## Routes — BC MatchReport (`routes.rs`)

Nouvelles constantes de path :

```rust
pub const MATCH_REPORT_STEP4:               &str = "/app/{space_id}/match-report/{mr_id}/step4";
pub const MATCH_REPORT_STEP3_TURN_SELECTOR: &str = "/app/{space_id}/match-report/{mr_id}/step3/turn-selector";
pub const MATCH_REPORT_STEP4_TURN_SELECTOR: &str = "/app/{space_id}/match-report/{mr_id}/step4/turn-selector";
pub const MATCH_REPORT_STEP3_TEMP_PLAYERS:  &str = "/app/{space_id}/match-report/{mr_id}/step3/temp-players";
pub const MATCH_REPORT_STEP4_TEMP_PLAYERS:  &str = "/app/{space_id}/match-report/{mr_id}/step4/temp-players";
pub const MATCH_REPORT_STEP3_ACTION_PANEL:  &str = "/app/{space_id}/match-report/{mr_id}/step3/action-panel";
pub const MATCH_REPORT_STEP4_ACTION_PANEL:  &str = "/app/{space_id}/match-report/{mr_id}/step4/action-panel";
pub const MATCH_REPORT_STEP3_LOG:           &str = "/app/{space_id}/match-report/{mr_id}/step3/log";
pub const MATCH_REPORT_STEP4_LOG:           &str = "/app/{space_id}/match-report/{mr_id}/step4/log";
pub const MATCH_REPORT_STEP3_ACTIONS:       &str = "/app/{space_id}/match-report/{mr_id}/step3/actions";
pub const MATCH_REPORT_STEP4_ACTIONS:       &str = "/app/{space_id}/match-report/{mr_id}/step4/actions";
pub const MATCH_REPORT_ACTION:              &str = "/app/{space_id}/match-report/{mr_id}/actions/{action_id}";
```

Méthodes builder correspondantes sur `Routes`.

## Routes — BC Players (`routes.rs`)

```rust
pub const MATCH_PLAYER_SELECTOR: &str = "/app/{space_id}/players/teams/{team_id}/match-selector";
```

---

## Router — câblage

### `match_report/router.rs`

```rust
.route(path::MATCH_REPORT_STEP4, get(actions_step_controller::get_step))
// step3 existait déjà — le câbler avec le même handler get_step
.route(path::MATCH_REPORT_STEP3, get(actions_step_controller::get_step))
.route(path::MATCH_REPORT_STEP3_TURN_SELECTOR, get(turn_selector_widget::get))
.route(path::MATCH_REPORT_STEP4_TURN_SELECTOR, get(turn_selector_widget::get))
.route(path::MATCH_REPORT_STEP3_TEMP_PLAYERS, get(temp_player_selector_widget::get))
.route(path::MATCH_REPORT_STEP4_TEMP_PLAYERS, get(temp_player_selector_widget::get))
.route(path::MATCH_REPORT_STEP3_ACTION_PANEL, get(action_panel_widget::get))
.route(path::MATCH_REPORT_STEP4_ACTION_PANEL, get(action_panel_widget::get))
.route(path::MATCH_REPORT_STEP3_LOG, get(action_log_widget::get))
.route(path::MATCH_REPORT_STEP4_LOG, get(action_log_widget::get))
.route(path::MATCH_REPORT_STEP3_ACTIONS, post(record_action_controller::post_action))
.route(path::MATCH_REPORT_STEP4_ACTIONS, post(record_action_controller::post_action))
.route(path::MATCH_REPORT_ACTION, delete(record_action_controller::delete_action))
```

### `players/router.rs`

```rust
.route(
    path::MATCH_PLAYER_SELECTOR,
    get(widgets::match_player_selector_widget::get),
)
```

---

## `mod.rs` à mettre à jour

| Fichier | Ajout |
|---|---|
| `match_report/io/web/mod.rs` | `pub mod actions_step_controller;` `pub mod record_action_controller;` `pub mod widgets;` |
| `match_report/io/web/widgets/mod.rs` | (nouveau) `pub mod turn_selector_widget;` etc. |
| `infrastructure/match_report/mod.rs` | `pub mod player_data_adapter;` |
| `players/io/web/mod.rs` | `pub mod widgets;` |
| `players/io/web/widgets/mod.rs` | (nouveau) `pub mod match_player_selector_widget;` |

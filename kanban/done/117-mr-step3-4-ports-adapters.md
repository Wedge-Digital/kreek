# BC match_report — Ports + adapters step3-4

**Priorité : haute**
**Dépend de :** 114
**Contexte :** match_report step3-4-actions — infrastructure

## Objectif

Définir et implémenter les deux nouveaux points d'extension inter-BCs nécessaires aux use cases step3-4 : `IPlayerDataPort` (nouveau) et l'extension `find_journalier_position` sur `ITeamDataPort`.

## Conception

Cf. `docs/specs/match-report/step3-4-actions/07-integration.md`

### Nouveau port `IPlayerDataPort` (`src/app/match_report/ports.rs`)

```rust
#[async_trait]
pub trait IPlayerDataPort: Send + Sync {
    async fn count_available_players(&self, team_id: &str) -> Result<usize, String>;
    async fn find_player_display(&self, player_id: &str) -> Option<String>;
}
```

### Extension `ITeamDataPort` (`src/app/match_report/ports.rs`)

```rust
async fn find_journalier_position(&self, team_id: &str) -> Option<JournalierPositionDto>;
```

Nouveau DTO :

```rust
pub struct JournalierPositionDto {
    pub position_uid:  String,
    pub position_name: String,
}
```

### Extension `IPlayerProjectionRepository` (`src/app/players/ports.rs`)

```rust
async fn find_by_id(
    &self,
    player_id: &PlayerId,
) -> Result<Option<PlayerProjection>, RepositoryError>;
```

Implémenter dans `src/app/players/io/repository/projection_repository.rs`.

### Nouveau `player_data_adapter.rs` (`src/infrastructure/match_report/`)

```rust
pub struct PlayerDataAdapter {
    player_projection_repo: Arc<dyn IPlayerProjectionRepository>,
}
```

- `count_available_players` → `find_by_team_id(team_id)` → `.len()`
- `find_player_display` → `find_by_id(player_id)` → `"{personal_name} (#{jersey})"` ou `None`

### Extension `team_data_adapter.rs` (`src/infrastructure/match_report/`)

Ajouter `reference_repo: Arc<dyn IReferenceRepository>` dans `TeamDataAdapter::new`.

Implémentation de `find_journalier_position` :
1. `team_repo.find_by_id(team_id)` → extrait `roster_id`
2. `reference_repo.find_team_by_uid(roster_id)` → `ref_team`
3. `ref_team.available_players.iter().find(|p| p.is_journalier)` → construit `JournalierPositionDto`

### Contexte BC MatchReport (`context.rs`)

Ajouter `player_data: Arc<dyn IPlayerDataPort>`.

### `main.rs`

```rust
let player_data_adapter = Arc::new(PlayerDataAdapter::new(
    Arc::clone(&players_context.projection_repo),
));
```

Passer `reference_repo` à `TeamDataAdapter::new` (si pas déjà fait).

## Checklist

- [ ] `IPlayerDataPort` trait + `JournalierPositionDto` dans `ports.rs`
- [ ] `find_journalier_position` sur `ITeamDataPort` + `JournalierPositionDto`
- [ ] `find_by_id` sur `IPlayerProjectionRepository` + implémentation SQL
- [ ] `player_data_adapter.rs` — `count_available_players` + `find_player_display`
- [ ] `TeamDataAdapter` — ajout `reference_repo` + `find_journalier_position`
- [ ] `context.rs` — champ `player_data`
- [ ] `main.rs` — instanciation de `PlayerDataAdapter`
- [ ] `mod.rs` infrastructure/match_report — `pub mod player_data_adapter;`

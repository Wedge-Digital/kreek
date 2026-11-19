# BC match_report — Ports + Adapters mercenaires

**Priorité : haute**
**Dépend de :** 124
**Contexte :** `docs/specs/match-report/step2-mercenaires/07-integration.md`

## Objectif

Ajouter les deux nouveaux DTOs et les deux nouvelles méthodes de port nécessaires au widget mercenary-selector, puis les implémenter dans les adapters d'infrastructure.

## Conception

### 1. ports.rs — src/app/match_report/ports.rs

Ajouter après `JournalierPositionDto` :

```rust
pub struct RosterPositionDto {
    pub position_uid:  String,
    pub position_name: String,
    pub base_cost:     u32,
    pub max_qty:       u8,
    pub is_journalier: bool,
}

pub struct PositionCountDto {
    pub position_uid: String,
    pub count:        u8,
}
```

Ajouter dans `ITeamDataPort` :

```rust
async fn find_roster_positions(&self, team_id: &str) -> Vec<RosterPositionDto>;
```

Ajouter dans `IPlayerDataPort` :

```rust
async fn find_player_counts_by_position(&self, team_id: &str) -> Vec<PositionCountDto>;
```

### 2. ref_team_data_adapter.rs — src/infrastructure/match_report/ref_team_data_adapter.rs

```rust
async fn find_roster_positions(&self, team_id: &str) -> Vec<RosterPositionDto> {
    let Ok(Some(team)) = self.team_repo.find_by_id(team_id).await else { return vec![]; };
    let roster_id = team.roster_id.to_string();
    let Some(ref_team) = self.reference_repo.find_team_by_uid(&roster_id) else { return vec![]; };
    ref_team
        .available_players
        .iter()
        .map(|p| RosterPositionDto {
            position_uid:  p.uid.clone(),
            position_name: p.position_name.clone(),
            base_cost:     p.cost,
            max_qty:       p.max_quantity,
            is_journalier: p.is_journalier,
        })
        .collect()
}
```

Les champs de `PlayerPosition` (references domain) : `uid`, `position_name`, `cost`, `max_quantity`, `is_journalier`.

### 3. player_data_adapter.rs — src/infrastructure/match_report/player_data_adapter.rs

```rust
async fn find_player_counts_by_position(&self, team_id: &str) -> Vec<PositionCountDto> {
    use crate::app::players::domain::player::TeamId;
    let tid = TeamId(team_id.to_string());
    let players = self
        .player_projection_repo
        .find_by_team_id(&tid)
        .await
        .unwrap_or_default();
    let mut counts: std::collections::HashMap<String, u8> = std::collections::HashMap::new();
    for p in &players {
        *counts.entry(p.roster_line_id.clone()).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .map(|(position_uid, count)| PositionCountDto { position_uid, count })
        .collect()
}
```

`PlayerProjection.roster_line_id` est le `position_uid` de la position de référence.

## Checklist

- [ ] `RosterPositionDto` et `PositionCountDto` ajoutés à `ports.rs`
- [ ] `find_roster_positions` ajouté au trait `ITeamDataPort`
- [ ] `find_player_counts_by_position` ajouté au trait `IPlayerDataPort`
- [ ] `RefTeamDataAdapter::find_roster_positions` implémenté
- [ ] `PlayerDataAdapter::find_player_counts_by_position` implémenté
- [ ] `cargo build` passe (trait implémenté intégralement)

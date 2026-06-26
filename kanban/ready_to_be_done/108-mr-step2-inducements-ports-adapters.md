# BC match_report — Ports extension + infrastructure adapters

**Priorité : haute**
**Dépend de :** 105
**Contexte :** match_report step2-inducements — ports inter-BC

## Objectif

Étendre les ports `ITeamDataPort` et `ICompetitionDataPort` avec les nouvelles méthodes nécessaires aux use cases inducements, et implémenter ces méthodes dans les adapters infrastructure.

## Conception

Cf. `docs/specs/match-report/step2-inducements/04-dtos.md`, `07-integration.md`

### `ITeamDataPort` (`ports.rs`)

```rust
// Existant — ajouter roster_id
pub struct TeamInfoDto {
    pub team_name:   String,
    pub coach_name:  String,
    pub roster_name: String,
    pub roster_id:   String,  // NOUVEAU
}

// Nouvelles méthodes
async fn find_team_value(&self, team_id: &str) -> Option<u32>;
async fn find_team_treasury(&self, team_id: &str) -> Option<u32>;
```

### `ICompetitionDataPort` (`ports.rs`)

```rust
pub struct TierRulesDto {
    pub allowed_inducements:  Vec<InducementSpecDto>,
    pub allowed_star_players: Vec<InducementSpecDto>,
}

pub struct InducementSpecDto {
    pub uid:       String,
    pub max_qty:   u8,
    pub unit_cost: u32,
}

async fn find_tier_rules_for_roster(
    &self,
    season_id: &str,
    roster_id: &str,
) -> Option<TierRulesDto>;
```

### `team_data_adapter.rs` (`src/infrastructure/match_report/`)

- `find_team_value` : requête SQL sur `teams_projection.team_value_kpo`
- `find_team_treasury` : requête SQL sur `teams_projection.treasury_kpo`
- Étendre `find_team_info` pour retourner `roster_id`

### `competition_data_adapter.rs` (`src/infrastructure/match_report/`)

- `find_tier_rules_for_roster` : en deux étapes
  1. SQL sur BC Competitions → UIDs autorisés pour `(season_id, roster_id)`
  2. Lookup in-memory BC References → enrichit chaque UID avec `max_qty` + `unit_cost`

## Checklist

- [ ] `TeamInfoDto.roster_id` ajouté
- [ ] `ITeamDataPort::find_team_value()` défini dans le trait
- [ ] `ITeamDataPort::find_team_treasury()` défini dans le trait
- [ ] `TierRulesDto` + `InducementSpecDto` dans `ports.rs`
- [ ] `ICompetitionDataPort::find_tier_rules_for_roster()` défini dans le trait
- [ ] `team_data_adapter` : implémente `find_team_value`, `find_team_treasury`, étend `find_team_info`
- [ ] `competition_data_adapter` : implémente `find_tier_rules_for_roster` (SQL + enrichissement References)
- [ ] Tests d'intégration sur les deux adapters

# Poules — Phase 5 : Use cases ✅

## Use case : `random_draw.rs`

Fichier : `src/app/competitions/use_cases/admin/random_draw.rs`

### Signature

```rust
pub async fn execute(
    season_id: &SeasonId,
    group_repo: &dyn IGroupRepository,
    team_port: &dyn ITeamInfoPort,
) -> Result<(), DrawError>
```

### Orchestration

1. Charger les équipes enrolled depuis le port `ITeamInfoPort`
2. Charger les groupes de la saison depuis `group_repo`
3. Si aucun groupe n'existe → erreur
4. Distribuer aléatoirement les équipes entre les groupes (round-robin shuffled)
5. Persister les assignations

### Erreurs

```rust
pub enum DrawError {
    NoGroups,
    NoTeams,
    Repository(String),
}
```

## Use case : `reset_groups.rs`

Fichier : `src/app/competitions/use_cases/admin/reset_groups.rs`

### Signature

```rust
pub async fn execute(
    season_id: &SeasonId,
    group_repo: &dyn IGroupRepository,
) -> Result<(), ResetError>
```

### Orchestration

1. Supprimer toutes les assignations team → group pour cette saison
2. Les groupes eux-mêmes sont conservés

## Use case : `assign_team_to_group.rs`

Fichier : `src/app/competitions/use_cases/admin/assign_team_to_group.rs`

### Signature

```rust
pub async fn execute(
    team_id: &str,
    group_id: &str,
    group_repo: &dyn IGroupRepository,
) -> Result<(), AssignError>
```

### Orchestration

1. Retirer l'équipe de son groupe actuel (si assignée)
2. Assigner l'équipe au nouveau groupe
3. Persister

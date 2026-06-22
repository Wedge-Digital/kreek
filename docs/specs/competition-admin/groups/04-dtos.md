# Poules — Phase 4 : Contrats de données ✅

## Fragment onglet — BC `competitions`

```rust
#[derive(Template)]
#[template(path = "admin/groups.html")]
pub struct GroupsTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub group_count: usize,
    pub team_count: usize,
}
```

## Widget unassigned pool

```rust
#[derive(Template)]
#[template(path = "admin/widgets/unassigned-pool.html")]
pub struct UnassignedPoolTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub teams: Vec<UnassignedTeamVm>,
}

pub struct UnassignedTeamVm {
    pub team_id: String,
    pub team_name: String,
    pub team_initials: String,
}
```

## Widget group cards

```rust
#[derive(Template)]
#[template(path = "admin/widgets/group-cards.html")]
pub struct GroupCardsTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub groups: Vec<GroupVm>,
}

pub struct GroupVm {
    pub group_id: String,
    pub name: String,
    pub teams: Vec<GroupTeamVm>,
}

pub struct GroupTeamVm {
    pub team_id: String,
    pub team_name: String,
    pub team_initials: String,
    pub coach_name: String,
    pub roster_name: String,
}
```

## DTOs d'entrée

### Assign

```rust
#[derive(Deserialize)]
pub struct AssignTeamBody {
    pub team_id: String,
    pub group_id: String,
}
```

### Random draw et reset

Pas de body — les IDs sont dans le path (competition_id, season_id).

## Port inter-BC

```rust
pub struct TeamInfoDto {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
}

#[async_trait]
pub trait ITeamInfoPort: Send + Sync {
    async fn find_enrolled_teams(&self, season_id: &str) -> Result<Vec<TeamInfoDto>, String>;
}
```

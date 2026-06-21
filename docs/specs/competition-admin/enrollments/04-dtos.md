# Inscriptions — Phase 4 : Contrats de données ✅

## Fragment onglet — BC `competitions`

Assemblage pur, pas de VMs. Les données sont chargées par les widgets.

```rust
#[derive(Template)]
#[template(path = "admin/enrollments.html")]
pub struct EnrollmentsTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
}
```

## Widget pending — BC `teams`

```rust
#[derive(Template)]
#[template(path = "widgets/pending-enrollments.html")]
pub struct PendingEnrollmentWidgetTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub teams: Vec<PendingTeamVm>,
}

pub struct PendingTeamVm {
    pub team_id: String,
    pub team_name: String,
    pub team_initials: String,
    pub coach_name: String,
    pub roster_name: String,
    pub tier_name: String,
}
```

## Widget enrolled — BC `teams`

```rust
#[derive(Template)]
#[template(path = "widgets/enrolled-teams.html")]
pub struct EnrolledTeamsWidgetTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub teams: Vec<EnrolledTeamVm>,
}

pub struct EnrolledTeamVm {
    pub team_id: String,
    pub team_name: String,
    pub team_initials: String,
    pub coach_name: String,
    pub roster_name: String,
    pub tier_name: String,
    pub group_name: Option<String>,
}
```

## DTOs d'entrée

### Widgets (GET)

```rust
#[derive(Deserialize)]
pub struct EnrollmentWidgetParams {
    pub competition_id: String,
    pub season_id: String,
}
```

### Actions (POST)

- `approve`, `reject`, `dismiss` : pas de body, `team_id` dans le path
- `approve-all` : pas de body, `competition_id` et `season_id` en query params

```rust
#[derive(Deserialize)]
pub struct ApproveAllParams {
    pub competition_id: String,
    pub season_id: String,
}
```

## Domain events — BC `teams`

```rust
pub enum TeamDomainEvent {
    TeamEnrollmentApproved {
        team_id: TeamId,
        competition_id: String,
        season_id: String,
        team_name: String,
        coach_name: String,
    },
    TeamEnrollmentRejected {
        team_id: TeamId,
        competition_id: String,
        season_id: String,
        team_name: String,
    },
    TeamDismissed {
        team_id: TeamId,
        competition_id: String,
        season_id: String,
        team_name: String,
    },
}
```

# Calendrier — Phase 4 : Contrats de données ✅

## Fragment onglet — assemblage pur

```rust
#[derive(Template)]
#[template(path = "admin/schedule.html")]
pub struct ScheduleTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
}
```

## Widget sidebar

```rust
#[derive(Template)]
#[template(path = "admin/widgets/schedule-sidebar.html")]
pub struct RoundSidebarTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub rounds: Vec<RoundItemVm>,
}

pub struct RoundItemVm {
    pub round_id: String,
    pub name: String,
    pub date_label: String,     // formaté : "12 — 26 sept." ou "15 oct."
    pub day_type: String,       // "match" | "rest"
    pub status: String,         // "upcoming" | "in-progress" | "validated"
}
```

## Widget round detail

```rust
#[derive(Template)]
#[template(path = "admin/widgets/schedule-round-detail.html")]
pub struct RoundDetailTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub round: RoundDetailVm,
    pub enrolled_teams: Vec<TeamOptionVm>,
}

pub struct RoundDetailVm {
    pub round_id: String,
    pub name: String,
    pub day_type: String,       // "fixed_date" | "time_frame" | "rest"
    pub date_start: String,     // formaté ISO pour <input type="date">
    pub date_end: String,
    pub fixture_count: usize,
    pub fixtures: Vec<FixtureVm>,
}

pub struct FixtureVm {
    pub fixture_id: String,
    pub home_team_name: String,
    pub away_team_name: String,
    pub group_name: Option<String>,
}

pub struct TeamOptionVm {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
}
```

## DTOs d'entrée

```rust
#[derive(Deserialize)]
pub struct AddRoundBody {
    pub name: String,
    pub day_type: String,           // "fixed_date" | "time_frame"
    pub date_start: Option<String>, // ISO "2025-10-24"
    pub date_end: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateRoundBody {
    pub name: String,
    pub day_type: String,
    pub date_start: Option<String>,
    pub date_end: Option<String>,
}

#[derive(Deserialize)]
pub struct AddMatchBody {
    pub home_team_id: String,
    pub away_team_id: String,
}
```

Les handlers parsent les dates `String` vers `time::Date` avant de les passer au domaine.

## Types temporels

- **DTOs d'entrée** (body HTTP) : `String` — le handler parse
- **Agrégat MatchDay** (domaine) : `Option<time::Date>`
- **VMs (sortie template)** : `String` — le handler formate pour l'affichage

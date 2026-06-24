# Step 1 — Sélection du match : Contrats de données

## DTO d'entrée — POST formulaire

```rust
#[derive(Deserialize)]
pub struct CreateMatchReportForm {
    pub competition_id: String,
    pub season_id: String,
    pub round_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
}
```

## Commande applicative

```rust
pub struct CreateMatchReportCommand {
    pub space_id: SpaceId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub created_by: CoachId,
}
```

## VMs de sortie — Page step1

```rust
#[derive(Template)]
#[template(path = "match_report/match-selection.html")]
pub struct MatchSelectionTemplate {
    pub competitions: Vec<CompetitionOptionVm>,
    pub seasons: Vec<SeasonOptionVm>,
    pub rounds: Vec<RoundOptionVm>,
    pub teams: Vec<TeamOptionVm>,
    pub selected: Option<SelectedMatchVm>,
    pub user_role: UserRoleVm,
    pub routes: AppRoutes,
}

pub struct CompetitionOptionVm {
    pub id: String,
    pub name: String,
    pub selected: bool,
}

pub struct SeasonOptionVm {
    pub id: String,
    pub name: String,
    pub selected: bool,
}

pub struct RoundOptionVm {
    pub id: String,
    pub name: String,
    pub dates: String,
    pub selected: bool,
}

pub struct TeamOptionVm {
    pub id: String,
    pub name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub tv: String,
    pub logo_url: Option<String>,
    pub is_own_team: bool,
}

pub struct SelectedMatchVm {
    pub match_report_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub round_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
}

pub enum UserRoleVm {
    Admin,
    Coach,
}
```

## VMs de sortie — Fragments cascade

```rust
#[derive(Template)]
#[template(path = "match_report/fragments/season-options.html")]
pub struct SeasonOptionsFragment {
    pub seasons: Vec<SeasonOptionVm>,
}

#[derive(Template)]
#[template(path = "match_report/fragments/round-options.html")]
pub struct RoundOptionsFragment {
    pub rounds: Vec<RoundOptionVm>,
}

#[derive(Template)]
#[template(path = "match_report/fragments/team-options.html")]
pub struct TeamOptionsFragment {
    pub teams: Vec<TeamOptionVm>,
    pub user_role: UserRoleVm,
}
```

## DTOs de port

Définis dans `ports.rs` (cf. 03-back.md) : `CompetitionOptionDto`, `SeasonOptionDto`, `RoundOptionDto`, `EnrolledTeamDto`. Les VMs sont construits directement depuis ces DTOs dans le handler (lecture pure, pas de domain service).

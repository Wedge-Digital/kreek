# Step 1 — Sélection du match : Architecture back

## BC propriétaire

Nouveau BC **`match_report`** — structure from scratch dans `src/app/match_report/`.

## Structure du BC

```
src/app/match_report/
├── mod.rs
├── context.rs                             ← MatchReportContext
├── routes.rs                              ← Routes du BC
├── router.rs                              ← Axum router
├── ports.rs                               ← ICompetitionDataPort + ITeamDataPort
├── domain/
│   ├── mod.rs
│   ├── match_report.rs                    ← Agrégat MatchReport (event-sourcé)
│   └── match_report_repository_port.rs
├── io/
│   ├── mod.rs
│   ├── web/
│   │   ├── mod.rs
│   │   ├── match_selection_controller.rs  ← Handlers step1
│   │   └── templates/
│   │       ├── match-selection.html       ← Page complète step1
│   │       └── fragments/
│   │           ├── season-options.html    ← Fragment select saisons
│   │           ├── round-options.html     ← Fragment select journées
│   │           └── team-options.html      ← Fragment selects équipes
│   ├── repository/
│   │   ├── mod.rs
│   │   └── match_report_repository.rs
│   └── app_events/
│       ├── mod.rs
│       └── pairing_created_listener.rs    ← Écoute l'app event PairingCreated
└── use_cases/
    ├── mod.rs
    └── create_match_report_use_case.rs
```

## Routes

| Route | Méthode | Handler | Description |
|-------|---------|---------|-------------|
| `/app/{space_id}/match-report/new` | GET | `new_match_report` | Formulaire vierge |
| `/app/{space_id}/match-report/{match_report_id}` | GET | `edit_match_report` | Rapport existant (pré-rempli) |
| `/app/{space_id}/match-report/new/seasons` | GET | `seasons_fragment` | Options saisons (cascade) |
| `/app/{space_id}/match-report/new/rounds` | GET | `rounds_fragment` | Options journées (cascade) |
| `/app/{space_id}/match-report/new/teams` | GET | `teams_fragment` | Options équipes (cascade) |
| `/app/{space_id}/match-report/new` | POST | `create_match_report` | Crée le rapport, redirect step2 |
| `/app/{space_id}/match-report/{match_report_id}` | POST | `update_match_selection` | Met à jour sélection, redirect step2 |

## Ports nécessaires (`match_report/ports.rs`)

### Port 1 — `ICompetitionDataPort`

Données du BC competitions nécessaires pour les selects en cascade.

```rust
pub struct CompetitionOptionDto {
    pub competition_id: String,
    pub name: String,
}

pub struct SeasonOptionDto {
    pub season_id: String,
    pub name: String,
}

pub struct RoundOptionDto {
    pub round_id: String,
    pub name: String,
    pub date_start: Option<String>,
    pub date_end: Option<String>,
}

#[async_trait]
pub trait ICompetitionDataPort: Send + Sync {
    async fn list_competitions_with_active_season(
        &self, space_id: &str
    ) -> Result<Vec<CompetitionOptionDto>, String>;

    async fn list_seasons(
        &self, competition_id: &str
    ) -> Result<Vec<SeasonOptionDto>, String>;

    async fn list_rounds(
        &self, season_id: &str
    ) -> Result<Vec<RoundOptionDto>, String>;

    async fn is_competition_admin(
        &self, competition_id: &str, coach_id: &str
    ) -> Result<bool, String>;
}
```

### Port 2 — `ITeamDataPort`

Données du BC teams : équipes enrolled avec TV, coach, roster.

```rust
pub struct EnrolledTeamDto {
    pub team_id: String,
    pub team_name: String,
    pub coach_id: String,
    pub coach_name: String,
    pub roster_name: String,
    pub team_value: u32,
    pub logo_url: Option<String>,
    pub game_phase: String,
}

#[async_trait]
pub trait ITeamDataPort: Send + Sync {
    async fn list_enrolled_teams(
        &self, season_id: &str
    ) -> Result<Vec<EnrolledTeamDto>, String>;

    async fn is_team_ready_to_play(
        &self, team_id: &str
    ) -> Result<bool, String>;
}
```

## Adapters (`src/infrastructure/match_report/`)

```
src/infrastructure/match_report/
├── mod.rs
├── competition_data_adapter.rs   ← Implémente ICompetitionDataPort via repos du BC competitions
└── team_data_adapter.rs          ← Implémente ITeamDataPort via ITeamRepository du BC teams
```

`TeamDataAdapter` s'appuie sur `ITeamRepository::find_enrolled_for_season()` (existant) avec enrichissement du DTO (`coach_id`, `team_value`).

`CompetitionDataAdapter` s'appuie sur `ICompetitionRepository`, `ISeasonRepository`, `IMatchDayRepository` (existants).

## Contrôle d'accès (dans le handler)

Le handler détermine le rôle de l'utilisateur courant pour adapter les selects :

1. Vérifier si admin espace (via `AuthSession` + données espace)
2. Sinon vérifier si admin compétition (via `ICompetitionDataPort::is_competition_admin`)
3. Sinon : coach lambda — filtrer les compétitions/saisons à celles où il a une équipe enrolled

Le rôle est transmis au template pour conditionner l'affichage des selects :
- **Admin** : deux selects d'équipe libres
- **Coach** : un select "mon équipe" (restreint à ses enrolled) + un select "adversaire" (toutes les enrolled)

## App event : PairingCreated

### Émetteur

BC competitions — nouvel app event à ajouter dans `shared_kernel/app_events/competitions_app_events.rs`.

```rust
PairingCreated {
    event_id: String,
    pairing_id: String,
    season_id: String,
    round_id: String,
    home_team_id: String,
    away_team_id: String,
    space_id: String,
}
```

### Listener

`match_report/io/app_events/pairing_created_listener.rs` — appelle `CreateMatchReportUseCase` pour créer automatiquement un match report en phase "selection" pré-rempli.

## Domain service

Pas nécessaire pour step1 — les données des ports sont des DTOs de lecture pour des selects, transformés directement en VMs dans le handler.

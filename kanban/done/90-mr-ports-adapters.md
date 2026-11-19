# BC match_report — Ports + adapters

**Priorité : haute**
**Dépend de :** 87
**Contexte :** match_report step1, couche infrastructure

## Objectif

Définir les ports `ICompetitionDataPort` et `ITeamDataPort` dans le BC match_report, et implémenter les adapters dans `src/infrastructure/match_report/`.

## Conception

Cf. `docs/specs/match-report/step1-selection/03-back.md`

### Fichiers

```
src/app/match_report/
└── ports.rs                                    ← traits + DTOs

src/infrastructure/match_report/
├── mod.rs
├── competition_data_adapter.rs                 ← ICompetitionDataPort via repos BC competitions
└── team_data_adapter.rs                        ← ITeamDataPort via ITeamRepository BC teams
```

### ICompetitionDataPort

- `list_competitions_with_active_season(space_id)` → `Vec<CompetitionOptionDto>`
- `list_seasons(competition_id)` → `Vec<SeasonOptionDto>` (ordre anti-chronologique)
- `list_rounds(season_id)` → `Vec<RoundOptionDto>`
- `is_competition_admin(competition_id, coach_id)` → `bool`

### ITeamDataPort

- `list_enrolled_teams(season_id)` → `Vec<EnrolledTeamDto>` (avec `game_phase`)
- `is_team_ready_to_play(team_id)` → `bool`

### Adapters

- `CompetitionDataAdapter` : reçoit `Arc<dyn ICompetitionRepository>`, `Arc<dyn ISeasonRepository>`, `Arc<dyn IMatchDayRepository>` du BC competitions
- `TeamDataAdapter` : reçoit `Arc<dyn ITeamRepository>` du BC teams. Proche du `TeamInfoAdapter` existant dans `infrastructure/competitions/`, enrichi avec `game_phase` et `coach_id`

## Checklist

- [ ] `ports.rs` : traits `ICompetitionDataPort` + `ITeamDataPort` avec DTOs
- [ ] `competition_data_adapter.rs` : implémentation via repos BC competitions
- [ ] `team_data_adapter.rs` : implémentation via `ITeamRepository`
- [ ] Ajouter `game_phase` et `coach_id` aux requêtes de la projection teams si manquants
- [ ] `cargo check` passe

# MR-RECAP-03 — Ports & adapters

## Objectif

Étendre `ICompetitionDataPort` (`find_round_context`), créer `ICoachDataPort` et
`ISppCalculatorPort` dans le BC `match_report`, et implémenter les 3 adapters correspondants.

## Dépendances

145 — `spp_calculator::domain::calculator::calculate()` doit exister pour que
`SppCalculatorAdapter` puisse l'appeler.

## Conception

Voir `docs/specs/match-report/recap/04-dtos.md` (section « DTOs de port ») et
`docs/specs/match-report/recap/07-integration.md`.

## Fichiers impactés

- `src/app/match_report/ports.rs`
- `src/infrastructure/match_report/competition_data_adapter.rs`
- `src/infrastructure/match_report/coach_data_adapter.rs` (nouveau)
- `src/infrastructure/match_report/spp_calculator_adapter.rs` (nouveau)

## Checklist

### `ICompetitionDataPort` (étendu)
- [ ] `find_round_context(&self, season_id: &str, round_id: &str) -> Option<RoundContextDto>`
- [ ] `RoundContextDto { competition_name, season_name, round_name }`
- [ ] Implémentation dans `competition_data_adapter.rs` — dégradation gracieuse (`None`) si une étape échoue

### `ICoachDataPort` (nouveau)
- [ ] Trait `find_coach_name(&self, coach_id: &str) -> Option<String>`
- [ ] `CoachDataAdapter` — appelle `ISpaceUserCacheRepository::find_user_by_id` (BC `spaces`, existant)

### `ISppCalculatorPort` (nouveau)
- [ ] Trait `calculate_match_spp(&self, home_actions: &[MatchAction], away_actions: &[MatchAction], home_roster_id: &str, away_roster_id: &str) -> SppMatchResult`
- [ ] `SppMatchResult { home: Vec<PlayerSppDto>, away: Vec<PlayerSppDto> }`, `PlayerSppDto { action_player: ActionPlayer, spp: u8 }`
- [ ] `SppCalculatorAdapter` — traduit `MatchAction` → entrées `spp_calculator`, appelle `calculate()`, retraduit le résultat

### Build
- [ ] Compiler sans erreur (`cargo build`)

# BC match_report — shared_kernel : VOs + MatchReporting

**Priorité : haute**
**Dépend de :** —
**Contexte :** match_report step1, préparation shared_kernel

## Objectif

Ajouter les value objects partagés nécessaires au BC match_report et le nouveau variant `MatchReporting` dans `GamePhase` du BC teams.

## Conception

### Nouveaux VOs dans `src/app/shared_kernel/common_types.rs`

```rust
pub type MatchReportId = EntityId;
pub type RoundId = EntityId;
pub type PairingId = EntityId;
```

### Nouveau variant GamePhase dans `src/app/teams/domain/team.rs`

```rust
pub enum GamePhase {
    ReadyToPlay,
    MatchReporting,  // ← nouveau
    PlayerImprovement,
    Recruitment,
    Dismissals,
    TemporaryRetirement,
    OffSeason,
}
```

Ajouter le handling dans `apply()` et `type_name()` pour un futur event `MatchReportingStarted`.

### Projection teams

Mettre à jour `update_projection_in_tx()` pour gérer le nouveau variant (si le pattern de la projection le nécessite).

## Checklist

- [ ] Ajouter `MatchReportId`, `RoundId`, `PairingId` dans `common_types.rs`
- [ ] Ajouter `MatchReporting` dans `GamePhase`
- [ ] Ajouter `MatchReportingStarted` domain event dans `TeamDomainEvent`
- [ ] Implémenter `apply()` pour `MatchReportingStarted` (transition `ReadyToPlay` → `MatchReporting`)
- [ ] Mettre à jour la projection teams si nécessaire
- [ ] `cargo check` passe
- [ ] Tests unitaires : `start_match_reporting` sur une team `ReadyToPlay` → OK, sur une team dans une autre phase → erreur

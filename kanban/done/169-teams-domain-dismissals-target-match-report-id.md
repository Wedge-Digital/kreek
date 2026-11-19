# BC `teams` — Domaine : cible de `DismissalsPhaseValidated` + `current_match_report_id`

**Priorité : haute**
**Dépend de :** rien (domaine pur)
**Contexte :** `teams/domain` — agrégat `Team`

## Objectif

Deux ajustements ponctuels à l'agrégat `Team`, préalables à la gestion du
bandeau d'état contextuel de la page de détail d'équipe. Spec complète :
`docs/specs/team-state-management/team-detail/02-07-conception.md` (Phase 6).

---

## Conception

### 1. `DismissalsPhaseValidated` cible directement `ReadyToPlay`

`apply()` (`domain/team.rs`) : la branche `DismissalsPhaseValidated`
transitionne aujourd'hui vers `GamePhase::TemporaryRetirement`. Simplification
temporaire tant que la carte 39 (retraite temporaire) n'est pas développée :

```rust
TeamDomainEvent::DismissalsPhaseValidated => {
    self.game_phase = Some(GamePhase::ReadyToPlay);  // était TemporaryRetirement
}
```

À revisiter (réintroduire `TemporaryRetirement` comme cible) quand la carte 39
sera livrée.

### 2. Nouveau champ `current_match_report_id`

```rust
pub struct Team {
    // ...champs existants...
    pub current_match_report_id: Option<MatchReportId>,
}
```

```rust
TeamDomainEvent::MatchReportingStarted { match_report_id } => {
    self.game_phase = Some(GamePhase::MatchReporting);
    self.current_match_report_id = Some(*match_report_id);
}
TeamDomainEvent::PostMatchSequenceStarted { .. } => {
    // ...effets existants inchangés...
    self.current_match_report_id = None;
}
```

Initialiser à `None` dans `Default for Team` et dans la branche `TeamCreated`
de `apply()`.

Aucun nouveau `TeamDomainEvent`, aucun nouveau `DomainError` — uniquement des
lectures de champs déjà portés par des events existants, et une correction de
cible de transition.

---

## Checklist

- [ ] `DismissalsPhaseValidated` → `GamePhase::ReadyToPlay` dans `apply()`
- [ ] Champ `current_match_report_id: Option<MatchReportId>` ajouté à `Team`
- [ ] `apply()` : peuplé sur `MatchReportingStarted`, vidé sur `PostMatchSequenceStarted`
- [ ] `Default for Team` et branche `TeamCreated` initialisent `current_match_report_id` à `None`
- [ ] Test `phase_sequence_advances_correctly` mis à jour (`Dismissals → ReadyToPlay`)
- [ ] Nouveau test : `MatchReportingStarted` peuple `current_match_report_id`
- [ ] Nouveau test : `PostMatchSequenceStarted` vide `current_match_report_id`

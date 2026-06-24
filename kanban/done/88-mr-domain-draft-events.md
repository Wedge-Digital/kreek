# BC match_report — Domaine : MatchReportDraft + events + rehydrate

**Priorité : haute**
**Dépend de :** 87
**Contexte :** match_report step1, couche domaine

## Objectif

Implémenter l'agrégat `MatchReportDraft`, les domain events step1, les value objects locaux, les erreurs domaine, la fonction `rehydrate()` et les tests unitaires.

## Conception

Cf. `docs/specs/match-report/step1-selection/06-domaine.md`

### Fichiers à créer

```
src/app/match_report/
├── domain/
│   ├── mod.rs
│   ├── match_report_draft.rs      ← struct MatchReportDraft + méthodes
│   ├── match_report_state.rs      ← enum MatchReportState + rehydrate()
│   ├── events.rs                  ← enum MatchReportDomainEvent
│   ├── value_objects.rs           ← MatchReportOrigin
│   └── error.rs                   ← DomainError
```

### MatchReportDraft

- `create()` → factory, valide `home != away`, retourne `(Self, MatchReportCreated)`
- `update_selection()` → valide `home != away`, retourne `(Self, SelectionUpdated)`
- `confirm_selection()` → consomme self (move), retourne `(MatchReportPreMatch, SelectionConfirmed)` — le type `MatchReportPreMatch` sera un placeholder pour l'instant (struct vide qui encapsule le Draft)

### rehydrate()

Fonction globale qui parcourt les events et reconstruit le `MatchReportState` typé.

### Tests unitaires

- `create_with_same_team_fails`
- `create_produces_created_event_and_draft`
- `update_selection_with_same_team_fails`
- `update_selection_produces_updated_draft`
- `confirm_selection_returns_prematch`
- `rehydrate_created_returns_draft`
- `rehydrate_created_then_updated_returns_draft`
- `rehydrate_created_then_confirmed_returns_prematch`
- `rehydrate_empty_stream_fails`
- `rehydrate_invalid_sequence_fails`

## Checklist

- [ ] `match_report_draft.rs` : struct + `create()` + `update_selection()` + `confirm_selection()`
- [ ] `events.rs` : `MatchReportDomainEvent` avec les 3 variants step1
- [ ] `value_objects.rs` : `MatchReportOrigin`
- [ ] `error.rs` : `DomainError` (SameTeam, InvalidEventSequence, EmptyEventStream)
- [ ] `match_report_state.rs` : enum `MatchReportState` + `rehydrate()`
- [ ] Placeholder `MatchReportPreMatch` (struct minimale pour la transition)
- [ ] 10 tests unitaires
- [ ] `cargo test` passe

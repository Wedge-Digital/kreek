# Inscriptions — Phase 6 : Domaine ✅

## Modifications sur l'agrégat Team — BC `teams`

Fichier : `src/app/teams/domain/team.rs`

### Nouvel état : `Rejected`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParticipationStatus {
    PendingEnrollment,
    Enrolled,
    Dismissed,
    Rejected,  // nouveau
}
```

### Nouvel événement : `TeamEnrollmentRejected`

```rust
TeamEnrollmentRejected {
    competition_id: String,
    season_id: String,
},
```

Ajouter le variant dans `event_type()` et les autres méthodes de matching de `TeamDomainEvent`.

### Nouvelle commande : `reject_enrollment()`

```rust
pub fn reject_enrollment(&self) -> Result<TeamDomainEvent, DomainError> {
    match self.participation_status {
        ParticipationStatus::PendingEnrollment => Ok(TeamDomainEvent::TeamEnrollmentRejected {
            competition_id: self.competition_id.clone().unwrap_or_default(),
            season_id: self.season_id.clone().unwrap_or_default(),
        }),
        _ => Err(DomainError::InvalidTransition {
            from: self.participation_status.clone(),
            to: ParticipationStatus::Rejected,
        }),
    }
}
```

### Apply pour `TeamEnrollmentRejected`

```rust
TeamDomainEvent::TeamEnrollmentRejected { .. } => {
    self.participation_status = ParticipationStatus::Rejected;
}
```

### Règles métier existantes (vérifiées)

- `enroll()` : seul `PendingEnrollment` → `Enrolled` est autorisé (déjà implémenté)
- `dismiss()` : seul `Enrolled` → `Dismissed` est autorisé (déjà implémenté)
- `reject_enrollment()` : seul `PendingEnrollment` → `Rejected` est autorisé (nouveau)

## Tests unitaires requis

Fichier : `src/app/teams/domain/team.rs` (module `#[cfg(test)]`)

### Transitions valides

1. `enroll` sur PendingEnrollment → produit `TeamEnrolled`, apply → status == Enrolled
2. `reject_enrollment` sur PendingEnrollment → produit `TeamEnrollmentRejected`, apply → status == Rejected
3. `dismiss` sur Enrolled → produit `TeamDismissed`, apply → status == Dismissed

### Transitions invalides

4. `enroll` sur Enrolled → erreur `InvalidTransition`
5. `enroll` sur Rejected → erreur `InvalidTransition`
6. `enroll` sur Dismissed → erreur `InvalidTransition`
7. `reject_enrollment` sur Enrolled → erreur `InvalidTransition`
8. `reject_enrollment` sur Rejected → erreur `InvalidTransition`
9. `dismiss` sur PendingEnrollment → erreur `InvalidTransition`
10. `dismiss` sur Dismissed → erreur `AlreadyDismissed`

### Hydratation (replay)

11. `hydrate([TeamCreated, TeamEnrolled])` → status == Enrolled
12. `hydrate([TeamCreated, TeamEnrollmentRejected])` → status == Rejected
13. `hydrate([TeamCreated, TeamEnrolled, TeamDismissed])` → status == Dismissed

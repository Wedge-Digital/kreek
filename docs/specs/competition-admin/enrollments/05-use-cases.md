# Inscriptions — Phase 5 : Use cases ✅

## Contexte

L'agrégat `Team` est event sourcé (méthodes `apply()` et `hydrate()` existantes). Les use cases persistent les domain events dans l'event store et les publient sur l'app event bus.

## Use case : `approve_enrollment.rs` — BC `teams`

Fichier : `src/app/teams/use_cases/approve_enrollment.rs`

### Signature

```rust
pub async fn execute(
    team_id: &EntityId,
    competition_id: String,
    competition_name: String,
    season_id: String,
    season_name: String,
    team_repo: &dyn ITeamRepository,
    event_bus: &EventBus,
) -> Result<(), ApproveError>
```

### Orchestration

1. Charger l'agrégat Team depuis l'event store (hydrate)
2. Vérifier que l'équipe n'est pas déjà inscrite à une autre compétition (règle métier)
3. Appeler `team.enroll(competition_id, competition_name, season_id, season_name)` → produit `TeamEnrolled`
4. Persister l'événement dans l'event store
5. Publier sur l'app event bus

### Erreurs

```rust
pub enum ApproveError {
    TeamNotFound,
    Domain(DomainError),
    Repository(String),
}
```

Note : réutilise la méthode `enroll()` existante sur l'agrégat.

## Use case : `reject_enrollment.rs` — BC `teams`

Fichier : `src/app/teams/use_cases/reject_enrollment.rs`

### Signature

```rust
pub async fn execute(
    team_id: &EntityId,
    team_repo: &dyn ITeamRepository,
    event_bus: &EventBus,
) -> Result<(), RejectError>
```

### Orchestration

1. Charger l'agrégat Team depuis l'event store (hydrate)
2. Appeler `team.reject_enrollment()` → produit `TeamEnrollmentRejected`
3. Persister l'événement dans l'event store
4. Publier sur l'app event bus

### Erreurs

```rust
pub enum RejectError {
    TeamNotFound,
    Domain(DomainError),
    Repository(String),
}
```

Note : nouvelle méthode `reject_enrollment()` à ajouter sur l'agrégat (phase 6).

## Use case : `dismiss_team.rs` — BC `teams`

Fichier : `src/app/teams/use_cases/dismiss_team.rs`

### Signature

```rust
pub async fn execute(
    team_id: &EntityId,
    team_repo: &dyn ITeamRepository,
    event_bus: &EventBus,
) -> Result<(), DismissError>
```

### Orchestration

1. Charger l'agrégat Team depuis l'event store (hydrate)
2. Appeler `team.dismiss()` → produit `TeamDismissed`
3. Persister l'événement dans l'event store
4. Publier sur l'app event bus

### Erreurs

```rust
pub enum DismissError {
    TeamNotFound,
    Domain(DomainError),
    Repository(String),
}
```

Note : réutilise la méthode `dismiss()` existante sur l'agrégat.

## Approve-all

Pas de use case distinct. Le handler itère sur les équipes pending de la saison et appelle `approve_enrollment::execute` pour chacune.

## Clôture des inscriptions — BC `competitions`

Pas de use case. Le handler met à jour directement le flag d'inscription sur la saison via `ISeasonRepository`.

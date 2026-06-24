# Step 1 — Sélection du match : Domaine

## Récapitulatif des règles métier (phases 1 à 5)

1. **Les deux équipes doivent être différentes** (`home_team_id != away_team_id`)
2. **Seules les équipes enrolled et en `ReadyToPlay`** sont sélectionnables — les équipes en `MatchReporting` ou autre phase sont exclues
3. **Coach lambda** : ne peut créer un rapport que pour un match impliquant une de ses propres équipes enrolled
4. **Admin compétition / Admin espace** : choix libre des deux équipes
5. **Les poules ne filtrent pas** les équipes sélectionnables
6. **Saisons en ordre anti-chronologique**, dernière saison pré-sélectionnée
7. **Deux modes de création** : formulaire vierge (use case Create) ou pré-créé par app event PairingCreated
8. **Un rapport pré-existant peut être mis à jour** (changement d'équipes) ou simplement confirmé (passage à step2)
9. **L'agrégat est event-sourcé** : persist events only, rehydratation par `apply()`
10. **Plusieurs rapports en phase Draft peuvent coexister** pour la même équipe (cas d'un calendrier pré-programmé). Le verrouillage (`MatchReporting`) ne s'applique qu'au `SelectionConfirmed`
11. **Au moment du `SelectionConfirmed`**, si une des deux équipes n'est plus en `ReadyToPlay`, la confirmation est refusée

## Pattern retenu : types par phase + event store (hybride)

Chaque phase du rapport de match est un **type distinct** qui encapsule le type précédent (comme `DraftTeam` → `RulesetSelectedTeam` → `RosterSelectedTeam` dans le BC `team_creation`). Chaque transition produit un **domain event** persisté dans l'event store. La rehydratation reconstruit le bon type en rejouant les events.

### Chaîne de types

```
MatchReportDraft (step1 : sélection)
  → confirm_selection() → SelectionConfirmed event →
MatchReportPreMatch (step2 : fan factor, journaliers, TV, inducements)
  → confirm_prematch() → PreMatchConfirmed event →
MatchReportInProgress (step3 : actions par tour)
  → confirm_actions() → ActionsConfirmed event →
MatchReportPostMatch (step5 : gains, fan factor final)
  → submit() → MatchReportSubmitted event →
MatchReportCompleted
```

### Enum de phase pour le repository

Le repository retourne un enum qui encapsule le type correct :

```rust
pub enum MatchReportState {
    Draft(MatchReportDraft),
    PreMatch(MatchReportPreMatch),
    InProgress(MatchReportInProgress),
    PostMatch(MatchReportPostMatch),
    Completed(MatchReportCompleted),
}
```

La rehydratation parcourt les events depuis l'event store et reconstruit progressivement le bon type. Le handler fait un `match` sur l'enum pour déterminer quelle page afficher ou quelle action est autorisée.

### Avantages

- **Sécurité compile-time** : impossible d'appeler `update_selection()` sur un `MatchReportPreMatch`
- **Données typées par phase** : chaque struct ne porte que les champs de sa phase
- **Event store** : historique complet, rehydratation déterministe, rebuildable
- **Reprise de saisie** : le repository retourne le bon type, le handler redirige vers la bonne étape

## Value objects

### Nouveaux VOs dans `shared_kernel/common_types.rs`

```rust
pub type MatchReportId = EntityId;
pub type RoundId = EntityId;
pub type PairingId = EntityId;
```

### VOs locaux au BC `match_report` (`domain/value_objects.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchReportOrigin {
    Manual,
    Pairing,
}
```

## Step 1 — Type `MatchReportDraft`

### Struct

```rust
#[derive(Debug, Clone)]
pub struct MatchReportDraft {
    pub id: MatchReportId,
    pub space_id: SpaceId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub created_by: CoachId,
    pub origin: MatchReportOrigin,
    pub version: u64,
}
```

### Factory

```rust
impl MatchReportDraft {
    pub fn create(
        id: MatchReportId,
        space_id: SpaceId,
        competition_id: CompetitionId,
        season_id: SeasonId,
        round_id: RoundId,
        home_team_id: TeamId,
        away_team_id: TeamId,
        created_by: CoachId,
        origin: MatchReportOrigin,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        if home_team_id == away_team_id {
            return Err(DomainError::SameTeam);
        }
        let event = MatchReportDomainEvent::MatchReportCreated { ... };
        let draft = Self::from_created_event(&event);
        Ok((draft, event))
    }
}
```

### Méthodes de commande

```rust
impl MatchReportDraft {
    /// Mise à jour de la sélection (admin uniquement — contrôle d'accès dans le handler)
    pub fn update_selection(
        &self,
        home_team_id: TeamId,
        away_team_id: TeamId,
        updated_by: CoachId,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        if home_team_id == away_team_id {
            return Err(DomainError::SameTeam);
        }
        let event = MatchReportDomainEvent::SelectionUpdated { ... };
        let updated = self.apply_selection_updated(&event);
        Ok((updated, event))
    }

    /// Confirmation — transition vers MatchReportPreMatch
    /// Le use case vérifie en amont que les deux équipes sont en ReadyToPlay
    /// via le port ITeamDataPort. Si une équipe n'est plus disponible,
    /// le use case refuse avant d'appeler cette méthode.
    pub fn confirm_selection(
        self,
        confirmed_by: CoachId,
    ) -> (MatchReportPreMatch, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::SelectionConfirmed { confirmed_by };
        let pre_match = MatchReportPreMatch::from_draft(self);
        (pre_match, event)
    }
}
```

Note : `confirm_selection` consomme `self` (move) — le compilateur interdit de réutiliser le `MatchReportDraft` après transition.

### Rehydratation

```rust
impl MatchReportDraft {
    fn from_created_event(event: &MatchReportDomainEvent) -> Self { ... }

    fn apply_selection_updated(&self, event: &MatchReportDomainEvent) -> Self { ... }
}
```

## Rehydratation globale

```rust
pub fn rehydrate(events: Vec<MatchReportDomainEvent>) -> Result<MatchReportState, DomainError> {
    let mut state: Option<MatchReportState> = None;

    for event in &events {
        state = Some(match (state, event) {
            // Création → Draft
            (None, MatchReportDomainEvent::MatchReportCreated { .. }) => {
                MatchReportState::Draft(MatchReportDraft::from_created_event(event))
            }
            // Draft → Draft (mise à jour sélection)
            (Some(MatchReportState::Draft(draft)), MatchReportDomainEvent::SelectionUpdated { .. }) => {
                MatchReportState::Draft(draft.apply_selection_updated(event))
            }
            // Draft → PreMatch (confirmation)
            (Some(MatchReportState::Draft(draft)), MatchReportDomainEvent::SelectionConfirmed { .. }) => {
                MatchReportState::PreMatch(MatchReportPreMatch::from_draft(draft))
            }
            // ... phases suivantes ajoutées dans les specs des pages suivantes
            _ => return Err(DomainError::InvalidEventSequence),
        });
    }

    state.ok_or(DomainError::EmptyEventStream)
}
```

## Domain events (step1)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MatchReportDomainEvent {
    MatchReportCreated {
        match_report_id: MatchReportId,
        space_id: SpaceId,
        competition_id: CompetitionId,
        season_id: SeasonId,
        round_id: RoundId,
        home_team_id: TeamId,
        away_team_id: TeamId,
        created_by: CoachId,
        origin: MatchReportOrigin,
    },
    SelectionUpdated {
        home_team_id: TeamId,
        away_team_id: TeamId,
        updated_by: CoachId,
    },
    SelectionConfirmed {
        confirmed_by: CoachId,
    },
}
```

## Erreurs domaine

```rust
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("les deux équipes doivent être différentes")]
    SameTeam,
    #[error("séquence d'événements invalide")]
    InvalidEventSequence,
    #[error("aucun événement dans le stream")]
    EmptyEventStream,
}
```

Note : `InvalidPhase` disparaît — le compilateur garantit les transitions par les types. Remplacé par `InvalidEventSequence` pour la rehydratation (cas défensif si l'event store contient des données corrompues).

`TeamNotAvailable` est levée par le use case (pas par l'agrégat) quand une équipe n'est plus en `ReadyToPlay` au moment de la confirmation. C'est une vérification applicative via le port, pas un invariant domaine.

## Tests unitaires

| Test | Règle couverte |
|------|----------------|
| `create_with_same_team_fails` | Règle 1 — home != away |
| `create_produces_created_event_and_draft` | Factory retourne le bon couple (Draft, Event) |
| `update_selection_with_same_team_fails` | Règle 1 sur update |
| `update_selection_produces_updated_draft` | Draft mis à jour avec nouvelles TeamId |
| `confirm_selection_returns_prematch` | Transition de type : Draft → PreMatch |
| `confirm_selection_consumes_draft` | Compile-time : Draft inutilisable après confirm |
| `rehydrate_created_returns_draft` | Un seul event → MatchReportState::Draft |
| `rehydrate_created_then_updated_returns_draft` | Deux events → Draft avec équipes mises à jour |
| `rehydrate_created_then_confirmed_returns_prematch` | Deux events → MatchReportState::PreMatch |
| `rehydrate_empty_stream_fails` | EmptyEventStream |
| `rehydrate_invalid_sequence_fails` | InvalidEventSequence (ex. SelectionUpdated sans Created) |

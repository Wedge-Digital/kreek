# BC `teams` — Agrégat `Team` event sourcé

**Priorité : haute**
**Dépend de :** `27-bc-teams-structure.md`
**Contexte :** `teams` — domaine pur

## Objectif

Modéliser l'agrégat `Team` en event sourcing : son état est entièrement dérivé du rejeu de ses événements domaine. Aucun état courant n'est persisté directement — seuls les événements le sont.

---

## Conception

### Événements domaine de `Team`

Tous les événements qui peuvent survenir sur une équipe, dans l'ordre chronologique possible :

Le format de sérialisation est **internally tagged** avec `#[serde(tag = "type")]`. Chaque événement produit un JSON autonome avec un champ `type` discriminant, directement stockable dans la colonne `payload JSONB` de l'event store.

Exemple de payload en base :
```json
{ "type": "TeamCreated",  "name": "Les Korrigans FC", "roster_id": "01J…", "treasury": 1000 }
{ "type": "TeamEnrolled", "competition_id": "01J…", "season_id": "01J…" }
{ "type": "TeamDismissed" }
```

La colonne `event_version` (défaut `"1.0"`) permet de gérer l'évolution du schéma d'un variant sans migration lourde. Les champs ajoutés en versions futures portent `#[serde(default)]` pour rester compatibles avec les anciens enregistrements.

### Value objects requis dans ce BC

Tous les newtypes doivent dériver `Serialize` / `Deserialize` pour apparaître dans les events persistés (cf. règle CLAUDE.md) :

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct TeamId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct SpaceId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct PlayerId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct PositionId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct CompetitionId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct SeasonId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct RosterId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct RosterName(pub String);

// Montants (pas de DomainError — validation implicite par u32)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] pub struct Kpo(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] pub struct KpoDelta(pub i32);

// CoachName et TeamName sont déjà définis dans shared_kernel — réutiliser
// UserId est déjà défini dans shared_kernel — réutiliser
```

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TeamDomainEvent {
    // Depuis BC team_creation
    TeamCreated {
        team_id:     TeamId,
        space_id:    SpaceId,
        name:        TeamName,
        roster_id:   RosterId,
        roster_name: RosterName,
        coach_id:    UserId,
        coach_name:  CoachName,
        treasury:    Kpo,
    },
    // Depuis BC competitions
    TeamEnrolled {
        competition_id: CompetitionId,
        season_id:      SeasonId,
    },
    // Action admin
    TeamDismissed,
    // Depuis BC match_report
    MatchPlayedReceived {
        result:              MatchResult,
        dedicated_fans_roll: u8,        // jet de dé 1D6 — pas de domaine propre
        treasury_income:     Kpo,
        spp_gains:           Vec<SppGain>,
    },
    // Actions coach — phases post-match
    PlayerImprovementApplied {
        player_id:   PlayerId,
        improvement: PlayerImprovement,
        value_delta: Kpo,
    },
    PlayerImprovementPhaseValidated,
    PlayerRecruited {
        position_id:    PositionId,
        base_value_kpo: Kpo,
        cost_kpo:       Kpo,
    },
    StaffBought { staff_type: StaffType, quantity: u8, cost_kpo: Kpo },
    RecruitmentPhaseValidated,
    PlayerFired {
        player_id:           PlayerId,
        value_kpo_at_firing: Kpo,
    },
    DismissalsPhaseValidated,
    PlayerRetiredTemporarily { player_id: PlayerId },
    RetirementPhaseValidated,
    // Depuis BC players — via app event bus
    PlayerValueUpdated { player_id: PlayerId, delta_kpo: KpoDelta },
    // Off-season
    PlayerNotReEngaged {
        player_id:            PlayerId,
        value_kpo_at_release: Kpo,
    },
    // Automatique
    CostlyMistakesApplied { roll: u8, incident: IncidentType, gp_lost: Kpo },
    // Admin
    GamePhaseOverridden {
        admin_id:   UserId,
        from_phase: Option<GamePhase>,  // enum, pas String
        to_phase:   GamePhase,
        reason:     Option<String>,     // texte libre — exception autorisée
    },
}
```

### État courant (dérivé du rejeu)

```rust
pub struct Team {
    // Identité
    pub id:           TeamId,
    pub space_id:     SpaceId,
    pub name:         TeamName,
    pub roster_id:    RosterId,
    pub roster_name:  RosterName,
    pub coach_id:     UserId,
    pub coach_name:   CoachName,
    // État de participation
    pub participation_status: ParticipationStatus,
    pub game_phase:           Option<GamePhase>,
    // Finances
    pub dedicated_fans: u8,      // compteur simple, pas de logique domaine propre
    pub treasury:       Kpo,
    pub team_value:     Kpo,
    // Séquence pour optimistic locking
    pub version:        u64,
}
```

### Pattern event sourcing

```rust
impl Team {
    /// Rejoue un événement sur l'agrégat — pure, sans effet de bord
    pub fn apply(mut self, event: &TeamDomainEvent) -> Self {
        match event {
            TeamDomainEvent::TeamCreated { team_id, space_id, name, roster_id,
                                           roster_name, coach_id, coach_name, treasury } => {
                self.id           = TeamId::from(team_id);
                self.space_id     = SpaceId::from(space_id);
                self.name         = TeamName::from(name);
                self.roster_id    = RosterId::from(roster_id);
                self.roster_name  = roster_name.clone();
                self.coach_id     = UserId::from(coach_id);
                self.coach_name   = coach_name.clone();
                self.treasury     = *treasury;
                self.participation_status = ParticipationStatus::PendingEnrollment;
                self.game_phase   = None;
            }
            TeamDomainEvent::TeamEnrolled { .. } => {
                self.participation_status = ParticipationStatus::Enrolled;
                self.game_phase           = Some(GamePhase::ReadyToPlay);
            }
            TeamDomainEvent::TeamDismissed => {
                self.participation_status = ParticipationStatus::Dismissed;
                self.game_phase           = None;
            }
            TeamDomainEvent::MatchPlayedReceived { result, dedicated_fans_roll, treasury_income, .. } => {
                self.dedicated_fans = compute_fans(*dedicated_fans_roll, result);
                self.treasury      += treasury_income;
                self.game_phase     = Some(GamePhase::PlayerImprovement);
            }
            TeamDomainEvent::PlayerImprovementPhaseValidated => {
                self.game_phase = Some(GamePhase::Recruitment);
            }
            TeamDomainEvent::RecruitmentPhaseValidated => {
                self.game_phase = Some(GamePhase::Dismissals);
            }
            TeamDomainEvent::DismissalsPhaseValidated => {
                self.game_phase = Some(GamePhase::TemporaryRetirement);
            }
            TeamDomainEvent::CostlyMistakesApplied { gp_lost, .. } => {
                self.treasury   = self.treasury.saturating_sub(*gp_lost);
                self.game_phase = Some(GamePhase::ReadyToPlay);
            }
            TeamDomainEvent::PlayerRecruited { base_value_kpo, cost_kpo, .. } => {
                self.team_value += base_value_kpo;
                self.treasury    = self.treasury.saturating_sub(*cost_kpo);
            }
            TeamDomainEvent::StaffBought { cost_kpo, .. } => {
                self.team_value += cost_kpo;
                self.treasury    = self.treasury.saturating_sub(*cost_kpo);
            }
            TeamDomainEvent::PlayerImprovementApplied { value_delta, .. } => {
                self.team_value += value_delta;
            }
            TeamDomainEvent::PlayerFired { value_kpo_at_firing, .. } => {
                self.team_value = self.team_value.saturating_sub(*value_kpo_at_firing);
            }
            TeamDomainEvent::PlayerNotReEngaged { value_kpo_at_release, .. } => {
                self.team_value = self.team_value.saturating_sub(*value_kpo_at_release);
            }
            TeamDomainEvent::PlayerValueUpdated { delta_kpo, .. } => {
                if *delta_kpo >= 0 {
                    self.team_value += *delta_kpo as u32;
                } else {
                    self.team_value = self.team_value.saturating_sub((-delta_kpo) as u32);
                }
            }
            _ => {}
        }
        self.version += 1;
        self
    }

    /// Hydrate l'agrégat depuis un flux d'événements
    pub fn hydrate(events: &[TeamDomainEvent]) -> Option<Self> {
        events.iter().fold(None, |acc, event| {
            Some(match acc {
                None    => Team::default().apply(event),
                Some(t) => t.apply(event),
            })
        })
    }
}
```

### Commandes (produisent des événements)

Les méthodes de commande valident les invariants puis retournent l'événement à persister — elles ne mutent pas l'agrégat directement :

```rust
impl Team {
    pub fn enroll(&self, competition_id: String, season_id: String)
        -> Result<TeamDomainEvent, DomainError>
    {
        match self.participation_status {
            ParticipationStatus::PendingEnrollment => Ok(TeamDomainEvent::TeamEnrolled { competition_id, season_id }),
            _ => Err(DomainError::InvalidTransition { ... }),
        }
    }

    pub fn dismiss(&self) -> Result<TeamDomainEvent, DomainError> { ... }

    pub fn receive_match_result(&self, ...) -> Result<TeamDomainEvent, DomainError> { ... }

    pub fn validate_improvement_phase(&self) -> Result<TeamDomainEvent, DomainError> { ... }
    // etc.
}
```

### Erreurs domaine

```rust
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("transition invalide : {from} → {to}")]
    InvalidTransition { from: String, to: String },
    #[error("équipe non inscrite")]
    NotEnrolled,
    #[error("équipe déjà renvoyée")]
    AlreadyDismissed,
    #[error("phase de jeu incorrecte")]
    WrongGamePhase,
}
```

---

## Checklist

- [ ] `TeamDomainEvent` enum complet avec `Serialize`/`Deserialize`
- [ ] Newtypes ID : `TeamId`, `SpaceId`, `PlayerId`, `PositionId`, `CompetitionId`, `SeasonId`, `RosterId` — tous avec `#[derive(Serialize, Deserialize)]`
- [ ] Newtypes monétaires : `Kpo(u32)`, `KpoDelta(i32)` — avec `#[derive(Serialize, Deserialize)]`
- [ ] `RosterName` newtype — réutiliser ou créer dans shared_kernel
- [ ] Vérifier que `TeamName`, `CoachName`, `UserId` du shared_kernel dérivent `Serialize`/`Deserialize` (requis pour les events)
- [ ] Value objects : `MatchResult`, `SppGain`, `PlayerImprovement`, `StaffType`, `IncidentType`
- [ ] `ParticipationStatus` + `GamePhase` enums
- [ ] Struct `Team` avec champ `version: u64`
- [ ] `Team::apply()` — pure, couvre tous les variants
- [ ] `Team::hydrate()` — fold sur les événements
- [ ] Commandes : `enroll()`, `dismiss()`, `receive_match_result()`, `validate_*_phase()`, `apply_costly_mistakes()`
- [ ] `DomainError` complet avec `thiserror`
- [ ] Tests unitaires : hydratation depuis séquence d'événements
- [ ] Tests unitaires : transitions valides et invalides

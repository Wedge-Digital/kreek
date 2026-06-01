# BC `teams` — Agrégat `Team` event sourcé

**Priorité : haute**
**Dépend de :** `27-bc-teams-structure.md`
**Contexte :** `teams` — domaine pur

## Objectif

Modéliser l'agrégat `Team` en event sourcing : son état est entièrement dérivé du rejeu de ses événements domaine. Aucun état courant n'est persisté directement — seuls les événements domaine le sont.

---

## Distinction app events / domain events

Le domaine `teams` ne connaît que ses propres **domain events**. Il ignore totalement l'existence des autres BCs et de leurs app events.

```
App event bus ──► Listener (couche IO)
                      │
                      ▼
                  Use case applicatif
                      │
                      ▼
                  Méthode domaine  ──► TeamDomainEvent
                                            │
                                            ▼
                                       Event store
```

Certains domain events sont produits en réponse à des commandes internes (actions coach, admin) ; d'autres sont produits en réponse à des app events reçus dans la couche IO — mais le domaine ne fait pas cette distinction. Tous les domain events ont des noms qui décrivent **ce qui s'est passé dans le domaine**, pas d'où vient le déclencheur.

---

## Conception

### Value objects requis dans ce BC

Tous les newtypes dérivant `Serialize`/`Deserialize` pour apparaître dans les events persistés :

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct TeamId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct SpaceId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct PlayerId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct PositionId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct CompetitionId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct SeasonId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct RosterId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct RosterName(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] pub struct Kpo(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] pub struct KpoDelta(pub i32);

// TeamName, CoachName, UserId — réutiliser depuis shared_kernel
// Vérifier qu'ils dérivent Serialize/Deserialize
```

### Événements domaine

Sérialisés en internally tagged avec `#[serde(tag = "type")]` :

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TeamDomainEvent {

    // ── Identité et cycle de vie ─────────────────────────────────────────
    // (produits en réponse à des app events reçus dans la couche IO)

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
    TeamEnrolled {
        competition_id: CompetitionId,
        season_id:      SeasonId,
    },
    TeamDismissed,

    // ── Séquence post-match ───────────────────────────────────────────────
    // PostMatchSequenceStarted : produit en réponse à l'app event MatchPlayed
    // (nommé en termes domaine — pas "MatchPlayedReceived")
    PostMatchSequenceStarted {
        result:              MatchResult,
        dedicated_fans:      u8,   // nouvelle valeur calculée (fans dévoués après le match)
        treasury_income:     Kpo,
        spp_gains:           Vec<SppGain>,
    },
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
    CostlyMistakesApplied { roll: u8, incident: IncidentType, gp_lost: Kpo },

    // ── Valeur joueur ─────────────────────────────────────────────────────
    // PlayerValueAdjusted : produit en réponse à l'app event PlayerValueChanged
    // du BC players (nommé en termes domaine — pas "PlayerValueChanged")
    PlayerValueAdjusted { player_id: PlayerId, delta_kpo: KpoDelta },

    // ── Off-season ────────────────────────────────────────────────────────
    OffSeasonStarted { season_id: SeasonId },
    PlayerReEngaged   { player_id: PlayerId },
    PlayerNotReEngaged {
        player_id:            PlayerId,
        value_kpo_at_release: Kpo,
    },
    OffSeasonCompleted,

    // ── Administration ────────────────────────────────────────────────────
    GamePhaseOverridden {
        admin_id:   UserId,
        from_phase: Option<GamePhase>,
        to_phase:   GamePhase,
        reason:     Option<String>,  // texte libre — exception à la règle primitive
    },

    // ── Modification d'identité ───────────────────────────────────────────
    TeamRenamed     { name: TeamName },
    InitialsChanged { initials: String },  // 2 caractères max, pas de VO dédié
    LogoChanged     { logo_url: String },  // URL — texte libre
}
```

### État courant (dérivé du rejeu)

```rust
pub struct Team {
    pub id:           TeamId,
    pub space_id:     SpaceId,
    pub name:         TeamName,
    pub roster_id:    RosterId,
    pub roster_name:  RosterName,
    pub coach_id:     UserId,
    pub coach_name:   CoachName,
    pub participation_status: ParticipationStatus,
    pub game_phase:           Option<GamePhase>,
    pub dedicated_fans: u8,   // compteur simple
    pub treasury:       Kpo,
    pub team_value:     Kpo,
    pub version:        u64,  // optimistic locking
}
```

### `Team::apply()` — rejeu pur

```rust
impl Team {
    pub fn apply(mut self, event: &TeamDomainEvent) -> Self {
        match event {
            TeamDomainEvent::TeamCreated { team_id, space_id, name, roster_id,
                                           roster_name, coach_id, coach_name, treasury } => {
                self.id           = team_id.clone();
                self.space_id     = space_id.clone();
                self.name         = name.clone();
                self.roster_id    = roster_id.clone();
                self.roster_name  = roster_name.clone();
                self.coach_id     = coach_id.clone();
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
            TeamDomainEvent::PostMatchSequenceStarted { dedicated_fans, treasury_income, .. } => {
                self.dedicated_fans = *dedicated_fans;
                self.treasury.0    += treasury_income.0;
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
                self.treasury.0  = self.treasury.0.saturating_sub(gp_lost.0);
                self.game_phase  = Some(GamePhase::ReadyToPlay);
            }
            TeamDomainEvent::PlayerRecruited { base_value_kpo, cost_kpo, .. } => {
                self.team_value.0 += base_value_kpo.0;
                self.treasury.0    = self.treasury.0.saturating_sub(cost_kpo.0);
            }
            TeamDomainEvent::StaffBought { cost_kpo, .. } => {
                self.team_value.0 += cost_kpo.0;
                self.treasury.0    = self.treasury.0.saturating_sub(cost_kpo.0);
            }
            TeamDomainEvent::PlayerImprovementApplied { value_delta, .. } => {
                self.team_value.0 += value_delta.0;
            }
            TeamDomainEvent::PlayerFired { value_kpo_at_firing, .. }
            | TeamDomainEvent::PlayerNotReEngaged { value_kpo_at_release: value_kpo_at_firing, .. } => {
                self.team_value.0 = self.team_value.0.saturating_sub(value_kpo_at_firing.0);
            }
            TeamDomainEvent::PlayerValueAdjusted { delta_kpo, .. } => {
                if delta_kpo.0 >= 0 {
                    self.team_value.0 += delta_kpo.0 as u32;
                } else {
                    self.team_value.0 = self.team_value.0.saturating_sub((-delta_kpo.0) as u32);
                }
            }
            TeamDomainEvent::OffSeasonCompleted => {
                self.participation_status = ParticipationStatus::PendingEnrollment;
                self.game_phase           = None;
            }
            TeamDomainEvent::TeamRenamed { name }      => { self.name = name.clone(); }
            TeamDomainEvent::GamePhaseOverridden { to_phase, .. } => {
                self.game_phase = Some(to_phase.clone());
            }
            _ => {}
        }
        self.version += 1;
        self
    }

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

### Commandes (produisent des domain events)

Les méthodes valident les invariants et retournent le domain event à persister. Elles ne mutent pas l'agrégat directement.

```rust
impl Team {
    pub fn enroll(&self, competition_id: CompetitionId, season_id: SeasonId)
        -> Result<TeamDomainEvent, DomainError>
    {
        match self.participation_status {
            ParticipationStatus::PendingEnrollment =>
                Ok(TeamDomainEvent::TeamEnrolled { competition_id, season_id }),
            _ => Err(DomainError::InvalidTransition { ... }),
        }
    }

    pub fn dismiss(&self)
        -> Result<TeamDomainEvent, DomainError> { ... }

    pub fn start_post_match_sequence(&self, result: MatchResult, fans_roll: u8,
                                     treasury_income: Kpo, spp_gains: Vec<SppGain>)
        -> Result<TeamDomainEvent, DomainError> { ... }

    pub fn validate_improvement_phase(&self) -> Result<TeamDomainEvent, DomainError> { ... }
    pub fn validate_recruitment_phase(&self) -> Result<TeamDomainEvent, DomainError> { ... }
    pub fn validate_dismissals_phase(&self)  -> Result<TeamDomainEvent, DomainError> { ... }
    pub fn validate_retirement_phase(&self)  -> Result<TeamDomainEvent, DomainError> { ... }
    pub fn override_phase(&self, admin_id: UserId, to: GamePhase, reason: Option<String>)
        -> Result<TeamDomainEvent, DomainError> { ... }
}
```

### Erreurs domaine

```rust
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("transition invalide")]
    InvalidTransition { from: ParticipationStatus, to: ParticipationStatus },
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

- [ ] `TeamDomainEvent` enum complet avec `#[serde(tag = "type")]`
- [ ] Newtypes ID : `TeamId`, `SpaceId`, `PlayerId`, `PositionId`, `CompetitionId`, `SeasonId`, `RosterId` — tous avec `Serialize`/`Deserialize`
- [ ] Newtypes monétaires : `Kpo(u32)`, `KpoDelta(i32)`
- [ ] `RosterName` newtype — créer dans shared_kernel
- [ ] Vérifier que `TeamName`, `CoachName`, `UserId` du shared_kernel dérivent `Serialize`/`Deserialize`
- [ ] Value objects : `MatchResult`, `SppGain`, `PlayerImprovement`, `StaffType`, `IncidentType`
- [ ] `ParticipationStatus` + `GamePhase` enums
- [ ] Struct `Team` avec champ `version: u64`
- [ ] `Team::apply()` couvre tous les variants
- [ ] `Team::hydrate()` — fold sur les événements
- [ ] Commandes : `enroll()`, `dismiss()`, `start_post_match_sequence()`, `validate_*_phase()`, `override_phase()`
- [ ] `DomainError` avec `thiserror` — pas de `String` dans les champs
- [ ] Tests unitaires : hydratation depuis séquence d'événements
- [ ] Tests unitaires : transitions valides et invalides

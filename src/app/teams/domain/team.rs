use serde::{Deserialize, Serialize};
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::{
    CompetitionId, PlayerId, PositionId, RosterId, SeasonId, SpaceId, UserId,
};
use crate::app::shared_kernel::team::TeamName;
use crate::app::teams::domain::value_objects::{
    IncidentType, Kpo, KpoDelta, MatchResult, PlayerImprovement, RosterName,
    SppGain, StaffType,
};
use crate::app::teams::domain::error::DomainError;

// ── États ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParticipationStatus {
    PendingEnrollment,
    Enrolled,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamePhase {
    ReadyToPlay,
    PlayerImprovement,
    Recruitment,
    Dismissals,
    TemporaryRetirement,
    OffSeason,
}

// ── Événements domaine ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TeamDomainEvent {
    // Identité et cycle de vie
    TeamCreated {
        team_id:     String,   // SUlid sérialisé en String
        space_id:    String,
        name:        String,   // TeamName inner value
        roster_id:   String,
        roster_name: String,
        coach_id:    String,
        coach_name:  String,
        treasury:    Kpo,      // budget restant à l'issue de la construction
    },
    TeamEnrolled {
        competition_id:   String,
        competition_name: String,
        season_id:        String,
        season_name:      String,
    },
    TeamDismissed,

    // Séquence post-match
    // Nommé en termes domaine — déclenché par l'app event MatchPlayed (IO layer)
    PostMatchSequenceStarted {
        result:          MatchResult,
        dedicated_fans:  u8,    // nouvelle valeur calculée (roll + modificateur)
        treasury_income: Kpo,
        spp_gains:       Vec<SppGain>,
    },
    PlayerImprovementApplied {
        player_id:   String,
        improvement: PlayerImprovement,
        value_delta: Kpo,
    },
    PlayerImprovementPhaseValidated,
    PlayerRecruited {
        position_id:    String,
        base_value_kpo: Kpo,
        cost_kpo:       Kpo,
    },
    StaffBought { staff_type: StaffType, quantity: u8, cost_kpo: Kpo },
    RecruitmentPhaseValidated,
    PlayerFired {
        player_id:           String,
        value_kpo_at_firing: Kpo,
    },
    DismissalsPhaseValidated,
    PlayerRetiredTemporarily { player_id: String },
    RetirementPhaseValidated,
    CostlyMistakesApplied { roll: u8, incident: IncidentType, gp_lost: Kpo },

    // Valeur joueur — déclenché par l'app event PlayerValueChanged (IO layer)
    PlayerValueAdjusted { player_id: String, delta_kpo: KpoDelta },

    // Off-season
    OffSeasonStarted { season_id: String },
    PlayerReEngaged   { player_id: String },
    PlayerNotReEngaged {
        player_id:            String,
        value_kpo_at_release: Kpo,
    },
    OffSeasonCompleted,

    // Administration
    GamePhaseOverridden {
        admin_id:   String,
        from_phase: Option<GamePhase>,
        to_phase:   GamePhase,
        reason:     Option<String>,
    },

    // Modification d'identité
    TeamRenamed     { name: String },
    InitialsChanged { initials: String },
    LogoChanged     { logo_url: String },
}

impl TeamDomainEvent {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::TeamCreated { .. }                  => "TeamCreated",
            Self::TeamEnrolled { .. }                 => "TeamEnrolled",
            Self::TeamDismissed                       => "TeamDismissed",
            Self::PostMatchSequenceStarted { .. }     => "PostMatchSequenceStarted",
            Self::PlayerImprovementApplied { .. }     => "PlayerImprovementApplied",
            Self::PlayerImprovementPhaseValidated     => "PlayerImprovementPhaseValidated",
            Self::PlayerRecruited { .. }              => "PlayerRecruited",
            Self::StaffBought { .. }                  => "StaffBought",
            Self::RecruitmentPhaseValidated           => "RecruitmentPhaseValidated",
            Self::PlayerFired { .. }                  => "PlayerFired",
            Self::DismissalsPhaseValidated            => "DismissalsPhaseValidated",
            Self::PlayerRetiredTemporarily { .. }     => "PlayerRetiredTemporarily",
            Self::RetirementPhaseValidated            => "RetirementPhaseValidated",
            Self::CostlyMistakesApplied { .. }        => "CostlyMistakesApplied",
            Self::PlayerValueAdjusted { .. }          => "PlayerValueAdjusted",
            Self::OffSeasonStarted { .. }             => "OffSeasonStarted",
            Self::PlayerReEngaged { .. }              => "PlayerReEngaged",
            Self::PlayerNotReEngaged { .. }           => "PlayerNotReEngaged",
            Self::OffSeasonCompleted                  => "OffSeasonCompleted",
            Self::GamePhaseOverridden { .. }          => "GamePhaseOverridden",
            Self::TeamRenamed { .. }                  => "TeamRenamed",
            Self::InitialsChanged { .. }              => "InitialsChanged",
            Self::LogoChanged { .. }                  => "LogoChanged",
        }
    }

    pub fn schema_version(&self) -> &'static str {
        "1.0"
    }
}

// ── Agrégat ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Team {
    pub id:           String,
    pub space_id:     String,
    pub name:         String,
    pub initials:     String,
    pub logo_url:     Option<String>,
    pub roster_id:    String,
    pub roster_name:  String,
    pub coach_id:     String,
    pub coach_name:   String,
    pub competition_id:   Option<String>,
    pub competition_name: Option<String>,
    pub season_id:        Option<String>,
    pub season_name:      Option<String>,
    pub participation_status: ParticipationStatus,
    pub game_phase:           Option<GamePhase>,
    pub dedicated_fans:  u8,
    pub treasury:        Kpo,
    pub team_value:      Kpo,
    pub version:         u64,
}

impl Default for Team {
    fn default() -> Self {
        Self {
            id:               String::new(),
            space_id:         String::new(),
            name:             String::new(),
            initials:         String::new(),
            logo_url:         None,
            roster_id:        String::new(),
            roster_name:      String::new(),
            coach_id:         String::new(),
            coach_name:       String::new(),
            competition_id:   None,
            competition_name: None,
            season_id:        None,
            season_name:      None,
            participation_status: ParticipationStatus::PendingEnrollment,
            game_phase:           None,
            dedicated_fans:  0,
            treasury:        Kpo(0),
            team_value:      Kpo(0),
            version:         0,
        }
    }
}

impl Team {
    /// Rejoue un événement sur l'agrégat — pure, sans effet de bord.
    pub fn apply(mut self, event: &TeamDomainEvent) -> Self {
        match event {
            TeamDomainEvent::TeamCreated {
                team_id, space_id, name, roster_id, roster_name, coach_id, coach_name, treasury,
            } => {
                self.id          = team_id.clone();
                self.space_id    = space_id.clone();
                self.name        = name.clone();
                self.initials    = initials_from(name);
                self.roster_id   = roster_id.clone();
                self.roster_name = roster_name.clone();
                self.coach_id    = coach_id.clone();
                self.coach_name  = coach_name.clone();
                self.treasury    = *treasury;
                self.participation_status = ParticipationStatus::PendingEnrollment;
                self.game_phase  = None;
            }
            TeamDomainEvent::TeamEnrolled {
                competition_id, competition_name, season_id, season_name,
            } => {
                self.competition_id   = Some(competition_id.clone());
                self.competition_name = Some(competition_name.clone());
                self.season_id        = Some(season_id.clone());
                self.season_name      = Some(season_name.clone());
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
            TeamDomainEvent::RetirementPhaseValidated => {
                self.game_phase = Some(GamePhase::OffSeason);
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
            TeamDomainEvent::PlayerFired { value_kpo_at_firing, .. } => {
                self.team_value.0 = self.team_value.0.saturating_sub(value_kpo_at_firing.0);
            }
            TeamDomainEvent::PlayerNotReEngaged { value_kpo_at_release, .. } => {
                self.team_value.0 = self.team_value.0.saturating_sub(value_kpo_at_release.0);
            }
            TeamDomainEvent::PlayerValueAdjusted { delta_kpo, .. } => {
                if delta_kpo.0 >= 0 {
                    self.team_value.0 += delta_kpo.0 as u32;
                } else {
                    self.team_value.0 = self.team_value.0.saturating_sub((-delta_kpo.0) as u32);
                }
            }
            TeamDomainEvent::OffSeasonStarted { .. } => {
                self.game_phase = Some(GamePhase::OffSeason);
            }
            TeamDomainEvent::OffSeasonCompleted => {
                self.participation_status = ParticipationStatus::PendingEnrollment;
                self.competition_id       = None;
                self.competition_name     = None;
                self.season_id            = None;
                self.season_name          = None;
                self.game_phase           = None;
            }
            TeamDomainEvent::GamePhaseOverridden { to_phase, .. } => {
                self.game_phase = Some(to_phase.clone());
            }
            TeamDomainEvent::TeamRenamed { name } => {
                self.name     = name.clone();
                self.initials = initials_from(name);
            }
            TeamDomainEvent::InitialsChanged { initials } => {
                self.initials = initials.clone();
            }
            TeamDomainEvent::LogoChanged { logo_url } => {
                self.logo_url = Some(logo_url.clone());
            }
            // Événements sans impact sur l'état de l'agrégat
            TeamDomainEvent::PlayerRetiredTemporarily { .. }
            | TeamDomainEvent::PlayerReEngaged { .. }
            | TeamDomainEvent::OffSeasonStarted { .. } => {}
        }
        self.version += 1;
        self
    }

    /// Hydrate l'agrégat en rejouant une séquence d'événements.
    pub fn hydrate(events: &[TeamDomainEvent]) -> Option<Self> {
        events.iter().fold(None, |acc, event| {
            Some(match acc {
                None    => Team::default().apply(event),
                Some(t) => t.apply(event),
            })
        })
    }

    // ── Commandes ──────────────────────────────────────────────────────────

    pub fn enroll(
        &self,
        competition_id:   String,
        competition_name: String,
        season_id:        String,
        season_name:      String,
    ) -> Result<TeamDomainEvent, DomainError> {
        match self.participation_status {
            ParticipationStatus::PendingEnrollment => Ok(TeamDomainEvent::TeamEnrolled {
                competition_id, competition_name, season_id, season_name,
            }),
            _ => Err(DomainError::InvalidTransition {
                from: self.participation_status.clone(),
                to:   ParticipationStatus::Enrolled,
            }),
        }
    }

    pub fn dismiss(&self) -> Result<TeamDomainEvent, DomainError> {
        match self.participation_status {
            ParticipationStatus::Enrolled =>
                Ok(TeamDomainEvent::TeamDismissed),
            ParticipationStatus::Dismissed =>
                Err(DomainError::AlreadyDismissed),
            _ => Err(DomainError::InvalidTransition {
                from: self.participation_status.clone(),
                to:   ParticipationStatus::Dismissed,
            }),
        }
    }

    pub fn start_post_match_sequence(
        &self,
        result:          MatchResult,
        fans_roll:       u8,
        treasury_income: Kpo,
        spp_gains:       Vec<SppGain>,
    ) -> Result<TeamDomainEvent, DomainError> {
        if self.game_phase != Some(GamePhase::ReadyToPlay) {
            return Err(DomainError::WrongGamePhase(self.game_phase.clone()));
        }
        let modifier = result.fan_modifier();
        let dedicated_fans = (self.dedicated_fans as i16 + fans_roll as i16 + modifier as i16)
            .max(0) as u8;
        Ok(TeamDomainEvent::PostMatchSequenceStarted {
            result,
            dedicated_fans,
            treasury_income,
            spp_gains,
        })
    }

    pub fn validate_improvement_phase(&self) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::PlayerImprovement)
            .map(|_| TeamDomainEvent::PlayerImprovementPhaseValidated)
    }

    pub fn validate_recruitment_phase(&self) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::Recruitment)
            .map(|_| TeamDomainEvent::RecruitmentPhaseValidated)
    }

    pub fn validate_dismissals_phase(&self) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::Dismissals)
            .map(|_| TeamDomainEvent::DismissalsPhaseValidated)
    }

    pub fn validate_retirement_phase(&self) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::TemporaryRetirement)
            .map(|_| TeamDomainEvent::RetirementPhaseValidated)
    }

    pub fn override_phase(
        &self,
        admin_id: String,
        to_phase: GamePhase,
        reason:   Option<String>,
    ) -> Result<TeamDomainEvent, DomainError> {
        if self.participation_status != ParticipationStatus::Enrolled {
            return Err(DomainError::NotEnrolled);
        }
        Ok(TeamDomainEvent::GamePhaseOverridden {
            admin_id,
            from_phase: self.game_phase.clone(),
            to_phase,
            reason,
        })
    }

    fn expect_phase(&self, expected: GamePhase) -> Result<(), DomainError> {
        if self.game_phase == Some(expected) {
            Ok(())
        } else {
            Err(DomainError::WrongGamePhase(self.game_phase.clone()))
        }
    }
}

/// Calcule les initiales (2 premières lettres des 2 premiers mots) depuis un nom.
fn initials_from(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn created_event() -> TeamDomainEvent {
        TeamDomainEvent::TeamCreated {
            team_id:     "01TEAM000000000000000000000".to_string(),
            space_id:    "01SPACE00000000000000000000".to_string(),
            name:        "Les Korrigans FC".to_string(),
            roster_id:   "01ROST000000000000000000000".to_string(),
            roster_name: "Elfes Sylvestres".to_string(),
            coach_id:    "01COACH00000000000000000000".to_string(),
            coach_name:  "Colonel Castor".to_string(),
            treasury:    Kpo(1000),
        }
    }

    fn enrolled_event() -> TeamDomainEvent {
        TeamDomainEvent::TeamEnrolled {
            competition_id:   "01COMP000000000000000000000".to_string(),
            competition_name: "Ligue de Condate".to_string(),
            season_id:        "01SEAS000000000000000000000".to_string(),
            season_name:      "Saison 2025".to_string(),
        }
    }

    #[test]
    fn hydrate_creates_team_from_created_event() {
        let events = vec![created_event()];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.name, "Les Korrigans FC");
        assert_eq!(team.initials, "LK");
        assert_eq!(team.participation_status, ParticipationStatus::PendingEnrollment);
        assert_eq!(team.version, 1);
    }

    #[test]
    fn enroll_transitions_to_enrolled() {
        let events = vec![created_event()];
        let team = Team::hydrate(&events).unwrap();
        let event = team.enroll(
            "01COMP000000000000000000000".to_string(),
            "Ligue de Condate".to_string(),
            "01SEAS000000000000000000000".to_string(),
            "Saison 2025".to_string(),
        ).unwrap();
        let team = team.apply(&event);
        assert_eq!(team.participation_status, ParticipationStatus::Enrolled);
        assert_eq!(team.game_phase, Some(GamePhase::ReadyToPlay));
    }

    #[test]
    fn cannot_enroll_enrolled_team() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        assert!(team.enroll(
            "01COMP000000000000000000000".to_string(),
            "Ligue".to_string(),
            "01SEAS000000000000000000000".to_string(),
            "Saison".to_string(),
        ).is_err());
    }

    #[test]
    fn dismiss_enrolled_team() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        let event = team.dismiss().unwrap();
        let team = team.apply(&event);
        assert_eq!(team.participation_status, ParticipationStatus::Dismissed);
        assert_eq!(team.game_phase, None);
    }

    #[test]
    fn cannot_dismiss_already_dismissed() {
        let events = vec![created_event(), enrolled_event(), TeamDomainEvent::TeamDismissed];
        let team = Team::hydrate(&events).unwrap();
        assert!(matches!(team.dismiss(), Err(DomainError::AlreadyDismissed)));
    }

    #[test]
    fn player_recruited_increases_team_value_and_decreases_treasury() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::PlayerRecruited {
                position_id:    "01POS000000000000000000000".to_string(),
                base_value_kpo: Kpo(95),
                cost_kpo:       Kpo(95),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.team_value, Kpo(95));
        assert_eq!(team.treasury, Kpo(905)); // 1000 initial - 95 coût
    }

    #[test]
    fn post_match_sequence_calculates_fans_correctly() {
        let events = vec![created_event(), enrolled_event()];
        let team   = Team::hydrate(&events).unwrap();
        let event  = team.start_post_match_sequence(
            MatchResult::Win, 4, Kpo(150), vec![],
        ).unwrap();
        if let TeamDomainEvent::PostMatchSequenceStarted { dedicated_fans, .. } = &event {
            // 0 (initial) + 4 (roll) + 1 (win modifier) = 5
            assert_eq!(*dedicated_fans, 5);
        } else {
            panic!("mauvais variant d'événement");
        }
    }

    #[test]
    fn phase_sequence_advances_correctly() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::PostMatchSequenceStarted {
                result: MatchResult::Win,
                dedicated_fans: 5,
                treasury_income: Kpo(150),
                spp_gains: vec![],
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.game_phase, Some(GamePhase::PlayerImprovement));

        let event = team.validate_improvement_phase().unwrap();
        let team  = team.apply(&event);
        assert_eq!(team.game_phase, Some(GamePhase::Recruitment));

        let event = team.validate_recruitment_phase().unwrap();
        let team  = team.apply(&event);
        assert_eq!(team.game_phase, Some(GamePhase::Dismissals));

        let event = team.validate_dismissals_phase().unwrap();
        let team  = team.apply(&event);
        assert_eq!(team.game_phase, Some(GamePhase::TemporaryRetirement));
    }

    #[test]
    fn initials_from_two_words() {
        assert_eq!(initials_from("Les Korrigans FC"), "LK");
        assert_eq!(initials_from("Nantes Undead"),    "NU");
        assert_eq!(initials_from("Skaven"),            "S");
    }
}

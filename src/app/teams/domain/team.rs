use crate::app::shared_kernel::common_types::{
    CoachId, CompetitionId, MatchReportId, PlayerId, PositionId, RosterId, SeasonId, SpaceId,
};
use crate::app::shared_kernel::staff_counts::{
    ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
};
use crate::app::shared_kernel::team::TeamId;
use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::value_objects::{
    DedicatedFans, IncidentType, Kpo, KpoDelta, MatchResult, PlayerImprovement, RosterName,
    SppGain, StaffQuantity, StaffType, TeamName,
};
use serde::{Deserialize, Serialize};

// ── États ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParticipationStatus {
    PendingEnrollment,
    Enrolled,
    Dismissed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamePhase {
    ReadyToPlay,
    MatchReporting,
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
        team_id: TeamId,
        space_id: SpaceId,
        competition_id: CompetitionId,
        competition_name: String,
        season_id: SeasonId,
        season_name: String,
        name: TeamName,
        logo_url: Option<String>,
        roster_id: RosterId,
        roster_name: RosterName,
        coach_id: CoachId,
        coach_name: String,
        treasury: Kpo,
        dedicated_fans: DedicatedFans,
        rerolls: RerollCount,
        apothecaries: ApothecaryCount,
        assistants: AssistantCount,
        cheerleaders: CheerleaderCount,
    },
    TeamEnrolled {
        competition_id: CompetitionId,
        competition_name: String,
        season_id: SeasonId,
        season_name: String,
    },
    TeamDismissed,
    TeamEnrollmentRejected {
        competition_id: Option<CompetitionId>,
        season_id: Option<SeasonId>,
    },

    // Rapport de match en cours de saisie
    MatchReportingStarted {
        match_report_id: MatchReportId,
    },

    // Séquence post-match
    // Nommé en termes domaine — déclenché par l'app event MatchPlayed (IO layer)
    PostMatchSequenceStarted {
        result: MatchResult,
        dedicated_fans: DedicatedFans,
        treasury_income: Kpo,
        spp_gains: Vec<SppGain>,
    },
    /// Défait la séquence d'après-match — le rapport a été dépublié pour
    /// correction. `dedicated_fans` est la valeur **absolue** restaurée, pas un
    /// delta : l'écrêtage à 0..20 n'est pas inversible.
    PostMatchSequenceReverted {
        match_report_id: MatchReportId,
        dedicated_fans: DedicatedFans,
        treasury_refund: Kpo,
    },
    PlayerImprovementApplied {
        player_id: PlayerId,
        improvement: PlayerImprovement,
        value_delta: Kpo,
    },
    PlayerImprovementPhaseValidated,
    PlayerRecruited {
        position_id: PositionId,
        base_value_kpo: Kpo,
        cost_kpo: Kpo,
    },
    StaffBought {
        staff_type: StaffType,
        quantity: StaffQuantity,
        cost_kpo: Kpo,
    },
    StaffDismissed {
        staff_type: StaffType,
        quantity: StaffQuantity,
        refund_kpo: Kpo,
    },
    RecruitmentPhaseValidated,
    PlayerFired {
        player_id: PlayerId,
        value_kpo_at_firing: Kpo,
    },
    DismissalsPhaseValidated,
    PlayerRetiredTemporarily {
        player_id: PlayerId,
    },
    RetirementPhaseValidated,
    CostlyMistakesApplied {
        roll: u8,
        incident: IncidentType,
        gp_lost: Kpo,
    },

    // Valeur joueur — déclenché par l'app event PlayerValueChanged (IO layer)
    PlayerValueAdjusted {
        player_id: PlayerId,
        delta_kpo: KpoDelta,
    },

    // Off-season
    OffSeasonStarted {
        season_id: SeasonId,
    },
    PlayerReEngaged {
        player_id: PlayerId,
    },
    PlayerNotReEngaged {
        player_id: PlayerId,
        value_kpo_at_release: Kpo,
    },
    OffSeasonCompleted,

    // Administration
    GamePhaseOverridden {
        admin_id: CoachId,
        from_phase: Option<GamePhase>,
        to_phase: GamePhase,
        reason: Option<String>,
    },

    // Modification d'identité
    TeamRenamed {
        name: TeamName,
    },
    InitialsChanged {
        initials: String,
    },
    LogoChanged {
        logo_url: String,
    },
}

impl TeamDomainEvent {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::TeamCreated { .. } => "TeamCreated",
            Self::TeamEnrolled { .. } => "TeamEnrolled",
            Self::TeamDismissed => "TeamDismissed",
            Self::TeamEnrollmentRejected { .. } => "TeamEnrollmentRejected",
            Self::MatchReportingStarted { .. } => "MatchReportingStarted",
            Self::PostMatchSequenceStarted { .. } => "PostMatchSequenceStarted",
            Self::PostMatchSequenceReverted { .. } => "PostMatchSequenceReverted",
            Self::PlayerImprovementApplied { .. } => "PlayerImprovementApplied",
            Self::PlayerImprovementPhaseValidated => "PlayerImprovementPhaseValidated",
            Self::PlayerRecruited { .. } => "PlayerRecruited",
            Self::StaffBought { .. } => "StaffBought",
            Self::StaffDismissed { .. } => "StaffDismissed",
            Self::RecruitmentPhaseValidated => "RecruitmentPhaseValidated",
            Self::PlayerFired { .. } => "PlayerFired",
            Self::DismissalsPhaseValidated => "DismissalsPhaseValidated",
            Self::PlayerRetiredTemporarily { .. } => "PlayerRetiredTemporarily",
            Self::RetirementPhaseValidated => "RetirementPhaseValidated",
            Self::CostlyMistakesApplied { .. } => "CostlyMistakesApplied",
            Self::PlayerValueAdjusted { .. } => "PlayerValueAdjusted",
            Self::OffSeasonStarted { .. } => "OffSeasonStarted",
            Self::PlayerReEngaged { .. } => "PlayerReEngaged",
            Self::PlayerNotReEngaged { .. } => "PlayerNotReEngaged",
            Self::OffSeasonCompleted => "OffSeasonCompleted",
            Self::GamePhaseOverridden { .. } => "GamePhaseOverridden",
            Self::TeamRenamed { .. } => "TeamRenamed",
            Self::InitialsChanged { .. } => "InitialsChanged",
            Self::LogoChanged { .. } => "LogoChanged",
        }
    }

    pub fn schema_version(&self) -> &'static str {
        "1.0"
    }
}

// ── Agrégat ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Team {
    pub id: TeamId,
    pub space_id: SpaceId,
    pub name: TeamName,
    pub initials: String,    // arch:ok texte libre calculé
    pub logo_url: Option<String>,
    pub roster_id: RosterId,
    pub roster_name: RosterName,
    pub coach_id: CoachId,
    pub coach_name: String,  // arch:ok texte libre dénormalisé
    pub competition_id: Option<CompetitionId>,
    pub competition_name: Option<String>,
    pub season_id: Option<SeasonId>,
    pub season_name: Option<String>,
    pub participation_status: ParticipationStatus,
    pub game_phase: Option<GamePhase>,
    pub dedicated_fans: DedicatedFans,
    pub treasury: Kpo,
    pub team_value: Kpo,
    pub rerolls: RerollCount,
    pub apothecaries: ApothecaryCount,
    pub assistants: AssistantCount,
    pub cheerleaders: CheerleaderCount,
    pub current_match_report_id: Option<MatchReportId>,
    /// Ce que la dernière séquence d'après-match a changé, pour pouvoir la
    /// défaire si le rapport est dépublié pour correction.
    ///
    /// État **dérivé** : reconstruit à chaque `hydrate()` depuis les événements
    /// existants, jamais persisté. C'est ce qui évite toute migration — les
    /// deux informations qu'il capture sont encore lisibles au moment précis où
    /// `apply(PostMatchSequenceStarted)` s'exécute, et perdues juste après.
    pub last_post_match: Option<LastPostMatch>,
    pub version: u64,
}

#[derive(Debug, Clone)]
pub struct LastPostMatch {
    pub match_report_id: MatchReportId,
    /// Les fans **d'avant** le match. L'événement ne stocke que la valeur
    /// post-écrêtage : `clamp(0, 20)` n'étant pas inversible, la restauration
    /// ne peut pas se faire par soustraction du modificateur.
    pub dedicated_fans_before: DedicatedFans,
    pub treasury_income: Kpo,
}

impl Default for Team {
    fn default() -> Self {
        Self {
            id: TeamId::new(),
            space_id: SpaceId::new(),
            name: TeamName::try_new("placeholder".to_string()).expect("valid"),
            initials: String::new(),
            logo_url: None,
            roster_id: RosterId(String::new()),
            roster_name: RosterName::try_new("placeholder".to_string()).expect("valid"),
            coach_id: CoachId::new(),
            coach_name: String::new(),
            competition_id: None,
            competition_name: None,
            season_id: None,
            season_name: None,
            participation_status: ParticipationStatus::PendingEnrollment,
            game_phase: None,
            dedicated_fans: DedicatedFans::try_new(0).expect("valid"),
            treasury: Kpo(0),
            team_value: Kpo(0),
            rerolls: RerollCount::default(),
            apothecaries: ApothecaryCount::default(),
            assistants: AssistantCount::default(),
            cheerleaders: CheerleaderCount::default(),
            current_match_report_id: None,
            last_post_match: None,
            version: 0,
        }
    }
}

impl Team {
    /// Rejoue un événement sur l'agrégat — pure, sans effet de bord.
    pub fn apply(mut self, event: &TeamDomainEvent) -> Self {
        match event {
            TeamDomainEvent::TeamCreated {
                team_id,
                space_id,
                competition_id,
                competition_name,
                season_id,
                season_name,
                name,
                logo_url,
                roster_id,
                roster_name,
                coach_id,
                coach_name,
                treasury,
                dedicated_fans,
                rerolls,
                apothecaries,
                assistants,
                cheerleaders,
            } => {
                self.id = *team_id;
                self.space_id = *space_id;
                self.competition_id = Some(*competition_id);
                self.competition_name = Some(competition_name.clone());
                self.season_id = Some(*season_id);
                self.season_name = Some(season_name.clone());
                self.name = name.clone();
                self.initials = initials_from(name.as_ref());
                self.logo_url = logo_url.clone();
                self.roster_id = roster_id.clone();
                self.roster_name = roster_name.clone();
                self.coach_id = *coach_id;
                self.coach_name = coach_name.clone();
                self.treasury = *treasury;
                self.dedicated_fans = *dedicated_fans;
                self.rerolls = *rerolls;
                self.apothecaries = *apothecaries;
                self.assistants = *assistants;
                self.cheerleaders = *cheerleaders;
                self.participation_status = ParticipationStatus::PendingEnrollment;
                self.game_phase = None;
            }
            TeamDomainEvent::TeamEnrolled {
                competition_id,
                competition_name,
                season_id,
                season_name,
            } => {
                self.competition_id = Some(*competition_id);
                self.competition_name = Some(competition_name.clone());
                self.season_id = Some(*season_id);
                self.season_name = Some(season_name.clone());
                self.participation_status = ParticipationStatus::Enrolled;
                self.game_phase = Some(GamePhase::ReadyToPlay);
            }
            TeamDomainEvent::TeamDismissed => {
                self.participation_status = ParticipationStatus::Dismissed;
                self.game_phase = None;
            }
            TeamDomainEvent::TeamEnrollmentRejected { .. } => {
                self.participation_status = ParticipationStatus::Rejected;
            }
            TeamDomainEvent::MatchReportingStarted { match_report_id } => {
                self.game_phase = Some(GamePhase::MatchReporting);
                self.current_match_report_id = Some(*match_report_id);
            }
            TeamDomainEvent::PostMatchSequenceStarted {
                dedicated_fans,
                treasury_income,
                ..
            } => {
                // Capturé AVANT les affectations qui suivent : `dedicated_fans`
                // va être écrasé et `current_match_report_id` remis à None.
                // Passé cet instant, ni l'un ni l'autre n'est reconstructible.
                self.last_post_match = self.current_match_report_id.map(|match_report_id| {
                    LastPostMatch {
                        match_report_id,
                        dedicated_fans_before: self.dedicated_fans,
                        treasury_income: *treasury_income,
                    }
                });
                self.dedicated_fans = *dedicated_fans;
                self.treasury.0 += treasury_income.0;
                self.game_phase = Some(GamePhase::PlayerImprovement);
                self.current_match_report_id = None;
            }
            TeamDomainEvent::PostMatchSequenceReverted {
                match_report_id,
                dedicated_fans,
                treasury_refund,
            } => {
                self.dedicated_fans = *dedicated_fans;
                self.treasury.0 = self.treasury.0.saturating_sub(treasury_refund.0);
                self.game_phase = Some(GamePhase::MatchReporting);
                // Restauré car `start_post_match_sequence` exige cette phase, et
                // la re-publication en dépend.
                self.current_match_report_id = Some(*match_report_id);
                // C'est ici que se joue l'idempotence : une seconde compensation
                // ne trouvera plus de dernier après-match et sera refusée.
                self.last_post_match = None;
            }
            TeamDomainEvent::PlayerImprovementPhaseValidated => {
                self.game_phase = Some(GamePhase::Recruitment);
            }
            TeamDomainEvent::RecruitmentPhaseValidated => {
                self.game_phase = Some(GamePhase::Dismissals);
            }
            TeamDomainEvent::DismissalsPhaseValidated => {
                // Simplification temporaire : la retraite temporaire (carte 39,
                // to_be_refined) n'étant pas encore implémentée, on revient
                // directement en ReadyToPlay plutôt que de bloquer l'équipe
                // dans une phase sans action possible.
                self.game_phase = Some(GamePhase::ReadyToPlay);
            }
            TeamDomainEvent::RetirementPhaseValidated => {
                self.game_phase = Some(GamePhase::OffSeason);
            }
            TeamDomainEvent::CostlyMistakesApplied { gp_lost, .. } => {
                self.treasury.0 = self.treasury.0.saturating_sub(gp_lost.0);
                self.game_phase = Some(GamePhase::ReadyToPlay);
            }
            TeamDomainEvent::PlayerRecruited {
                base_value_kpo,
                cost_kpo,
                ..
            } => {
                self.team_value.0 += base_value_kpo.0;
                self.treasury.0 = self.treasury.0.saturating_sub(cost_kpo.0);
            }
            TeamDomainEvent::StaffBought {
                staff_type,
                quantity,
                cost_kpo,
            } => {
                let qty = quantity.into_inner();
                match staff_type {
                    StaffType::Reroll => self.rerolls.0 = self.rerolls.0.saturating_add(qty),
                    StaffType::Apothecary => {
                        self.apothecaries.0 = self.apothecaries.0.saturating_add(qty)
                    }
                    StaffType::Assistant => {
                        self.assistants.0 = self.assistants.0.saturating_add(qty)
                    }
                    StaffType::Cheerleader => {
                        self.cheerleaders.0 = self.cheerleaders.0.saturating_add(qty)
                    }
                    StaffType::FansFactor => {}
                }
                self.team_value.0 += cost_kpo.0;
                self.treasury.0 = self.treasury.0.saturating_sub(cost_kpo.0);
            }
            TeamDomainEvent::StaffDismissed {
                staff_type,
                quantity,
                refund_kpo,
            } => {
                let qty = quantity.into_inner();
                match staff_type {
                    StaffType::Apothecary => {
                        self.apothecaries.0 = self.apothecaries.0.saturating_sub(qty)
                    }
                    StaffType::Assistant => {
                        self.assistants.0 = self.assistants.0.saturating_sub(qty)
                    }
                    StaffType::Cheerleader => {
                        self.cheerleaders.0 = self.cheerleaders.0.saturating_sub(qty)
                    }
                    _ => {} // Reroll, FansFactor : non renvoyables
                }
                self.team_value.0 = self.team_value.0.saturating_sub(refund_kpo.0);
                self.treasury.0 += refund_kpo.0;
            }
            TeamDomainEvent::PlayerImprovementApplied { value_delta, .. } => {
                self.team_value.0 += value_delta.0;
            }
            TeamDomainEvent::PlayerFired {
                value_kpo_at_firing,
                ..
            } => {
                self.team_value.0 = self.team_value.0.saturating_sub(value_kpo_at_firing.0);
            }
            TeamDomainEvent::PlayerNotReEngaged {
                value_kpo_at_release,
                ..
            } => {
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
                self.competition_id = None;
                self.competition_name = None;
                self.season_id = None;
                self.season_name = None;
                self.game_phase = None;
            }
            TeamDomainEvent::GamePhaseOverridden { to_phase, .. } => {
                self.game_phase = Some(to_phase.clone());
            }
            TeamDomainEvent::TeamRenamed { name } => {
                self.name = name.clone();
                self.initials = initials_from(name.as_ref());
            }
            TeamDomainEvent::InitialsChanged { initials } => {
                self.initials = initials.clone();
            }
            TeamDomainEvent::LogoChanged { logo_url } => {
                self.logo_url = Some(logo_url.clone());
            }
            // Événements sans impact sur l'état de l'agrégat
            TeamDomainEvent::PlayerRetiredTemporarily { .. }
            | TeamDomainEvent::PlayerReEngaged { .. } => {}
        }
        self.version += 1;
        self
    }

    /// Hydrate l'agrégat en rejouant une séquence d'événements.
    pub fn hydrate(events: &[TeamDomainEvent]) -> Option<Self> {
        events.iter().fold(None, |acc, event| {
            Some(match acc {
                None => Team::default().apply(event),
                Some(t) => t.apply(event),
            })
        })
    }

    // ── Commandes ──────────────────────────────────────────────────────────

    pub fn enroll(
        &self,
        competition_id: CompetitionId,
        competition_name: String,
        season_id: SeasonId,
        season_name: String,
    ) -> Result<TeamDomainEvent, DomainError> {
        match self.participation_status {
            ParticipationStatus::PendingEnrollment => Ok(TeamDomainEvent::TeamEnrolled {
                competition_id,
                competition_name,
                season_id,
                season_name,
            }),
            _ => Err(DomainError::InvalidTransition {
                from: self.participation_status.clone(),
                to: ParticipationStatus::Enrolled,
            }),
        }
    }

    pub fn reject_enrollment(&self) -> Result<TeamDomainEvent, DomainError> {
        match self.participation_status {
            ParticipationStatus::PendingEnrollment => {
                Ok(TeamDomainEvent::TeamEnrollmentRejected {
                    competition_id: self.competition_id,
                    season_id: self.season_id,
                })
            }
            _ => Err(DomainError::InvalidTransition {
                from: self.participation_status.clone(),
                to: ParticipationStatus::Rejected,
            }),
        }
    }

    pub fn dismiss(&self) -> Result<TeamDomainEvent, DomainError> {
        match self.participation_status {
            ParticipationStatus::Enrolled => Ok(TeamDomainEvent::TeamDismissed),
            ParticipationStatus::Dismissed => Err(DomainError::AlreadyDismissed),
            _ => Err(DomainError::InvalidTransition {
                from: self.participation_status.clone(),
                to: ParticipationStatus::Dismissed,
            }),
        }
    }

    pub fn start_match_reporting(
        &self,
        match_report_id: MatchReportId,
    ) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::ReadyToPlay)?;
        Ok(TeamDomainEvent::MatchReportingStarted { match_report_id })
    }

    pub fn start_post_match_sequence(
        &self,
        result: MatchResult,
        fan_mod: i8,
        treasury_income: Kpo,
        spp_gains: Vec<SppGain>,
    ) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::MatchReporting)?;
        // fan_mod est déjà la valeur finale saisie par le coach sur le rapport
        // de match (bornée -2..2 côté BC match_report) — appliquée telle
        // quelle, aucun recalcul via le résultat du match.
        let raw = (self.dedicated_fans.into_inner() as i16 + fan_mod as i16).max(0) as u8;
        let dedicated_fans =
            DedicatedFans::try_new(raw.min(20)).expect("clamped to valid range");
        Ok(TeamDomainEvent::PostMatchSequenceStarted {
            result,
            dedicated_fans,
            treasury_income,
            spp_gains,
        })
    }

    /// Défait la séquence d'après-match d'un rapport dépublié pour correction.
    ///
    /// Refuse si l'équipe a quitté la phase d'amélioration — elle aurait alors
    /// pu recruter ou acheter du staff, et la trésorerie ne serait plus celle
    /// qu'on croit défaire. Refuse aussi si le dernier après-match ne concerne
    /// pas ce rapport, ce qui rend l'opération idempotente.
    pub fn revert_post_match_sequence(
        &self,
        match_report_id: MatchReportId,
    ) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::PlayerImprovement)?;
        let last = self
            .last_post_match
            .as_ref()
            .filter(|l| l.match_report_id == match_report_id)
            .ok_or(DomainError::NoPostMatchToRevert)?;

        Ok(TeamDomainEvent::PostMatchSequenceReverted {
            match_report_id,
            dedicated_fans: last.dedicated_fans_before,
            treasury_refund: last.treasury_income,
        })
    }

    pub fn buy_staff(
        &self,
        staff_type: StaffType,
        quantity: StaffQuantity,
        cost_kpo: Kpo,
    ) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::Recruitment)?;
        match staff_type {
            StaffType::Reroll | StaffType::Assistant | StaffType::Cheerleader => {}
            _ => return Err(DomainError::StaffTypeNotBuyable),
        }
        if self.treasury.0 < cost_kpo.0 {
            return Err(DomainError::InsufficientTreasury);
        }
        Ok(TeamDomainEvent::StaffBought {
            staff_type,
            quantity,
            cost_kpo,
        })
    }

    pub fn dismiss_staff(
        &self,
        staff_type: StaffType,
        quantity: StaffQuantity,
        refund_kpo: Kpo,
    ) -> Result<TeamDomainEvent, DomainError> {
        self.expect_phase(GamePhase::Dismissals)?;
        let owned = match staff_type {
            StaffType::Apothecary => self.apothecaries.0,
            StaffType::Assistant => self.assistants.0,
            StaffType::Cheerleader => self.cheerleaders.0,
            _ => return Err(DomainError::StaffTypeNotDismissable),
        };
        if quantity.into_inner() > owned {
            return Err(DomainError::InsufficientStaff);
        }
        Ok(TeamDomainEvent::StaffDismissed {
            staff_type,
            quantity,
            refund_kpo,
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

    /// Enregistre l'effet sur `team_value` d'un achat de compétence/caractéristique
    /// déjà validé côté BC `players` (registre d'un fait déjà survenu — pas de
    /// garde ici, `players` a déjà vérifié SPP/phase/accès au moment de l'achat).
    pub fn apply_player_improvement(
        &self,
        player_id: PlayerId,
        improvement: PlayerImprovement,
        value_delta: Kpo,
    ) -> TeamDomainEvent {
        TeamDomainEvent::PlayerImprovementApplied { player_id, improvement, value_delta }
    }

    pub fn override_phase(
        &self,
        admin_id: CoachId,
        to_phase: GamePhase,
        reason: Option<String>,
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
    use crate::app::shared_kernel::common_types::{
        CompetitionId, CoachId, RosterId, SeasonId, SpaceId,
    };
    use crate::app::shared_kernel::staff_counts::{
        ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
    };
    use crate::app::shared_kernel::team::TeamId;
    use crate::app::teams::domain::value_objects::{DedicatedFans, RosterName, TeamName};

    fn team_id() -> TeamId { TeamId::try_new("00000000000000000000000001").unwrap() }
    fn space_id() -> SpaceId { SpaceId::try_new("00000000000000000000000002").unwrap() }
    fn competition_id() -> CompetitionId { CompetitionId::try_new("00000000000000000000000003").unwrap() }
    fn season_id() -> SeasonId { SeasonId::try_new("00000000000000000000000004").unwrap() }
    fn roster_id() -> RosterId { RosterId::try_new("00000000000000000000000005").unwrap() }
    fn coach_id() -> CoachId { CoachId::try_new("00000000000000000000000006").unwrap() }
    fn match_report_id() -> MatchReportId { MatchReportId::try_new("00000000000000000000000007").unwrap() }

    fn created_event() -> TeamDomainEvent {
        TeamDomainEvent::TeamCreated {
            team_id: team_id(),
            space_id: space_id(),
            competition_id: competition_id(),
            competition_name: "Ligue de Condate".to_string(),
            season_id: season_id(),
            season_name: "Saison 2025".to_string(),
            name: TeamName::try_new("Les Korrigans FC".to_string()).unwrap(),
            logo_url: None,
            roster_id: roster_id(),
            roster_name: RosterName::try_new("Elfes Sylvestres".to_string()).unwrap(),
            coach_id: coach_id(),
            coach_name: "Colonel Castor".to_string(),
            treasury: Kpo(1000),
            dedicated_fans: DedicatedFans::try_new(2).unwrap(),
            rerolls: RerollCount(3),
            apothecaries: ApothecaryCount(1),
            assistants: AssistantCount(2),
            cheerleaders: CheerleaderCount(3),
        }
    }

    fn enrolled_event() -> TeamDomainEvent {
        TeamDomainEvent::TeamEnrolled {
            competition_id: competition_id(),
            competition_name: "Ligue de Condate".to_string(),
            season_id: season_id(),
            season_name: "Saison 2025".to_string(),
        }
    }

    #[test]
    fn hydrate_creates_team_from_created_event() {
        let events = vec![created_event()];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.name.as_ref(), "Les Korrigans FC");
        assert_eq!(team.initials, "LK");
        assert_eq!(
            team.participation_status,
            ParticipationStatus::PendingEnrollment
        );
        assert_eq!(team.version, 1);
    }

    #[test]
    fn enroll_transitions_to_enrolled() {
        let events = vec![created_event()];
        let team = Team::hydrate(&events).unwrap();
        let event = team
            .enroll(
                competition_id(),
                "Ligue de Condate".to_string(),
                season_id(),
                "Saison 2025".to_string(),
            )
            .unwrap();
        let team = team.apply(&event);
        assert_eq!(team.participation_status, ParticipationStatus::Enrolled);
        assert_eq!(team.game_phase, Some(GamePhase::ReadyToPlay));
    }

    #[test]
    fn cannot_enroll_enrolled_team() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        assert!(team
            .enroll(
                competition_id(),
                "Ligue".to_string(),
                season_id(),
                "Saison".to_string(),
            )
            .is_err());
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
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::TeamDismissed,
        ];
        let team = Team::hydrate(&events).unwrap();
        assert!(matches!(team.dismiss(), Err(DomainError::AlreadyDismissed)));
    }

    #[test]
    fn player_recruited_increases_team_value_and_decreases_treasury() {
        let pos_id = PositionId::try_new("00000000000000000000000009").unwrap();
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::PlayerRecruited {
                position_id: pos_id,
                base_value_kpo: Kpo(95),
                cost_kpo: Kpo(95),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.team_value, Kpo(95));
        assert_eq!(team.treasury, Kpo(905)); // 1000 initial - 95 coût
    }

    #[test]
    fn post_match_sequence_calculates_fans_correctly() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted {
                match_report_id: match_report_id(),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        let event = team
            .start_post_match_sequence(MatchResult::Win, 2, Kpo(150), vec![])
            .unwrap();
        if let TeamDomainEvent::PostMatchSequenceStarted { dedicated_fans, .. } = &event {
            // 2 (initial: 1 base + 1 amélioration) + 2 (fan_mod du rapport) = 4
            assert_eq!(dedicated_fans.into_inner(), 4);
        } else {
            panic!("mauvais variant d'événement");
        }
    }

    #[test]
    fn post_match_sequence_clamps_fans_at_20() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted {
                match_report_id: match_report_id(),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        // 2 fans + fan_mod 100 → clampé à 20 (la borne -2..2 est imposée côté
        // match_report/FanFactorMod, pas ici — ce test vérifie le clamp mécanique)
        let event = team
            .start_post_match_sequence(MatchResult::Win, 100, Kpo(0), vec![])
            .unwrap();
        if let TeamDomainEvent::PostMatchSequenceStarted { dedicated_fans, .. } = &event {
            assert_eq!(dedicated_fans.into_inner(), 20);
        } else {
            panic!("mauvais variant d'événement");
        }
    }

    #[test]
    fn post_match_sequence_negative_fan_mod_never_goes_below_zero() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted {
                match_report_id: match_report_id(),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        // 2 fans - 2 (fan_mod minimal du rapport) = 0
        let event = team
            .start_post_match_sequence(MatchResult::Loss, -2, Kpo(0), vec![])
            .unwrap();
        if let TeamDomainEvent::PostMatchSequenceStarted { dedicated_fans, .. } = &event {
            assert_eq!(dedicated_fans.into_inner(), 0);
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
                dedicated_fans: DedicatedFans::try_new(5).unwrap(),
                treasury_income: Kpo(150),
                spp_gains: vec![],
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.game_phase, Some(GamePhase::PlayerImprovement));

        let event = team.validate_improvement_phase().unwrap();
        let team = team.apply(&event);
        assert_eq!(team.game_phase, Some(GamePhase::Recruitment));

        let event = team.validate_recruitment_phase().unwrap();
        let team = team.apply(&event);
        assert_eq!(team.game_phase, Some(GamePhase::Dismissals));

        let event = team.validate_dismissals_phase().unwrap();
        let team = team.apply(&event);
        assert_eq!(team.game_phase, Some(GamePhase::ReadyToPlay));
    }

    #[test]
    fn initials_from_two_words() {
        assert_eq!(initials_from("Les Korrigans FC"), "LK");
        assert_eq!(initials_from("Nantes Undead"), "NU");
        assert_eq!(initials_from("Skaven"), "S");
    }

    fn recruitment_phase_team() -> Team {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::PostMatchSequenceStarted {
                result: MatchResult::Win,
                dedicated_fans: DedicatedFans::try_new(5).unwrap(),
                treasury_income: Kpo(150),
                spp_gains: vec![],
            },
            TeamDomainEvent::PlayerImprovementPhaseValidated,
            // treasury = 1000 + 150 = 1150, phase = Recruitment
        ];
        Team::hydrate(&events).unwrap()
    }

    fn dismissals_phase_team() -> Team {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::PostMatchSequenceStarted {
                result: MatchResult::Win,
                dedicated_fans: DedicatedFans::try_new(5).unwrap(),
                treasury_income: Kpo(150),
                spp_gains: vec![],
            },
            TeamDomainEvent::PlayerImprovementPhaseValidated,
            TeamDomainEvent::RecruitmentPhaseValidated,
            // phase = Dismissals
        ];
        Team::hydrate(&events).unwrap()
    }

    #[test]
    fn buy_staff_hors_phase_retourne_erreur() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        // Phase ReadyToPlay — pas Recruitment
        assert!(matches!(
            team.buy_staff(StaffType::Reroll, StaffQuantity::try_new(1).unwrap(), Kpo(50)),
            Err(DomainError::WrongGamePhase(_))
        ));
    }

    #[test]
    fn buy_staff_type_non_autorise_retourne_erreur() {
        let team = recruitment_phase_team();
        assert!(matches!(
            team.buy_staff(StaffType::Apothecary, StaffQuantity::try_new(1).unwrap(), Kpo(50)),
            Err(DomainError::StaffTypeNotBuyable)
        ));
        assert!(matches!(
            team.buy_staff(StaffType::FansFactor, StaffQuantity::try_new(1).unwrap(), Kpo(50)),
            Err(DomainError::StaffTypeNotBuyable)
        ));
    }

    #[test]
    fn buy_staff_tresorerie_insuffisante_retourne_erreur() {
        let team = recruitment_phase_team();
        // treasury = 1150, coût = 2000
        assert!(matches!(
            team.buy_staff(StaffType::Reroll, StaffQuantity::try_new(1).unwrap(), Kpo(2000)),
            Err(DomainError::InsufficientTreasury)
        ));
    }

    #[test]
    fn buy_staff_met_a_jour_compteur_et_tresorerie() {
        let team = recruitment_phase_team();
        assert_eq!(team.rerolls.0, 3);
        assert_eq!(team.treasury.0, 1150);

        let event = team
            .buy_staff(StaffType::Reroll, StaffQuantity::try_new(2).unwrap(), Kpo(100))
            .unwrap();
        let team = team.apply(&event);

        assert_eq!(team.rerolls.0, 5);
        assert_eq!(team.treasury.0, 1050);
    }

    #[test]
    fn buy_assistant_met_a_jour_compteur() {
        let team = recruitment_phase_team();
        let event = team
            .buy_staff(StaffType::Assistant, StaffQuantity::try_new(1).unwrap(), Kpo(10))
            .unwrap();
        let team = team.apply(&event);
        assert_eq!(team.assistants.0, 3); // 2 initial + 1
    }

    #[test]
    fn dismiss_staff_hors_phase_retourne_erreur() {
        let team = recruitment_phase_team();
        assert!(matches!(
            team.dismiss_staff(
                StaffType::Assistant,
                StaffQuantity::try_new(1).unwrap(),
                Kpo(10)
            ),
            Err(DomainError::WrongGamePhase(_))
        ));
    }

    #[test]
    fn dismiss_staff_reroll_retourne_erreur() {
        let team = dismissals_phase_team();
        assert!(matches!(
            team.dismiss_staff(
                StaffType::Reroll,
                StaffQuantity::try_new(1).unwrap(),
                Kpo(50)
            ),
            Err(DomainError::StaffTypeNotDismissable)
        ));
    }

    #[test]
    fn dismiss_staff_quantite_insuffisante_retourne_erreur() {
        let team = dismissals_phase_team();
        // 1 apothecary initial
        assert!(matches!(
            team.dismiss_staff(
                StaffType::Apothecary,
                StaffQuantity::try_new(2).unwrap(),
                Kpo(50)
            ),
            Err(DomainError::InsufficientStaff)
        ));
    }

    #[test]
    fn dismiss_staff_met_a_jour_compteur_et_tresorerie() {
        let team = dismissals_phase_team();
        let treasury_before = team.treasury.0;

        let event = team
            .dismiss_staff(
                StaffType::Assistant,
                StaffQuantity::try_new(1).unwrap(),
                Kpo(10),
            )
            .unwrap();
        let team = team.apply(&event);

        assert_eq!(team.assistants.0, 1); // 2 - 1
        assert_eq!(team.treasury.0, treasury_before + 10);
    }

    // ── reject_enrollment ────────────────────────────────────────────────

    #[test]
    fn reject_enrollment_transitions_to_rejected() {
        let events = vec![created_event()];
        let team = Team::hydrate(&events).unwrap();
        let event = team.reject_enrollment().unwrap();
        let team = team.apply(&event);
        assert_eq!(team.participation_status, ParticipationStatus::Rejected);
    }

    #[test]
    fn cannot_reject_enrolled_team() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        assert!(team.reject_enrollment().is_err());
    }

    #[test]
    fn cannot_reject_already_rejected_team() {
        let events = vec![
            created_event(),
            TeamDomainEvent::TeamEnrollmentRejected {
                competition_id: Some(competition_id()),
                season_id: Some(season_id()),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert!(team.reject_enrollment().is_err());
    }

    #[test]
    fn cannot_enroll_rejected_team() {
        let events = vec![
            created_event(),
            TeamDomainEvent::TeamEnrollmentRejected {
                competition_id: Some(competition_id()),
                season_id: Some(season_id()),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert!(team
            .enroll(
                competition_id(),
                "Ligue".to_string(),
                season_id(),
                "Saison".to_string(),
            )
            .is_err());
    }

    #[test]
    fn start_match_reporting_from_ready_to_play() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.game_phase, Some(GamePhase::ReadyToPlay));

        let event = team.start_match_reporting(match_report_id()).unwrap();
        let team = team.apply(&event);
        assert_eq!(team.game_phase, Some(GamePhase::MatchReporting));
    }

    #[test]
    fn start_match_reporting_sets_current_match_report_id() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        let event = team.start_match_reporting(match_report_id()).unwrap();
        let team = team.apply(&event);
        assert_eq!(team.current_match_report_id, Some(match_report_id()));
    }

    #[test]
    fn post_match_sequence_clears_current_match_report_id() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted {
                match_report_id: match_report_id(),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.current_match_report_id, Some(match_report_id()));

        let event = team
            .start_post_match_sequence(MatchResult::Win, 4, Kpo(150), vec![])
            .unwrap();
        let team = team.apply(&event);
        assert_eq!(team.current_match_report_id, None);
    }

    #[test]
    fn start_match_reporting_wrong_phase_fails() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::PostMatchSequenceStarted {
                result: MatchResult::Win,
                dedicated_fans: DedicatedFans::try_new(5).unwrap(),
                treasury_income: Kpo(150),
                spp_gains: vec![],
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.game_phase, Some(GamePhase::PlayerImprovement));
        assert!(matches!(
            team.start_match_reporting(match_report_id()),
            Err(DomainError::WrongGamePhase(_))
        ));
    }

    #[test]
    fn cannot_dismiss_pending_team() {
        let events = vec![created_event()];
        let team = Team::hydrate(&events).unwrap();
        assert!(team.dismiss().is_err());
    }

    // ── hydratation séquences complètes ──────────────────────────────────

    #[test]
    fn hydrate_created_then_enrolled() {
        let events = vec![created_event(), enrolled_event()];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.participation_status, ParticipationStatus::Enrolled);
        assert_eq!(team.competition_name.as_deref(), Some("Ligue de Condate"));
        assert_eq!(team.version, 2);
    }

    #[test]
    fn hydrate_created_then_rejected() {
        let events = vec![
            created_event(),
            TeamDomainEvent::TeamEnrollmentRejected {
                competition_id: Some(competition_id()),
                season_id: Some(season_id()),
            },
        ];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.participation_status, ParticipationStatus::Rejected);
        assert_eq!(team.version, 2);
    }

    #[test]
    fn hydrate_created_enrolled_then_dismissed() {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::TeamDismissed,
        ];
        let team = Team::hydrate(&events).unwrap();
        assert_eq!(team.participation_status, ParticipationStatus::Dismissed);
        assert_eq!(team.game_phase, None);
        assert_eq!(team.version, 3);
    }
    // ── revert_post_match_sequence — compensation d'une dépublication ────────

    /// Équipe en phase Amélioration après un après-match, prête à être
    /// compensée. `fan_mod` et `gain` sont ceux du rapport.
    fn team_after_post_match(fan_mod: i8, gain: Kpo) -> Team {
        let events = vec![
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted { match_report_id: match_report_id() },
        ];
        let team = Team::hydrate(&events).unwrap();
        let started = team
            .start_post_match_sequence(MatchResult::Win, fan_mod, gain, vec![])
            .unwrap();
        Team::hydrate(&[
            created_event(),
            enrolled_event(),
            TeamDomainEvent::MatchReportingStarted { match_report_id: match_report_id() },
            started,
        ])
        .unwrap()
    }

    fn revert(team: &Team) -> Team {
        let event = team.revert_post_match_sequence(match_report_id()).unwrap();
        team.clone().apply(&event)
    }

    /// Le test décisif de la feature : l'équipe part de 2 fans, le rapport
    /// donne +100, l'écrêtage plafonne à 20. Restaurer par soustraction
    /// donnerait -80 ; seul l'instantané rend les 2 fans d'origine.
    #[test]
    fn revert_restaure_les_fans_ecretes_a_vingt() {
        let team = team_after_post_match(100, Kpo(0));
        assert_eq!(team.dedicated_fans.into_inner(), 20, "précondition : écrêtage atteint");

        let reverted = revert(&team);

        assert_eq!(reverted.dedicated_fans.into_inner(), 2);
    }

    /// Symétrique au plancher : -100 écrête à 0, et retrancher -100 donnerait
    /// 100 au lieu des 2 fans d'origine.
    #[test]
    fn revert_restaure_les_fans_apres_plancher_a_zero() {
        let team = team_after_post_match(-100, Kpo(0));
        assert_eq!(team.dedicated_fans.into_inner(), 0, "précondition : plancher atteint");

        let reverted = revert(&team);

        assert_eq!(reverted.dedicated_fans.into_inner(), 2);
    }

    #[test]
    fn revert_soustrait_le_gain_de_tresorerie() {
        let team = team_after_post_match(0, Kpo(150));
        assert_eq!(team.treasury.0, 1150);

        let reverted = revert(&team);

        assert_eq!(reverted.treasury.0, 1000);
    }

    #[test]
    fn revert_repasse_en_match_reporting_avec_le_rapport_courant() {
        let reverted = revert(&team_after_post_match(1, Kpo(100)));

        assert_eq!(reverted.game_phase, Some(GamePhase::MatchReporting));
        assert_eq!(reverted.current_match_report_id, Some(match_report_id()));
    }

    #[test]
    fn revert_refuse_si_la_phase_a_deja_avance() {
        let team = team_after_post_match(1, Kpo(100))
            .apply(&TeamDomainEvent::PlayerImprovementPhaseValidated);

        assert!(matches!(
            team.revert_post_match_sequence(match_report_id()),
            Err(DomainError::WrongGamePhase(_))
        ));
    }

    #[test]
    fn revert_refuse_un_autre_match_report_id() {
        let team = team_after_post_match(1, Kpo(100));
        let autre = MatchReportId::try_new("00000000000000000000000099").unwrap();

        assert!(matches!(
            team.revert_post_match_sequence(autre),
            Err(DomainError::NoPostMatchToRevert)
        ));
    }

    /// Règle 11 : rejouer la compensation ne doit rien produire de plus.
    #[test]
    fn un_second_revert_est_refuse() {
        let reverted = revert(&team_after_post_match(1, Kpo(100)));

        assert!(matches!(
            reverted.revert_post_match_sequence(match_report_id()),
            Err(DomainError::WrongGamePhase(_))
        ));
    }

    /// Règle 8 : le nombre de corrections n'est pas limité, donc le cycle doit
    /// converger — republier après avoir dépublié doit rendre exactement l'état
    /// de la première publication.
    #[test]
    fn publier_depublier_republier_converge_vers_le_meme_etat() {
        let apres_publication = team_after_post_match(2, Kpo(150));
        let reverted = revert(&apres_publication);

        let republie = reverted
            .clone()
            .apply(&reverted.start_post_match_sequence(MatchResult::Win, 2, Kpo(150), vec![]).unwrap());

        assert_eq!(republie.dedicated_fans, apres_publication.dedicated_fans);
        assert_eq!(republie.treasury, apres_publication.treasury);
        assert_eq!(republie.game_phase, apres_publication.game_phase);
    }
}

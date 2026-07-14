use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::match_impact::{
    CasualtyCount, FoulCount, InterceptionCount, InjuryType, MatchContext, MatchReportId,
    MatchesPlayedCount, MvpCount, PassCount, PersistentInjuryCount, PlayerInjuryRecord,
    PlayerParticipationStatus, SppEarned, StatAdjustment, TouchdownCount,
};
use crate::app::players::domain::value_objects::{
    JerseyVo, PositionNameVo, RosterLineId, SkillId, SkillName, SppCost,
};
use crate::app::shared_kernel::common_types::SpaceId;
use serde::{Deserialize, Serialize};

// ── Value objects ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spp(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueKpo(pub u32);

// ── Compétences acquises ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquiredSkill {
    pub skill_id:   SkillId,
    pub skill_name: SkillName,
    pub mode:       AcquisitionMode,
    pub spp_cost:   SppCost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcquisitionMode {
    Chosen,
    Random,
}

// ── Agrégat Player ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Player {
    pub id:              PlayerId,
    pub team_id:         TeamId,
    pub space_id:        SpaceId,
    pub position_name:  PositionNameVo,
    pub roster_line_id:  RosterLineId,
    pub jersey:          Option<JerseyVo>,
    pub base_skills:     Vec<SkillId>,
    pub acquired_skills: Vec<AcquiredSkill>,
    pub spp:             Spp,
    pub value:           ValueKpo,

    // ── Impact des rapports de match ───────────────────────────────────────────
    pub participation_status:       PlayerParticipationStatus,
    pub career_touchdowns:          TouchdownCount,
    pub career_passes:              PassCount,
    pub career_interceptions:       InterceptionCount,
    pub career_casualties:          CasualtyCount,
    pub career_mvps:                MvpCount,
    pub career_fouls:               FoulCount,
    pub career_persistent_injuries: PersistentInjuryCount,
    pub injuries:                   Vec<PlayerInjuryRecord>,
    pub stat_adjustments:           Vec<StatAdjustment>,
    pub matches_played:             MatchesPlayedCount,

    /// Version courante de l'agrégat (nombre d'events déjà appliqués) — permet à
    /// l'appelant de connaître la prochaine version à utiliser pour `append()`,
    /// même pattern que `teams::Team::version`.
    pub version: i32,
}

impl Player {
    /// Reconstruit l'état de l'agrégat en rejouant une séquence d'events.
    /// Retourne `None` si la séquence est vide.
    pub fn from_events(events: &[PlayerDomainEvent]) -> Option<Self> {
        let mut state: Option<Self> = None;
        for event in events {
            state = Self::apply(state, event);
        }
        state
    }

    fn apply(current: Option<Self>, event: &PlayerDomainEvent) -> Option<Self> {
        match event {
            PlayerDomainEvent::PlayerCreated {
                player_id, team_id, space_id, position_name, roster_line_id,
                jersey, base_skills, starting_spp, starting_value,
            } => {
                if current.is_some() {
                    return current;
                }
                Some(Self {
                    id:              player_id.clone(),
                    team_id:         team_id.clone(),
                    space_id:        space_id.clone(),
                    position_name:     position_name.clone(),
                    roster_line_id:  roster_line_id.clone(),
                    jersey:          *jersey,
                    base_skills:     base_skills.clone(),
                    acquired_skills: vec![],
                    spp:             *starting_spp,
                    value:           *starting_value,
                    participation_status:       PlayerParticipationStatus::Available,
                    career_touchdowns:          TouchdownCount::default(),
                    career_passes:              PassCount::default(),
                    career_interceptions:       InterceptionCount::default(),
                    career_casualties:          CasualtyCount::default(),
                    career_mvps:                MvpCount::default(),
                    career_fouls:               FoulCount::default(),
                    career_persistent_injuries: PersistentInjuryCount::default(),
                    injuries:                   vec![],
                    stat_adjustments:           vec![],
                    matches_played:             MatchesPlayedCount::default(),
                    version:                    1,
                })
            }
            PlayerDomainEvent::InitialSkillEarned {
                skill_id, skill_name, mode, spp_cost, value_delta, ..
            } => {
                let mut player = current?;
                player.acquired_skills.push(AcquiredSkill {
                    skill_id:   skill_id.clone(),
                    skill_name: skill_name.clone(),
                    mode:       *mode,
                    spp_cost:   *spp_cost,
                });
                player.value = ValueKpo(player.value.0 + value_delta.0);
                player.version += 1;
                Some(player)
            }

            PlayerDomainEvent::TouchdownScored { spp_earned, .. } => {
                let mut player = current?;
                player.spp = Spp(player.spp.0 + spp_earned.into_inner());
                player.career_touchdowns.0 += 1;
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::PassCompleted { spp_earned, .. } => {
                let mut player = current?;
                player.spp = Spp(player.spp.0 + spp_earned.into_inner());
                player.career_passes.0 += 1;
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::InterceptionMade { spp_earned, .. } => {
                let mut player = current?;
                player.spp = Spp(player.spp.0 + spp_earned.into_inner());
                player.career_interceptions.0 += 1;
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::CasualtyInflicted { spp_earned, .. } => {
                let mut player = current?;
                player.spp = Spp(player.spp.0 + spp_earned.into_inner());
                player.career_casualties.0 += 1;
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::MatchMvpNamed { spp_earned, .. } => {
                let mut player = current?;
                player.spp = Spp(player.spp.0 + spp_earned.into_inner());
                player.career_mvps.0 += 1;
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::FoulCommitted { .. } => {
                let mut player = current?;
                player.career_fouls.0 += 1;
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::InjurySustained { context, injury_type, .. } => {
                let mut player = current?;
                player.injuries.push(PlayerInjuryRecord {
                    injury_type: injury_type.clone(),
                    context:     context.clone(),
                });
                match injury_type {
                    InjuryType::Commotion => {}
                    InjuryType::Mort => {
                        player.participation_status = PlayerParticipationStatus::Dead;
                    }
                    InjuryType::BlessureSerieuse => {
                        player.participation_status = PlayerParticipationStatus::MissingNextGame;
                        player.career_persistent_injuries.0 += 1;
                    }
                    InjuryType::Amoche => {
                        player.participation_status = PlayerParticipationStatus::MissingNextGame;
                    }
                    InjuryType::Sequel { stat } => {
                        player.participation_status = PlayerParticipationStatus::MissingNextGame;
                        player.stat_adjustments.push(StatAdjustment { stat: *stat, malus: 1 });
                    }
                }
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::PlayerAvailabilityRestored { .. } => {
                let mut player = current?;
                if player.participation_status == PlayerParticipationStatus::MissingNextGame {
                    player.participation_status = PlayerParticipationStatus::Available;
                }
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::MatchConcluded { .. } => {
                let mut player = current?;
                player.matches_played.0 += 1;
                player.version += 1;
                Some(player)
            }
        }
    }

    // ── Méthodes de commande — infaillibles, aucune garde métier (BR14) ─────────
    // Ne construisent que l'event : toute la logique vit dans apply() ci-dessus.

    pub fn record_touchdown(&self, context: MatchContext, spp_earned: SppEarned) -> PlayerDomainEvent {
        PlayerDomainEvent::TouchdownScored {
            player_id: self.id.clone(), team_id: self.team_id.clone(), context, spp_earned,
        }
    }
    pub fn record_pass(&self, context: MatchContext, spp_earned: SppEarned) -> PlayerDomainEvent {
        PlayerDomainEvent::PassCompleted {
            player_id: self.id.clone(), team_id: self.team_id.clone(), context, spp_earned,
        }
    }
    pub fn record_interception(&self, context: MatchContext, spp_earned: SppEarned) -> PlayerDomainEvent {
        PlayerDomainEvent::InterceptionMade {
            player_id: self.id.clone(), team_id: self.team_id.clone(), context, spp_earned,
        }
    }
    pub fn record_casualty(&self, context: MatchContext, spp_earned: SppEarned) -> PlayerDomainEvent {
        PlayerDomainEvent::CasualtyInflicted {
            player_id: self.id.clone(), team_id: self.team_id.clone(), context, spp_earned,
        }
    }
    pub fn record_mvp(&self, context: MatchContext, spp_earned: SppEarned) -> PlayerDomainEvent {
        PlayerDomainEvent::MatchMvpNamed {
            player_id: self.id.clone(), team_id: self.team_id.clone(), context, spp_earned,
        }
    }
    pub fn record_foul(&self, context: MatchContext) -> PlayerDomainEvent {
        PlayerDomainEvent::FoulCommitted {
            player_id: self.id.clone(), team_id: self.team_id.clone(), context,
        }
    }
    pub fn record_injury(&self, context: MatchContext, injury_type: InjuryType) -> PlayerDomainEvent {
        PlayerDomainEvent::InjurySustained {
            player_id: self.id.clone(), team_id: self.team_id.clone(), context, injury_type,
        }
    }
    pub fn restore_availability(&self, match_report_id: MatchReportId) -> PlayerDomainEvent {
        PlayerDomainEvent::PlayerAvailabilityRestored {
            player_id: self.id.clone(), team_id: self.team_id.clone(), match_report_id,
        }
    }
    pub fn record_match_concluded(&self, context: MatchContext, team_score: u8, opponent_score: u8) -> PlayerDomainEvent {
        PlayerDomainEvent::MatchConcluded {
            player_id: self.id.clone(), team_id: self.team_id.clone(), context, team_score, opponent_score,
        }
    }
}

#[cfg(test)]
mod match_impact_tests {
    use super::*;
    use crate::app::players::domain::match_impact::{RoundId, StatKind};

    fn sample_player() -> Player {
        let created = PlayerDomainEvent::PlayerCreated {
            player_id:      PlayerId("p1".into()),
            team_id:        TeamId("t1".into()),
            space_id:       SpaceId::new(),
            position_name:  PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey:         None,
            base_skills:    vec![],
            starting_spp:   Spp(0),
            starting_value: ValueKpo(100_000),
        };
        Player::from_events(&[created]).unwrap()
    }

    fn sample_context() -> MatchContext {
        MatchContext {
            match_report_id:    MatchReportId("mr1".into()),
            round_id:           RoundId("r1".into()),
            round_label:        "Journée 5".into(),
            opponent_team_id:   TeamId("opponent".into()),
            opponent_team_name: "Bone Crushers".into(),
        }
    }

    #[test]
    fn touchdown_credits_spp_and_increments_counter() {
        let player = sample_player();
        let event = player.record_touchdown(sample_context(), SppEarned::try_new(3).unwrap());
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.spp.0, 3);
        assert_eq!(player.career_touchdowns.0, 1);
    }

    #[test]
    fn foul_increments_counter_without_spp() {
        let player = sample_player();
        let event = player.record_foul(sample_context());
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.spp.0, 0);
        assert_eq!(player.career_fouls.0, 1);
    }

    #[test]
    fn commotion_is_logged_without_status_or_counter_change() {
        let player = sample_player();
        let event = player.record_injury(sample_context(), InjuryType::Commotion);
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.injuries.len(), 1);
        assert_eq!(player.participation_status, PlayerParticipationStatus::Available);
        assert_eq!(player.career_persistent_injuries.0, 0);
        assert!(player.stat_adjustments.is_empty());
    }

    #[test]
    fn death_sets_dead_status() {
        let player = sample_player();
        let event = player.record_injury(sample_context(), InjuryType::Mort);
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.participation_status, PlayerParticipationStatus::Dead);
    }

    #[test]
    fn serious_injury_sets_missing_next_game_and_increments_persistent_counter() {
        let player = sample_player();
        let event = player.record_injury(sample_context(), InjuryType::BlessureSerieuse);
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.participation_status, PlayerParticipationStatus::MissingNextGame);
        assert_eq!(player.career_persistent_injuries.0, 1);
        assert!(player.stat_adjustments.is_empty());
    }

    #[test]
    fn amoche_sets_missing_next_game_without_counter_or_adjustment() {
        let player = sample_player();
        let event = player.record_injury(sample_context(), InjuryType::Amoche);
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.participation_status, PlayerParticipationStatus::MissingNextGame);
        assert_eq!(player.career_persistent_injuries.0, 0);
        assert!(player.stat_adjustments.is_empty());
    }

    #[test]
    fn sequel_sets_missing_next_game_and_adds_stat_adjustment_without_persistent_counter() {
        let player = sample_player();
        let event = player.record_injury(
            sample_context(),
            InjuryType::Sequel { stat: StatKind::Ag },
        );
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.participation_status, PlayerParticipationStatus::MissingNextGame);
        assert_eq!(player.career_persistent_injuries.0, 0);
        assert_eq!(player.stat_adjustments.len(), 1);
        assert_eq!(player.stat_adjustments[0].stat, StatKind::Ag);
    }

    #[test]
    fn availability_restored_only_changes_missing_next_game_players() {
        let player = sample_player();
        assert_eq!(player.participation_status, PlayerParticipationStatus::Available);
        let event = player.restore_availability(MatchReportId("mr2".into()));
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.participation_status, PlayerParticipationStatus::Available);
    }

    #[test]
    fn availability_restored_lifts_missing_next_game_to_available() {
        let player = sample_player();
        let injury_event = player.record_injury(sample_context(), InjuryType::Amoche);
        let player = Player::apply(Some(player), &injury_event).unwrap();
        assert_eq!(player.participation_status, PlayerParticipationStatus::MissingNextGame);

        let event = player.restore_availability(MatchReportId("mr2".into()));
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.participation_status, PlayerParticipationStatus::Available);
    }

    #[test]
    fn availability_restored_does_not_affect_dead_players() {
        let player = sample_player();
        let injury_event = player.record_injury(sample_context(), InjuryType::Mort);
        let player = Player::apply(Some(player), &injury_event).unwrap();
        assert_eq!(player.participation_status, PlayerParticipationStatus::Dead);

        let event = player.restore_availability(MatchReportId("mr2".into()));
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.participation_status, PlayerParticipationStatus::Dead);
    }

    #[test]
    fn match_concluded_increments_matches_played() {
        let player = sample_player();
        assert_eq!(player.matches_played.0, 0);
        let event = player.record_match_concluded(sample_context(), 2, 1);
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.matches_played.0, 1);
    }

    #[test]
    fn match_concluded_does_not_affect_other_counters() {
        let player = sample_player();
        let event = player.record_match_concluded(sample_context(), 2, 1);
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.career_touchdowns.0, 0);
        assert_eq!(player.spp.0, 0);
        assert_eq!(player.participation_status, PlayerParticipationStatus::Available);
    }
}

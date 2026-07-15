use crate::app::players::domain::error::DomainError;
use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::match_impact::{
    CasualtyCount, FoulCount, InterceptionCount, InjuryType, MatchContext, MatchReportId,
    MatchesPlayedCount, MvpCount, PassCount, PersistentInjuryCount, PlayerInjuryRecord,
    PlayerParticipationStatus, SppEarned, StatAdjustment, StatKind, TouchdownCount,
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
    pub value_delta: ValueKpo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcquisitionMode {
    Chosen,
    Random,
}

// ── Augmentations de caractéristiques ───────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StatIncrease {
    pub stat:        StatKind,
    pub spp_cost:    SppCost,
    pub value_delta: ValueKpo,
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
    pub stat_increases:  Vec<StatIncrease>,
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
                    stat_increases:  vec![],
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
                    value_delta: *value_delta,
                });
                player.value = ValueKpo(player.value.0 + value_delta.0);
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::PlayerSkillPurchased {
                skill_id, skill_name, mode, spp_cost, value_delta, ..
            } => {
                let mut player = current?;
                player.acquired_skills.push(AcquiredSkill {
                    skill_id:   skill_id.clone(),
                    skill_name: skill_name.clone(),
                    mode:       *mode,
                    spp_cost:   *spp_cost,
                    value_delta: *value_delta,
                });
                player.value = ValueKpo(player.value.0 + value_delta.0);
                player.version += 1;
                Some(player)
            }
            PlayerDomainEvent::PlayerStatIncreased { stat, spp_cost, value_delta, .. } => {
                let mut player = current?;
                player.stat_increases.push(StatIncrease {
                    stat: *stat,
                    spp_cost: *spp_cost,
                    value_delta: *value_delta,
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

    // ── Dépense de SPP (phase PlayerImprovement) — méthodes faillibles ──────────
    // Exception à BR14 : ces 2 méthodes portent de vraies gardes métier (SPP
    // insuffisant, compétence déjà possédée), contrairement aux méthodes
    // ci-dessus qui ne font qu'enregistrer des faits déjà validés par match_report.

    /// SPP encore disponibles — dérivé, jamais stocké (cohérent avec l'event sourcing).
    pub fn spp_remaining(&self) -> u32 {
        let spent: u32 = self.acquired_skills.iter().map(|s| s.spp_cost.into_inner() as u32).sum::<u32>()
            + self.stat_increases.iter().map(|s| s.spp_cost.into_inner() as u32).sum::<u32>();
        self.spp.0.saturating_sub(spent)
    }

    /// Niveau de la prochaine amélioration dans la matrice de coût — compteur
    /// unique partagé entre compétences et caractéristiques, plafonné à 6.
    pub fn next_improvement_level(&self) -> u8 {
        ((self.acquired_skills.len() + self.stat_increases.len()) as u8 + 1).min(6)
    }

    pub fn purchase_skill(
        &self,
        skill_id: SkillId,
        skill_name: SkillName,
        category_css: String,
        mode: AcquisitionMode,
        spp_cost: SppCost,
        value_delta: ValueKpo,
    ) -> Result<PlayerDomainEvent, DomainError> {
        let already_acquired = self.base_skills.contains(&skill_id)
            || self.acquired_skills.iter().any(|s| s.skill_id == skill_id);
        if already_acquired {
            return Err(DomainError::SkillAlreadyAcquired);
        }
        if self.spp_remaining() < spp_cost.into_inner() as u32 {
            return Err(DomainError::InsufficientSpp);
        }
        Ok(PlayerDomainEvent::PlayerSkillPurchased {
            player_id: self.id.clone(), team_id: self.team_id.clone(),
            skill_id, skill_name, category_css, mode, spp_cost, value_delta,
        })
    }

    pub fn increase_stat(
        &self,
        stat: StatKind,
        spp_cost: SppCost,
        value_delta: ValueKpo,
    ) -> Result<PlayerDomainEvent, DomainError> {
        if self.spp_remaining() < spp_cost.into_inner() as u32 {
            return Err(DomainError::InsufficientSpp);
        }
        Ok(PlayerDomainEvent::PlayerStatIncreased {
            player_id: self.id.clone(), team_id: self.team_id.clone(),
            stat, spp_cost, value_delta,
        })
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

#[cfg(test)]
mod improvement_tests {
    use super::*;
    use crate::app::players::domain::match_impact::StatKind;

    fn sample_player_with_spp(spp: u32) -> Player {
        let created = PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()), team_id: TeamId("t1".into()), space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey: None,
            base_skills: vec![SkillId::try_new("existing-base").unwrap()],
            starting_spp: Spp(spp), starting_value: ValueKpo(100_000),
        };
        Player::from_events(&[created]).unwrap()
    }

    fn skill(id: &str) -> SkillId { SkillId::try_new(id).unwrap() }
    fn name(n: &str) -> SkillName { SkillName::try_new(n).unwrap() }

    #[test]
    fn purchase_skill_nominal_appends_skill_and_credits_value() {
        let player = sample_player_with_spp(50);
        let event = player.purchase_skill(
            skill("block"), name("Bloc"), "type-general".into(),
            AcquisitionMode::Chosen, SppCost::try_new(6).unwrap(), ValueKpo(20_000),
        ).unwrap();
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.acquired_skills.len(), 1);
        assert_eq!(player.value.0, 120_000);
        assert_eq!(player.spp_remaining(), 44);
    }

    #[test]
    fn purchase_already_base_skill_is_rejected() {
        let player = sample_player_with_spp(50);
        let result = player.purchase_skill(
            skill("existing-base"), name("Existant"), "type-general".into(),
            AcquisitionMode::Chosen, SppCost::try_new(6).unwrap(), ValueKpo(20_000),
        );
        assert!(matches!(result, Err(DomainError::SkillAlreadyAcquired)));
    }

    #[test]
    fn purchase_already_acquired_skill_is_rejected() {
        let player = sample_player_with_spp(50);
        let event = player.purchase_skill(
            skill("block"), name("Bloc"), "type-general".into(),
            AcquisitionMode::Chosen, SppCost::try_new(6).unwrap(), ValueKpo(20_000),
        ).unwrap();
        let player = Player::apply(Some(player), &event).unwrap();

        let result = player.purchase_skill(
            skill("block"), name("Bloc"), "type-general".into(),
            AcquisitionMode::Chosen, SppCost::try_new(6).unwrap(), ValueKpo(20_000),
        );
        assert!(matches!(result, Err(DomainError::SkillAlreadyAcquired)));
    }

    #[test]
    fn purchase_skill_insufficient_spp_is_rejected() {
        let player = sample_player_with_spp(5);
        let result = player.purchase_skill(
            skill("block"), name("Bloc"), "type-general".into(),
            AcquisitionMode::Chosen, SppCost::try_new(6).unwrap(), ValueKpo(20_000),
        );
        assert!(matches!(result, Err(DomainError::InsufficientSpp)));
    }

    #[test]
    fn increase_stat_nominal_credits_value_and_spends_spp() {
        let player = sample_player_with_spp(20);
        let event = player.increase_stat(StatKind::Ma, SppCost::try_new(14).unwrap(), ValueKpo(20_000)).unwrap();
        let player = Player::apply(Some(player), &event).unwrap();
        assert_eq!(player.stat_increases.len(), 1);
        assert_eq!(player.value.0, 120_000);
        assert_eq!(player.spp_remaining(), 6);
    }

    #[test]
    fn increase_stat_insufficient_spp_is_rejected() {
        let player = sample_player_with_spp(5);
        let result = player.increase_stat(StatKind::St, SppCost::try_new(14).unwrap(), ValueKpo(60_000));
        assert!(matches!(result, Err(DomainError::InsufficientSpp)));
    }

    #[test]
    fn next_improvement_level_counts_skills_and_stats_together_capped_at_6() {
        let mut player = sample_player_with_spp(1000);
        for i in 0u8..8 {
            assert_eq!(player.next_improvement_level(), (i + 1).min(6));
            let event = if i % 2 == 0 {
                player.purchase_skill(
                    skill(&format!("s{i}")), name(&format!("Skill{i}")), "type-general".into(),
                    AcquisitionMode::Chosen, SppCost::try_new(1).unwrap(), ValueKpo(0),
                ).unwrap()
            } else {
                player.increase_stat(StatKind::Pa, SppCost::try_new(1).unwrap(), ValueKpo(0)).unwrap()
            };
            player = Player::apply(Some(player), &event).unwrap();
        }
        assert_eq!(player.next_improvement_level(), 6);
    }

    #[test]
    fn spp_remaining_accounts_for_mixed_skill_and_stat_spending() {
        let player = sample_player_with_spp(100);
        let skill_event = player.purchase_skill(
            skill("block"), name("Bloc"), "type-general".into(),
            AcquisitionMode::Chosen, SppCost::try_new(6).unwrap(), ValueKpo(0),
        ).unwrap();
        let player = Player::apply(Some(player), &skill_event).unwrap();
        let stat_event = player.increase_stat(StatKind::Ag, SppCost::try_new(14).unwrap(), ValueKpo(0)).unwrap();
        let player = Player::apply(Some(player), &stat_event).unwrap();
        assert_eq!(player.spp_remaining(), 80);
    }
}

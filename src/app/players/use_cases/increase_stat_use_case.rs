use crate::app::players::domain::error::DomainError;
use crate::app::players::ports::{IPlayerRepository, ISkillCatalogPort, RepositoryError};
use crate::app::players::use_cases::commands::IncreaseStatCommand;
use crate::app::players::use_cases::improvement_cost_service::resolve_stat_cost;
use crate::common::services::event_bus::event_bus::EventBus;

pub enum IncreaseStatError {
    PlayerNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: IncreaseStatCommand,
    player_repo: &dyn IPlayerRepository,
    catalog: &dyn ISkillCatalogPort,
    event_bus: &EventBus,
) -> Result<(), IncreaseStatError> {
    let player = player_repo
        .find_by_id(&cmd.player_id)
        .await
        .map_err(IncreaseStatError::Repository)?
        .ok_or(IncreaseStatError::PlayerNotFound)?;

    let level = player.next_improvement_level();
    let (cost, value_delta) = resolve_stat_cost(catalog, cmd.stat, level);

    let event = player
        .increase_stat(cmd.stat, cost, value_delta)
        .map_err(IncreaseStatError::Domain)?;

    player_repo
        .append(&player.id, &player.team_id, &event, player.version + 1)
        .await
        .map_err(IncreaseStatError::Repository)?;

    let _ = event_bus.send(event.to_enveloppe(&player.id.0));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::events::PlayerDomainEvent;
    use crate::app::players::domain::match_impact::StatKind;
    use crate::app::players::domain::player::{Player, PlayerId, Spp, TeamId, ValueKpo};
    use crate::app::players::domain::value_objects::{
        PositionNameVo, RosterLineId, SkillId, SkillName, SppCost,
    };
    use crate::app::players::ports::{
        PositionAccessDto, PositionCatalogEntryDto, RepositoryError, SkillCatalogEntryDto,
        SkillCostLevelDto,
    };
    use crate::app::shared_kernel::identity::ids::SpaceId;
    use std::sync::Mutex;

    struct FakeCatalog;
    impl ISkillCatalogPort for FakeCatalog {
        fn find_skill(&self, _: &str) -> Option<SkillCatalogEntryDto> {
            None
        }
        fn find_position(&self, _: &str) -> Option<PositionCatalogEntryDto> {
            None
        }
        fn position_access(&self, _: &str) -> Option<PositionAccessDto> {
            None
        }
        fn cost_for_level(&self, level: u8, _: bool) -> Option<SkillCostLevelDto> {
            Some(SkillCostLevelDto {
                level,
                chosen_primary: 0,
                chosen_secondary: 0,
                random: 0,
                characteristic: 14,
            })
        }
        fn skill_value_delta(&self, _: bool) -> u32 {
            0
        }
        fn stat_value_delta(&self, _: StatKind) -> u32 {
            20_000
        }
        fn touchdown_spp(&self) -> u8 {
            3
        }
        fn pass_spp(&self) -> u8 {
            1
        }
        fn interception_spp(&self) -> u8 {
            2
        }
        fn casualty_spp(&self) -> u8 {
            2
        }
        fn mvp_spp(&self) -> u8 {
            4
        }
    }

    /// Fake fidèle au vrai repository : stocke les events bruts, rejoue via
    /// `from_events()` à la lecture (`Player::apply` est privée, jamais
    /// appelée hors du module domaine).
    struct FakePlayerRepo(Mutex<Vec<PlayerDomainEvent>>);
    #[async_trait::async_trait]
    impl IPlayerRepository for FakePlayerRepo {
        async fn append(
            &self,
            _: &PlayerId,
            _: &TeamId,
            event: &PlayerDomainEvent,
            _version: i32,
        ) -> Result<(), RepositoryError> {
            self.0.lock().unwrap().push(event.clone());
            Ok(())
        }
        async fn find_by_id(&self, _: &PlayerId) -> Result<Option<Player>, RepositoryError> {
            Ok(Player::from_events(&self.0.lock().unwrap()))
        }
        async fn find_by_team_id(&self, _: &TeamId) -> Result<Vec<Player>, RepositoryError> {
            Ok(vec![])
        }
        async fn find_events_by_id(
            &self,
            _: &PlayerId,
        ) -> Result<Vec<PlayerDomainEvent>, RepositoryError> {
            Ok(vec![])
        }
        async fn has_spent_spp_since_match(
            &self,
            _: &TeamId,
            _: &str,
        ) -> Result<bool, RepositoryError> {
            Ok(false)
        }
    }

    fn created_event() -> PlayerDomainEvent {
        PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey: None,
            base_skills: vec![],
            starting_spp: Spp(100),
            starting_value: ValueKpo(100_000),
        }
    }

    #[tokio::test]
    async fn increase_stat_credits_value_and_appends_stat_increase() {
        let repo = FakePlayerRepo(Mutex::new(vec![created_event()]));
        let cmd = IncreaseStatCommand {
            player_id: PlayerId("p1".into()),
            stat: StatKind::Ma,
        };
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();
        execute(cmd, &repo, &FakeCatalog, &event_bus)
            .await
            .ok()
            .expect("achat valide");

        let player = Player::from_events(&repo.0.lock().unwrap()).unwrap();
        assert_eq!(player.stat_increases.len(), 1);
        assert_eq!(player.value.0, 120_000);
        assert_eq!(player.spp_remaining(), 86);
    }

    #[test]
    fn skill_and_stat_purchases_share_the_same_level_counter() {
        let player = Player::from_events(&[created_event()]).unwrap();
        let skill_event = player
            .purchase_skill(
                SkillId::try_new("block").unwrap(),
                SkillName::try_new("Bloc").unwrap(),
                "type-general".into(),
                crate::app::players::domain::player::AcquisitionMode::Chosen,
                SppCost::try_new(1).unwrap(),
                ValueKpo(0),
            )
            .unwrap();
        let player = Player::from_events(&[created_event(), skill_event]).unwrap();
        assert_eq!(player.next_improvement_level(), 2);
    }
}

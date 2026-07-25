//! Test d'intégration bout-en-bout du pipeline "dépense de SPP" côté
//! `teams` : un achat exécuté via `purchase_skill_use_case` (BC `players`)
//! publie un domain event sur son bus interne, `players_app_event_publisher`
//! le convertit en app event, et `player_improvement_listener` (BC `teams`)
//! construit `PlayerImprovementApplied` — `team_value` doit refléter la
//! valeur ajoutée. Vrai `EventBus`, vrais repositories Postgres, pas de mock.

#![cfg(test)]

use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::{AcquisitionMode, PlayerId as DomainPlayerId};
use crate::app::players::domain::value_objects::{PositionNameVo, RosterLineId, SkillId};
use crate::app::players::io::app_events::app_event_publisher::players_app_event_publisher;
use crate::app::players::io::repository::player_repository::PgPlayerRepository;
use crate::app::players::ports::IPlayerRepository;
use crate::app::players::use_cases::commands::PurchaseSkillCommand;
use crate::app::players::use_cases::purchase_skill_use_case;
use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;
use crate::app::shared_kernel::common_types::{CoachId, CompetitionId, RosterId, SeasonId, SpaceId};
use crate::app::shared_kernel::staff_counts::{ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount};
use crate::app::shared_kernel::team::TeamId as SharedTeamId;
use crate::app::teams::domain::team::TeamDomainEvent;
use crate::app::teams::domain::value_objects::{DedicatedFans, Kpo, RosterName, TeamName};
use crate::app::teams::io::app_events::player_improvement_listener;
use crate::app::teams::io::repository::team_repository::TeamRepository;
use crate::app::teams::ports::ITeamRepository;
use crate::common::services::event_bus::event_bus::new_bus;
use crate::infrastructure::players::skill_catalog_adapter::SkillCatalogAdapter;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

async fn wait_for<F, Fut>(mut check: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..50 {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("condition never satisfied within timeout");
}

async fn seed_team(team_repo: &dyn ITeamRepository, team_id: &str) -> u64 {
    let event = TeamDomainEvent::TeamCreated {
        team_id: SharedTeamId::try_new(team_id).unwrap(),
        space_id: SpaceId::new(),
        competition_id: CompetitionId::new(),
        competition_name: "Ligue de Condate".into(),
        season_id: SeasonId::new(),
        season_name: "Saison 2025".into(),
        name: TeamName::try_new("Les Korrigans FC".to_string()).unwrap(),
        logo_url: None,
        roster_id: RosterId::try_new("00000000000000000000000004").unwrap(),
        roster_name: RosterName::try_new("Elfes Sylvestres".to_string()).unwrap(),
        coach_id: CoachId::new(),
        coach_name: "Colonel Castor".into(),
        treasury: Kpo(1000),
        dedicated_fans: DedicatedFans::try_new(2).unwrap(),
        rerolls: RerollCount(3),
        apothecaries: ApothecaryCount(1),
        assistants: AssistantCount(2),
        cheerleaders: CheerleaderCount(3),
    };
    team_repo.append(team_id, &event, 0).await.unwrap()
}

async fn seed_player(player_repo: &dyn IPlayerRepository, player_id: &str, team_id: &str) {
    let created = PlayerDomainEvent::PlayerCreated {
        player_id: DomainPlayerId(player_id.into()),
        team_id: crate::app::players::domain::player::TeamId(team_id.into()),
        space_id: SpaceId::new(),
        position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
        roster_line_id: RosterLineId::try_new("HUMAN__HUMAN_LINEMAN".to_string()).unwrap(),
        jersey: None,
        base_skills: vec![],
        starting_spp: crate::app::players::domain::player::Spp(50),
        starting_value: crate::app::players::domain::player::ValueKpo(100_000),
    };
    player_repo
        .append(&DomainPlayerId(player_id.into()), &crate::app::players::domain::player::TeamId(team_id.into()), &created, 1)
        .await
        .unwrap();
}

#[sqlx::test]
async fn purchasing_a_skill_credits_team_value_via_app_event(pool: PgPool) {
    let team_repo: Arc<dyn ITeamRepository> = Arc::new(TeamRepository::new(pool.clone()));
    let player_repo: Arc<dyn IPlayerRepository> = Arc::new(PgPlayerRepository::new(pool));
    let ref_repo = Arc::new(InMemoryReferenceRepository::load_for_tests());
    let catalog = SkillCatalogAdapter::new(ref_repo);

    let internal_bus = new_bus();
    let app_event_bus = new_bus();
    players_app_event_publisher(&internal_bus, app_event_bus.clone());
    player_improvement_listener::init(&app_event_bus, team_repo.clone());

    let team_id = ulid::Ulid::new().to_string();
    let player_id = ulid::Ulid::new().to_string();
    seed_team(team_repo.as_ref(), &team_id).await;
    seed_player(player_repo.as_ref(), &player_id, &team_id).await;

    let team_before = team_repo.find_by_id(&team_id).await.unwrap().unwrap();

    let cmd = PurchaseSkillCommand {
        player_id: DomainPlayerId(player_id.clone()),
        skill_id: SkillId::try_new("BLOCK").unwrap(),
        mode: AcquisitionMode::Chosen,
    };
    if let Err(e) = purchase_skill_use_case::execute(cmd, player_repo.as_ref(), &catalog, &internal_bus).await {
        match e {
            purchase_skill_use_case::PurchaseSkillError::PlayerNotFound => panic!("PlayerNotFound"),
            purchase_skill_use_case::PurchaseSkillError::Cost(ce) => panic!("Cost: {ce:?}"),
            purchase_skill_use_case::PurchaseSkillError::Domain(de) => panic!("Domain: {de}"),
            purchase_skill_use_case::PurchaseSkillError::Repository(re) => panic!("Repository: {re}"),
        }
    }

    wait_for(|| {
        let team_repo = team_repo.clone();
        let team_id = team_id.clone();
        let baseline = team_before.team_value.0;
        async move {
            team_repo
                .find_by_id(&team_id)
                .await
                .ok()
                .flatten()
                .map(|t| t.team_value.0 > baseline)
                .unwrap_or(false)
        }
    })
    .await;

    let team_after = team_repo.find_by_id(&team_id).await.unwrap().unwrap();
    // BLOCK est primary (GENERAL) pour HUMAN_LINEMAN → +20 kPo (table officielle)
    assert_eq!(team_after.team_value.0, team_before.team_value.0 + 20);
}

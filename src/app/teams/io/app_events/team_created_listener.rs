use crate::app::shared_kernel::app_events::team_creation_app_events::TeamCreationAppEvent;
use crate::app::shared_kernel::common_types::{
    CoachId, CompetitionId, EntityId, RosterId, SeasonId, SpaceId,
};
use crate::app::shared_kernel::staff_counts::{
    ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
};
use crate::app::shared_kernel::team::TeamId;
use crate::app::teams::domain::team::TeamDomainEvent;
use crate::app::teams::domain::value_objects::{DedicatedFans, Kpo, RosterName, TeamName};
use crate::app::teams::ports::{ITeamRepository, RepositoryError};
use crate::app::teams::use_cases::approve_enrollment;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

pub fn init(app_event_bus: &EventBus, team_repo: Arc<dyn ITeamRepository>) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(app_event) =
                        serde_json::from_value::<TeamCreationAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    let TeamCreationAppEvent::TeamCreated {
                        team_id,
                        space_id,
                        competition_id,
                        competition_name,
                        season_id,
                        season_name,
                        team_name,
                        roster_id,
                        roster_name,
                        coach_id,
                        coach_name,
                        logo_url,
                        treasury,
                        rerolls,
                        apothecaries,
                        assistants,
                        cheerleaders,
                        fans_factor,
                        auto_enroll,
                        ..
                    } = app_event;
                    let Some(tid) = TeamId::try_new(&team_id).ok() else { continue };
                    let Some(sid) = SpaceId::try_new(&space_id).ok() else { continue };
                    let Some(cid) = CompetitionId::try_new(&competition_id).ok() else { continue };
                    let Some(seas_id) = SeasonId::try_new(&season_id).ok() else { continue };
                    let Some(rid) = RosterId::try_new(&roster_id).ok() else { continue };
                    let Some(coa_id) = CoachId::try_new(&coach_id).ok() else { continue };
                    let tname = TeamName::try_new(team_name)
                        .unwrap_or_else(|_| TeamName::try_new("Unknown".to_string()).unwrap());
                    let rname = RosterName::try_new(roster_name)
                        .unwrap_or_else(|_| RosterName::try_new("Unknown".to_string()).unwrap());
                    let fans = DedicatedFans::try_new((fans_factor + 1).min(20))
                        .unwrap_or_else(|_| DedicatedFans::try_new(0).unwrap());
                    let domain_event = TeamDomainEvent::TeamCreated {
                        team_id: tid,
                        space_id: sid,
                        competition_id: cid,
                        competition_name,
                        season_id: seas_id,
                        season_name,
                        name: tname,
                        logo_url,
                        roster_id: rid,
                        roster_name: rname,
                        coach_id: coa_id,
                        coach_name,
                        treasury: Kpo(treasury),
                        dedicated_fans: fans,
                        rerolls: RerollCount::new(rerolls).unwrap_or_default(),
                        apothecaries: ApothecaryCount::new(apothecaries).unwrap_or_default(),
                        assistants: AssistantCount::new(assistants).unwrap_or_default(),
                        cheerleaders: CheerleaderCount::new(cheerleaders).unwrap_or_default(),
                    };
                    tracing::info!("teams team_created_listener: received TeamCreated for {team_id}");
                    if let Err(e) = team_repo.append(&team_id, &domain_event, 0).await {
                        match e {
                            RepositoryError::ConcurrentWrite => tracing::warn!(
                                "teams team_created_listener: TeamCreated déjà persisté pour {team_id}"
                            ),
                            other => {
                                tracing::error!(
                                    "teams team_created_listener: échec append pour {team_id}: {other}"
                                );
                                continue;
                            }
                        }
                    }

                    if auto_enroll {
                        tracing::info!("teams team_created_listener: auto-enrolling {team_id}");
                        if let Ok(eid) = EntityId::try_new(&team_id) {
                            if let Err(e) =
                                approve_enrollment::execute(&eid, team_repo.as_ref()).await
                            {
                                tracing::error!(
                                    "teams team_created_listener: auto-enroll failed for {team_id}: {e:?}"
                                );
                            }
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("teams team_created_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

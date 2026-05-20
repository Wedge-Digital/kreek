use std::sync::Arc;
use crate::app::competitions::domain::cache_repository_port::{CachedSpace, CachedUser, ICompetitionsCacheRepository};
use crate::app::shared_kernel::app_events::spaces_app_events::SpacesAppEvent;
use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::lib::services::event_bus::event_bus::EventBus;

pub fn space_created_listener(bus: &EventBus, repo: Arc<dyn ICompetitionsCacheRepository>) {
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if event.event_type != SpacesAppEvent::SPACE_CREATED { continue; }
                    let repo  = Arc::clone(&repo);
                    tokio::spawn(async move {
                        let app_event = match serde_json::from_value::<SpacesAppEvent>(event.payload) {
                            Ok(s)  => s,
                            Err(e) => { tracing::error!("space_created: payload invalide: {e}"); return; }
                        };
                        let SpacesAppEvent::SpaceCreated { space_id, space_name, space_logo, created_by, .. } = app_event else { return; };
                        if let Err(e) = repo.add_space(&CachedSpace { space_id, space_name, space_logo }).await {
                            tracing::error!("space_created: failed to persist space: {e}");
                            return;
                        }
                        if let Err(e) = repo.subscribe(&created_by, &space_id, &SpaceProfile::SpaceAdmin).await {
                            tracing::error!("space_created: failed to persist founder subscription: {e}");
                        }
                    });
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("space_created_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::app::competitions::domain::cache_repository_port::CompetitionsCacheError;
    use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, EventId, SpaceId};
    use crate::app::shared_kernel::space_name::SpaceName;
    use crate::lib::event_envelope::EventEnvelope;
    use crate::lib::services::event_bus::event_bus::new_bus;
    use time::OffsetDateTime;
    use tokio::sync::Mutex;

    struct FakeRepo {
        added_spaces:  Mutex<Vec<CachedSpace>>,
        subscriptions: Mutex<Vec<(CoachId, SpaceId, SpaceProfile)>>,
    }

    #[async_trait]
    impl ICompetitionsCacheRepository for FakeRepo {
        async fn add_user(&self, _: &CachedUser)                                              -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn remove_user(&self, _: &CoachId)                                              -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn remove_space(&self, _: &SpaceId)                                             -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn unsubscribe(&self, _: &CoachId, _: &SpaceId)                                -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn list_members_for_space(&self, _: &SpaceId)                                   -> Result<Vec<CachedUser>, CompetitionsCacheError> { Ok(vec![]) }
        async fn subscribe(&self, coach: &CoachId, space: &SpaceId, profile: &SpaceProfile)  -> Result<(), CompetitionsCacheError> {
            self.subscriptions.lock().await.push((*coach, *space, profile.clone()));
            Ok(())
        }
        async fn add_space(&self, space: &CachedSpace) -> Result<(), CompetitionsCacheError> {
            self.added_spaces.lock().await.push(space.clone());
            Ok(())
        }
    }

    #[tokio::test]
    async fn space_created_event_adds_space_and_founder_subscription_to_cache() {
        let bus  = new_bus();
        let repo = Arc::new(FakeRepo {
            added_spaces:  Mutex::new(vec![]),
            subscriptions: Mutex::new(vec![]),
        });

        space_created_listener(&bus, Arc::clone(&repo) as Arc<dyn ICompetitionsCacheRepository>);

        let space_id   = SpaceId::new();
        let created_by = CoachId::new();
        let app_event  = SpacesAppEvent::SpaceCreated {
            event_id:   EventId::new(),
            space_id,
            space_name: SpaceName::try_new("TestSpace").unwrap(),
            space_logo: CloudinaryImage::try_new("https://res.cloudinary.com/demo/image/upload/sample.jpg").unwrap(),
            created_by,
        };
        let _ = bus.send(EventEnvelope {
            event_id:    EventId::new().to_string(),
            emitter:     space_id.to_string(),
            event_type:  SpacesAppEvent::SPACE_CREATED.to_string(),
            tags:        serde_json::json!({}),
            payload:     serde_json::to_value(&app_event).unwrap(),
            occurred_at: OffsetDateTime::now_utc(),
        });

        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let spaces = repo.added_spaces.lock().await;
        assert_eq!(spaces.len(), 1);
        assert_eq!(spaces[0].space_id, space_id);
        assert_eq!(spaces[0].space_name.value(), "TestSpace");

        let subs = repo.subscriptions.lock().await;
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].0, created_by);
        assert_eq!(subs[0].1, space_id);
        assert_eq!(subs[0].2, SpaceProfile::SpaceAdmin);
    }
}
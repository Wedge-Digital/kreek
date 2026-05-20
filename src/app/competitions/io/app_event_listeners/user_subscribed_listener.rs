use std::sync::Arc;
use crate::app::competitions::domain::cache_repository_port::{ICompetitionsCacheRepository, UserSubscription};
use crate::app::shared_kernel::app_events::spaces_app_events::SpacesAppEvent;
use crate::lib::services::event_bus::event_bus::EventBus;

pub fn user_subscribed_listener(bus: &EventBus, repo: Arc<dyn ICompetitionsCacheRepository>) {
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if event.event_type != SpacesAppEvent::USER_SUBSCRIBED { continue; }
                    let repo  = Arc::clone(&repo);
                    tokio::spawn(async move {
                        let sub = match serde_json::from_value::<SpacesAppEvent>(event.payload) {
                            Ok(s)  => s,
                            Err(e) => { tracing::error!("user_subscribed: payload invalide: {e}"); return; }
                        };
                        let SpacesAppEvent::UserSubscribed { user_id, space_id, space_profile, .. } = sub else { return; };
                        if let Err(e) = repo.subscribe(&user_id, &space_id, &space_profile).await {
                            tracing::error!("user_subscribed: failed to persist: {e}");
                        }
                    });
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("user_subscribed_listener: lagged by {n}");
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
    use crate::app::competitions::domain::cache_repository_port::{CachedSpace, CachedUser, CompetitionsCacheError};
    use crate::app::shared_kernel::authorization::SpaceProfile;
    use crate::app::shared_kernel::common_types::{CoachId, EventId, SpaceId};
    use crate::lib::event_envelope::EventEnvelope;
    use crate::lib::services::event_bus::event_bus::new_bus;
    use time::OffsetDateTime;
    use tokio::sync::Mutex;

    struct FakeRepo { subscriptions: Mutex<Vec<(CoachId, SpaceId, SpaceProfile)>> }

    #[async_trait]
    impl ICompetitionsCacheRepository for FakeRepo {
        async fn add_user(&self, _: &CachedUser)                                              -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn remove_user(&self, _: &CoachId)                                              -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn add_space(&self, _: &CachedSpace)                                            -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn remove_space(&self, _: &SpaceId)                                             -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn unsubscribe(&self, _: &CoachId, _: &SpaceId)                                -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn list_members_for_space(&self, _: &SpaceId)                                   -> Result<Vec<CachedUser>, CompetitionsCacheError> { Ok(vec![]) }
        async fn subscribe(&self, coach_id: &CoachId, space_id: &SpaceId, profile: &SpaceProfile) -> Result<(), CompetitionsCacheError> {
            self.subscriptions.lock().await.push((*coach_id, *space_id, profile.clone()));
            Ok(())
        }
    }

    #[tokio::test]
    async fn user_subscribed_event_registers_subscription_in_cache() {
        let bus  = new_bus();
        let repo = Arc::new(FakeRepo { subscriptions: Mutex::new(vec![]) });

        user_subscribed_listener(&bus, Arc::clone(&repo) as Arc<dyn ICompetitionsCacheRepository>);

        let coach_id  = CoachId::new();
        let space_id  = SpaceId::new();
        let app_event = SpacesAppEvent::UserSubscribed {
            event_id:      EventId::new(),
            user_id:       coach_id,
            space_id,
            space_profile: SpaceProfile::SimpleUser,
        };
        let _ = bus.send(EventEnvelope {
            event_id:    EventId::new().to_string(),
            emitter:     coach_id.to_string(),
            event_type:  SpacesAppEvent::USER_SUBSCRIBED.to_string(),
            tags:        serde_json::json!({}),
            payload:     serde_json::to_value(&app_event).unwrap(),
            occurred_at: OffsetDateTime::now_utc(),
        });

        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let subs = repo.subscriptions.lock().await;
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].0, coach_id);
        assert_eq!(subs[0].1, space_id);
        assert_eq!(subs[0].2, SpaceProfile::SimpleUser);
    }
}
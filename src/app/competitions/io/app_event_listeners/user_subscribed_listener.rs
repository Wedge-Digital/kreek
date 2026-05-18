use std::sync::{Arc, Mutex};
use crate::app::competitions::domain::cache_repository_port::{ICompetitionsCacheRepository, UserSubscription};
use crate::app::shared_kernel::app_events::spaces_app_events::SpacesAppEvent;
use crate::lib::event_envelope::EventEnvelope;
use crate::lib::services::event_bus::event_bus::EventBus;

pub fn user_subscribed_listener(app_event_bus: Arc<Mutex<EventBus>>, repo: Arc<dyn ICompetitionsCacheRepository>) {
    app_event_bus.lock().unwrap().subscribe(SpacesAppEvent::USER_SUBSCRIBED, move |event: &EventEnvelope| {
        let repo  = Arc::clone(&repo);
        let event = event.clone();

        tokio::spawn(async move {
            let sub = match serde_json::from_value::<SpacesAppEvent>(event.payload) {
                Ok(s)  => s,
                Err(e) => { tracing::error!("user_subscribed: payload invalide: {e}"); return; }
            };

            let SpacesAppEvent::UserSubscribed { user_id , space_id, space_profile, ..  } = sub else {
                return;
            };

            if let Err(e) = repo.subscribe(
                &user_id,
                &space_id,
                &space_profile).await {
                tracing::error!("user_subscribed: failed to persist: {e}");
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::app::competitions::domain::cache_repository_port::{CachedSpace, CachedUser, CompetitionsCacheError};
    use crate::app::shared_kernel::authorization::SpaceProfile;
    use crate::app::shared_kernel::common_types::{CoachId, EventId, SpaceId};
    use time::OffsetDateTime;

    struct FakeCacheRepository {
        subscriptions: Mutex<Vec<(CoachId, SpaceId, SpaceProfile)>>,
    }

    #[async_trait]
    impl ICompetitionsCacheRepository for FakeCacheRepository {
        async fn add_user(&self, _: &CachedUser)                                              -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn remove_user(&self, _: &CoachId)                                              -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn add_space(&self, _: &CachedSpace)                                            -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn remove_space(&self, _: &SpaceId)                                             -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn unsubscribe(&self, _: &CoachId, _: &SpaceId)                                -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn subscribe(&self, coach_id: &CoachId, space_id: &SpaceId, profile: &SpaceProfile) -> Result<(), CompetitionsCacheError> {
            self.subscriptions.lock().unwrap().push((*coach_id, *space_id, profile.clone()));
            Ok(())
        }
    }

    #[tokio::test]
    async fn user_subscribed_event_registers_subscription_in_cache() {
        // Given
        let app_event_bus = Arc::new(Mutex::new(EventBus::new()));
        let repo          = Arc::new(FakeCacheRepository { subscriptions: Mutex::new(vec![]) });

        user_subscribed_listener(Arc::clone(&app_event_bus), Arc::clone(&repo) as Arc<dyn ICompetitionsCacheRepository>);

        let coach_id  = CoachId::new();
        let space_id  = SpaceId::new();
        let app_event = SpacesAppEvent::UserSubscribed {
            event_id:     EventId::new(),
            user_id:      coach_id,
            space_id,
            space_profile: SpaceProfile::SimpleUser,
        };

        // When
        app_event_bus.lock().unwrap().publish(&EventEnvelope {
            event_id:    EventId::new().to_string(),
            emitter:     coach_id.to_string(),
            event_type:  SpacesAppEvent::USER_SUBSCRIBED.to_string(),
            tags:        serde_json::json!({}),
            payload:     serde_json::to_value(&app_event).unwrap(),
            occurred_at: OffsetDateTime::now_utc(),
        });

        tokio::task::yield_now().await;

        // Then
        let subs = repo.subscriptions.lock().unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].0, coach_id);
        assert_eq!(subs[0].1, space_id);
        assert_eq!(subs[0].2, SpaceProfile::SimpleUser);
    }
}
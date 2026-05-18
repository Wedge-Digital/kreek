use std::sync::{Arc, Mutex};
use crate::app::competitions::domain::cache_repository_port::{CachedUser, ICompetitionsCacheRepository};
use crate::app::shared_kernel::app_events::auth_app_events::AuthAppEvent;
use crate::app::spaces::domain::user::User;
use crate::lib::event_envelope::EventEnvelope;
use crate::lib::services::event_bus::event_bus::EventBus;

pub fn user_created_listener(app_event_bus: Arc<Mutex<EventBus>>, repo: Arc<dyn ICompetitionsCacheRepository>) {

    app_event_bus.lock().unwrap().subscribe(AuthAppEvent::ACCOUNT_CREATED, move |event: &EventEnvelope | {
        let repo  = Arc::clone(&repo);
        let event = event.clone();

        tokio::spawn(async move {
            let sub = match serde_json::from_value::<AuthAppEvent>(event.payload) {
                Ok(s)  => s,
                Err(e) => { tracing::error!("payload invalide: {e}"); return; }
            };

            let AuthAppEvent::AccountCreated { user_id, user_name, email, ..  } = sub else {
                return;
            };

            if let Err(e) = repo.add_user(&CachedUser {
                id:    user_id,
                coach_name:  user_name,
                coach_icon: None,
                email
            }).await {
                tracing::error!("failed to persist: {e}");
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::app::shared_kernel::app_events::auth_app_events::AuthAppEvent;
    use crate::app::shared_kernel::coach_name::CoachName;
    use crate::app::shared_kernel::common_types::{CoachId, EventId, SpaceId, UserId};
    use crate::app::shared_kernel::email::Email;
    use time::OffsetDateTime;
    use crate::app::competitions::domain::cache_repository_port::{CachedSpace, CompetitionsCacheError};
    use crate::app::shared_kernel::authorization::SpaceProfile;

    struct FakeUserCacheRepository {
        added: Mutex<Vec<CachedUser>>,
    }

    #[async_trait]
    impl ICompetitionsCacheRepository for FakeUserCacheRepository {
        async fn add_user(&self, user: &CachedUser) -> Result<(), CompetitionsCacheError> {
            self.added.lock().unwrap().push(user.clone());
            Ok(())
        }

        async fn remove_user(&self, _id: &CoachId)                                              -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn add_space(&self, _space: &CachedSpace)                                         -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn remove_space(&self, _space_id: &SpaceId)                                       -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn subscribe(&self, _coach_id: &CoachId, _space_id: &SpaceId, _profile: &SpaceProfile) -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn unsubscribe(&self, _coach_id: &CoachId, _space_id: &SpaceId)                   -> Result<(), CompetitionsCacheError> { Ok(()) }
    }

    #[tokio::test]
    async fn account_created_app_event_adds_user_to_cache() {
        // Given
        let app_event_bus = Arc::new(Mutex::new(EventBus::new()));
        let repo          = Arc::new(FakeUserCacheRepository { added: Mutex::new(vec![]) });

        user_created_listener(Arc::clone(&app_event_bus), Arc::clone(&repo) as Arc<dyn ICompetitionsCacheRepository>);

        let user_id   = UserId::new();
        let app_event = AuthAppEvent::AccountCreated {
            event_id:  EventId::new(),
            user_id,
            user_name: CoachName::try_new("TestCoach").unwrap(),
            email:     Email::try_new("test@example.com").unwrap(),
        };

        // When
        let envelope = crate::lib::event_envelope::EventEnvelope {
            event_id:    EventId::new().to_string(),
            emitter:     user_id.to_string(),
            event_type:  AuthAppEvent::ACCOUNT_CREATED.to_string(),
            tags:        serde_json::json!({}),
            payload:     serde_json::to_value(&app_event).unwrap(),
            occurred_at: OffsetDateTime::now_utc(),
        };
        app_event_bus.lock().unwrap().publish(&envelope);

        tokio::task::yield_now().await;

        // Then
        let added = repo.added.lock().unwrap();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].id, user_id);
        assert_eq!(added[0].coach_name.clone().into_inner(), "TestCoach");
        assert_eq!(added[0].email.value(), "test@example.com");
    }
}

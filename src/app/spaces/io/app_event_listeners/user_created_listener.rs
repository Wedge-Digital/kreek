use std::sync::{Arc, Mutex};
use crate::app::shared_kernel::app_events::auth_app_events::AuthAppEvent;
use crate::app::spaces::domain::space_repository_port::user_cache_repository_ports::ISpaceUserCacheRepository;
use crate::app::spaces::domain::user::User;
use crate::lib::event_envelope::EventEnvelope;
use crate::lib::services::event_bus::event_bus::EventBus;

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::app::shared_kernel::app_events::auth_app_events::AuthAppEvent;
    use crate::app::shared_kernel::coach_name::CoachName;
    use crate::app::shared_kernel::common_types::{CoachId, EventId, UserId};
    use crate::app::shared_kernel::email::Email;
    use crate::app::spaces::domain::space_repository_port::user_cache_repository_ports::SpaceUserCacheRepositoryError;
    use time::OffsetDateTime;

    struct FakeUserCacheRepository {
        added: Mutex<Vec<User>>,
    }

    #[async_trait]
    impl ISpaceUserCacheRepository for FakeUserCacheRepository {
        async fn add_user(&self, user: &User) -> Result<(), SpaceUserCacheRepositoryError> {
            self.added.lock().unwrap().push(user.clone());
            Ok(())
        }
        async fn find_user_by_id(&self, _: &CoachId) -> Result<User, SpaceUserCacheRepositoryError> {
            Err(SpaceUserCacheRepositoryError::UserNotFoundInCache)
        }
        async fn find_all_users(&self) -> Result<Vec<User>, SpaceUserCacheRepositoryError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn account_created_app_event_adds_user_to_cache() {
        // Given
        let app_event_bus = Arc::new(Mutex::new(EventBus::new()));
        let repo          = Arc::new(FakeUserCacheRepository { added: Mutex::new(vec![]) });

        user_created_listener(Arc::clone(&app_event_bus), Arc::clone(&repo) as Arc<dyn ISpaceUserCacheRepository>);

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
        assert_eq!(added[0].name.clone().into_inner(), "TestCoach");
        assert_eq!(added[0].email.value(), "test@example.com");
    }
}

pub fn user_created_listener(app_event_bus: Arc<Mutex<EventBus>>, repo: Arc<dyn ISpaceUserCacheRepository>) {

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

            if let Err(e) = repo.add_user(&User {
                id:    user_id,
                name:  user_name,
                email,
                icon: None
            }).await {
                tracing::error!("failed to persist: {e}");
            }
        });
    });
}
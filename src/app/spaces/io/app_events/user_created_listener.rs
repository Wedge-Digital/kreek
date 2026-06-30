use crate::app::shared_kernel::app_events::auth_app_events::AuthAppEvent;
use crate::app::spaces::domain::space_repository_port::user_cache_repository_port::ISpaceUserCacheRepository;
use crate::app::spaces::domain::user::User;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

pub fn user_created_listener(bus: &EventBus, repo: Arc<dyn ISpaceUserCacheRepository>) {
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if event.event_type != AuthAppEvent::ACCOUNT_CREATED {
                        continue;
                    }
                    let repo = Arc::clone(&repo);
                    tokio::spawn(async move {
                        let sub = match serde_json::from_value::<AuthAppEvent>(event.payload) {
                            Ok(s) => s,
                            Err(e) => {
                                tracing::error!("user_created: payload invalide: {e}");
                                return;
                            }
                        };
                        let AuthAppEvent::AccountCreated {
                            user_id,
                            user_name,
                            email,
                            ..
                        } = sub
                        else {
                            return;
                        };
                        if let Err(e) = repo
                            .add_user(&User {
                                id: user_id,
                                name: user_name,
                                email,
                                icon: None,
                            })
                            .await
                        {
                            tracing::error!("user_created: failed to persist: {e}");
                        }
                    });
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("spaces::user_created_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::app_events::auth_app_events::AuthAppEvent;
    use crate::app::shared_kernel::coach_name::CoachName;
    use crate::app::shared_kernel::common_types::{CoachId, EventId, SpaceId, UserId};
    use crate::app::shared_kernel::email::Email;
    use crate::app::spaces::domain::space_repository_port::user_cache_repository_port::SpaceUserCacheRepositoryError;
    use crate::common::event_envelope::EventEnvelope;
    use crate::common::services::event_bus::event_bus::new_bus;
    use async_trait::async_trait;
    use time::OffsetDateTime;
    use tokio::sync::Mutex;

    struct FakeUserCacheRepository {
        added: Mutex<Vec<User>>,
    }

    #[async_trait]
    impl ISpaceUserCacheRepository for FakeUserCacheRepository {
        async fn add_user(&self, user: &User) -> Result<(), SpaceUserCacheRepositoryError> {
            self.added.lock().await.push(user.clone());
            Ok(())
        }
        async fn find_user_by_id(
            &self,
            _: &CoachId,
        ) -> Result<User, SpaceUserCacheRepositoryError> {
            Err(SpaceUserCacheRepositoryError::UserNotFoundInCache)
        }
        async fn find_all_users(&self) -> Result<Vec<User>, SpaceUserCacheRepositoryError> {
            Ok(vec![])
        }
        async fn list_members_for_space(
            &self,
            _: &SpaceId,
        ) -> Result<Vec<User>, SpaceUserCacheRepositoryError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn account_created_app_event_adds_user_to_cache() {
        let bus = new_bus();
        let repo = Arc::new(FakeUserCacheRepository {
            added: Mutex::new(vec![]),
        });

        user_created_listener(
            &bus,
            Arc::clone(&repo) as Arc<dyn ISpaceUserCacheRepository>,
        );

        let user_id = UserId::new();
        let app_event = AuthAppEvent::AccountCreated {
            event_id: EventId::new(),
            user_id,
            user_name: CoachName::try_new("TestCoach").unwrap(),
            email: Email::try_new("test@example.com").unwrap(),
        };
        let _ = bus.send(EventEnvelope {
            event_id: EventId::new().to_string(),
            emitter: user_id.to_string(),
            event_type: AuthAppEvent::ACCOUNT_CREATED.to_string(),
            tags: serde_json::json!({}),
            payload: serde_json::to_value(&app_event).unwrap(),
            occurred_at: OffsetDateTime::now_utc(),
        });

        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let added = repo.added.lock().await;
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].id, user_id);
        assert_eq!(added[0].name.clone().into_inner(), "TestCoach");
        assert_eq!(added[0].email.value(), "test@example.com");
    }
}

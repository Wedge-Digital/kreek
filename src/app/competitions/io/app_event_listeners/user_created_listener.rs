use std::sync::Arc;
use crate::app::competitions::domain::cache_repository_port::{CachedUser, ICompetitionsCacheRepository};
use crate::app::shared_kernel::app_events::auth_app_events::AuthAppEvent;
use crate::lib::services::event_bus::event_bus::EventBus;

pub fn user_created_listener(bus: &EventBus, repo: Arc<dyn ICompetitionsCacheRepository>) {
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if event.event_type != AuthAppEvent::ACCOUNT_CREATED { continue; }
                    let repo  = Arc::clone(&repo);
                    tokio::spawn(async move {
                        let sub = match serde_json::from_value::<AuthAppEvent>(event.payload) {
                            Ok(s)  => s,
                            Err(e) => { tracing::error!("payload invalide: {e}"); return; }
                        };
                        let AuthAppEvent::AccountCreated { user_id, user_name, email, .. } = sub else { return; };
                        if let Err(e) = repo.add_user(&CachedUser { id: user_id, coach_name: user_name, coach_icon: None, email }).await {
                            tracing::error!("failed to persist: {e}");
                        }
                    });
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("competitions::user_created_listener: lagged by {n}");
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
    use crate::app::competitions::domain::cache_repository_port::{CachedSpace, CompetitionsCacheError};
    use crate::app::shared_kernel::app_events::auth_app_events::AuthAppEvent;
    use crate::app::shared_kernel::authorization::SpaceProfile;
    use crate::app::shared_kernel::coach_name::CoachName;
    use crate::app::shared_kernel::common_types::{CoachId, EventId, SpaceId, UserId};
    use crate::app::shared_kernel::email::Email;
    use crate::lib::event_envelope::EventEnvelope;
    use crate::lib::services::event_bus::event_bus::new_bus;
    use time::OffsetDateTime;
    use tokio::sync::Mutex;

    struct FakeRepo { added: Mutex<Vec<CachedUser>> }

    #[async_trait]
    impl ICompetitionsCacheRepository for FakeRepo {
        async fn add_user(&self, user: &CachedUser) -> Result<(), CompetitionsCacheError> {
            self.added.lock().await.push(user.clone());
            Ok(())
        }
        async fn remove_user(&self, _: &CoachId)                                                  -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn add_space(&self, _: &CachedSpace)                                                -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn remove_space(&self, _: &SpaceId)                                                 -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn subscribe(&self, _: &CoachId, _: &SpaceId, _: &SpaceProfile)                    -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn unsubscribe(&self, _: &CoachId, _: &SpaceId)                                    -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn list_members_for_space(&self, _: &SpaceId)                                       -> Result<Vec<CachedUser>, CompetitionsCacheError> { Ok(vec![]) }
    }

    #[tokio::test]
    async fn account_created_app_event_adds_user_to_cache() {
        let bus  = new_bus();
        let repo = Arc::new(FakeRepo { added: Mutex::new(vec![]) });

        user_created_listener(&bus, Arc::clone(&repo) as Arc<dyn ICompetitionsCacheRepository>);

        let user_id   = UserId::new();
        let app_event = AuthAppEvent::AccountCreated {
            event_id:  EventId::new(),
            user_id,
            user_name: CoachName::try_new("TestCoach").unwrap(),
            email:     Email::try_new("test@example.com").unwrap(),
        };
        let _ = bus.send(EventEnvelope {
            event_id:    EventId::new().to_string(),
            emitter:     user_id.to_string(),
            event_type:  AuthAppEvent::ACCOUNT_CREATED.to_string(),
            tags:        serde_json::json!({}),
            payload:     serde_json::to_value(&app_event).unwrap(),
            occurred_at: OffsetDateTime::now_utc(),
        });

        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let added = repo.added.lock().await;
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].id, user_id);
        assert_eq!(added[0].coach_name.clone().into_inner(), "TestCoach");
        assert_eq!(added[0].email.value(), "test@example.com");
    }
}
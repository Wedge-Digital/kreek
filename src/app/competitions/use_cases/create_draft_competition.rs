use std::collections::HashSet;
use crate::app::competitions::domain::cache_repository_port::{CompetitionsCacheError, ICompetitionsCacheRepository};
use crate::app::competitions::domain::competition::Competition;
use crate::app::competitions::domain::competition_repository_port::{CompetitionRepositoryError, ICompetitionRepository};
use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, CompetitionId, EventId, SpaceId};
use crate::app::shared_kernel::competition_name::CompetitionName;
use crate::lib::services::event_bus::event_bus::EventBus;

pub struct CreateDraftCompetitionCommand {
    pub space_id:   SpaceId,
    pub created_by: CoachId,
    pub name:       CompetitionName,
    pub logo:       CloudinaryImage,
    pub admin_ids:  Vec<CoachId>,
}

#[derive(Debug)]
pub enum CreateDraftCompetitionError {
    CompetitionNameAlreadyTaken,
    InvalidAdminId(CoachId),
    Database(String),
}

impl From<CompetitionRepositoryError> for CreateDraftCompetitionError {
    fn from(e: CompetitionRepositoryError) -> Self {
        match e {
            CompetitionRepositoryError::CompetitionNameAlreadyTaken => CreateDraftCompetitionError::CompetitionNameAlreadyTaken,
            CompetitionRepositoryError::Database(msg)               => CreateDraftCompetitionError::Database(msg),
        }
    }
}

impl From<CompetitionsCacheError> for CreateDraftCompetitionError {
    fn from(e: CompetitionsCacheError) -> Self {
        CreateDraftCompetitionError::Database(e.to_string())
    }
}

pub async fn execute(
    cmd:   CreateDraftCompetitionCommand,
    repo:  &dyn ICompetitionRepository,
    cache: &dyn ICompetitionsCacheRepository,
    bus:   &EventBus,
) -> Result<CompetitionId, CreateDraftCompetitionError> {
    if repo.name_exists_in_space(&cmd.name, &cmd.space_id).await? {
        return Err(CreateDraftCompetitionError::CompetitionNameAlreadyTaken);
    }

    let members = cache.list_members_for_space(&cmd.space_id).await?;
    let member_ids: HashSet<String> = members.iter().map(|m| m.id.to_string()).collect();
    for admin_id in &cmd.admin_ids {
        if !member_ids.contains(&admin_id.to_string()) {
            return Err(CreateDraftCompetitionError::InvalidAdminId(*admin_id));
        }
    }

    let competition = Competition::new(
        cmd.space_id,
        cmd.name.clone(),
        cmd.logo.clone(),
        cmd.admin_ids.clone(),
    );
    let competition_id = competition.id;

    repo.save(&competition).await?;

    let _ = bus.send(CompetitionsDomainEvent::CompetitionCreated {
        event_id:       EventId::new(),
        competition_id: competition.id,
        space_id:       competition.space_id,
        created_by:     cmd.created_by,
        name:           competition.name,
        logo:           competition.logo,
        admin_ids:      competition.admin_ids,
    }.to_enveloppe());

    Ok(competition_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::app::competitions::domain::cache_repository_port::{CachedSpace, CachedUser, CompetitionsCacheError, ICompetitionsCacheRepository};
    use crate::app::competitions::domain::competition_repository_port::{CompetitionRepositoryError, ICompetitionRepository};
    use crate::app::shared_kernel::authorization::SpaceProfile;
    use crate::app::shared_kernel::coach_icon::CoachIcon;
    use crate::app::shared_kernel::coach_name::CoachName;
    use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, SpaceId};
    use crate::app::shared_kernel::email::Email;
    use crate::lib::services::event_bus::event_bus::new_bus;

    const LOGO: &str = "https://res.cloudinary.com/demo/image/upload/sample.jpg";

    fn make_user(coach_id: CoachId) -> CachedUser {
        CachedUser {
            id:         coach_id,
            coach_name: CoachName::try_new("Coach Test").unwrap(),
            coach_icon: None,
            email:      Email::try_new("coach@test.com").unwrap(),
        }
    }

    struct FakeCompetitionRepo { pub name_taken: bool }

    #[async_trait]
    impl ICompetitionRepository for FakeCompetitionRepo {
        async fn name_exists_in_space(&self, _: &CompetitionName, _: &SpaceId) -> Result<bool, CompetitionRepositoryError> {
            Ok(self.name_taken)
        }
        async fn save(&self, _: &Competition) -> Result<(), CompetitionRepositoryError> {
            Ok(())
        }
    }

    struct FakeCache { pub members: Vec<CachedUser> }

    #[async_trait]
    impl ICompetitionsCacheRepository for FakeCache {
        async fn add_user(&self, _: &CachedUser)                                        -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn remove_user(&self, _: &CoachId)                                        -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn add_space(&self, _: &CachedSpace)                                      -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn remove_space(&self, _: &SpaceId)                                       -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn subscribe(&self, _: &CoachId, _: &SpaceId, _: &SpaceProfile)          -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn unsubscribe(&self, _: &CoachId, _: &SpaceId)                          -> Result<(), CompetitionsCacheError> { Ok(()) }
        async fn list_members_for_space(&self, _: &SpaceId)                            -> Result<Vec<CachedUser>, CompetitionsCacheError> {
            Ok(self.members.clone())
        }
    }

    fn make_cmd(admin_ids: Vec<CoachId>) -> CreateDraftCompetitionCommand {
        CreateDraftCompetitionCommand {
            space_id:   SpaceId::new(),
            created_by: CoachId::new(),
            name:       CompetitionName::try_new("Ligue Alpha").unwrap(),
            logo:       CloudinaryImage::try_new(LOGO).unwrap(),
            admin_ids,
        }
    }

    #[tokio::test]
    async fn success_emits_competition_created_event() {
        let coach_id = CoachId::new();
        let cmd      = make_cmd(vec![coach_id]);
        let cache    = FakeCache { members: vec![make_user(coach_id)] };
        let bus      = new_bus();
        let mut rx   = bus.subscribe();

        let result = execute(cmd, &FakeCompetitionRepo { name_taken: false }, &cache, &bus).await;

        assert!(result.is_ok());
        let envelope = rx.try_recv().unwrap();
        assert_eq!(envelope.event_type, "CompetitionCreated");
    }

    #[tokio::test]
    async fn rejects_duplicate_name() {
        let cmd    = make_cmd(vec![]);
        let cache  = FakeCache { members: vec![] };
        let result = execute(cmd, &FakeCompetitionRepo { name_taken: true }, &cache, &new_bus()).await;
        assert!(matches!(result, Err(CreateDraftCompetitionError::CompetitionNameAlreadyTaken)));
    }

    #[tokio::test]
    async fn rejects_unknown_admin_id() {
        let unknown_id = CoachId::new();
        let cmd        = make_cmd(vec![unknown_id]);
        let cache      = FakeCache { members: vec![] };
        let result = execute(cmd, &FakeCompetitionRepo { name_taken: false }, &cache, &new_bus()).await;
        assert!(matches!(result, Err(CreateDraftCompetitionError::InvalidAdminId(_))));
    }
}
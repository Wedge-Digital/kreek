use async_trait::async_trait;

/// Port de persistance du BC teams — implémentation en carte 29.
#[async_trait]
pub trait ITeamRepository: Send + Sync {}

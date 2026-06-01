use async_trait::async_trait;
use sqlx::PgPool;

pub struct TeamRepository {
    #[allow(dead_code)]
    pool: PgPool,
}

impl TeamRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl crate::app::teams::ports::ITeamRepository for TeamRepository {}

use crate::app::shared_kernel::authorization::SpaceAuthorization;
use crate::app::shared_kernel::coach_icon::CoachIcon;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, SpaceId};
use crate::app::shared_kernel::space_name::SpaceName;
use crate::app::spaces::domain::coach::Coach;
use crate::app::spaces::domain::Space::Space;
use crate::app::spaces::domain::ports::{ISpaceRepository, SpaceRepositoryError};
use async_trait::async_trait;
use sqlx::PgPool;

fn db_err(e: impl std::fmt::Display) -> SpaceRepositoryError {
    SpaceRepositoryError::Database(e.to_string())
}

#[derive(sqlx::FromRow)]
struct SpaceCoachRow {
    space_id:        String,
    space_name:      String,
    space_icon_path: String,
    coach_id:        Option<String>,
    coach_name:      Option<String>,
    coach_icon:      Option<String>,
    profile:         Option<String>,
}

#[derive(Clone)]
pub struct SpaceRepository {
    pool: PgPool,
}

impl SpaceRepository {
    pub fn new(pool: PgPool) -> Self {
        SpaceRepository { pool }
    }
}

#[async_trait]
impl ISpaceRepository for SpaceRepository {
    async fn save(&self, space: &Space) -> Result<(), SpaceRepositoryError> {
        sqlx::query(include_str!("sql/insert_space.sql"))
            .bind(space.id.to_string())
            .bind(space.name.as_ref())
            .bind(space.logo.as_ref())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.code().as_deref() == Some("23505") {
                        return SpaceRepositoryError::SpaceNameAlreadyTaken;
                    }
                }
                SpaceRepositoryError::Database(e.to_string())
            })?;

        Ok(())
    }

    async fn add_member(
        &self,
        space_id: &SpaceId,
        coach_id: &CoachId,
        profile: &SpaceAuthorization,
    ) -> Result<(), SpaceRepositoryError> {
        sqlx::query(include_str!("sql/add_space_member.sql"))
            .bind(space_id.to_string())
            .bind(coach_id.to_string())
            .bind(profile.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(ref db_err) = e {
                    if db_err.code().as_deref() == Some("23505") {
                        return SpaceRepositoryError::CoachAlreadyMember;
                    }
                }
                SpaceRepositoryError::Database(e.to_string())
            })?;

        Ok(())
    }

    async fn find_by_id(&self, id: &SpaceId) -> Result<Option<Space>, SpaceRepositoryError> {
        let rows = sqlx::query_as::<_, SpaceCoachRow>(include_str!("sql/find_space_by_id.sql"))
            .bind(id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

        if rows.is_empty() {
            return Ok(None);
        }

        let first = &rows[0];

        let space_id = SpaceId::from_string(&first.space_id).map_err(db_err)?;
        let space_name = SpaceName::try_new(&first.space_name).map_err(db_err)?;
        let logo = CloudinaryImage::try_new(
            first.space_icon_path.clone(),
        )
        .map_err(db_err)?;

        let mut coaches = Vec::new();
        for row in &rows {
            let (Some(ref raw_id), Some(ref raw_name), Some(ref raw_icon), Some(ref raw_profile)) =
                (&row.coach_id, &row.coach_name, &row.coach_icon, &row.profile)
            else {
                continue;
            };

            let coach_id = CoachId::from_string(raw_id).map_err(db_err)?;
            let coach_name = CoachName::try_new(raw_name.clone()).map_err(db_err)?;
            let coach_icon = CoachIcon::try_new(raw_icon.clone()).map_err(db_err)?;
            let profile = SpaceAuthorization::try_from(raw_profile.as_str())
                .map_err(SpaceRepositoryError::Database)?;

            coaches.push(Coach::new(coach_id, coach_name, profile, coach_icon));
        }

        Ok(Some(Space::new(space_id, space_name, logo, coaches)))
    }
}
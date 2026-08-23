use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::coach_icon::CoachIcon;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, SpaceId};
use crate::app::shared_kernel::identity::space_name::SpaceName;
use crate::app::spaces::domain::coach::Coach;
use crate::app::spaces::domain::space::Space;
use crate::app::spaces::domain::space_repository_port::space_repository_port::{
    ISpaceRepository, SpaceRepositoryError, SpaceSummary,
};
use async_trait::async_trait;
use sqlx::PgPool;

fn db_err(e: impl std::fmt::Display) -> SpaceRepositoryError {
    SpaceRepositoryError::Database(e.to_string())
}

#[derive(sqlx::FromRow)]
struct SpaceCoachRow {
    space_id: String,
    space_name: String,
    space_icon_path: String,
    coach_id: Option<String>,
    coach_name: Option<String>,
    coach_icon: Option<String>,
    profile: Option<String>,
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
        sqlx::query(include_str!("sql/space/insert_space.sql"))
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
        profile: &SpaceProfile,
    ) -> Result<(), SpaceRepositoryError> {
        sqlx::query(include_str!("sql/space/add_space_member.sql"))
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

    async fn join_spaces(
        &self,
        space_ids: &[SpaceId],
        coach_id: &CoachId,
    ) -> Result<(), SpaceRepositoryError> {
        let ids: Vec<String> = space_ids.iter().map(|id| id.to_string()).collect();
        sqlx::query(include_str!("sql/space/join_spaces.sql"))
            .bind(&ids)
            .bind(coach_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn find_member_profile(
        &self,
        coach_id: &CoachId,
        space_id: &SpaceId,
    ) -> Result<Option<SpaceProfile>, SpaceRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            profile: String,
        }

        let row = sqlx::query_as::<_, Row>(include_str!("sql/space/find_user_for_space.sql"))
            .bind(coach_id.to_string())
            .bind(space_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        match row {
            None => Ok(None),
            Some(r) => SpaceProfile::try_from(r.profile.as_str())
                .map(Some)
                .map_err(SpaceRepositoryError::Database),
        }
    }

    async fn find_all(&self) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            space_name: String,
            space_icon_path: String,
        }

        let rows = sqlx::query_as::<_, Row>(include_str!("sql/space/find_all_spaces.sql"))
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

        rows.into_iter()
            .map(|r| {
                Ok(SpaceSummary {
                    id: r.id,
                    name: r.space_name,
                    logo: CloudinaryImage::try_new(&r.space_icon_path).map_err(db_err)?,
                })
            })
            .collect()
    }

    async fn find_by_coach_id(
        &self,
        coach_id: &CoachId,
    ) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            space_name: String,
            space_icon_path: String,
        }

        let rows = sqlx::query_as::<_, Row>(include_str!("sql/space/find_spaces_by_coach_id.sql"))
            .bind(coach_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

        rows.into_iter()
            .map(|r| {
                Ok(SpaceSummary {
                    id: r.id,
                    name: r.space_name,
                    logo: CloudinaryImage::try_new(&r.space_icon_path).map_err(db_err)?,
                })
            })
            .collect()
    }

    async fn find_by_id(&self, id: &SpaceId) -> Result<Option<Space>, SpaceRepositoryError> {
        let rows =
            sqlx::query_as::<_, SpaceCoachRow>(include_str!("sql/space/find_space_by_id.sql"))
                .bind(id.to_string())
                .fetch_all(&self.pool)
                .await
                .map_err(db_err)?;

        if rows.is_empty() {
            return Ok(None);
        }

        let first = &rows[0];

        let space_id = SpaceId::try_new(&first.space_id).map_err(db_err)?;
        let space_name = SpaceName::try_new(&first.space_name).map_err(db_err)?;
        let logo = CloudinaryImage::try_new(first.space_icon_path.clone()).map_err(db_err)?;

        // Le `LEFT JOIN` rend une ligne à colonnes nulles pour un espace sans
        // membre. C'est `coach_id` qui distingue ce cas — jamais l'icône, qui
        // est nullable pour un membre bien réel.
        //
        // Les deux étaient confondus : le filtre exigeait les quatre colonnes,
        // et tout coach sans avatar était sauté. Sur la base de démonstration
        // où aucun des trente-huit membres n'en a, l'agrégat se chargeait
        // systématiquement vide.
        let mut coaches = Vec::new();
        for row in &rows {
            let (Some(ref raw_id), Some(ref raw_name), Some(ref raw_profile)) =
                (&row.coach_id, &row.coach_name, &row.profile)
            else {
                continue;
            };

            let coach_id = CoachId::try_new(raw_id).map_err(db_err)?;
            let coach_name = CoachName::try_new(raw_name.clone()).map_err(db_err)?;
            let profile = SpaceProfile::try_from(raw_profile.as_str())
                .map_err(SpaceRepositoryError::Database)?;

            // Une icône illisible n'est pas un membre illisible : on rend le
            // coach sans son avatar plutôt que de le faire disparaître, ce qui
            // est exactement le défaut qu'on vient de corriger.
            let coach_icon = match &row.coach_icon {
                Some(raw) => CoachIcon::try_new(raw.clone()).ok(),
                None => None,
            };

            coaches.push(Coach::new(coach_id, coach_name, profile, coach_icon));
        }

        Ok(Some(Space::new(space_id, space_name, logo, coaches)))
    }
}

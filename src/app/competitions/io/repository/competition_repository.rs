use crate::app::competitions::domain::competition::Competition;
use crate::app::competitions::domain::competition_repository_port::{
    CompetitionBaseInfo, CompetitionRepositoryError, CompetitionSummary, CompetitionWithSeasons,
    ICompetitionRepository, SeasonOption,
};
use crate::app::shared_kernel::bloodbowl::competition_name::CompetitionName;
use crate::app::shared_kernel::bloodbowl::competition_profile::CompetitionProfile;
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, SpaceId};
use async_trait::async_trait;
use sqlx::PgPool;
use std::collections::HashMap;

fn db_err(e: impl std::fmt::Display) -> CompetitionRepositoryError {
    CompetitionRepositoryError::Database(e.to_string())
}

#[derive(Clone)]
pub struct CompetitionRepository {
    pool: PgPool,
}

impl CompetitionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ICompetitionRepository for CompetitionRepository {
    async fn name_exists_in_space(
        &self,
        name: &CompetitionName,
        space_id: &SpaceId,
    ) -> Result<bool, CompetitionRepositoryError> {
        let exists: bool =
            sqlx::query_scalar(include_str!("sql/competitions/find_by_name_in_space.sql"))
                .bind(space_id.to_string())
                .bind(name.value())
                .fetch_one(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(exists)
    }

    async fn save(&self, competition: &Competition) -> Result<(), CompetitionRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;

        sqlx::query(include_str!("sql/competitions/insert_competition.sql"))
            .bind(competition.id.to_string())
            .bind(competition.space_id.to_string())
            .bind(competition.name.value())
            .bind(competition.logo.as_ref())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;

        for admin_id in &competition.admin_ids {
            sqlx::query(include_str!(
                "sql/competitions/insert_competition_member.sql"
            ))
            .bind(competition.id.to_string())
            .bind(admin_id.to_string())
            .bind(CompetitionProfile::CompetitionAdmin.as_str())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }

        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    async fn find_by_space_id(
        &self,
        space_id: &SpaceId,
    ) -> Result<Vec<CompetitionSummary>, CompetitionRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            name: String,
            logo: String,
            season_id: Option<String>,
            status: Option<String>,
            season_count: i64,
        }

        let rows = sqlx::query_as::<_, Row>(include_str!("sql/competitions/find_by_space_id.sql"))
            .bind(space_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

        Ok(rows
            .into_iter()
            .map(|r| CompetitionSummary {
                id: r.id,
                name: r.name,
                logo: r.logo,
                season_id: r.season_id,
                status: r.status,
                season_count: r.season_count,
            })
            .collect())
    }

    async fn find_space_id(
        &self,
        competition_id: &CompetitionId,
    ) -> Result<Option<String>, CompetitionRepositoryError> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT space_id FROM competitions WHERE id = $1")
                .bind(competition_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| CompetitionRepositoryError::Database(e.to_string()))?;
        Ok(row.map(|r| r.0))
    }

    async fn find_base_info(
        &self,
        competition_id: &CompetitionId,
    ) -> Result<Option<CompetitionBaseInfo>, CompetitionRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            competition_name: String,
            logo: Option<String>,
            coach_id: Option<String>,
            coach_name: Option<String>,
        }

        let rows =
            sqlx::query_as::<_, Row>(include_str!("sql/competitions/find_competition_by_id.sql"))
                .bind(competition_id.to_string())
                .fetch_all(&self.pool)
                .await
                .map_err(db_err)?;

        if rows.is_empty() {
            return Ok(None);
        }

        let name = rows[0].competition_name.clone();
        let logo = rows[0].logo.clone();
        let admin_ids = rows.iter().filter_map(|r| r.coach_id.clone()).collect();
        let admin_names = rows.into_iter().filter_map(|r| r.coach_name).collect();

        Ok(Some(CompetitionBaseInfo {
            name,
            logo,
            admin_ids,
            admin_names,
        }))
    }

    async fn find_with_seasons(
        &self,
        space_id: &SpaceId,
    ) -> Result<Vec<CompetitionWithSeasons>, CompetitionRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            competition_id: String,
            competition_name: String,
            season_id: String,
            season_name: String,
            status: String,
        }

        let rows = sqlx::query_as::<_, Row>(include_str!(
            "sql/competitions/find_competitions_with_seasons.sql"
        ))
        .bind(space_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        // Le `Vec` garde l'ordre du SQL — `c.name ASC` pour les compétitions,
        // `s.created_at DESC` pour les saisons de chacune — et l'index dit où
        // ranger une saison sans reparcourir ce qui précède.
        //
        // La recherche linéaire d'avant coûtait O(N×M). L'échelle actuelle ne
        // s'en plaignait pas ; c'est le motif qu'on retire, pas une lenteur
        // constatée (carte 06).
        let mut index: HashMap<String, usize> = HashMap::new();
        let mut result: Vec<CompetitionWithSeasons> = vec![];
        for r in rows {
            let saison = SeasonOption {
                season_id: r.season_id,
                season_name: r.season_name,
                status: r.status,
            };
            match index.get(&r.competition_id) {
                Some(&i) => result[i].seasons.push(saison),
                None => {
                    index.insert(r.competition_id.clone(), result.len());
                    result.push(CompetitionWithSeasons {
                        competition_id: r.competition_id,
                        competition_name: r.competition_name,
                        seasons: vec![saison],
                    });
                }
            }
        }

        Ok(result)
    }

    async fn update_base_info(
        &self,
        competition_id: &CompetitionId,
        name: &CompetitionName,
        logo: &CloudinaryImage,
        admin_ids: &[CoachId],
    ) -> Result<(), CompetitionRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;

        let found: Option<String> =
            sqlx::query_scalar(include_str!("sql/competitions/update_base_info.sql"))
                .bind(name.value())
                .bind(logo.as_ref())
                .bind(competition_id.to_string())
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?;

        if found.is_none() {
            return Err(CompetitionRepositoryError::CompetitionNotFound);
        }

        sqlx::query("DELETE FROM competitions_members WHERE competition_id = $1")
            .bind(competition_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;

        for admin_id in admin_ids {
            sqlx::query(include_str!(
                "sql/competitions/insert_competition_member.sql"
            ))
            .bind(competition_id.to_string())
            .bind(admin_id.to_string())
            .bind(CompetitionProfile::CompetitionAdmin.as_str())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }

        tx.commit().await.map_err(db_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    // `SpaceId` est un ULID : un « space-1 » lisible échoue à sa validation.
    const ESPACE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    const AUTRE_ESPACE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";

    async fn competition(pool: &PgPool, id: &str, space_id: &str, name: &str) {
        sqlx::query(
            "INSERT INTO competitions (id, space_id, name, logo) VALUES ($1, $2, $3, 'logo.png')",
        )
        .bind(id)
        .bind(space_id)
        .bind(name)
        .execute(pool)
        .await
        .expect("insertion de la compétition de test");
    }

    async fn saison(pool: &PgPool, id: &str, competition_id: &str, nom: &str, cree_a: i64) {
        sqlx::query(
            "INSERT INTO competition_seasons (id, competition_id, name, status, created_at)
             VALUES ($1, $2, $3, 'ready', $4)",
        )
        .bind(id)
        .bind(competition_id)
        .bind(nom)
        .bind(time::OffsetDateTime::from_unix_timestamp(cree_a).unwrap())
        .execute(pool)
        .await
        .expect("insertion de la saison de test");
    }

    /// Le groupement n'avait **aucun test** avant la carte 06 : ni son résultat,
    /// ni les deux ordres qu'il doit préserver. La recherche linéaire qu'il
    /// remplace aurait pu être remplacée par n'importe quoi sans que rien ne
    /// proteste.
    ///
    /// Les deux ordres viennent du SQL — `c.name ASC`, puis `s.created_at DESC`
    /// à l'intérieur de chaque compétition — et l'indexation doit les rendre
    /// tels quels.
    #[sqlx::test]
    async fn les_saisons_sont_groupees_par_competition_dans_l_ordre_du_sql(pool: PgPool) {
        // Insérées à l'envers de l'ordre attendu, pour que le test échoue si
        // l'ordre venait de l'insertion plutôt que du SQL.
        competition(&pool, "c-zeta", ESPACE, "Zeta").await;
        competition(&pool, "c-alpha", ESPACE, "Alpha").await;
        competition(&pool, "c-ailleurs", AUTRE_ESPACE, "Ailleurs").await;

        saison(&pool, "s-z1", "c-zeta", "Zeta S1", 1_700_000_000).await;
        saison(
            &pool,
            "s-a-vieille",
            "c-alpha",
            "Alpha vieille",
            1_700_000_000,
        )
        .await;
        saison(
            &pool,
            "s-a-recente",
            "c-alpha",
            "Alpha récente",
            1_800_000_000,
        )
        .await;
        saison(&pool, "s-hors", "c-ailleurs", "Hors espace", 1_700_000_000).await;

        let repo = CompetitionRepository::new(pool.clone());
        let trouve = repo
            .find_with_seasons(&SpaceId::try_new(ESPACE).unwrap())
            .await
            .expect("la lecture doit réussir");

        assert_eq!(
            trouve
                .iter()
                .map(|c| c.competition_name.as_str())
                .collect::<Vec<_>>(),
            vec!["Alpha", "Zeta"],
            "les compétitions suivent `c.name ASC`, et l'autre espace est exclu"
        );
        assert_eq!(
            trouve[0]
                .seasons
                .iter()
                .map(|s| s.season_name.as_str())
                .collect::<Vec<_>>(),
            vec!["Alpha récente", "Alpha vieille"],
            "les saisons suivent `s.created_at DESC` dans leur compétition"
        );
        assert_eq!(trouve[1].seasons.len(), 1);
    }

    /// Une compétition sans saison ne sort pas : la requête est une jointure
    /// interne. Constaté plutôt que voulu — le noter évite qu'on croie à un
    /// oubli du groupement.
    #[sqlx::test]
    async fn une_competition_sans_saison_n_apparait_pas(pool: PgPool) {
        competition(&pool, "c-vide", ESPACE, "Sans saison").await;

        let repo = CompetitionRepository::new(pool.clone());
        let trouve = repo
            .find_with_seasons(&SpaceId::try_new(ESPACE).unwrap())
            .await
            .unwrap();

        assert!(trouve.is_empty());
    }
}

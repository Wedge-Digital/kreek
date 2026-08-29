use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::competition_notifications::CompetitionNotifications;
use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::competitions::domain::competition_season::CompetitionSeason;
use crate::app::competitions::domain::competition_structure::CompetitionStructure;
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonBaseInfo, SeasonFull, SeasonRepositoryError,
};
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use async_trait::async_trait;
use sqlx::PgPool;

fn db_err(e: impl std::fmt::Display) -> SeasonRepositoryError {
    SeasonRepositoryError::Database(e.to_string())
}

#[derive(Clone)]
pub struct SeasonRepository {
    pool: PgPool,
}

impl SeasonRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ISeasonRepository for SeasonRepository {
    async fn save(&self, season: &CompetitionSeason) -> Result<(), SeasonRepositoryError> {
        sqlx::query(include_str!("sql/seasons/insert_season.sql"))
            .bind(season.id.to_string())
            .bind(season.competition_id.to_string())
            .bind(season.name.as_ref())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn find_latest_season_id(
        &self,
        competition_id: &CompetitionId,
    ) -> Result<Option<SeasonId>, SeasonRepositoryError> {
        let id: Option<String> =
            sqlx::query_scalar(include_str!("sql/seasons/find_latest_season_id.sql"))
                .bind(competition_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        Ok(id.and_then(|s| SeasonId::try_new(&s).ok()))
    }

    async fn find_space_id(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<String>, SeasonRepositoryError> {
        // Le saut, en une jointure : la saison hérite de l'espace de sa
        // compétition et n'en porte pas.
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT c.space_id
               FROM competition_seasons s
               JOIN competitions c ON c.id = s.competition_id
              WHERE s.id = $1",
        )
        .bind(season_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;
        Ok(row.map(|r| r.0))
    }

    async fn find_base_info(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<SeasonBaseInfo>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            name: String,
        }

        let row: Option<Row> =
            sqlx::query_as::<_, Row>(include_str!("sql/seasons/find_base_info.sql"))
                .bind(season_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        Ok(row.map(|r| SeasonBaseInfo { name: r.name }))
    }

    async fn find_rules(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<CompetitionRules>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            rules: Option<String>,
        }

        let row: Option<Row> =
            sqlx::query_as::<_, Row>(include_str!("sql/seasons/select_rules.sql"))
                .bind(season_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        let Some(Some(json)) = row.map(|r| r.rules) else {
            return Ok(None);
        };

        serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))
    }

    async fn save_rules(
        &self,
        season_id: &SeasonId,
        name: &str,
        rules: &CompetitionRules,
    ) -> Result<(), SeasonRepositoryError> {
        let json = serde_json::to_string(rules)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;

        let found: Option<String> =
            sqlx::query_scalar(include_str!("sql/seasons/update_rules.sql"))
                .bind(name)
                .bind(json)
                .bind(season_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        if found.is_none() {
            return Err(SeasonRepositoryError::SeasonNotFound);
        }
        Ok(())
    }

    async fn find_structure(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<CompetitionStructure>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            structure: Option<String>,
        }

        let row: Option<Row> =
            sqlx::query_as::<_, Row>(include_str!("sql/seasons/select_structure.sql"))
                .bind(season_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        let Some(Some(json)) = row.map(|r| r.structure) else {
            return Ok(None);
        };

        serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))
    }

    async fn save_structure(
        &self,
        season_id: &SeasonId,
        structure: &CompetitionStructure,
    ) -> Result<(), SeasonRepositoryError> {
        let json = serde_json::to_string(structure)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;

        let found: Option<String> =
            sqlx::query_scalar(include_str!("sql/seasons/update_structure.sql"))
                .bind(json)
                .bind(season_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        if found.is_none() {
            return Err(SeasonRepositoryError::SeasonNotFound);
        }
        Ok(())
    }

    async fn save_structure_and_prune_groups(
        &self,
        season_id: &SeasonId,
        structure: &CompetitionStructure,
        kept_ids: &[String],
    ) -> Result<u64, SeasonRepositoryError> {
        let json = serde_json::to_string(structure)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;

        let mut tx = self.pool.begin().await.map_err(db_err)?;

        // `update_structure_keep_status.sql` et non `update_structure.sql` :
        // celui-ci pose `status = 'structure_selected'`, ce qui ferait régresser
        // une saison en cours sous `ready`.
        let found: Option<String> =
            sqlx::query_scalar(include_str!("sql/seasons/update_structure_keep_status.sql"))
                .bind(json)
                .bind(season_id.to_string())
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?;

        if found.is_none() {
            return Err(SeasonRepositoryError::SeasonNotFound);
        }

        // Compté **avant** la suppression : après, la cascade a déjà fait
        // disparaître les lignes, et `rows_affected()` du `DELETE` ne compte que
        // les poules, jamais les affectations qu'il emporte.
        let defaites: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM competition_group_teams t
             JOIN competition_groups g ON g.id = t.group_id
             WHERE g.season_id = $1 AND NOT (g.id = ANY($2))",
        )
        .bind(season_id.to_string())
        .bind(kept_ids)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?;

        // `NOT (id = ANY($2))` supporte la liste vide : retirer **toutes** les
        // poules n'est donc pas un cas particulier, tout part.
        sqlx::query("DELETE FROM competition_groups WHERE season_id = $1 AND NOT (id = ANY($2))")
            .bind(season_id.to_string())
            .bind(kept_ids)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;

        // **Les poules gardées sont projetées ici, et pas plus tard.**
        //
        // `ensure_groups_from_structure` fait le même travail, mais seulement à
        // l'ouverture de l'onglet Poules. Sans cette écriture, un renommage
        // n'atteindrait la table qu'au prochain passage de quelqu'un sur cet
        // onglet : la déclaration dirait « Poule renommée », la table
        // « Poule 1 », et les deux écrans se contrediraient sans que rien ne le
        // signale. Une poule neuve, elle, n'existerait qu'en déclaration — donc
        // sans pouvoir recevoir d'équipe.
        //
        // Constaté à l'écran : le retrait fonctionnait, le renommage non.
        for (position, groupe) in structure.ranking_group.groups().iter().enumerate() {
            sqlx::query(
                "INSERT INTO competition_groups (id, season_id, name, position)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, position = EXCLUDED.position",
            )
            .bind(groupe.id.as_ref())
            .bind(season_id.to_string())
            .bind(groupe.name.as_ref())
            .bind(position as i32)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }

        tx.commit().await.map_err(db_err)?;
        Ok(defaites as u64)
    }

    async fn find_invitations(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<CompetitionInvitations>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            invitations: Option<String>,
        }

        let row: Option<Row> =
            sqlx::query_as::<_, Row>(include_str!("sql/seasons/select_invitations.sql"))
                .bind(season_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        let Some(Some(json)) = row.map(|r| r.invitations) else {
            return Ok(None);
        };

        serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))
    }

    async fn save_invitations(
        &self,
        season_id: &SeasonId,
        invitations: &CompetitionInvitations,
        notifications: &CompetitionNotifications,
    ) -> Result<(), SeasonRepositoryError> {
        let json = serde_json::to_string(invitations)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;
        let json_notifs = serde_json::to_string(notifications)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;

        let found: Option<String> =
            sqlx::query_scalar(include_str!("sql/seasons/update_invitations.sql"))
                .bind(json)
                .bind(json_notifs)
                .bind(season_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        if found.is_none() {
            return Err(SeasonRepositoryError::SeasonNotFound);
        }
        Ok(())
    }

    async fn save_visibility(
        &self,
        season_id: &SeasonId,
        invitations: &CompetitionInvitations,
    ) -> Result<(), SeasonRepositoryError> {
        let json = serde_json::to_string(invitations)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;

        let found: Option<String> =
            sqlx::query_scalar(include_str!("sql/seasons/update_visibility.sql"))
                .bind(json)
                .bind(season_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        if found.is_none() {
            return Err(SeasonRepositoryError::SeasonNotFound);
        }
        Ok(())
    }

    async fn find_notifications(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<CompetitionNotifications>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            notifications: Option<String>,
        }

        let row: Option<Row> =
            sqlx::query_as::<_, Row>(include_str!("sql/seasons/select_notifications.sql"))
                .bind(season_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        // Saison inconnue et colonne `NULL` se répondent pareil : dans les deux
        // cas il n'y a pas de réglage enregistré, et c'est au domaine de dire
        // ce que vaut son absence.
        let Some(Some(json)) = row.map(|r| r.notifications) else {
            return Ok(None);
        };

        serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))
    }

    async fn save_notifications(
        &self,
        season_id: &SeasonId,
        notifications: &CompetitionNotifications,
    ) -> Result<(), SeasonRepositoryError> {
        let json = serde_json::to_string(notifications)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;

        let found: Option<String> =
            sqlx::query_scalar(include_str!("sql/seasons/update_notifications.sql"))
                .bind(json)
                .bind(season_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        if found.is_none() {
            return Err(SeasonRepositoryError::SeasonNotFound);
        }
        Ok(())
    }

    async fn find_full(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<SeasonFull>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            season_id: String,
            season_name: String,
            status: String,
            rules: Option<String>,
            structure: Option<String>,
            competition_id: String,
            competition_name: String,
        }

        let row: Option<Row> = sqlx::query_as(include_str!("sql/seasons/find_season_full.sql"))
            .bind(season_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        let Some(r) = row else {
            tracing::warn!(
                "find_full: season {} not found in competition_seasons",
                season_id
            );
            return Ok(None);
        };

        let rules = r
            .rules
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());
        let structure = r
            .structure
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());

        Ok(Some(SeasonFull {
            season_id: r.season_id,
            season_name: r.season_name,
            status: r.status,
            competition_id: r.competition_id,
            competition_name: r.competition_name,
            rules,
            structure,
        }))
    }

    async fn set_ready(&self, season_id: &SeasonId) -> Result<(), SeasonRepositoryError> {
        let found: Option<String> = sqlx::query_scalar(include_str!("sql/seasons/set_ready.sql"))
            .bind(season_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        if found.is_none() {
            return Err(SeasonRepositoryError::SeasonNotFound);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::season_name::SeasonName;

    /// Les saisons de test naissent par le vrai dépôt, donc par
    /// `insert_season.sql` — celui qui n'écrit que trois colonnes. C'est tout
    /// l'intérêt : un test qui poserait `notifications` lui-même ne dirait rien
    /// du chemin réel.
    async fn semer_saison(pool: &PgPool) -> SeasonId {
        let competition_id = CompetitionId::new();
        sqlx::query(
            "INSERT INTO competitions (id, space_id, name, logo) VALUES ($1, $2, 'Coupe', '')",
        )
        .bind(competition_id.to_string())
        .bind(crate::app::shared_kernel::identity::ids::SpaceId::new().to_string())
        .execute(pool)
        .await
        .expect("insertion de la compétition");

        let saison = CompetitionSeason::new(
            competition_id,
            SeasonName::try_new("Saison 1").expect("nom de saison"),
        );
        SeasonRepository::new(pool.clone())
            .save(&saison)
            .await
            .expect("insertion de la saison");
        saison.id
    }

    /// **La seconde moitié de R8**, et la preuve que le `NOT NULL` de la carte
    /// 366 ne casse pas la création : `insert_season.sql` n'écrit toujours que
    /// `(id, competition_id, name)`, c'est le `DEFAULT` qui remplit le reste.
    #[sqlx::test]
    async fn une_saison_neuve_naît_avec_les_quatre_notifications_allumées(pool: PgPool) {
        let season_id = semer_saison(&pool).await;

        let reglages = SeasonRepository::new(pool)
            .find_notifications(&season_id)
            .await
            .expect("lecture des réglages")
            .expect("une saison neuve porte des réglages, jamais NULL");

        assert_eq!(reglages, CompetitionNotifications::default());
        assert!(
            reglages.registration_open.0
                && reglages.round_eve.0
                && reglages.round_closing.0
                && reglages.registration_deadline.0,
            "les quatre doivent être allumées : {reglages:?}"
        );
    }

    /// **Le verrou.** Sans lui, la correction de la 366 ne serait qu'un
    /// nettoyage ponctuel : un futur `INSERT` oubliant la colonne recreuserait
    /// le trou, et le test de sérialisation continuerait de passer — ce qu'il a
    /// fait pendant que 318 saisons devenaient notifiantes.
    #[sqlx::test]
    async fn la_colonne_ne_peut_plus_retomber_à_null(pool: PgPool) {
        let season_id = semer_saison(&pool).await;

        let echec =
            sqlx::query("UPDATE competition_seasons SET notifications = NULL WHERE id = $1")
                .bind(season_id.to_string())
                .execute(&pool)
                .await;

        assert!(
            echec.is_err(),
            "la base a accepté un NULL : la contrainte NOT NULL manque"
        );
    }
}

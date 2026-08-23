//! Le journal des envois de notification — deux opérations, et c'est tout.
//!
//! # `claim` puis `confirm`, jamais l'inverse
//!
//! `claim` réserve le créneau **avant** l'envoi : c'est l'index unique qui
//! départage deux crons parallèles, et zéro ligne rendue signifie « déjà
//! envoyé ». La base tranche, le code n'arbitre rien — c'est tout R3.
//!
//! Entre les deux, la ligne existe avec `sent_at` à `NULL`. Si l'envoi échoue,
//! elle **reste** dans cet état : c'est un échec constaté, que R1 veut
//! journalisé et que R9 interdit de rejouer le lendemain.
//!
//! # Pas de trait
//!
//! Un seul implémenteur, et un seul consommateur — le use case de la carte 339,
//! dans le même BC. L'abstraction viendra avec un second implémenteur, pas
//! avant.

use crate::app::competitions::domain::notification_delivery::DeliveryKey;
use sqlx::PgPool;

#[derive(Clone)]
pub struct NotificationDeliveryRepository {
    pool: PgPool,
}

#[derive(Debug)]
pub enum DeliveryError {
    Database(String),
}

impl std::fmt::Display for DeliveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeliveryError::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

fn db_err(e: impl std::fmt::Display) -> DeliveryError {
    DeliveryError::Database(e.to_string())
}

/// Une saison candidate à une notification, avec ce que les gabarits en
/// demandent. DTO de lecture : primitives assumées.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SeasonCandidate {
    pub season_id: String,
    pub competition_id: String,
    pub space_id: String,
    pub space_name: String,
    pub competition_name: String,
    pub season_name: String,
}

impl NotificationDeliveryRepository {
    /// Les saisons dont une journée démarre à cette date. Les journées `rest`
    /// sont exclues par la requête — une journée de repos n'a rien à annoncer.
    pub async fn seasons_with_round_starting(
        &self,
        date: &str,
    ) -> Result<Vec<SeasonCandidate>, DeliveryError> {
        self.candidates(
            include_str!("sql/notifications/list_seasons_with_round_starting.sql"),
            date,
        )
        .await
    }

    /// Les saisons dont une journée à fenêtre temporelle clôt à cette date.
    pub async fn seasons_with_round_closing(
        &self,
        date: &str,
    ) -> Result<Vec<SeasonCandidate>, DeliveryError> {
        self.candidates(
            include_str!("sql/notifications/list_seasons_with_round_closing.sql"),
            date,
        )
        .await
    }

    /// Les saisons dont la date limite d'inscription vaut cette date.
    pub async fn seasons_with_deadline(
        &self,
        date: &str,
    ) -> Result<Vec<SeasonCandidate>, DeliveryError> {
        self.candidates(
            include_str!("sql/notifications/list_seasons_with_deadline.sql"),
            date,
        )
        .await
    }

    async fn candidates(
        &self,
        sql: &str,
        date: &str,
    ) -> Result<Vec<SeasonCandidate>, DeliveryError> {
        sqlx::query_as::<_, SeasonCandidate>(sql)
            .bind(date)
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)
    }

    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// `true` si le créneau vient d'être réservé, `false` s'il l'était déjà.
    ///
    /// Le second cas n'est pas une erreur : c'est le fonctionnement normal d'un
    /// cron relancé le même jour, et la seule chose qui empêche un coach de
    /// recevoir deux fois le même e-mail.
    pub async fn claim(&self, cle: &DeliveryKey) -> Result<bool, DeliveryError> {
        let reserve: Option<i32> =
            sqlx::query_scalar(include_str!("sql/notifications/claim_delivery.sql"))
                .bind(cle.notification_type.as_str())
                .bind(cle.season_id.to_string())
                .bind(cle.round_id.as_ref().map(|r| r.to_string()))
                .bind(cle.target_date.as_ref())
                .bind(cle.coach_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        Ok(reserve.is_some())
    }

    /// Atteste que l'e-mail est parti. Tant que ce n'est pas fait, la ligne
    /// réservée ne prouve rien.
    pub async fn confirm(&self, cle: &DeliveryKey) -> Result<(), DeliveryError> {
        sqlx::query(include_str!("sql/notifications/confirm_delivery.sql"))
            .bind(cle.notification_type.as_str())
            .bind(cle.season_id.to_string())
            .bind(cle.round_id.as_ref().map(|r| r.to_string()))
            .bind(cle.target_date.as_ref())
            .bind(cle.coach_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
}

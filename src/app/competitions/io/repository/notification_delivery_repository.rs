//! Le journal des envois de notification — deux écritures et une lecture.
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
//! # Ce que la lecture sert, et pourquoi elle est ici
//!
//! `count_by_round` (carte 544) rend au panneau des présences ce que
//! l'expédition a produit : combien de coachs ont reçu leur e-mail, combien non.
//! Ces comptes étaient jusque-là calculés par le use case, journalisés, puis
//! **perdus** — l'organisateur dont le serveur de messagerie tombait voyait son
//! panneau se recharger normalement et croyait ses quatorze e-mails partis.
//!
//! Les relire ici plutôt que de les persister n'est pas une économie de colonne :
//! c'est refuser une seconde vérité. Le journal sait qui a été réservé et qui a
//! été attesté ; un compteur écrit à côté devrait être tenu d'accord avec lui.
//!
//! # Pas de trait
//!
//! Un seul implémenteur, et deux consommateurs du même BC. L'abstraction viendra
//! avec un second implémenteur, pas avant.

use crate::app::competitions::domain::notification_delivery::DeliveryKey;
use sqlx::PgPool;

/// Un envoi du journal, vu de l'écran.
///
/// DTO de lecture : les primitives y sont assumées, ce type ne portant aucun
/// invariant à protéger.
///
/// **`reserves` compte les créneaux, `attestes` les e-mails partis.** La
/// différence est le nombre d'échecs — nommer directement un champ `echecs`
/// aurait figé une soustraction dans le DTO, là où les deux nombres bruts se
/// lisent aussi bien et laissent la vue dire ce qu'elle veut.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeliveryCountDto {
    pub notification_type: String,
    pub target_date: String,
    pub reserves: i64,
    pub attestes: i64,
}

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

    /// Ce que le journal sait d'une journée, une ligne par type et par date
    /// d'envoi.
    ///
    /// L'ouverture porte l'échéance de la campagne, chaque relance porte son jour :
    /// la **dernière** ligne d'un type est donc son envoi le plus récent.
    pub async fn count_by_round(
        &self,
        season_id: &str,
        round_id: &str,
    ) -> Result<Vec<DeliveryCountDto>, DeliveryError> {
        sqlx::query_as::<_, DeliveryCountDto>(include_str!(
            "sql/notifications/count_deliveries_by_round.sql"
        ))
        .bind(season_id)
        .bind(round_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)
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

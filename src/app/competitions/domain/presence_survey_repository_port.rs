//! Ce que la campagne de présence demande à sa persistance.
//!
//! Trois méthodes, et pas quatre. `find_by_token` appartient à l'unité
//! `reponse-coach` et y sera ajoutée : c'est elle qui sait ce que sa route
//! publique charge, et déclarer ici une méthode qu'aucun appelant n'utilise
//! ferait porter à cette carte une décision qui n'est pas la sienne. Le trait se
//! rouvrira alors par un ajout, pas par une reprise.

use crate::app::competitions::domain::presence_survey::PresenceSurvey;
use async_trait::async_trait;

#[derive(Debug)]
pub enum PresenceSurveyRepositoryError {
    Database(String),
}

impl std::fmt::Display for PresenceSurveyRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

/// Une ligne de la barre latérale de l'onglet Présences.
///
/// DTO de lecture : les primitives y sont assumées, ces types n'ayant aucun
/// invariant à protéger.
///
/// **Il ne porte pas de `statut`**, et c'est une correction assumée de la
/// conception : R23 fait de la clôture un calcul, et la produire en SQL mettrait
/// la règle dans une requête — à deux endroits, puisque l'agrégat la porte déjà.
/// Le DTO rend `deadline` et `close_le` bruts, et `statut_de` tranche.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SurveySummaryDto {
    pub round_id: String,
    pub round_name: String,
    pub round_position: i32,
    pub is_rest: bool,
    /// `None` quand la journée ne porte aucune campagne — la barre latérale
    /// affiche toutes les journées, pas seulement celles qu'on a sondées.
    pub deadline: Option<String>,
    pub close_le: Option<String>,
    pub appariee: Option<bool>,
    pub attendues: i64,
    pub reponses: i64,
    pub presents: i64,
}

#[async_trait]
pub trait IPresenceSurveyRepository: Send + Sync {
    /// La campagne d'une journée, avec **toutes** ses réponses.
    ///
    /// R2 garantissant une campagne vivante par journée, la journée suffit à la
    /// nommer : le client n'a jamais à connaître un identifiant de campagne
    /// qu'il ne lit nulle part.
    async fn find_by_round(
        &self,
        round_id: &str,
    ) -> Result<Option<PresenceSurvey>, PresenceSurveyRepositoryError>;

    /// Écrit la campagne **et toutes ses réponses**, en une transaction.
    ///
    /// Réécrire quatorze lignes pour un seul changement de présence est le prix
    /// d'un **chemin d'écriture unique** vers la colonne `presence`. Un
    /// `save_answer` ciblé serait plus économe de quelques microsecondes et
    /// ouvrirait une seconde porte — celle que la phase 6 verrouille en faisant
    /// d'`enregistrer` le seul maître d'une `Presence`. R19, R13, R21 et R28
    /// tiennent parce qu'il n'existe qu'un chemin ; en ouvrir un second les
    /// rendrait contournables par un appelant pressé, et rien ne le signalerait.
    async fn save(&self, survey: &PresenceSurvey) -> Result<(), PresenceSurveyRepositoryError>;

    /// L'état de chaque journée de la saison — **une requête, pas une par
    /// journée**.
    ///
    /// La barre latérale les affiche toutes ; les compter une à une ferait vingt
    /// allers-retours pour une colonne. Les journées sans campagne y figurent
    /// avec leurs compteurs à zéro : une journée absente de la liste passerait
    /// pour un trou.
    async fn list_summaries(
        &self,
        season_id: &str,
    ) -> Result<Vec<SurveySummaryDto>, PresenceSurveyRepositoryError>;
}

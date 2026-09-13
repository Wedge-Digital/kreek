//! Ce que la campagne de présence demande à sa persistance.
//!
//! Trois méthodes en carte 510, cinq depuis la 523, six depuis la 529.
//! `find_by_token` et `find_landing_labels` ont été ajoutées par l'unité
//! `reponse-coach`, `list_open_surveys_for_season` par l'encart du coach — chaque
//! unité sachant seule ce que son écran charge. Le trait se rouvre donc **par des
//! ajouts**, pas par des reprises, comme la 510 l'avait annoncé.

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

/// De quoi titrer la page publique de réponse.
///
/// DTO de lecture : les primitives y sont assumées, ces types n'ayant aucun
/// invariant à protéger.
///
/// **Les dates sont brutes**, et non un libellé composé. « Du 12 au 19 octobre »
/// se compose déjà dans la couche web (`dates_de`) ; le refaire en SQL le mettrait
/// à deux endroits, et un DTO de lecture porte des données, pas des libellés.
///
/// **Le nom de l'équipe n'y est pas.** Il vient d'`ITeamInfoPort` : une jointure
/// vers les tables de `teams` serait l'exacte violation que la souveraineté des
/// données entre BCs nomme.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LandingLabelsDto {
    pub round_name: String,
    pub round_date_start: Option<String>,
    pub round_date_end: Option<String>,
    pub competition_id: String,
    pub competition_name: String,
    pub season_id: String,
    pub space_id: String,
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
    /// La campagne d'un jeton, avec **toutes** ses réponses — pas seulement celle
    /// que le jeton désigne : l'agrégat n'existe pas à moitié, et `enregistrer`
    /// vérifie R19 sur l'ensemble.
    ///
    /// **Un jeton inconnu rend `Ok(None)`, jamais une erreur.** C'est cette
    /// distinction qui produit la page « lien inconnu » (R26) plutôt qu'un `500` —
    /// et un jeton tronqué par un client mail est un cas courant, pas une panne.
    ///
    /// Corollaire : le dépôt ne distingue pas non plus « jeton mal formé » de
    /// « jeton inconnu ». R26 veut que la page publique ne révèle **jamais** si un
    /// jeton a existé ; deux erreurs distinctes ici donneraient deux réponses
    /// distinctes là-bas.
    async fn find_by_token(
        &self,
        token: &str,
    ) -> Result<Option<PresenceSurvey>, PresenceSurveyRepositoryError>;

    /// De quoi titrer la page publique. `None` sur un jeton inconnu, pour la même
    /// raison que `find_by_token`.
    async fn find_landing_labels(
        &self,
        token: &str,
    ) -> Result<Option<LandingLabelsDto>, PresenceSurveyRepositoryError>;

    async fn list_summaries(
        &self,
        season_id: &str,
    ) -> Result<Vec<SurveySummaryDto>, PresenceSurveyRepositoryError>;

    /// Les campagnes ouvertes de la saison, **avec toutes leurs réponses**.
    ///
    /// Elle rend des agrégats et non un DTO de lecture : l'encart a besoin de
    /// `statut()`, de la réponse de chaque équipe et de son horodatage — trois
    /// questions du domaine qu'un DTO aurait aplaties, et que la vue aurait dû
    /// reconstituer. C'est exactement ce que « un view model transpose, il ne
    /// dérive pas » interdit.
    ///
    /// **`maintenant` sert à élaguer, pas à décider.** R23 fait de la clôture un
    /// calcul : le SQL écarte ce qui est manifestement clos, `statut(maintenant)`
    /// tranche ensuite. Le filtre vit donc à deux endroits, et c'est assumé —
    /// l'alternative serait de charger les vingt campagnes d'une saison à chaque
    /// affichage d'une page que la plupart des visiteurs ouvrent sans être
    /// concernés.
    ///
    /// Ce qui rend le compromis sûr est une **asymétrie** : un élagage trop large
    /// ne coûte qu'une campagne chargée pour rien, que le domaine écarte ; un
    /// élagage trop étroit fait disparaître une campagne ouverte, et le domaine
    /// n'a plus rien à rattraper. C'est pour ce seul motif que le SQL compare avec
    /// `>=` là où `statut_de` compare avec `>`.
    ///
    /// **Deux requêtes, quel que soit le nombre de campagnes.** Les campagnes,
    /// puis toutes leurs réponses d'un coup. Réutiliser la lecture au singulier
    /// aurait fait `1 + N` allers-retours — ce que la requête de liste cherchait
    /// justement à éviter.
    async fn list_open_surveys_for_season(
        &self,
        season_id: &str,
        maintenant: &str,
    ) -> Result<Vec<PresenceSurvey>, PresenceSurveyRepositoryError>;
}

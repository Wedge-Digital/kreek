use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
use async_trait::async_trait;

pub struct PairingDisplayDto {
    pub pairing_id: String,
    pub round_id: String,
    pub round_name: String,
    pub round_position: i32,
    pub round_date_start: Option<String>,
    pub round_date_end: Option<String>,
    pub round_day_type: String,
    pub home_team_id: String,
    pub home_team_name: String,
    pub home_roster_name: String,
    pub home_coach_name: String,
    pub home_logo_url: Option<String>,
    pub home_initials: String,
    pub away_team_id: String,
    pub away_team_name: String,
    pub away_roster_name: String,
    pub away_coach_name: String,
    pub away_logo_url: Option<String>,
    pub away_initials: String,
    pub match_status: String,
    pub home_score: Option<i32>,
    pub away_score: Option<i32>,
    pub home_casualties: Option<i32>,
    pub away_casualties: Option<i32>,
    pub match_report_url: Option<String>,
}

/// Un résultat de match, toutes compétitions/saisons d'un espace confondues —
/// pour le widget "Derniers résultats" de la page d'accueil. `season_id`,
/// `competition_id`, `home_team_id` et `away_team_id` ne sont pas affichés :
/// ils servent uniquement au calcul d'autorisation du lien vers le rapport.
pub struct LatestResultDto {
    pub pairing_id: String,
    pub season_id: String,
    pub competition_id: String,
    pub competition_name: String,
    pub round_name: String,
    pub home_team_id: String,
    pub home_team_name: String,
    pub home_score: Option<i32>,
    pub away_team_id: String,
    pub away_team_name: String,
    pub away_score: Option<i32>,
    pub match_report_url: Option<String>,
    pub published_at: Option<time::OffsetDateTime>,
}

/// Données d'affichage nécessaires pour projeter un pairing nouvellement créé
/// dans `competition_match_display_proj`, en plus de son id et des ids
/// d'équipes déjà portés par `Pairing`. Construit par l'appelant à partir de
/// données déjà résolues (`MatchDay`, `TeamInfoDto`) — le repository ne fait
/// aucune résolution, seulement l'écriture atomique pairing + projection.
pub struct NewPairingProjection {
    pub season_id: String,
    pub round_name: String,
    pub round_position: i32,
    pub round_date_start: Option<String>,
    pub round_date_end: Option<String>,
    pub round_day_type: String,
    pub home_team_name: String,
    pub home_roster_name: String,
    pub home_coach_name: String,
    pub home_logo_url: Option<String>,
    pub away_team_name: String,
    pub away_roster_name: String,
    pub away_coach_name: String,
    pub away_logo_url: Option<String>,
}

#[derive(Debug)]
pub enum MatchDayRepositoryError {
    Database(String),
    /// La journée porte déjà des appariements — R11, constatée **sous le
    /// verrou** et non avant lui.
    ///
    /// Le use case vérifie déjà cette condition en chargeant la journée, mais
    /// il la lit hors transaction : deux organisateurs qui régénèrent la même
    /// journée franchissent tous les deux la garde avant que l'un ait écrit.
    /// Le verrou seul ne les départagerait pas — il sérialiserait deux
    /// écritures fautives. C'est la relecture sous verrou qui refuse la
    /// seconde.
    PairingsAlreadyExist,
}

impl std::fmt::Display for MatchDayRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(e) => write!(f, "database error: {}", e),
            Self::PairingsAlreadyExist => {
                write!(f, "la journée porte déjà des appariements")
            }
        }
    }
}

#[async_trait]
pub trait IMatchDayRepository: Send + Sync {
    async fn find_by_season(
        &self,
        season_id: &str,
    ) -> Result<Vec<MatchDay>, MatchDayRepositoryError>;

    async fn find_by_id(
        &self,
        match_day_id: &str,
    ) -> Result<Option<MatchDay>, MatchDayRepositoryError>;

    async fn save_match_day(&self, match_day: &MatchDay) -> Result<(), MatchDayRepositoryError>;

    async fn delete_match_day(&self, match_day_id: &str) -> Result<(), MatchDayRepositoryError>;

    /// Écrit **tous** les appariements d'une journée, ou aucun.
    ///
    /// Elle ouvre une transaction, verrouille la journée, revérifie qu'elle est
    /// vide, puis écrit les paires et leurs projections ensemble. Une panne au
    /// troisième appariement ne laisse donc pas une journée à moitié appariée,
    /// et deux générations concurrentes ne peuvent pas se superposer.
    ///
    /// **Pas d'implémentation par défaut qui bouclerait sur `save_pairing`** :
    /// elle compilerait, les faux de test n'auraient rien à changer, et le vrai
    /// dépôt resterait non atomique le jour où quelqu'un oublierait de la
    /// redéfinir. Un verrou qui se laisse oublier n'est pas un verrou.
    async fn save_pairings(
        &self,
        match_day_id: &str,
        pairings: &[(Pairing, NewPairingProjection)],
    ) -> Result<(), MatchDayRepositoryError>;

    /// Écrit **un** appariement, sur une journée qui peut déjà en porter —
    /// l'ajout manuel d'un match, la recréation d'une rencontre annulée.
    /// Contrairement à `save_pairings`, elle ne juge pas de l'état de la
    /// journée.
    async fn save_pairing(
        &self,
        match_day_id: &str,
        pairing: &Pairing,
        projection: &NewPairingProjection,
    ) -> Result<(), MatchDayRepositoryError>;

    /// Pairing déjà programmé pour ces deux équipes dans cette journée, s'il
    /// existe.
    ///
    /// Sert à ne pas en recréer un lors de la publication d'un rapport
    /// **manuel** : l'agrégat du rapport n'apprend jamais l'identifiant du
    /// pairing créé pour lui, donc une republication après correction en
    /// créerait un second, et le match apparaîtrait deux fois au calendrier.
    async fn find_pairing_id(
        &self,
        match_day_id: &str,
        home_team_id: &str,
        away_team_id: &str,
    ) -> Result<Option<String>, MatchDayRepositoryError>;

    async fn delete_pairing(&self, pairing_id: &str) -> Result<(), MatchDayRepositoryError>;

    async fn ensure_match_days_from_structure(
        &self,
        season_id: &str,
        entries: &[(String, String, String, Option<String>, Option<String>)],
    ) -> Result<(), MatchDayRepositoryError>;

    async fn list_resultats(
        &self,
        season_id: &str,
        cursor_position: Option<i32>,
        limit_rounds: u32,
    ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError>;

    async fn list_calendrier(
        &self,
        season_id: &str,
        cursor_position: Option<i32>,
        limit_rounds: u32,
    ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError>;

    /// Les matchs d'une équipe sur une saison, **les trois statuts confondus**.
    ///
    /// Ni curseur ni plafond de journées, contrairement aux deux ci-dessus :
    /// une saison compte des centaines de matchs, une équipe en joue dix à
    /// quinze. Le tri n'est pas non plus le leur — voir le `.sql`.
    async fn list_team_matches(
        &self,
        season_id: &str,
        team_id: &str,
    ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError>;

    /// Derniers matchs `completed` d'un espace, toutes compétitions/saisons
    /// confondues, triés par date réelle de publication décroissante.
    async fn list_latest_completed_results(
        &self,
        space_id: &str,
        limit: i64,
    ) -> Result<Vec<LatestResultDto>, MatchDayRepositoryError>;
}

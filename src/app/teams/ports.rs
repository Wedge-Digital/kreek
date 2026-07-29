use crate::app::teams::domain::team::{Team, TeamDomainEvent};
use async_trait::async_trait;

#[async_trait]
pub trait IPlayerCountPort: Send + Sync {
    async fn count_for_team(&self, team_id: &str) -> u32;
}

// ── ACL vers le BC `players` (valeur de l'effectif, pour le calcul de la TV) ───

/// Un joueur, vu par `teams` : ce qu'il vaut, et s'il tiendra sa place au
/// prochain match.
///
/// `available_for_next_match` est délibérément un booléen et non le statut de
/// `players` : traduire le vocabulaire de l'autre BC est le rôle de l'adapter.
/// La règle « un indisponible vaut zéro et appelle un journalier » est, elle,
/// une règle de `teams` et vit dans son domaine.
pub struct PlayerValueDto {
    pub player_id: String,
    pub value_kpo: u32,
    pub available_for_next_match: bool,
}

#[async_trait]
pub trait IPlayerValuePort: Send + Sync {
    async fn find_valued_players(&self, team_id: &str) -> Vec<PlayerValueDto>;
}

// ── ACL vers le BC `references` (roster, staff, journalier) ────────────────────

/// La ligne de roster que le règlement désigne comme journalier, et son prix —
/// un journalier vaut le prix de cette ligne.
pub struct JourneymanTypeDto {
    pub position_name: String,
    pub price_kpo: u32,
}

pub trait IJourneymanTypePort: Send + Sync {
    fn journeyman_type_for_roster(&self, roster_id: &str) -> JourneymanTypeDto;
}

/// Les prix de staff sont globaux et non propres à un roster — ils voyagent
/// ici pour éviter un second appel de port au moment de calculer la TV, et
/// parce que la structure conviendra le jour où un roster aura ses tarifs.
pub struct RosterInfoDto {
    pub logo: Option<String>,
    pub reroll_cost: u32,
    pub apothecary_price: u32,
    pub assistant_price: u32,
    pub cheerleader_price: u32,
}

pub trait IRosterInfoPort: Send + Sync {
    fn find_roster_info(&self, roster_id: &str) -> Option<RosterInfoDto>;
}

#[derive(Debug)]
pub enum RepositoryError {
    ConcurrentWrite,
    Serialization(serde_json::Error),
    Deserialization(serde_json::Error),
    Database(sqlx::Error),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConcurrentWrite => write!(f, "écriture concurrente détectée"),
            Self::Serialization(e) => write!(f, "erreur de sérialisation : {e}"),
            Self::Deserialization(e) => write!(f, "erreur de désérialisation : {e}"),
            Self::Database(e) => write!(f, "erreur base de données : {e}"),
        }
    }
}

#[async_trait]
pub trait ITeamRepository: Send + Sync {
    /// Appende un événement dans l'event store.
    /// Retourne la nouvelle version. Échoue avec ConcurrentWrite si
    /// expected_version ne correspond pas à la version courante en base.
    async fn append(
        &self,
        team_id: &str,
        event: &TeamDomainEvent,
        expected_version: u64,
    ) -> Result<u64, RepositoryError>;

    /// Charge tous les événements d'une équipe et hydrate l'agrégat par rejeu.
    async fn find_by_id(&self, team_id: &str) -> Result<Option<Team>, RepositoryError>;

    /// Liste les équipes inscrites à une saison par statut.
    async fn find_by_season_and_status(
        &self,
        season_id: &str,
        status: &str,
    ) -> Result<Vec<TeamEnrollmentRow>, RepositoryError>;

    async fn find_enrolled_for_season(
        &self,
        season_id: &str,
    ) -> Result<Vec<TeamCardRow>, RepositoryError>;
}

pub struct TeamEnrollmentRow {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub status: String,
}

pub struct TeamCardRow {
    pub team_id: String,
    pub team_name: String,
    pub coach_id: String,
    pub coach_name: String,
    pub roster_name: String,
    pub logo_url: Option<String>,
    pub team_value: u32,
    pub game_phase: Option<String>,
}

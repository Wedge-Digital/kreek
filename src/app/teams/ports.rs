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

pub struct CatalogPositionDto {
    pub uid: String,
    pub position_name: String,
    pub cost: u32,
    pub max_quantity: u8,
    pub is_journeyman: bool,
}

/// Limite de cumul entre postes — « pas plus de 3 joueurs parmi Ogre, Troll,
/// Minotaure, Rat Ogre ». Quatre rosters sur trente en ont.
pub struct CrossLimitDto {
    pub max: u32,
    pub position_uids: Vec<String>,
}

pub struct StaffPriceDto {
    pub uid: String,
    pub name: String,
    pub price: u32,
    pub max_quantity: u32,
}

/// Tout ce que `teams` a besoin de savoir d'un roster, en un seul appel.
///
/// Les prix de staff sont globaux et non propres à un roster — ils voyagent ici
/// pour éviter un second aller-retour, et parce que la structure conviendra le
/// jour où un roster aura ses tarifs.
///
/// `reroll_base_cost` est le **prix de base** : le doublement hors création est
/// une règle de saison, appliquée par le domaine, pas par le catalogue.
pub struct RosterCatalogDto {
    pub logo: Option<String>,
    pub reroll_base_cost: u32,
    pub positions: Vec<CatalogPositionDto>,
    pub cross_limits: Vec<CrossLimitDto>,
    pub allowed_staff: Vec<String>,
    pub staff_prices: Vec<StaffPriceDto>,
}

impl RosterCatalogDto {
    /// Prix d'une ligne de staff, ou zéro si le corpus ne la porte pas : mieux
    /// vaut une TV incomplète qu'un démarrage impossible.
    pub fn staff_price(&self, uid: &str) -> u32 {
        self.staff_prices
            .iter()
            .find(|s| s.uid == uid)
            .map(|s| s.price)
            .unwrap_or(0)
    }
}

pub trait IRosterCatalogPort: Send + Sync {
    fn find_catalog(&self, roster_id: &str) -> Option<RosterCatalogDto>;
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

    /// Applique un lot d'événements atomiquement — une seule transaction pour
    /// les N événements, la projection et le grand livre. Retourne la version
    /// du dernier. Un conflit sur n'importe lequel fait tout échouer.
    async fn append_batch(
        &self,
        team_id: &str,
        events: &[TeamDomainEvent],
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

use crate::app::teams::domain::team::{GamePhase, Team, TeamDomainEvent};
use async_trait::async_trait;

// ── ACL vers le BC `players` (valeur de l'effectif, pour le calcul de la TV) ───

/// Un joueur de l'effectif, vu par `teams`.
///
/// `available_for_next_match` est délibérément un booléen et non le statut de
/// `players` : traduire le vocabulaire de l'autre BC est le rôle de l'adapter.
/// Les règles qui s'en déduisent — « un indisponible vaut zéro et appelle un
/// journalier » — sont, elles, des règles de `teams` et vivent dans son domaine.
pub struct SquadMemberDto {
    pub player_id: String,
    pub roster_line_id: String,
    pub personal_name: String,
    pub position_name: String,
    pub spp: u32,
    pub value_kpo: u32,
    pub available_for_next_match: bool,
}

/// Consultation de l'effectif. Rend l'effectif **entier**, drapeau de
/// disponibilité compris, et laisse l'appelant filtrer : le brouillon de
/// recrutement compte les quotas par poste sur tout l'effectif, quand le calcul
/// de valeur d'équipe ne somme que les disponibles. Un port qui filtrerait à la
/// source servirait l'un et trahirait l'autre.
#[async_trait]
pub trait ISquadPort: Send + Sync {
    async fn find_squad(&self, team_id: &str) -> Vec<SquadMemberDto>;
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

// ── Brouillon de phase ────────────────────────────────────────────────────────

/// La moitié **persistée** d'un brouillon : les lignes accumulées et leur
/// version. Le use case la complète avec le catalogue du roster, l'effectif et
/// la trésorerie pour reconstituer l'agrégat (cartes 262 et 267).
///
/// `state` est **opaque** ici : la forme des lignes diffère selon la phase, et
/// appartient à ces agrégats. Le repository ne connaît qu'« une équipe, une
/// phase, un état, une version ».
#[derive(Debug, Clone)]
pub struct PhaseDraftState {
    pub team_id: String,
    pub space_id: String,
    pub phase: GamePhase,
    pub state: serde_json::Value,
    pub version: u32,
}

/// Le nom de phase stocké en base. Seules deux phases ont un brouillon ; une
/// autre valeur est un bug d'appelant, pas un cas nominal — d'où l'erreur
/// explicite plutôt qu'un silence.
pub fn draft_phase_key(phase: &GamePhase) -> Result<&'static str, RepositoryError> {
    match phase {
        GamePhase::Recruitment => Ok("Recruitment"),
        GamePhase::Dismissals => Ok("Dismissals"),
        autre => Err(RepositoryError::PhaseWithoutDraft(autre.clone())),
    }
}

#[async_trait]
pub trait IPhaseDraftRepository: Send + Sync {
    /// Le panier persisté d'une équipe pour cette phase, ou `None` si le coach
    /// n'a encore rien mis dedans.
    async fn load(
        &self,
        team_id: &str,
        phase: &GamePhase,
    ) -> Result<Option<PhaseDraftState>, RepositoryError>;

    /// Écriture gardée par la version. `expected_version` à zéro crée la ligne ;
    /// au-delà, elle met à jour. Les deux échouent en `ConcurrentWrite` si un
    /// autre onglet est passé avant. Retourne la nouvelle version.
    async fn save(
        &self,
        draft: &PhaseDraftState,
        expected_version: u32,
    ) -> Result<u32, RepositoryError>;

    async fn delete(&self, team_id: &str, phase: &GamePhase) -> Result<(), RepositoryError>;
}

#[derive(Debug)]
pub enum RepositoryError {
    ConcurrentWrite,
    Serialization(serde_json::Error),
    Deserialization(serde_json::Error),
    Database(sqlx::Error),
    /// Une phase sans brouillon possible a été passée à `IPhaseDraftRepository`.
    PhaseWithoutDraft(GamePhase),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConcurrentWrite => write!(f, "écriture concurrente détectée"),
            Self::Serialization(e) => write!(f, "erreur de sérialisation : {e}"),
            Self::Deserialization(e) => write!(f, "erreur de désérialisation : {e}"),
            Self::Database(e) => write!(f, "erreur base de données : {e}"),
            Self::PhaseWithoutDraft(p) => {
                write!(f, "aucun brouillon n'existe pour la phase {p:?}")
            }
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

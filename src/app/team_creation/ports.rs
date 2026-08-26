use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::team_creation::domain::creation_rules::CreationRules;
use crate::app::team_creation::domain::ruleset::Ruleset;
use crate::app::team_creation::domain::team_draft::DraftTeam;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use async_trait::async_trait;

// ── Anti-Corruption Layer — données de référence ─────────────────────────────

/// Limite de cumul entre plusieurs postes d'un même roster.
pub struct CrossLimitDto {
    pub max: u32,
    pub position_uids: Vec<String>,
}

pub struct RosterDefinition {
    pub uid: String,
    pub name: String,
    pub reroll_cost: u32,
    pub cross_limits: Vec<CrossLimitDto>,
    pub available_players: Vec<PlayerPositionDefinition>,
    pub allowed_staff_uids: Vec<String>,
    pub leagues: Vec<String>,
    pub special_rules: Vec<String>,
}

pub struct PlayerPositionDefinition {
    pub uid: String,
    pub position_name: String,
    pub cost: u32,
    pub max_quantity: u8,
    pub ma: u8,
    pub st: u8,
    pub ag: u8,
    pub pa: u8,
    pub av: u8,
    pub skills: Vec<SkillDefinition>,
}

pub struct SkillDefinition {
    pub uid: String,
    pub name: String,
}

pub struct StaffDefinition {
    pub uid: String,
    pub name: String,
    pub price: u32,
    pub max_quantity: u8,
}

pub struct SkillCostResult {
    pub spp_cost: u8,
}

pub struct SkillPricingDefinition {
    pub chosen_primary: u8,
    pub chosen_secondary: u8,
    pub random: u8,
}

pub trait IReferenceDataPort: Send + Sync {
    fn find_roster_definition(&self, roster_uid: &str) -> Option<RosterDefinition>;
    fn list_staff_definitions(&self) -> Vec<StaffDefinition>;
    fn resolve_skill_cost(
        &self,
        roster_line_id: &str,
        skill_id: &str,
        mode: &str,
    ) -> Option<SkillCostResult>;
    fn resolve_skill_name(&self, skill_uid: &str) -> Option<String>;
    fn resolve_base_skills(&self, roster_line_id: &str) -> Vec<String>;
    fn skill_pricing_level_1(&self) -> Option<SkillPricingDefinition>;
}

#[derive(Debug)]
pub enum RepositoryError {
    NotFound,
    PersistenceError(String),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepositoryError::NotFound => write!(f, "Équipe introuvable"),
            RepositoryError::PersistenceError(msg) => write!(f, "Erreur de persistance : {}", msg),
        }
    }
}

#[async_trait]
pub trait ITeamDraftRepository: Send + Sync {
    async fn save(&self, team: &DraftTeam, space_id: &str) -> Result<(), RepositoryError>;
    /// L'espace d'un brouillon, ou `None` s'il n'existe pas (carte 321).
    ///
    /// Un brouillon **n'est pas encore une équipe** : il vit dans
    /// `team_drafts` et n'apparaît dans `team_proj` qu'à la soumission. C'est
    /// ce décalage qui a fait qu'un résolveur `team_id` lisant la seule
    /// projection a cassé la création d'équipe.
    async fn find_space_id(&self, id: &TeamId) -> Result<Option<String>, RepositoryError>;

    async fn find_by_id(&self, id: &TeamId) -> Result<Option<DraftTeam>, RepositoryError>;
    async fn find_by_coach_and_space(
        &self,
        coach_id: &str,
        space_id: &str,
    ) -> Result<Vec<DraftTeam>, RepositoryError>;
}

#[async_trait]
pub trait ITeamRosterRepository: Send + Sync {
    async fn save(&self, team: &RosterSelectedTeam, space_id: &str) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: &TeamId) -> Result<Option<RosterSelectedTeam>, RepositoryError>;
    async fn mark_submitted(&self, id: &TeamId) -> Result<(), RepositoryError>;
    async fn find_submitted_ids_for_space(
        &self,
        space_id: &str,
    ) -> Result<Vec<String>, RepositoryError>;
}

/// Port de lecture pour les données de référence (rulesets).
/// Conservé pour compatibilité — la source principale est désormais CreationRules.
pub trait RulesetRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<Ruleset>, RepositoryError>;
    async fn find_all(&self) -> Result<Vec<Ruleset>, RepositoryError>;
}

/// Ce que le BC `competitions` sait d'une saison au moment d'y créer une équipe.
///
/// Le **statut** en fait partie, et ce n'est pas un détail : une saison est
/// joignable dès que ses règles sont posées, soit trois étapes avant d'être
/// prête. Une équipe créée dans cette fenêtre ne s'inscrit jamais — la
/// configuration des invitations n'existe pas encore, et l'inscription
/// automatique retombe silencieusement sur « non » (carte 407).
///
/// DTO de lecture : les primitives y sont permises.
pub struct SeasonCreationData {
    /// La saison est entièrement configurée et ouverte aux inscriptions.
    ///
    /// C'est **l'adapter** qui traduit : `team_creation` n'a pas à connaître le
    /// vocabulaire de statuts du BC `competitions`, c'est tout l'objet de l'ACL.
    pub prete: bool,
    /// Le statut brut, **pour le journal uniquement** — jamais pour décider.
    /// C'est lui qui rendra la ligne de refus exploitable.
    pub statut: String,
    pub rules: Option<CreationRules>,
}

#[async_trait]
pub trait ICompetitionCreationRulesPort: Send + Sync {
    /// Remonte la donnée brute **sans trancher** : c'est
    /// `use_cases::season_access_service::acces_creation` qui décide, et lui
    /// se teste sans base ni HTTP.
    async fn find_season_creation_data(&self, season_id: &str) -> Option<SeasonCreationData>;
}

// ── ACL vers le BC `competitions` (noms d'affichage compétition/saison) ────────

#[async_trait]
pub trait ICompetitionDisplayPort: Send + Sync {
    async fn find_competition_name(&self, competition_id: &str) -> Option<String>;
    async fn find_season_name(&self, season_id: &str) -> Option<String>;
}

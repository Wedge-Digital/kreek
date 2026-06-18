use crate::app::shared_kernel::team::TeamId;
use crate::app::team_creation::domain::ruleset::Ruleset;
use crate::app::team_creation::domain::team_draft::DraftTeam;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use async_trait::async_trait;

// ── Anti-Corruption Layer — données de référence ─────────────────────────────

pub struct RosterDefinition {
    pub uid: String,
    pub name: String,
    pub reroll_cost: u32,
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

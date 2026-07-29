use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::{Player, PlayerId, TeamId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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

// ── Projection read model ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquiredSkillProjection {
    pub skill_id: String,
    pub skill_name: String,
    #[serde(default)]
    pub category_css: String,
    pub mode: String,
    pub spp_cost: i32,
}

#[derive(Debug, Clone)]
pub struct PlayerProjection {
    pub player_id: String,
    pub team_id: String,
    pub space_id: String,
    pub position_name: String,
    pub roster_line_id: String,
    pub personal_name: String,
    pub jersey: Option<i16>,
    pub base_skills: Vec<String>,
    pub acquired_skills: Vec<AcquiredSkillProjection>,
    pub spp: i32,
    pub value_kpo: i32,
    pub participation_status: String,
}

#[async_trait]
pub trait IPlayerProjectionRepository: Send + Sync {
    async fn find_by_team_id(
        &self,
        team_id: &TeamId,
    ) -> Result<Vec<PlayerProjection>, RepositoryError>;

    async fn find_by_id(
        &self,
        player_id: &str,
    ) -> Result<Option<PlayerProjection>, RepositoryError>;
}

// ── Event store port ───────────────────────────────────────────────────────────

#[async_trait]
pub trait IPlayerRepository: Send + Sync {
    async fn append(
        &self,
        player_id: &PlayerId,
        team_id: &TeamId,
        event: &PlayerDomainEvent,
        version: i32,
    ) -> Result<(), RepositoryError>;

    async fn find_by_id(&self, player_id: &PlayerId) -> Result<Option<Player>, RepositoryError>;

    async fn find_by_team_id(&self, team_id: &TeamId) -> Result<Vec<Player>, RepositoryError>;

    /// Events bruts d'un joueur, ordonnés (version ASC) — contrairement à
    /// `find_by_id` (agrégat hydraté final), nécessaire pour reconstruire un
    /// historique à la lecture (ex. historique de matchs, `match_history_service`).
    async fn find_events_by_id(
        &self,
        player_id: &PlayerId,
    ) -> Result<Vec<PlayerDomainEvent>, RepositoryError>;

    /// Un joueur de cette équipe a-t-il dépensé des SPP **depuis** ce match ?
    ///
    /// Question posée par le garde-fou de correction d'un rapport publié : une
    /// correction rétroactive ne doit pas retirer des SPP déjà convertis en
    /// compétence ou en caractéristique.
    ///
    /// Répond sur l'ensemble de l'effectif, la projection ne portant pas
    /// l'historique nécessaire.
    async fn has_spent_spp_since_match(
        &self,
        team_id: &TeamId,
        match_report_id: &str,
    ) -> Result<bool, RepositoryError>;
}

// ── ACL vers le BC `references` (catalogue de compétences, matrice de coût) ────
// DTOs propres à `players` — jamais les types du domaine `references`
// (règle CLAUDE.md « Adapters inter-BCs »).

pub struct SkillCatalogEntryDto {
    pub skill_id: String,
    pub name: String,
    pub category: String,
    pub is_elite: bool,
}

pub struct PositionAccessDto {
    pub primary_categories: Vec<String>,
    pub secondary_categories: Vec<String>,
}

/// Entrée catalogue complète pour un poste — stats de base, compétences de
/// base, coût, accès aux catégories. Contrairement à `PositionAccessDto`
/// (juste les catégories accessibles, utilisé pour la validation d'achat),
/// couvre l'affichage joueur et l'initialisation à la création d'un joueur.
pub struct PositionCatalogEntryDto {
    pub position_name: String,
    pub cost: u32,
    pub ma: u8,
    pub st: u8,
    pub ag: u8,
    pub pa: u8,
    pub av: u8,
    pub base_skills: Vec<String>,
    pub primary_categories: Vec<String>,
    pub secondary_categories: Vec<String>,
}

/// Coûts en SPP pour un niveau de la matrice, déjà résolus pour le statut
/// élite demandé (l'adaptateur applique le repli élite→standard quand la
/// donnée élite n'est pas renseignée) — unité SPP, à ne pas confondre avec
/// `ISkillCatalogPort::skill_value_delta`/`stat_value_delta`, qui donnent la
/// valeur d'équipe ajoutée, en kPo.
pub struct SkillCostLevelDto {
    pub level: u8,
    pub chosen_primary: u32,
    pub chosen_secondary: u32,
    pub random: u32,
    pub characteristic: u32,
}

pub trait ISkillCatalogPort: Send + Sync {
    fn find_skill(&self, skill_id: &str) -> Option<SkillCatalogEntryDto>;
    fn find_position(&self, roster_line_id: &str) -> Option<PositionCatalogEntryDto>;
    fn position_access(&self, roster_line_id: &str) -> Option<PositionAccessDto>;
    fn cost_for_level(&self, level: u8, is_elite: bool) -> Option<SkillCostLevelDto>;

    /// Valeur (kPo) ajoutée par l'achat d'une compétence primary/secondary.
    fn skill_value_delta(&self, is_secondary_access: bool) -> u32;
    /// Valeur (kPo) ajoutée par une augmentation de la caractéristique donnée.
    fn stat_value_delta(&self, stat: crate::app::players::domain::match_impact::StatKind) -> u32;

    // ── Barème SPP par type d'action (règle Blood Bowl standard, fixe) ────────
    fn touchdown_spp(&self) -> u8;
    fn pass_spp(&self) -> u8;
    fn interception_spp(&self) -> u8;
    fn casualty_spp(&self) -> u8;
    fn mvp_spp(&self) -> u8;
}

// ── ACL vers le BC `teams` (état de l'équipe pour l'autorisation) ──────────────
// DTO propre à `players` — jamais le type domaine `teams::domain::team::Team`
// (règle CLAUDE.md « Adapters inter-BCs »).

/// Ce dont `players` a besoin de l'équipe pour l'affichage et les gardes-fous
/// d'autorisation (achat de compétence, augmentation de stat) — jamais
/// l'agrégat `Team` complet.
pub struct TeamRosterInfoDto {
    pub team_name: String,
    pub coach_id: String,
    pub competition_id: Option<String>,
    /// Remplace la comparaison `team.game_phase == Some(GamePhase::PlayerImprovement)`
    /// — `players` n'a pas à connaître l'énumération `GamePhase` de `teams`.
    pub in_player_improvement_phase: bool,
}

#[async_trait]
pub trait IPlayerRosterPort: Send + Sync {
    async fn find_team_info(&self, team_id: &str) -> Option<TeamRosterInfoDto>;
}

// ── ACL vers le BC `competitions` (admins, pour l'autorisation) ────────────────

pub struct CompetitionAdminInfoDto {
    pub admin_ids: Vec<String>,
    pub admin_names: Vec<String>,
}

#[async_trait]
pub trait IPlayerCompetitionPort: Send + Sync {
    async fn find_admin_info(&self, competition_id: &str) -> Option<CompetitionAdminInfoDto>;
}

// ── ACL vers le BC `spaces` (profil membre, pour l'autorisation) ───────────────
// `SpaceProfile` vit dans `shared_kernel` (pas dans `spaces`) — réutilisable
// tel quel sans DTO supplémentaire.

#[async_trait]
pub trait IPlayerSpaceMemberPort: Send + Sync {
    async fn find_member_profile(
        &self,
        coach_id: &crate::app::shared_kernel::identity::ids::CoachId,
        space_id: &crate::app::shared_kernel::identity::ids::SpaceId,
    ) -> Option<crate::app::shared_kernel::identity::authorization::SpaceProfile>;
}

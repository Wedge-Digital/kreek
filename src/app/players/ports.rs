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

    /// Cumul des ajustements de caractéristiques, toutes sources confondues —
    /// séquelles, augmentations SPP, customisations. Ce sont des **deltas** :
    /// la base du poste vit dans `references`, et c'est ce qui permet à la
    /// projection de s'écrire sans interroger ce BC.
    pub ma_delta: i16,
    pub st_delta: i16,
    pub ag_delta: i16,
    pub pa_delta: i16,
    pub av_delta: i16,
}

#[async_trait]
pub trait IPlayerProjectionRepository: Send + Sync {
    /// L'espace auquel appartient ce joueur, ou `None` s'il n'existe pas.
    ///
    /// Lue depuis la **projection** et non reconstruite depuis l'event store :
    /// le contrôle d'appartenance s'exécute sur chaque requête, et rejouer un
    /// agrégat à chaque fois serait hors de prix. La projection est écrite dans
    /// la même transaction que l'événement, donc la donnée est fiable.
    async fn find_space_id(&self, player_id: &str) -> Result<Option<String>, RepositoryError>;

    async fn find_by_team_id(
        &self,
        team_id: &TeamId,
    ) -> Result<Vec<PlayerProjection>, RepositoryError>;

    async fn find_by_id(
        &self,
        player_id: &str,
    ) -> Result<Option<PlayerProjection>, RepositoryError>;

    /// Les numéros de maillot **portés** dans l'équipe.
    ///
    /// Sert à attribuer le premier libre à un nouveau venu. Passe par le
    /// repository, et non par une requête à part, pour que le filtre
    /// d'appartenance vive au même endroit que celui des autres lectures : un
    /// maillot laissé par un renvoyé doit redevenir attribuable, et c'est
    /// exactement ce qu'une seconde requête avait déjà failli manquer.
    async fn jerseys_by_team_id(&self, team_id: &TeamId) -> Result<Vec<u16>, RepositoryError>;

    /// Nombre de joueurs alignables au prochain match — les indisponibles
    /// (`MissingNextGame`, `Retired`, `Dead`) en sont exclus.
    ///
    /// Distinct de `find_by_team_id().len()`, qui donne l'effectif total : c'est
    /// ce comptage-là qui détermine le nombre de journaliers, et les confondre
    /// prive de renfort une équipe amoindrie par les blessures.
    async fn count_available_by_team_id(&self, team_id: &TeamId) -> Result<usize, RepositoryError>;
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

    /// Appende plusieurs événements, potentiellement sur des joueurs différents.
    ///
    /// L'édition d'effectif touche tout un lot de joueurs d'un coup : soit le
    /// lot entier passe, soit rien, sans quoi un doublon de maillot pourrait
    /// exister le temps d'un échec partiel. L'implémentation par défaut est
    /// séquentielle — correcte mais non atomique — pour que les doublures de
    /// test n'aient rien à écrire ; c'est l'implémentation Postgres qui tient
    /// la promesse d'atomicité.
    async fn append_batch(
        &self,
        entries: Vec<(PlayerId, TeamId, PlayerDomainEvent, i32)>,
    ) -> Result<(), RepositoryError> {
        for (player_id, team_id, event, version) in entries {
            self.append(&player_id, &team_id, &event, version).await?;
        }
        Ok(())
    }

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
    /// Libellé humain de la catégorie (« Général », « Force », …). Il vit dans
    /// les données de `references` : le dériver ici obligerait `players` à
    /// tenir sa propre table de traduction, qui divergerait au premier
    /// référentiel ajouté.
    pub category_label: String,
    pub description: String,
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

// ── Panier de customisation ───────────────────────────────────────────────────

/// Le panier persisté, tel que la base le rend. `state` ne porte **que les
/// lignes** — le reste est rechargé à chaque hydratation.
#[derive(Debug, Clone)]
pub struct CustomisationBasketState {
    pub player_id: String,
    pub space_id: String,
    pub state: serde_json::Value,
    pub version: u32,
    /// Dernière modification. Porté jusqu'ici parce que la **péremption est une
    /// règle métier** : le repository expose l'horodatage, le domaine décide.
    pub updated_at: time::OffsetDateTime,
}

#[async_trait]
pub trait ICustomisationBasketRepository: Send + Sync {
    /// Le panier d'un joueur, ou `None` si personne n'a encore rien mis dedans.
    async fn load(
        &self,
        player_id: &str,
    ) -> Result<Option<CustomisationBasketState>, RepositoryError>;

    /// Écriture gardée par la version. `expected_version` à zéro crée la ligne ;
    /// au-delà, elle met à jour. Les deux échouent en `ConcurrentWrite` si un
    /// autre onglet est passé avant. Retourne la nouvelle version.
    async fn save(
        &self,
        basket: &CustomisationBasketState,
        expected_version: u32,
    ) -> Result<u32, RepositoryError>;

    /// **Idempotent** : un panier déjà absent n'est pas une erreur. L'annulation
    /// se clique deux fois sans produire de message d'échec.
    async fn delete(&self, player_id: &str) -> Result<(), RepositoryError>;
}

pub trait ISkillCatalogPort: Send + Sync {
    fn find_skill(&self, skill_id: &str) -> Option<SkillCatalogEntryDto>;

    /// Le catalogue **complet**, sans filtre d'accès de poste.
    ///
    /// Élargissement assumé du contrat : jusqu'ici `players` ne consultait le
    /// catalogue que pour des compétences dont il avait déjà l'identifiant. La
    /// customisation en a besoin entier, puisqu'elle ignore par définition les
    /// règles d'accès du poste — c'est ce qui la distingue du `skill-picker` de
    /// `references`, qui filtre et tarife.
    fn list_all_skills(&self) -> Vec<SkillCatalogEntryDto>;
    fn find_position(&self, roster_line_id: &str) -> Option<PositionCatalogEntryDto>;
    fn position_access(&self, roster_line_id: &str) -> Option<PositionAccessDto>;
    fn cost_for_level(&self, level: u8, is_elite: bool) -> Option<SkillCostLevelDto>;

    /// Valeur (kPo) ajoutée par l'achat d'une compétence primary/secondary.
    /// Cf. `IReferenceRepository::improvement_skill_value_delta` : l'accès
    /// **et** l'élitisme, jamais l'un sans l'autre.
    fn skill_value_delta(&self, is_secondary_access: bool, is_elite: bool) -> u32;
    /// Valeur (kPo) ajoutée par une augmentation de la caractéristique donnée.
    fn stat_value_delta(&self, stat: crate::app::players::domain::match_impact::StatKind) -> u32;

    // ── Barème SPP ─────────────────────────────────────────────────────────────
    /// Le barème du joueur, résolu depuis sa ligne de roster.
    ///
    /// Il n'est pas le même pour tous : la règle spéciale `BRAWLIN_BRUTES`
    /// inverse touchdown et sortie. `players` passe la ligne de poste qu'il
    /// porte déjà et ne résout rien lui-même — le corpus appartient à
    /// `references`.
    fn spp_scale_for_roster_line(&self, roster_line_id: &str) -> SppScaleDto;
}

/// Le barème SPP vu par `players` — DTO propre, jamais le modèle de
/// `references` (règle « Adapters inter-BCs »).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SppScaleDto {
    pub touchdown: u8,
    pub pass: u8,
    pub interception: u8,
    pub casualty: u8,
    pub mvp: u8,
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

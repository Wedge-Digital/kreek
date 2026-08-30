use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
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
    /// Le numéro de maillot, que `players` attribue et possède. `None` tant
    /// qu'aucun ne lui a été donné — la page de renvois affiche alors un tiret
    /// plutôt qu'un zéro, qui se lirait comme un numéro.
    pub jersey: Option<u8>,
    pub personal_name: String,
    pub position_name: String,
    pub spp: u32,
    pub value_kpo: u32,
    pub available_for_next_match: bool,
}

/// Les deux droits qu'un visiteur **ne tient pas de la propriété** de l'équipe.
///
/// Un seul port pour deux questions, là où `players` en a deux : les siens
/// servent ailleurs, ceux-ci ne répondent qu'à « ce visiteur peut-il modifier
/// cet effectif ? ». Deux ports auraient valu deux câblages dans `main.rs`
/// pour un seul appelant.
///
/// La propriété n'y figure pas, et c'est délibéré : `Team` porte déjà
/// `coach_id`. Elle se décide sans aucun aller-retour, et c'est pour ça qu'elle
/// est évaluée en premier.
///
/// # `coach_name` en plus de `coach_id`
///
/// Une compétition stocke ses administrateurs **des deux façons** —
/// `admin_ids` et `admin_names`. `can_spend_spp`, qui garde l'écriture,
/// interroge les deux. N'en reprendre qu'une priverait du bouton des
/// administrateurs qui l'ont aujourd'hui, et l'affichage cesserait de suivre
/// l'autorisation — le défaut même que cette carte corrige.
/// Le hasard, derrière un port.
///
/// Le précédent du projet tire en dur dans le use case (`random_draw.rs:44`), et
/// ses tests ne portent que sur la répartition, jamais sur le tirage. **Ça ne
/// suffit pas ici** : il faut pouvoir prouver qu'à 345 kPo un 1 donne un
/// incident majeur et retire exactement 170. Sans jet forçable, ce test n'existe
/// pas, et la table de la carte 408 reste non vérifiée de bout en bout.
pub trait IDiceRoller: Send + Sync {
    fn d6(&self) -> u8;
    fn d3(&self) -> u8;
    /// Un **couple**, et non deux appels à `d6` : les deux dés d'une
    /// catastrophe sont un seul geste, et un test qui enchaîne deux dés truqués
    /// devient vite illisible.
    fn two_d6(&self) -> (u8, u8);
}

#[async_trait]
pub trait ITeamAccessPort: Send + Sync {
    async fn is_space_admin(&self, coach_id: &CoachId, space_id: &SpaceId) -> bool;

    async fn is_competition_admin(
        &self,
        competition_id: &str,
        coach_id: &str,
        coach_name: &str,
    ) -> bool;
}

/// Consultation de l'effectif. Rend l'effectif **entier**, drapeau de
/// disponibilité compris, et laisse l'appelant filtrer : le panier de
/// recrutement compte les quotas par poste sur tout l'effectif, quand le calcul
/// de valeur d'équipe ne somme que les disponibles. Un port qui filtrerait à la
/// source servirait l'un et trahirait l'autre.
#[async_trait]
pub trait ISquadPort: Send + Sync {
    async fn find_squad(&self, team_id: &str) -> Vec<SquadMemberDto>;
}

// ── ACL vers le BC `competitions` (contexte d'un match) ───────────────────────

/// Ce qu'un rapport de match doit à son affichage dans le relevé : la journée,
/// les deux équipes, et le score s'il existe.
///
/// **Deux `Option` qui ne disent pas la même absence.** `find_match_context`
/// rend `None` quand le match n'a **aucune ligne d'affichage** — un rapport créé
/// à la main, tant que la carte 427 n'est pas livrée. Les scores, eux, valent
/// `None` quand le match **est en cours**. Confondre les deux ferait perdre son
/// en-tête de journée à un match qu'on connaît parfaitement.
pub struct MatchContextDto {
    pub round_name: String,
    pub home_team_id: String,
    pub home_team_name: String,
    pub away_team_id: String,
    pub away_team_name: String,
    pub home_score: Option<u8>,
    pub away_score: Option<u8>,
}

#[async_trait]
pub trait IMatchContextPort: Send + Sync {
    async fn find_match_context(&self, match_report_id: &str) -> Option<MatchContextDto>;
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
    pub ma: u8,
    pub st: u8,
    pub ag: u8,
    pub pa: u8,
    pub av: u8,
    /// Compétences de base, **déjà traduites** par l'adapter. `teams` affiche
    /// des noms, il n'a que faire des uids du corpus de référence.
    pub skills: Vec<SkillBadgeDto>,
}

/// Limite de cumul entre postes — « pas plus de 3 joueurs parmi Ogre, Troll,
/// Minotaure, Rat Ogre ». Quatre rosters sur trente en ont.
/// Une compétence telle qu'elle s'affiche : son nom traduit et sa catégorie,
/// qui décide de sa couleur. La catégorie voyage **brute** — c'est la couche
/// web qui choisit la classe CSS, pas l'adapter.
pub struct SkillBadgeDto {
    pub name: String,
    pub category: String,
}

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
    /// La règle « Lineman a vil prix » — le prix de base des linemen ne compte
    /// pas dans la valeur d'équipe.
    ///
    /// Une règle, pas un identifiant de corpus : `teams` n'a pas à connaître
    /// `LOW_COST_LINEMEN`, c'est l'adapter qui traduit. Précédent :
    /// `FAVOURED_OF_CHOOSE_`, en dur dans `team_creation`.
    pub linemen_are_free: bool,
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

// ── Panier de phase ────────────────────────────────────────────────────────

/// La moitié **persistée** d'un panier : les lignes accumulées et leur
/// version. Le use case la complète avec le catalogue du roster, l'effectif et
/// la trésorerie pour reconstituer l'agrégat (cartes 262 et 267).
///
/// `state` est **opaque** ici : la forme des lignes diffère selon la phase, et
/// appartient à ces agrégats. Le repository ne connaît qu'« une équipe, une
/// phase, un état, une version ».
#[derive(Debug, Clone)]
pub struct PhaseBasketState {
    pub team_id: String,
    pub space_id: String,
    pub phase: GamePhase,
    pub state: serde_json::Value,
    pub version: u32,
}

/// Le nom de phase stocké en base. Seules deux phases ont un panier ; une
/// autre valeur est un bug d'appelant, pas un cas nominal — d'où l'erreur
/// explicite plutôt qu'un silence.
pub fn basket_phase_key(phase: &GamePhase) -> Result<&'static str, RepositoryError> {
    match phase {
        GamePhase::Recruitment => Ok("Recruitment"),
        GamePhase::Dismissals => Ok("Dismissals"),
        autre => Err(RepositoryError::PhaseWithoutBasket(autre.clone())),
    }
}

#[async_trait]
pub trait IPhaseBasketRepository: Send + Sync {
    /// Le panier persisté d'une équipe pour cette phase, ou `None` si le coach
    /// n'a encore rien mis dedans.
    async fn load(
        &self,
        team_id: &str,
        phase: &GamePhase,
    ) -> Result<Option<PhaseBasketState>, RepositoryError>;

    /// Écriture gardée par la version. `expected_version` à zéro crée la ligne ;
    /// au-delà, elle met à jour. Les deux échouent en `ConcurrentWrite` si un
    /// autre onglet est passé avant. Retourne la nouvelle version.
    async fn save(
        &self,
        basket: &PhaseBasketState,
        expected_version: u32,
    ) -> Result<u32, RepositoryError>;

    async fn delete(&self, team_id: &str, phase: &GamePhase) -> Result<(), RepositoryError>;
}

/// Une ligne du grand livre, avec l'événement qui l'a produite.
///
/// DTO de lecture : primitifs acceptés (règle CQRS du `CLAUDE.md`). `direction`
/// et `reason` sont les chaînes écrites par `as_str` — c'est le service qui les
/// repasse par `parse`, et qui refuse le relevé si l'une est inconnue.
///
/// `payload` est `None` quand l'événement manque : le `LEFT JOIN` garde la
/// ligne, le détail se replie sur le motif seul.
pub struct TreasuryMovementRow {
    pub event_version: i64,
    pub direction: String,
    pub amount_kpo: i32,
    pub reason: String,
    pub balance_after_kpo: i32,
    pub occurred_at: time::OffsetDateTime,
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum RepositoryError {
    ConcurrentWrite,
    Serialization(serde_json::Error),
    Deserialization(serde_json::Error),
    Database(sqlx::Error),
    /// Une phase sans panier possible a été passée à `IPhaseBasketRepository`.
    PhaseWithoutBasket(GamePhase),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConcurrentWrite => write!(f, "écriture concurrente détectée"),
            Self::Serialization(e) => write!(f, "erreur de sérialisation : {e}"),
            Self::Deserialization(e) => write!(f, "erreur de désérialisation : {e}"),
            Self::Database(e) => write!(f, "erreur base de données : {e}"),
            Self::PhaseWithoutBasket(p) => {
                write!(f, "aucun panier n'existe pour la phase {p:?}")
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
    /// L'espace auquel appartient cette équipe, ou `None` si elle n'existe pas
    /// (carte 324).
    ///
    /// Lue depuis la projection : `find_by_id` rejoue l'agrégat depuis l'event
    /// store, hors de prix pour un contrôle exécuté à chaque requête.
    async fn find_space_id(&self, team_id: &str) -> Result<Option<String>, RepositoryError>;

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

    /// Liste toutes les équipes d'un coach dans un space, tous statuts confondus.
    async fn find_by_coach_and_space(
        &self,
        coach_id: &str,
        space_id: &str,
    ) -> Result<Vec<MyTeamRow>, RepositoryError>;
    /// Le grand livre d'une équipe, dans l'ordre des versions.
    ///
    /// **Aucun code de production ne lisait cette table** avant la carte 435 :
    /// elle est alimentée depuis l'origine, dans la transaction de l'append, et
    /// seuls les tests du dépôt la relisaient.
    async fn list_treasury_movements(
        &self,
        team_id: &str,
    ) -> Result<Vec<TreasuryMovementRow>, RepositoryError>;
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

pub struct MyTeamRow {
    pub team_id: String,
    pub team_name: String,
    pub roster_name: String,
    pub logo_url: Option<String>,
    pub status: String,
    pub game_phase: Option<String>,
}

use crate::app::match_report::domain::value_objects::{ActionPlayer, MatchAction};
use async_trait::async_trait;

#[async_trait]
pub trait IPlayerDataPort: Send + Sync {
    async fn count_available_players(&self, team_id: &str) -> Result<usize, String>;
    async fn find_player_display(&self, player_id: &str) -> Option<String>;
    async fn find_player_position(&self, player_id: &str) -> Option<String>;
    async fn find_player_counts_by_position(&self, team_id: &str) -> Vec<PositionCountDto>;

    /// Un joueur de cette équipe a-t-il dépensé des SPP depuis ce match ?
    /// Garde-fou de la correction : une correction rétroactive ne doit pas
    /// retirer des SPP déjà convertis en amélioration.
    async fn has_spent_spp_since_match(
        &self,
        team_id: &str,
        match_report_id: &str,
    ) -> Result<bool, String>;
}

#[async_trait]
pub trait ICompetitionDataPort: Send + Sync {
    async fn is_competition_admin(
        &self,
        competition_id: &str,
        coach_id: &str,
    ) -> Result<bool, String>;

    async fn find_tier_rules_for_roster(
        &self,
        season_id: &str,
        roster_id: &str,
    ) -> Option<TierRulesDto>;

    async fn find_round_context(&self, season_id: &str, round_id: &str) -> Option<RoundContextDto>;

    /// La saison autorise-t-elle les matchs hors calendrier (carte 550) ?
    ///
    /// **`true` quand la réponse est inconnue** — saison introuvable, colonne
    /// jamais réglée, panne de lecture. C'est le comportement de toujours, et
    /// refuser une saisie sur une erreur d'infrastructure priverait un coach de
    /// son rapport sans qu'il puisse rien y faire.
    ///
    /// La garde qui s'appuie dessus ne protège donc que ce qui a été
    /// explicitement interdit — ce qui est exactement son objet.
    async fn autorise_hors_calendrier(&self, season_id: &str) -> bool;

    /// Programme une rencontre au calendrier et rend son identifiant
    /// d'appariement (carte 552).
    ///
    /// # Une commande, et non une consultation — c'est délibéré
    ///
    /// Le `CLAUDE.md` réserve le port à la lecture synchrone et l'app event à la
    /// propagation d'effet. Ce cas n'est ni l'un ni l'autre : le rapport a
    /// besoin de l'identifiant **immédiatement** pour le porter dès sa
    /// création, et un événement ne rend rien à son émetteur.
    ///
    /// C'est exactement ce qui manquait : `competitions` fabriquait
    /// l'appariement pour le compte du rapport et ne le lui disait jamais. En
    /// renversant le sens — le rapport demande, puis naît avec la réponse — le
    /// `pairing_id` est renseigné dès le premier événement, et toutes les
    /// recherches inverses cessent d'être aveugles.
    ///
    /// L'invariant de journée (carte 551) s'applique sans rien de plus :
    /// l'implémentation passe par le même use case que l'ajout d'un match par
    /// un commissaire.
    async fn creer_appariement(
        &self,
        space_id: &str,
        competition_id: &str,
        season_id: &str,
        round_id: &str,
        home_team_id: &str,
        away_team_id: &str,
    ) -> Result<String, CreationAppariementError>;
}

/// Pourquoi une rencontre n'a pas pu être programmée (carte 552).
///
/// Trois cas distincts et non un `String` : le contrôleur en tire trois
/// réponses HTTP différentes, et un message libre l'obligerait à lire du texte
/// pour décider d'un code.
#[derive(Debug)]
pub enum CreationAppariementError {
    /// L'une des deux équipes joue déjà cette journée (carte 551).
    DejaEngagee { equipe: String, adversaire: String },
    /// Une équipe n'est pas inscrite à cette saison.
    NonEnrolee(Vec<String>),
    /// Journée introuvable, identifiant invalide, panne de dépôt.
    Indisponible(String),
}

#[derive(Debug, Clone)]
pub struct RoundContextDto {
    pub competition_name: String,
    pub season_name: String,
    pub round_name: String,
}

#[derive(Debug, Default)]
pub struct TierRulesDto {
    pub allowed_inducements: Vec<InducementSpecDto>,
    pub allowed_star_players: Vec<InducementSpecDto>,
}

#[derive(Debug, Clone)]
pub struct InducementSpecDto {
    pub uid: String,
    pub max_qty: u8,
    pub unit_cost: u32,
}

#[derive(Default)]
pub struct TeamInfoDto {
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub roster_id: String,
    pub logo_url: Option<String>,
    pub dedicated_fans: u32,
}

#[async_trait]
pub trait ITeamDataPort: Send + Sync {
    async fn is_team_ready_to_play(&self, team_id: &str) -> Result<bool, String>;

    /// L'équipe est-elle encore en phase d'amélioration des joueurs ?
    ///
    /// Question du garde-fou de correction. Toutes les actions qui rendraient la
    /// correction impossible — recrutement, staff, match suivant — exigent une
    /// phase ultérieure : cette seule réponse suffit à les exclure toutes.
    async fn is_team_in_player_improvement(&self, team_id: &str) -> Result<bool, String>;

    async fn is_coach_of_team(&self, team_id: &str, user_id: &str) -> Result<bool, String>;

    async fn find_team_info(&self, team_id: &str) -> Option<TeamInfoDto>;

    async fn find_team_value(&self, team_id: &str) -> Option<u32>;

    async fn find_team_treasury(&self, team_id: &str) -> Option<u32>;

    async fn find_journeyman_position(&self, team_id: &str) -> Option<JourneymanPositionDto>;

    async fn find_roster_positions(&self, team_id: &str) -> Vec<RosterPositionDto>;
}

#[derive(Debug)]
pub struct JourneymanPositionDto {
    pub position_uid: String,
    pub position_name: String,
}

pub struct RosterPositionDto {
    pub position_uid: String,
    pub position_name: String,
    pub base_cost: u32,
    pub max_qty: u8,
    pub is_journeyman: bool,
    /// Les mots-clefs que porte cette ligne de roster — son espèce et son rôle.
    /// C'est par leur union que l'écran sait quelles Haines valent la peine
    /// d'être proposées en premier (carte 402).
    pub keywords: Vec<String>,
}

pub struct PositionCountDto {
    pub position_uid: String,
    pub count: u8,
}

/// Un mot-clef **haïssable**, tel que le règlement le connaît.
///
/// `hate_skill_uid` n'est pas optionnel ici : le corpus le porte pour tout
/// mot-clef haïssable (carte 399), et sa présence dans ce DTO est ce qui permet
/// au use case de figer la compétence dans l'action sans que personne ait à la
/// résoudre plus tard.
pub struct KeywordDto {
    pub uid: String,
    pub label: String,
    pub hate_skill_uid: String,
}

/// Ce que le règlement connaît comme mots-clefs haïssables.
///
/// **Un port dédié, et non une méthode de plus sur `ITeamDataPort`** : celui-ci
/// répond « que sait-on de cette équipe ? », celui-là « quels mots-clefs le
/// règlement connaît-il ? ». Les fondre obligerait à passer un `team_id` à une
/// question qui n'en a pas.
pub trait IKeywordCatalogPort: Send + Sync {
    /// Les mots-clefs haïssables, et **eux seuls**.
    ///
    /// Le port ne rend jamais les huit autres — un mot-clef de poste existe au
    /// corpus mais ne se hait pas. Le faire filtrer par chaque appelant serait
    /// la garantie qu'un l'oublie ; le DTO ne porte donc pas de drapeau, son
    /// existence dans la réponse **est** le drapeau.
    fn list_hateable(&self) -> Vec<KeywordDto>;
    fn find_hateable(&self, uid: &str) -> Option<KeywordDto>;
}

#[async_trait]
pub trait ICoachDataPort: Send + Sync {
    async fn find_coach_name(&self, coach_id: &str) -> Option<String>;
}

/// Consultation du profil d'un membre d'espace, pour les contrôles d'accès.
///
/// Le BC `match_report` ne connaît pas le BC propriétaire des espaces : il pose
/// la seule question dont il a besoin. Sans ce port, le contrôle d'accès du
/// recap devrait atteindre le contexte de ce BC directement — une référence
/// croisée entre BCs, et un contrôle non testable puisque `AppState` n'est pas
/// constructible en test unitaire.
#[async_trait]
pub trait ISpaceAdminPort: Send + Sync {
    async fn is_space_admin(&self, user_id: &str, space_id: &str) -> bool;
}

#[async_trait]
pub trait ISppCalculatorPort: Send + Sync {
    async fn calculate_match_spp(
        &self,
        home_actions: &[MatchAction],
        away_actions: &[MatchAction],
        home_roster_id: &str,
        away_roster_id: &str,
    ) -> SppMatchResult;
}

#[derive(Debug, Default)]
pub struct SppMatchResult {
    pub home: Vec<PlayerSppDto>,
    pub away: Vec<PlayerSppDto>,
}

#[derive(Debug, Clone)]
pub struct PlayerSppDto {
    pub action_player: ActionPlayer,
    pub spp: u8,
}

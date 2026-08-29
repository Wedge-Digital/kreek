use crate::app::ranking::domain::ranking_line::RankingLine;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use async_trait::async_trait;
use std::collections::HashMap;
use time::OffsetDateTime;

// ── Repository interne (event-sourcing/projection du BC, table append-only) ────

/// Dernière ligne de classement enregistrée pour une équipe — compteurs
/// cumulés depuis le début de la saison, pas seulement les points. DTO de
/// lecture (query), primitifs acceptés (règle CQRS du CLAUDE.md).
#[derive(Clone)]
pub struct RankingLineRow {
    /// Typé et non `String`, contrairement aux autres champs : c'est le seul qui
    /// doit franchir la frontière du domaine (`TeamStanding`). Le décodage a lieu
    /// une fois, au repository, plutôt qu'à chaque lecture — et un id illisible y
    /// devient une erreur bruyante au lieu d'une équipe qui disparaît du
    /// classement sans que personne ne le remarque.
    pub team_id: TeamId,
    pub matches_played: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub ranking_points: u32,
    /// Part bonus du total, déjà comprise dans `ranking_points`.
    pub bonus_points: u32,
    /// Compteurs de départage. `diff_td` n'y figure pas : dérivé.
    pub td_for: u32,
    pub td_against: u32,
    pub casualties: u32,
    pub fouls: u32,
    pub completions: u32,
}

/// Une ligne de classement **avec son contexte**, pour le rejeu (carte 418).
///
/// `RankingLineRow` ne porte que les cumuls : c'est tout ce dont le classement
/// affiché a besoin. Le rejeu, lui, reconstruit des `RankingLine` entières et a
/// donc besoin de la journée, du rapport, de la compétition et de l'horodatage —
/// sans eux, les lignes réécrites perdraient le lien avec le match qui les a
/// produites.
///
/// DTO de lecture (query) : primitives acceptées, sauf `team_id`, décodé une
/// fois au dépôt comme dans `RankingLineRow`.
#[derive(Debug, Clone)]
pub struct RankingLineFullRow {
    /// Les cinq identifiants sont **typés**, pas des `String`, contrairement à
    /// l'usage des DTO de lecture. Ils doivent tous franchir la frontière du
    /// domaine pour reconstruire une `RankingLine`, et le décodage a lieu une
    /// fois au dépôt — même raison que pour `RankingLineRow::team_id` : un id
    /// illisible y devient une erreur bruyante, au lieu d'être remplacé plus
    /// loin par un identifiant neuf qui réécrirait la ligne en silence.
    pub team_id: TeamId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub match_report_id: MatchReportId,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    pub matches_played: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub ranking_points: u32,
    pub bonus_points: u32,
    pub td_for: u32,
    pub td_against: u32,
    pub casualties: u32,
    pub fouls: u32,
    pub completions: u32,
}

#[derive(Debug)]
pub enum RankingRepositoryError {
    Database(String),
    /// Une colonne d'une ligne lue n'est pas décodable vers son type de domaine
    /// (id d'équipe qui n'est pas un ULID). Distinct de `Database` : la requête
    /// a réussi, c'est la donnée stockée qui est incohérente — l'étiquette
    /// « base de données » enverrait sur une fausse piste.
    MalformedRow(String),
}

impl std::fmt::Display for RankingRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(e) => write!(f, "erreur base de données : {e}"),
            Self::MalformedRow(e) => write!(f, "ligne de classement illisible : {e}"),
        }
    }
}

#[async_trait]
pub trait IRankingRepository: Send + Sync {
    /// Dernière ligne d'une équipe pour une saison — `None` si l'équipe n'a
    /// encore aucune ligne (première apparition dans le classement).
    async fn find_latest_line(
        &self,
        season_id: &str,
        team_id: &str,
    ) -> Result<Option<RankingLineRow>, RankingRepositoryError>;

    /// Dernière ligne de chaque équipe ayant au moins une ligne pour la saison.
    async fn find_latest_lines_for_season(
        &self,
        season_id: &str,
    ) -> Result<Vec<RankingLineRow>, RankingRepositoryError>;

    /// Insère plusieurs lignes dans une seule transaction — utilisé pour les
    /// 2 lignes d'un même match (jamais l'une sans l'autre).
    async fn insert_lines(&self, lines: &[RankingLine]) -> Result<(), RankingRepositoryError>;

    /// Supprime les lignes d'un match — les 2 à la fois, jamais l'une sans
    /// l'autre.
    ///
    /// Appelé quand un rapport est dépublié pour correction. Aucun recalcul
    /// n'est nécessaire : le garde-fou « à chaud » garantit qu'aucune des deux
    /// équipes n'a rejoué depuis, donc que ces lignes sont les dernières. Les
    /// lignes du match précédent redeviennent mécaniquement les dernières, et
    /// elles portent déjà les cumuls d'avant ce match.
    async fn delete_lines_for_match(
        &self,
        match_report_id: &str,
    ) -> Result<(), RankingRepositoryError>;

    /// Toutes les lignes de la saison, **dans l'ordre où elles ont été
    /// enregistrées**.
    ///
    /// L'ordre est celui de `sequence`, pas de `recorded_at`. Le dépôt teste
    /// déjà pourquoi : une ligne peut porter un horodatage antérieur à celle qui
    /// la précède, et « c'est l'ordre d'enregistrement qui compte, jamais le
    /// timestamp seul ». Rejouer dans l'ordre des horodatages lirait ces
    /// lignes-là à l'envers, et la différence de deux cumuls rendrait un écart
    /// négatif — une erreur, sur des données pourtant saines.
    async fn find_all_lines_for_season(
        &self,
        season_id: &str,
    ) -> Result<Vec<RankingLineFullRow>, RankingRepositoryError>;

    /// Remplace **tout** le classement d'une saison, en une seule transaction.
    ///
    /// Et non un `delete` suivi d'un `insert_lines` : ce sont deux transactions,
    /// et l'échec de la seconde laisserait la saison **sans classement du tout**.
    ///
    /// Le `DELETE` porte sur `season_id` et non sur une liste d'identifiants :
    /// une ligne orpheline que le rejeu n'aurait pas relue serait sinon
    /// conservée par une opération qui prétend l'avoir remplacée.
    async fn replace_lines_for_season(
        &self,
        season_id: &str,
        lines: &[RankingLine],
    ) -> Result<(), RankingRepositoryError>;

    // ── Points manuels (carte 450) ────────────────────────────────────────────
    //
    // Ils vivent dans leur propre table, `ranking__manual_points`, et non dans
    // `ranking_lines` : le rejeu recalcule celle-ci depuis zéro à partir des
    // cumuls de match, et effacerait donc tout ce qu'on y aurait rangé — sans
    // qu'aucune erreur ne le signale, puisque le rejeu réussit.

    /// Total par équipe, agrégé par la base.
    ///
    /// **Une lecture distincte de `list_manual_points`, et non un `SUM` en
    /// Rust** : le classement ne veut qu'un nombre par équipe, et un `GROUP BY`
    /// coûte moins qu'une liste rapatriée puis repliée. Les équipes sans ligne
    /// sont **absentes** de la carte — elle n'est jamais complète, et son
    /// consommateur lit zéro pour les absentes.
    async fn find_manual_totals_for_season(
        &self,
        season_id: &str,
    ) -> Result<HashMap<String, i32>, RankingRepositoryError>;

    /// Chaque ligne avec son motif, pour la page de gestion — qui doit montrer
    /// *pourquoi*, ce qu'un total ne dit pas.
    async fn list_manual_points(
        &self,
        season_id: &str,
    ) -> Result<Vec<ManualPointRow>, RankingRepositoryError>;

    async fn insert_manual_points(
        &self,
        season_id: &str,
        team_id: &str,
        points: i32,
        reason: &str,
        awarded_by: &str,
    ) -> Result<(), RankingRepositoryError>;

    /// Rend le nombre de lignes supprimées — zéro vaut « introuvable ».
    ///
    /// **La saison est dans le `WHERE`**, et ce n'est pas une précaution
    /// décorative : `space_scope` résout les identifiants du chemin qui ont un
    /// résolveur, et `{point_id}` n'en a aucun. Sans `AND season_id`, un
    /// identifiant deviné supprimerait la ligne d'une autre compétition. Le
    /// refermer par le `WHERE` plutôt que par un contrôle applicatif, c'est la
    /// leçon de la carte 416 : un contrôle s'écrit, puis s'oublie.
    async fn delete_manual_points(
        &self,
        id: i64,
        season_id: &str,
    ) -> Result<u64, RankingRepositoryError>;
}

/// Une ligne de point manuel, telle que la page de gestion l'affiche. DTO de
/// lecture : primitifs acceptés (règle CQRS du `CLAUDE.md`).
pub struct ManualPointRow {
    pub id: i64,
    pub team_id: String,
    pub points: i32,
    pub reason: Option<String>,
    pub awarded_by: String,
    pub awarded_at: OffsetDateTime,
}

// ── ACL vers le BC `competitions` (règles de classement + équipes inscrites) ───
// `ranking` ne parle jamais directement à `teams` — uniquement à `competitions`,
// qui ré-expose ce dont `ranking` a besoin via son propre port vers `teams`
// (`ITeamInfoPort`, déjà en place).

/// Config d'un bonus de classement (offensif/défensif/agressif) — même forme pour
/// les trois : activation + un seuil + des points. Le sens de `threshold` est porté
/// par le champ qui contient ce DTO (`offensive`/`defensive`/`aggressive`). DTO de
/// lecture (query), primitifs acceptés (règle CQRS du CLAUDE.md).
#[derive(Clone)]
pub struct BonusRuleInfo {
    pub activated: bool,
    pub threshold: u32, // min_td (off) | max_td_conceded (def) | min_casualties (agg)
    pub points: u32,
}

#[derive(Clone)]
pub struct RankingRulesInfo {
    pub win_points: u32,
    pub draw_points: u32,
    pub lose_points: u32,
    pub offensive: BonusRuleInfo,
    pub defensive: BonusRuleInfo,
    pub aggressive: BonusRuleInfo,
    /// Critères de départage tels que configurés pour la compétition. **L'ordre
    /// du vecteur porte la priorité** — aucun champ de rang.
    pub tiebreakers: Vec<TiebreakSettingInfo>,
}

/// Un critère de départage et son activation, vus du BC `ranking`. Le code est
/// résolu contre le catalogue par `TiebreakCriterion::from_code`.
#[derive(Clone)]
pub struct TiebreakSettingInfo {
    pub code: String,
    pub activated: bool,
}

#[derive(Clone)]
pub struct EnrolledTeamInfo {
    pub team_id: String,
    pub team_name: String,
}

/// Poule (groupe) de la saison, avec les ids des équipes qui lui sont
/// assignées — reflète `competitions::IGroupRepository::find_groups`, sans
/// exposer son type. Une saison sans poule (ou une seule) reste affichée en
/// classement unique côté widget ; `ranking` n'a pas à connaître pourquoi.
pub struct RankingGroupInfo {
    pub group_id: String,
    pub group_name: String,
    pub team_ids: Vec<String>,
}

#[async_trait]
pub trait IRankingCompetitionPort: Send + Sync {
    async fn find_ranking_rules(&self, season_id: &str) -> Option<RankingRulesInfo>;
    async fn find_enrolled_teams(&self, season_id: &str) -> Vec<EnrolledTeamInfo>;
    async fn find_groups(&self, season_id: &str) -> Vec<RankingGroupInfo>;
}

// ── ACL d'autorisation (carte 450) ────────────────────────────────────────────

/// Qui a le droit d'attribuer ou de retirer des points manuels.
///
/// **Deux méthodes et non une `is_admin`.** Les autorisations viennent de deux
/// sources indépendantes — la compétition porte ses administrateurs, l'espace
/// porte son `SpaceProfile` — et les fondre en une seule réponse cacherait
/// **laquelle** a répondu. Le BC `competitions` a fait ce choix inverse dans
/// `require_admin_access`, et la carte 426 a dû écrire deux tests distincts pour
/// séparer à nouveau ce que le `||` avait mélangé : sans eux, supprimer l'une
/// des deux branches ne rougissait rien.
///
/// Un échec de lecture rend `false` : refuser est le comportement sûr, et
/// remonter une erreur d'infrastructure jusqu'à l'écran n'apprendrait rien de
/// plus au commissaire.
#[async_trait]
pub trait IRankingAdminPort: Send + Sync {
    async fn is_competition_admin(&self, user_id: &str, competition_id: &str) -> bool;
    async fn is_space_admin(&self, user_id: &str, space_id: &str) -> bool;
}

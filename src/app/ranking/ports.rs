use crate::app::ranking::domain::ranking_line::RankingLine;
use crate::app::shared_kernel::team::TeamId;
use async_trait::async_trait;

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

use crate::app::ranking::domain::ranking_line::RankingLine;
use async_trait::async_trait;

// ── Repository interne (event-sourcing/projection du BC, table append-only) ────

/// Dernière ligne de classement enregistrée pour une équipe — compteurs
/// cumulés depuis le début de la saison, pas seulement les points. DTO de
/// lecture (query), primitifs acceptés (règle CQRS du CLAUDE.md).
pub struct RankingLineRow {
    pub team_id: String,
    pub matches_played: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub ranking_points: u32,
}

#[derive(Debug)]
pub enum RankingRepositoryError {
    Database(String),
}

impl std::fmt::Display for RankingRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(e) => write!(f, "erreur base de données : {e}"),
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

pub struct RankingRulesInfo {
    pub win_points: u32,
    pub draw_points: u32,
    pub lose_points: u32,
}

pub struct EnrolledTeamInfo {
    pub team_id: String,
    pub team_name: String,
}

#[async_trait]
pub trait IRankingCompetitionPort: Send + Sync {
    async fn find_ranking_rules(&self, season_id: &str) -> Option<RankingRulesInfo>;
    async fn find_enrolled_teams(&self, season_id: &str) -> Vec<EnrolledTeamInfo>;
}

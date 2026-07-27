use crate::app::ranking::ports::IRankingRepository;
use crate::app::shared_kernel::common_types::MatchReportId;

#[derive(Debug)]
pub enum RevertMatchRankingError {
    Repository(String),
}

/// Retire du classement les 2 lignes d'un match dépublié.
///
/// Symétrique de `record_match_ranking_use_case`, mais bien plus simple : ni
/// règles de compétition à charger, ni recalcul en cascade. Le garde-fou
/// « à chaud » garantit qu'aucune des deux équipes n'a rejoué depuis, donc que
/// ces lignes sont les dernières — les lignes du match précédent, qui portent
/// les cumuls d'avant, redeviennent les dernières d'elles-mêmes.
///
/// Idempotent : un second appel supprime zéro ligne.
pub async fn execute(
    match_report_id: &MatchReportId,
    repo:            &dyn IRankingRepository,
) -> Result<(), RevertMatchRankingError> {
    repo.delete_lines_for_match(&match_report_id.to_string())
        .await
        .map_err(|e| RevertMatchRankingError::Repository(e.to_string()))
}

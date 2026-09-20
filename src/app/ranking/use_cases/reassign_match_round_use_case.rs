use crate::app::ranking::ports::IRankingRepository;
use crate::app::shared_kernel::bloodbowl::ids::{MatchReportId, RoundId};

#[derive(Debug)]
pub enum ReassignMatchRoundError {
    Repository(String),
}

/// Date les lignes d'un match d'une autre journée (carte 557).
///
/// Encore plus simple que le retrait : la journée d'une ligne n'entre dans
/// aucun calcul de cumul ni d'ordre — `sequence` fait foi. La colonne change,
/// et rien d'autre.
///
/// Idempotent : rejouer pose la même valeur.
#[tracing::instrument(skip_all, fields(match_report_id = ?match_report_id, round_id = ?round_id))]
pub async fn execute(
    match_report_id: &MatchReportId,
    round_id: &RoundId,
    repo: &dyn IRankingRepository,
) -> Result<(), ReassignMatchRoundError> {
    repo.reassign_round_for_match(&match_report_id.to_string(), &round_id.to_string())
        .await
        .map_err(|e| ReassignMatchRoundError::Repository(e.to_string()))
}

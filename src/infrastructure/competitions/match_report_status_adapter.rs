use crate::app::competitions::ports::IMatchReportStatusPort;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use async_trait::async_trait;
use std::sync::Arc;

/// Phase d'un rapport publié, telle que la projection `match_report_proj` la
/// nomme. La traduction « publié = suppression interdite » vit ici : le BC
/// `competitions` ne connaît pas les états d'un rapport de match, et le BC
/// `match_report` n'a pas à savoir ce que `competitions` en déduit.
const PUBLISHED_PHASE: &str = "Published";

pub struct MatchReportStatusAdapter {
    match_report_repo: Arc<dyn IMatchReportRepository>,
}

impl MatchReportStatusAdapter {
    pub fn new(match_report_repo: Arc<dyn IMatchReportRepository>) -> Self {
        Self { match_report_repo }
    }
}

#[async_trait]
impl IMatchReportStatusPort for MatchReportStatusAdapter {
    async fn find_published_pairings(&self, pairing_ids: &[String]) -> Result<Vec<String>, String> {
        if pairing_ids.is_empty() {
            return Ok(vec![]);
        }

        let phases = self
            .match_report_repo
            .find_phases_by_pairings(pairing_ids)
            .await
            .map_err(|e| e.to_string())?;

        Ok(phases
            .into_iter()
            .filter(|(_, phase)| phase == PUBLISHED_PHASE)
            .map(|(pairing_id, _)| pairing_id)
            .collect())
    }
}

use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
use crate::app::match_report::domain::value_objects::{D3Roll, MatchReportOrigin};
use crate::app::shared_kernel::common_types::{
    CoachId, CompetitionId, MatchReportId, RoundId, SeasonId, SpaceId,
};
use crate::app::shared_kernel::team::TeamId;

#[derive(Debug, Clone)]
pub struct MatchReportPreMatch {
    pub id: MatchReportId,
    pub space_id: SpaceId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub created_by: CoachId,
    pub origin: MatchReportOrigin,
    pub pairing_id: Option<String>,
    pub home_fan_roll: Option<D3Roll>,
    pub away_fan_roll: Option<D3Roll>,
    pub version: u64,
}

use crate::app::match_report::domain::events::MatchReportDomainEvent;

impl MatchReportPreMatch {
    pub fn record_fan_factor(
        &self,
        home_fan_roll: D3Roll,
        away_fan_roll: D3Roll,
        recorded_by: CoachId,
    ) -> (Self, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::FanFactorRecorded {
            home_fan_roll,
            away_fan_roll,
            recorded_by,
        };
        let mut updated = self.clone();
        updated.home_fan_roll = Some(home_fan_roll);
        updated.away_fan_roll = Some(away_fan_roll);
        updated.version += 1;
        (updated, event)
    }

    pub fn from_draft(draft: MatchReportDraft) -> Self {
        Self {
            id: draft.id,
            space_id: draft.space_id,
            competition_id: draft.competition_id,
            season_id: draft.season_id,
            round_id: draft.round_id,
            home_team_id: draft.home_team_id,
            away_team_id: draft.away_team_id,
            created_by: draft.created_by,
            origin: draft.origin,
            pairing_id: draft.pairing_id,
            home_fan_roll: None,
            away_fan_roll: None,
            version: draft.version + 1,
        }
    }
}

use crate::app::match_report::domain::value_objects::MatchReportOrigin;
use crate::app::shared_kernel::common_types::{
    CoachId, CompetitionId, MatchReportId, RoundId, SeasonId, SpaceId,
};
use crate::app::shared_kernel::team::TeamId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MatchReportDomainEvent {
    MatchReportCreated {
        match_report_id: MatchReportId,
        space_id: SpaceId,
        competition_id: CompetitionId,
        season_id: SeasonId,
        round_id: RoundId,
        home_team_id: TeamId,
        away_team_id: TeamId,
        created_by: CoachId,
        origin: MatchReportOrigin,
        #[serde(default)]
        pairing_id: Option<String>,
    },
    SelectionUpdated {
        home_team_id: TeamId,
        away_team_id: TeamId,
        updated_by: CoachId,
    },
    SelectionConfirmed {
        confirmed_by: CoachId,
    },
    MatchReportCancelled {
        reason: String,
    },
}

impl MatchReportDomainEvent {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::MatchReportCreated { .. } => "MatchReportCreated",
            Self::SelectionUpdated { .. } => "SelectionUpdated",
            Self::SelectionConfirmed { .. } => "SelectionConfirmed",
            Self::MatchReportCancelled { .. } => "MatchReportCancelled",
        }
    }

    pub fn schema_version(&self) -> &'static str {
        "1.0"
    }
}

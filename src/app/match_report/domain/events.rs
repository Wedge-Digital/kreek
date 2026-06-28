use crate::app::match_report::domain::value_objects::{
    ActionId, ActionPlayer, D3Roll, InducementPurchase, MatchActionType, MatchReportOrigin,
    TeamSide, TeamValue, TempPlayer,
};
use crate::app::shared_kernel::inducement_definition::InducementId;
use crate::app::shared_kernel::common_types::{
    CoachId, CompetitionId, MatchReportId, RoundId, SeasonId, SpaceId,
};
use crate::app::shared_kernel::team::TeamId;
use crate::app::match_report::domain::value_objects::TurnNumber;
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
    FanFactorRecorded {
        home_fan_roll: D3Roll,
        away_fan_roll: D3Roll,
        recorded_by: CoachId,
    },
    MatchReportCancelled {
        reason: String,
    },
    TeamValuesRecorded {
        home_team_value: TeamValue,
        away_team_value: TeamValue,
        recorded_by: CoachId,
    },
    InducementsRecorded {
        team_id: TeamId,
        purchases: Vec<InducementPurchase>,
        recorded_by: CoachId,
    },
    StarPlayerEngaged {
        team_id: TeamId,
        star_player_uid: InducementId,
        recorded_by: CoachId,
    },
    TempPlayersInitialized {
        team_id: TeamId,
        players: Vec<TempPlayer>,
    },
    TempPlayersReset {
        team_id: TeamId,
    },
    ActionRecorded {
        action_id: ActionId,
        team_side: TeamSide,
        turn: TurnNumber,
        player: ActionPlayer,
        action: MatchActionType,
        player_display_name: String,
        recorded_by: CoachId,
    },
    ActionDeleted {
        action_id: ActionId,
        team_side: TeamSide,
        deleted_by: CoachId,
    },
}

impl MatchReportDomainEvent {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::MatchReportCreated { .. } => "MatchReportCreated",
            Self::SelectionUpdated { .. } => "SelectionUpdated",
            Self::SelectionConfirmed { .. } => "SelectionConfirmed",
            Self::FanFactorRecorded { .. } => "FanFactorRecorded",
            Self::MatchReportCancelled { .. } => "MatchReportCancelled",
            Self::TeamValuesRecorded { .. } => "TeamValuesRecorded",
            Self::InducementsRecorded { .. } => "InducementsRecorded",
            Self::StarPlayerEngaged { .. } => "StarPlayerEngaged",
            Self::TempPlayersInitialized { .. } => "TempPlayersInitialized",
            Self::TempPlayersReset { .. } => "TempPlayersReset",
            Self::ActionRecorded { .. } => "ActionRecorded",
            Self::ActionDeleted { .. } => "ActionDeleted",
        }
    }

    pub fn schema_version(&self) -> &'static str {
        "1.0"
    }
}

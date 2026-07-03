use crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish;
use crate::app::match_report::domain::value_objects::{
    D3Roll, FanFactorMod, InducementPurchase, MatchAction, MatchGain, MatchReportOrigin, TempPlayer,
};
use crate::app::shared_kernel::inducement_definition::InducementId;
use crate::app::shared_kernel::common_types::{
    CoachId, CompetitionId, MatchReportId, RoundId, SeasonId, SpaceId,
};
use crate::app::shared_kernel::team::TeamId;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct MatchReportPublished {
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
    pub home_dedicated_fans: u32,
    pub away_dedicated_fans: u32,
    pub home_inducements: Option<Vec<InducementPurchase>>,
    pub away_inducements: Option<Vec<InducementPurchase>>,
    pub star_engagements: Vec<(TeamId, InducementId)>,
    pub home_temp_players: Vec<TempPlayer>,
    pub away_temp_players: Vec<TempPlayer>,
    pub home_actions: Vec<MatchAction>,
    pub away_actions: Vec<MatchAction>,
    pub version: u64,
    pub home_gain: MatchGain,
    pub away_gain: MatchGain,
    pub home_fan_mod: FanFactorMod,
    pub away_fan_mod: FanFactorMod,
    pub summary_title: Option<String>,
    pub summary_body: Option<String>,
    pub published_by: CoachId,
    pub published_at: DateTime<Utc>,
}

impl MatchReportPublished {
    pub fn from_ready_to_publish(
        rtp: &MatchReportReadyToPublish,
        published_by: CoachId,
        published_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: rtp.id.clone(),
            space_id: rtp.space_id.clone(),
            competition_id: rtp.competition_id.clone(),
            season_id: rtp.season_id.clone(),
            round_id: rtp.round_id.clone(),
            home_team_id: rtp.home_team_id.clone(),
            away_team_id: rtp.away_team_id.clone(),
            created_by: rtp.created_by,
            origin: rtp.origin,
            pairing_id: rtp.pairing_id.clone(),
            home_fan_roll: rtp.home_fan_roll,
            away_fan_roll: rtp.away_fan_roll,
            home_dedicated_fans: rtp.home_dedicated_fans,
            away_dedicated_fans: rtp.away_dedicated_fans,
            home_inducements: rtp.home_inducements.clone(),
            away_inducements: rtp.away_inducements.clone(),
            star_engagements: rtp.star_engagements.clone(),
            home_temp_players: rtp.home_temp_players.clone(),
            away_temp_players: rtp.away_temp_players.clone(),
            home_actions: rtp.home_actions.clone(),
            away_actions: rtp.away_actions.clone(),
            version: rtp.version + 1,
            home_gain: rtp.home_gain,
            away_gain: rtp.away_gain,
            home_fan_mod: rtp.home_fan_mod,
            away_fan_mod: rtp.away_fan_mod,
            summary_title: rtp.summary_title.clone(),
            summary_body: rtp.summary_body.clone(),
            published_by,
            published_at,
        }
    }
}

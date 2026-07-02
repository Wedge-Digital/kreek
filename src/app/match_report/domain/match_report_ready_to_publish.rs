use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::value_objects::{
    D3Roll, FanFactorMod, InducementPurchase, MatchAction, MatchGain, MatchReportOrigin, TempPlayer,
};
use crate::app::shared_kernel::inducement_definition::InducementId;
use crate::app::shared_kernel::common_types::{
    CoachId, CompetitionId, MatchReportId, RoundId, SeasonId, SpaceId,
};
use crate::app::shared_kernel::team::TeamId;

#[derive(Debug, Clone)]
pub struct MatchReportReadyToPublish {
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
    // champs post-match
    pub home_gain: MatchGain,
    pub away_gain: MatchGain,
    pub home_fan_mod: FanFactorMod,
    pub away_fan_mod: FanFactorMod,
    pub summary_title: Option<String>,
    pub summary_body: Option<String>,
}

impl MatchReportReadyToPublish {
    pub fn from_pre_match(
        pm: &MatchReportPreMatch,
        home_gain: MatchGain,
        away_gain: MatchGain,
        home_fan_mod: FanFactorMod,
        away_fan_mod: FanFactorMod,
        summary_title: Option<String>,
        summary_body: Option<String>,
    ) -> Self {
        Self {
            id: pm.id.clone(),
            space_id: pm.space_id.clone(),
            competition_id: pm.competition_id.clone(),
            season_id: pm.season_id.clone(),
            round_id: pm.round_id.clone(),
            home_team_id: pm.home_team_id.clone(),
            away_team_id: pm.away_team_id.clone(),
            created_by: pm.created_by.clone(),
            origin: pm.origin,
            pairing_id: pm.pairing_id.clone(),
            home_fan_roll: pm.home_fan_roll,
            away_fan_roll: pm.away_fan_roll,
            home_dedicated_fans: pm.home_dedicated_fans,
            away_dedicated_fans: pm.away_dedicated_fans,
            home_inducements: pm.home_inducements.clone(),
            away_inducements: pm.away_inducements.clone(),
            star_engagements: pm.star_engagements.clone(),
            home_temp_players: pm.home_temp_players.clone(),
            away_temp_players: pm.away_temp_players.clone(),
            home_actions: pm.home_actions.clone(),
            away_actions: pm.away_actions.clone(),
            version: pm.version + 1,
            home_gain,
            away_gain,
            home_fan_mod,
            away_fan_mod,
            summary_title,
            summary_body,
        }
    }

    pub fn into_pre_match(self) -> MatchReportPreMatch {
        MatchReportPreMatch {
            id: self.id,
            space_id: self.space_id,
            competition_id: self.competition_id,
            season_id: self.season_id,
            round_id: self.round_id,
            home_team_id: self.home_team_id,
            away_team_id: self.away_team_id,
            created_by: self.created_by,
            origin: self.origin,
            pairing_id: self.pairing_id,
            home_fan_roll: self.home_fan_roll,
            away_fan_roll: self.away_fan_roll,
            home_dedicated_fans: self.home_dedicated_fans,
            away_dedicated_fans: self.away_dedicated_fans,
            home_team_value: None,
            away_team_value: None,
            home_inducements: self.home_inducements,
            away_inducements: self.away_inducements,
            star_engagements: self.star_engagements,
            home_temp_players: self.home_temp_players,
            away_temp_players: self.away_temp_players,
            home_actions: self.home_actions,
            away_actions: self.away_actions,
            version: self.version,
        }
    }

    pub fn record_post_match(
        &self,
        home_gain: MatchGain,
        away_gain: MatchGain,
        home_fan_mod: FanFactorMod,
        away_fan_mod: FanFactorMod,
        summary_title: Option<String>,
        summary_body: Option<String>,
        recorded_by: CoachId,
    ) -> (Self, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::PostMatchRecorded {
            home_gain,
            away_gain,
            home_fan_mod,
            away_fan_mod,
            summary_title: summary_title.clone(),
            summary_body: summary_body.clone(),
            recorded_by,
        };
        let mut updated = self.clone();
        updated.home_gain = home_gain;
        updated.away_gain = away_gain;
        updated.home_fan_mod = home_fan_mod;
        updated.away_fan_mod = away_fan_mod;
        updated.summary_title = summary_title;
        updated.summary_body = summary_body;
        updated.version += 1;
        (updated, event)
    }
}

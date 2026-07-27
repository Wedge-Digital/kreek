use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_published::MatchReportPublished;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::value_objects::{
    D3Roll, DedicatedFans, FanFactorMod, InducementPurchase, MatchAction, MatchGain,
    MatchReportOrigin, TempPlayer,
};
use crate::app::shared_kernel::inducement_definition::InducementId;
use crate::app::shared_kernel::common_types::{
    CoachId, CompetitionId, MatchReportId, RoundId, SeasonId, SpaceId,
};
use crate::app::shared_kernel::team::TeamId;
use chrono::Utc;

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
    pub home_dedicated_fans: DedicatedFans,
    pub away_dedicated_fans: DedicatedFans,
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
    /// Vrai quand ce rapport a déjà été publié puis dépublié pour correction.
    ///
    /// N'existe pas sur `MatchReportPreMatch` : `into_pre_match()` est une
    /// conversion transitoire interne aux use cases, jamais persistée. Une fois
    /// en `ReadyToPublish`, le rapport y reste — `rehydrate` mute l'état en
    /// place pour tous les événements d'édition.
    pub was_published_before: bool, // arch:ok fait booléen sans invariant à protéger
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
            // Un rapport qui atteint cet état pour la première fois n'a jamais
            // été publié. Seul `unpublish()` positionne ce drapeau.
            was_published_before: false,
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

    pub fn publish(&self, published_by: CoachId) -> (MatchReportPublished, MatchReportDomainEvent) {
        let published_at = Utc::now();
        let event = MatchReportDomainEvent::MatchReportPublished {
            published_by,
            published_at,
        };
        let published = MatchReportPublished::from_ready_to_publish(self, published_by, published_at);
        (published, event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::MatchReportOrigin;
    use crate::app::shared_kernel::common_types::{CompetitionId, RoundId, SeasonId, SpaceId};

    fn make_rtp(summary_title: Option<String>, summary_body: Option<String>) -> MatchReportReadyToPublish {
        MatchReportReadyToPublish {
            id: MatchReportId::new(),
            space_id: SpaceId::new(),
            competition_id: CompetitionId::new(),
            season_id: SeasonId::new(),
            round_id: RoundId::new(),
            home_team_id: TeamId::new(),
            away_team_id: TeamId::new(),
            created_by: CoachId::new(),
            origin: MatchReportOrigin::Manual,
            pairing_id: None,
            home_fan_roll: None,
            away_fan_roll: None,
            home_dedicated_fans: DedicatedFans::default(),
            away_dedicated_fans: DedicatedFans::default(),
            home_inducements: None,
            away_inducements: None,
            star_engagements: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
            home_actions: vec![],
            away_actions: vec![],
            version: 5,
            home_gain: MatchGain::try_new(10_000).unwrap(),
            away_gain: MatchGain::try_new(5_000).unwrap(),
            home_fan_mod: FanFactorMod::try_new(1).unwrap(),
            away_fan_mod: FanFactorMod::try_new(-1).unwrap(),
            summary_title,
            summary_body,
            was_published_before: false,
        }
    }

    #[test]
    fn publish_produces_published_state_with_all_fields_copied() {
        let rtp = make_rtp(Some("Titre".to_string()), Some("Corps".to_string()));
        let published_by = CoachId::new();
        let before = Utc::now();
        let (published, _event) = rtp.publish(published_by);

        assert_eq!(published.id, rtp.id);
        assert_eq!(published.home_team_id, rtp.home_team_id);
        assert_eq!(published.summary_title, rtp.summary_title);
        assert_eq!(published.summary_body, rtp.summary_body);
        assert_eq!(published.published_by, published_by);
        assert!(published.published_at >= before);
    }

    #[test]
    fn publish_succeeds_without_summary() {
        let rtp = make_rtp(None, None);
        let (published, _event) = rtp.publish(CoachId::new());
        assert!(published.summary_title.is_none());
        assert!(published.summary_body.is_none());
    }

    #[test]
    fn publish_increments_version() {
        let rtp = make_rtp(None, None);
        let (published, _event) = rtp.publish(CoachId::new());
        assert_eq!(published.version, rtp.version + 1);
    }
}

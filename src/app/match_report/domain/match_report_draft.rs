use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::value_objects::MatchReportOrigin;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;

#[derive(Debug, Clone)]
pub struct MatchReportDraft {
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
    pub version: u64,
}

impl MatchReportDraft {
    pub fn create(
        id: MatchReportId,
        space_id: SpaceId,
        competition_id: CompetitionId,
        season_id: SeasonId,
        round_id: RoundId,
        home_team_id: TeamId,
        away_team_id: TeamId,
        created_by: CoachId,
        origin: MatchReportOrigin,
        pairing_id: Option<String>,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        if home_team_id == away_team_id {
            return Err(DomainError::SameTeam);
        }
        let event = MatchReportDomainEvent::MatchReportCreated {
            match_report_id: id,
            space_id,
            competition_id,
            season_id,
            round_id,
            home_team_id,
            away_team_id,
            created_by,
            origin,
            pairing_id,
        };
        let draft = Self::from_created_event(&event);
        Ok((draft, event))
    }

    pub fn update_selection(
        &self,
        home_team_id: TeamId,
        away_team_id: TeamId,
        updated_by: CoachId,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        if home_team_id == away_team_id {
            return Err(DomainError::SameTeam);
        }
        let event = MatchReportDomainEvent::SelectionUpdated {
            home_team_id,
            away_team_id,
            updated_by,
        };
        let updated = self.apply_selection_updated(&event);
        Ok((updated, event))
    }

    pub fn cancel(self, reason: String) -> MatchReportDomainEvent {
        MatchReportDomainEvent::MatchReportCancelled {
            reason,
            home_team_id: Some(self.home_team_id),
            away_team_id: Some(self.away_team_id),
        }
    }

    pub fn confirm_selection(
        self,
        confirmed_by: CoachId,
    ) -> (MatchReportPreMatch, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::SelectionConfirmed { confirmed_by };
        let pre_match = MatchReportPreMatch::from_draft(self);
        (pre_match, event)
    }

    pub(crate) fn from_created_event(event: &MatchReportDomainEvent) -> Self {
        match event {
            MatchReportDomainEvent::MatchReportCreated {
                match_report_id,
                space_id,
                competition_id,
                season_id,
                round_id,
                home_team_id,
                away_team_id,
                created_by,
                origin,
                pairing_id,
            } => Self {
                id: *match_report_id,
                space_id: *space_id,
                competition_id: *competition_id,
                season_id: *season_id,
                round_id: *round_id,
                home_team_id: *home_team_id,
                away_team_id: *away_team_id,
                created_by: *created_by,
                origin: *origin,
                pairing_id: pairing_id.clone(),
                version: 1,
            },
            _ => unreachable!("from_created_event appelé avec un mauvais event"),
        }
    }

    pub(crate) fn apply_selection_updated(&self, event: &MatchReportDomainEvent) -> Self {
        match event {
            MatchReportDomainEvent::SelectionUpdated {
                home_team_id,
                away_team_id,
                ..
            } => Self {
                home_team_id: *home_team_id,
                away_team_id: *away_team_id,
                version: self.version + 1,
                ..self.clone()
            },
            _ => unreachable!("apply_selection_updated appelé avec un mauvais event"),
        }
    }
}

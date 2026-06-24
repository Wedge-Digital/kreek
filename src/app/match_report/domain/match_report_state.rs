use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;

#[derive(Debug)]
pub enum MatchReportState {
    Draft(MatchReportDraft),
    PreMatch(MatchReportPreMatch),
}

pub fn rehydrate(events: Vec<MatchReportDomainEvent>) -> Result<MatchReportState, DomainError> {
    if events.is_empty() {
        return Err(DomainError::EmptyEventStream);
    }

    let mut state: Option<MatchReportState> = None;

    for event in &events {
        state = Some(match (state, event) {
            (None, MatchReportDomainEvent::MatchReportCreated { .. }) => {
                MatchReportState::Draft(MatchReportDraft::from_created_event(event))
            }
            (
                Some(MatchReportState::Draft(draft)),
                MatchReportDomainEvent::SelectionUpdated { .. },
            ) => MatchReportState::Draft(draft.apply_selection_updated(event)),
            (
                Some(MatchReportState::Draft(draft)),
                MatchReportDomainEvent::SelectionConfirmed { .. },
            ) => MatchReportState::PreMatch(MatchReportPreMatch::from_draft(draft)),
            _ => return Err(DomainError::InvalidEventSequence),
        });
    }

    state.ok_or(DomainError::EmptyEventStream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::MatchReportOrigin;
    use crate::app::shared_kernel::common_types::{
        CoachId, CompetitionId, MatchReportId, RoundId, SeasonId, SpaceId,
    };
    use crate::app::shared_kernel::team::TeamId;

    fn test_ids() -> (
        MatchReportId,
        SpaceId,
        CompetitionId,
        SeasonId,
        RoundId,
        TeamId,
        TeamId,
        CoachId,
    ) {
        (
            MatchReportId::new(),
            SpaceId::new(),
            CompetitionId::new(),
            SeasonId::new(),
            RoundId::new(),
            TeamId::new(),
            TeamId::new(),
            CoachId::new(),
        )
    }

    fn created_event(
        mr_id: MatchReportId,
        space_id: SpaceId,
        comp_id: CompetitionId,
        season_id: SeasonId,
        round_id: RoundId,
        home_id: TeamId,
        away_id: TeamId,
        coach_id: CoachId,
    ) -> MatchReportDomainEvent {
        MatchReportDomainEvent::MatchReportCreated {
            match_report_id: mr_id,
            space_id,
            competition_id: comp_id,
            season_id,
            round_id,
            home_team_id: home_id,
            away_team_id: away_id,
            created_by: coach_id,
            origin: MatchReportOrigin::Manual,
        }
    }

    // ── create ───────────────────────────────────────────────────────────

    #[test]
    fn create_with_same_team_fails() {
        let team_id = TeamId::new();
        let result = MatchReportDraft::create(
            MatchReportId::new(),
            SpaceId::new(),
            CompetitionId::new(),
            SeasonId::new(),
            RoundId::new(),
            team_id,
            team_id,
            CoachId::new(),
            MatchReportOrigin::Manual,
        );
        assert_eq!(result.unwrap_err(), DomainError::SameTeam);
    }

    #[test]
    fn create_produces_created_event_and_draft() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let (draft, event) = MatchReportDraft::create(
            mr_id,
            space_id,
            comp_id,
            season_id,
            round_id,
            home_id,
            away_id,
            coach_id,
            MatchReportOrigin::Pairing,
        )
        .unwrap();

        assert_eq!(draft.id, mr_id);
        assert_eq!(draft.home_team_id, home_id);
        assert_eq!(draft.away_team_id, away_id);
        assert_eq!(draft.origin, MatchReportOrigin::Pairing);
        assert_eq!(draft.version, 1);
        assert!(matches!(
            event,
            MatchReportDomainEvent::MatchReportCreated { .. }
        ));
    }

    // ── update_selection ─────────────────────────────────────────────────

    #[test]
    fn update_selection_with_same_team_fails() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let (draft, _) = MatchReportDraft::create(
            mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            MatchReportOrigin::Manual,
        )
        .unwrap();

        let new_team = TeamId::new();
        let result = draft.update_selection(new_team, new_team, coach_id);
        assert_eq!(result.unwrap_err(), DomainError::SameTeam);
    }

    #[test]
    fn update_selection_produces_updated_draft() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let (draft, _) = MatchReportDraft::create(
            mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            MatchReportOrigin::Manual,
        )
        .unwrap();

        let new_home = TeamId::new();
        let new_away = TeamId::new();
        let (updated, event) = draft.update_selection(new_home, new_away, coach_id).unwrap();

        assert_eq!(updated.home_team_id, new_home);
        assert_eq!(updated.away_team_id, new_away);
        assert_eq!(updated.version, 2);
        assert!(matches!(
            event,
            MatchReportDomainEvent::SelectionUpdated { .. }
        ));
    }

    // ── confirm_selection ────────────────────────────────────────────────

    #[test]
    fn confirm_selection_returns_prematch() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let (draft, _) = MatchReportDraft::create(
            mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            MatchReportOrigin::Manual,
        )
        .unwrap();

        let (pre_match, event) = draft.confirm_selection(coach_id);
        assert_eq!(pre_match.id, mr_id);
        assert_eq!(pre_match.home_team_id, home_id);
        assert_eq!(pre_match.version, 2);
        assert!(matches!(
            event,
            MatchReportDomainEvent::SelectionConfirmed { .. }
        ));
    }

    // ── rehydrate ────────────────────────────────────────────────────────

    #[test]
    fn rehydrate_created_returns_draft() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let events = vec![created_event(
            mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
        )];
        let state = rehydrate(events).unwrap();
        assert!(matches!(state, MatchReportState::Draft(_)));
        if let MatchReportState::Draft(draft) = state {
            assert_eq!(draft.id, mr_id);
            assert_eq!(draft.version, 1);
        }
    }

    #[test]
    fn rehydrate_created_then_updated_returns_draft() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let new_home = TeamId::new();
        let new_away = TeamId::new();
        let events = vec![
            created_event(
                mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            ),
            MatchReportDomainEvent::SelectionUpdated {
                home_team_id: new_home,
                away_team_id: new_away,
                updated_by: coach_id,
            },
        ];
        let state = rehydrate(events).unwrap();
        if let MatchReportState::Draft(draft) = state {
            assert_eq!(draft.home_team_id, new_home);
            assert_eq!(draft.away_team_id, new_away);
            assert_eq!(draft.version, 2);
        } else {
            panic!("attendu Draft");
        }
    }

    #[test]
    fn rehydrate_created_then_confirmed_returns_prematch() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let events = vec![
            created_event(
                mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            ),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: coach_id,
            },
        ];
        let state = rehydrate(events).unwrap();
        assert!(matches!(state, MatchReportState::PreMatch(_)));
        if let MatchReportState::PreMatch(pm) = state {
            assert_eq!(pm.id, mr_id);
            assert_eq!(pm.version, 2);
        }
    }

    #[test]
    fn rehydrate_empty_stream_fails() {
        let result = rehydrate(vec![]);
        assert_eq!(result.unwrap_err(), DomainError::EmptyEventStream);
    }

    #[test]
    fn rehydrate_invalid_sequence_fails() {
        let result = rehydrate(vec![MatchReportDomainEvent::SelectionConfirmed {
            confirmed_by: CoachId::new(),
        }]);
        assert_eq!(result.unwrap_err(), DomainError::InvalidEventSequence);
    }
}

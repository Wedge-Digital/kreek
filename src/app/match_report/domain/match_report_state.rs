use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::shared_kernel::common_types::MatchReportId;

#[derive(Debug)]
pub struct MatchReportCancelled {
    pub id: MatchReportId,
    pub reason: String, // arch:ok texte libre
}

#[derive(Debug)]
pub enum MatchReportState {
    Draft(MatchReportDraft),
    PreMatch(MatchReportPreMatch),
    Cancelled(MatchReportCancelled),
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
            (
                Some(MatchReportState::Draft(draft)),
                MatchReportDomainEvent::MatchReportCancelled { reason, .. },
            ) => MatchReportState::Cancelled(MatchReportCancelled {
                id: draft.id,
                reason: reason.clone(),
            }),
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::FanFactorRecorded {
                    home_fan_roll,
                    away_fan_roll,
                    ..
                },
            ) => {
                let mut updated = pm;
                updated.home_fan_roll = Some(*home_fan_roll);
                updated.away_fan_roll = Some(*away_fan_roll);
                updated.version += 1;
                MatchReportState::PreMatch(updated)
            }
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::TeamValuesRecorded { home_team_value, away_team_value, .. },
            ) => {
                let mut updated = pm;
                updated.home_team_value = Some(*home_team_value);
                updated.away_team_value = Some(*away_team_value);
                updated.version += 1;
                MatchReportState::PreMatch(updated)
            }
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::InducementsRecorded { team_id, purchases, .. },
            ) => {
                let mut updated = pm;
                if team_id == &updated.home_team_id {
                    updated.home_inducements = Some(purchases.clone());
                } else {
                    updated.away_inducements = Some(purchases.clone());
                }
                updated.version += 1;
                MatchReportState::PreMatch(updated)
            }
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::StarPlayerEngaged { team_id, star_player_uid, .. },
            ) => {
                let mut updated = pm;
                updated.star_engagements.push((team_id.clone(), star_player_uid.clone()));
                updated.version += 1;
                MatchReportState::PreMatch(updated)
            }
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::TempPlayersInitialized { team_id, players },
            ) => {
                let mut updated = pm;
                if team_id == &updated.home_team_id {
                    updated.home_temp_players = players.clone();
                } else {
                    updated.away_temp_players = players.clone();
                }
                updated.version += 1;
                MatchReportState::PreMatch(updated)
            }
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::TempPlayersReset { team_id },
            ) => {
                let mut updated = pm;
                if team_id == &updated.home_team_id {
                    updated.home_temp_players = vec![];
                } else {
                    updated.away_temp_players = vec![];
                }
                updated.version += 1;
                MatchReportState::PreMatch(updated)
            }
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::ActionRecorded {
                    action_id, team_side, turn, player, action, player_display_name, ..
                },
            ) => {
                use crate::app::match_report::domain::value_objects::{MatchAction, TeamSide};
                let entry = MatchAction {
                    id: action_id.clone(),
                    turn: *turn,
                    player: player.clone(),
                    action: action.clone(),
                    player_display_name: player_display_name.clone(),
                };
                let mut updated = pm;
                match team_side {
                    TeamSide::Home => updated.home_actions.push(entry),
                    TeamSide::Away => updated.away_actions.push(entry),
                }
                updated.version += 1;
                MatchReportState::PreMatch(updated)
            }
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::ActionDeleted { action_id, team_side, .. },
            ) => {
                use crate::app::match_report::domain::value_objects::TeamSide;
                let mut updated = pm;
                match team_side {
                    TeamSide::Home => updated.home_actions.retain(|a| &a.id != action_id),
                    TeamSide::Away => updated.away_actions.retain(|a| &a.id != action_id),
                }
                updated.version += 1;
                MatchReportState::PreMatch(updated)
            }
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::MatchReportCancelled { reason, .. },
            ) => MatchReportState::Cancelled(MatchReportCancelled {
                id: pm.id,
                reason: reason.clone(),
            }),
            _ => return Err(DomainError::InvalidEventSequence),
        });
    }

    state.ok_or(DomainError::EmptyEventStream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::{D3Roll, MatchReportOrigin};
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
            pairing_id: None,
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
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

    // ── record_fan_factor ─────────────────────────────────────────────

    fn make_pre_match() -> MatchReportPreMatch {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let (draft, _) = MatchReportDraft::create(
            mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            MatchReportOrigin::Manual, None,
        ).unwrap();
        let (pm, _) = draft.confirm_selection(coach_id);
        pm
    }

    #[test]
    fn record_fan_factor_emet_evenement() {
        let pm = make_pre_match();
        let (_, event) = pm.record_fan_factor(
            D3Roll::try_new(2).unwrap(),
            D3Roll::try_new(1).unwrap(),
            CoachId::new(),
        );
        assert!(matches!(event, MatchReportDomainEvent::FanFactorRecorded { .. }));
    }

    #[test]
    fn record_fan_factor_met_a_jour_les_champs() {
        let pm = make_pre_match();
        let (updated, _) = pm.record_fan_factor(
            D3Roll::try_new(3).unwrap(),
            D3Roll::try_new(1).unwrap(),
            CoachId::new(),
        );
        assert_eq!(updated.home_fan_roll, Some(D3Roll::try_new(3).unwrap()));
        assert_eq!(updated.away_fan_roll, Some(D3Roll::try_new(1).unwrap()));
        assert_eq!(updated.version, pm.version + 1);
    }

    // ── rehydrate fan factor ────────────────────────────────────────────

    #[test]
    fn rehydrate_fan_factor_recorded() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let events = vec![
            created_event(mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id),
            MatchReportDomainEvent::SelectionConfirmed { confirmed_by: coach_id },
            MatchReportDomainEvent::FanFactorRecorded {
                home_fan_roll: D3Roll::try_new(2).unwrap(),
                away_fan_roll: D3Roll::try_new(3).unwrap(),
                recorded_by: coach_id,
            },
        ];
        let state = rehydrate(events).unwrap();
        if let MatchReportState::PreMatch(pm) = state {
            assert_eq!(pm.home_fan_roll, Some(D3Roll::try_new(2).unwrap()));
            assert_eq!(pm.away_fan_roll, Some(D3Roll::try_new(3).unwrap()));
            assert_eq!(pm.version, 3);
        } else {
            panic!("attendu PreMatch");
        }
    }

    #[test]
    fn rehydrate_double_fan_factor_last_wins() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let events = vec![
            created_event(mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id),
            MatchReportDomainEvent::SelectionConfirmed { confirmed_by: coach_id },
            MatchReportDomainEvent::FanFactorRecorded {
                home_fan_roll: D3Roll::try_new(1).unwrap(),
                away_fan_roll: D3Roll::try_new(1).unwrap(),
                recorded_by: coach_id,
            },
            MatchReportDomainEvent::FanFactorRecorded {
                home_fan_roll: D3Roll::try_new(3).unwrap(),
                away_fan_roll: D3Roll::try_new(2).unwrap(),
                recorded_by: coach_id,
            },
        ];
        let state = rehydrate(events).unwrap();
        if let MatchReportState::PreMatch(pm) = state {
            assert_eq!(pm.home_fan_roll, Some(D3Roll::try_new(3).unwrap()));
            assert_eq!(pm.away_fan_roll, Some(D3Roll::try_new(2).unwrap()));
            assert_eq!(pm.version, 4);
        } else {
            panic!("attendu PreMatch");
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

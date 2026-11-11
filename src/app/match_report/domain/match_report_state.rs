use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_published::MatchReportPublished;
use crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish;
use crate::app::match_report::domain::mvp_draw::{DesignatedPlayer, DrawnMvp};
use crate::app::match_report::domain::value_objects::CorrectionEligibility;
use crate::app::match_report::domain::value_objects::NoStallingBonuses;
use crate::app::match_report::domain::value_objects::{ActionId, TeamSide};
use crate::app::shared_kernel::bloodbowl::ids::{MatchReportId, RoundId};
use crate::app::shared_kernel::identity::ids::SpaceId;

#[derive(Debug)]
pub struct MatchReportCancelled {
    pub id: MatchReportId,
    pub reason: String, // arch:ok texte libre
}

#[derive(Debug)]
pub enum MatchReportState {
    Draft(MatchReportDraft),
    PreMatch(MatchReportPreMatch),
    ReadyToPublish(MatchReportReadyToPublish),
    Published(MatchReportPublished),
    Cancelled(MatchReportCancelled),
}

impl MatchReportState {
    /// Le récapitulatif des coups de pouce propose-t-il l'annulation (carte
    /// 628, R3) ? En avant-match seulement, et si le domaine l'accepterait.
    pub fn can_cancel_inducements(&self) -> bool {
        match self {
            Self::PreMatch(pm) => pm.can_cancel_inducements(),
            Self::Draft(_) | Self::ReadyToPublish(_) | Self::Published(_) | Self::Cancelled(_) => {
                false
            }
        }
    }

    /// L'espace du rapport, quel que soit son état.
    ///
    /// `None` pour un rapport annulé : cet état ne garde que son identifiant et
    /// son motif. Les appelants d'aujourd'hui — la publication d'app events sur
    /// des faits d'avant-match — ne l'atteignent pas.
    pub fn space_id(&self) -> Option<&SpaceId> {
        match self {
            Self::Draft(d) => Some(&d.space_id),
            Self::PreMatch(pm) => Some(&pm.space_id),
            Self::ReadyToPublish(rtp) => Some(&rtp.space_id),
            Self::Published(p) => Some(&p.space_id),
            Self::Cancelled(_) => None,
        }
    }

    /// Rattache le rapport à une autre journée (carte 557), quel que soit son
    /// avancement : `(version attendue à l'append, événement)`.
    ///
    /// **Un seul point pour les quatre états** : la règle est la même partout,
    /// et la répartir dans quatre structs aurait donné quatre occasions de la
    /// faire dériver. `None` quand la journée est déjà celle-là — rien à
    /// écrire, et un événement vide dans le flux n'apprendrait rien.
    pub fn reassign_round(
        &self,
        round_id: RoundId,
    ) -> Result<Option<(u64, MatchReportDomainEvent)>, DomainError> {
        let (courante, version) = match self {
            Self::Draft(d) => (&d.round_id, d.version),
            Self::PreMatch(pm) => (&pm.round_id, pm.version),
            Self::ReadyToPublish(rtp) => (&rtp.round_id, rtp.version),
            Self::Published(p) => (&p.round_id, p.version),
            Self::Cancelled(_) => return Err(DomainError::ReportCancelled),
        };
        if courante == &round_id {
            return Ok(None);
        }
        Ok(Some((
            version,
            MatchReportDomainEvent::RoundReassigned { round_id },
        )))
    }
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
            // Le démarrage se rejoue en deux temps (carte 631) : la
            // confirmation pose « Je saisis tout » et son auteur comme
            // démarreur, `EntryModeChosen` les remplace s'il suit. Un flux
            // écrit avant la carte n'a que le premier temps.
            (
                Some(MatchReportState::Draft(draft)),
                MatchReportDomainEvent::SelectionConfirmed { confirmed_by },
            ) => MatchReportState::PreMatch(MatchReportPreMatch::from_draft(draft, *confirmed_by)),
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::EntryModeChosen { mode, started_by },
            ) => MatchReportState::PreMatch(pm.choose_entry_mode(*mode, *started_by)),
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
                    home_dedicated_fans,
                    away_dedicated_fans,
                    ..
                },
            ) => {
                let mut updated = pm;
                updated.home_fan_roll = Some(*home_fan_roll);
                updated.away_fan_roll = Some(*away_fan_roll);
                updated.home_dedicated_fans = *home_dedicated_fans;
                updated.away_dedicated_fans = *away_dedicated_fans;
                updated.version += 1;
                MatchReportState::PreMatch(updated)
            }
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::TeamValuesRecorded {
                    home_team_value,
                    away_team_value,
                    ..
                },
            ) => {
                let mut updated = pm;
                updated.home_team_value = Some(*home_team_value);
                updated.away_team_value = Some(*away_team_value);
                updated.version += 1;
                MatchReportState::PreMatch(updated)
            }
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::InducementsRecorded {
                    team_id,
                    purchases,
                    recorded_by,
                },
            ) => {
                let mut updated = pm;
                updated.star_engagements.retain(|(tid, _)| tid != team_id);
                if team_id == &updated.home_team_id {
                    updated.home_inducements = Some(purchases.clone());
                    updated.home_inducements_recorded_by = Some(*recorded_by);
                } else {
                    updated.away_inducements = Some(purchases.clone());
                    updated.away_inducements_recorded_by = Some(*recorded_by);
                }
                updated.version += 1;
                MatchReportState::PreMatch(updated)
            }
            // `PreMatch` seulement (carte 626, R3) : en `ReadyToPublish`, la
            // dépense de trésorerie est figée, l'arête reste invalide.
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::InducementsCancelled { .. },
            ) => MatchReportState::PreMatch(pm.apply_inducements_cancelled()),
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::StarPlayerEngaged {
                    team_id,
                    star_player_uid,
                    ..
                },
            ) => {
                let mut updated = pm;
                updated
                    .star_engagements
                    .push((team_id.clone(), star_player_uid.clone()));
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
                MatchReportDomainEvent::TempPlayersReset { team_id, .. },
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
                    action_id,
                    team_side,
                    turn,
                    player,
                    action,
                    player_display_name,
                    player_position,
                    ..
                },
            ) => {
                use crate::app::match_report::domain::value_objects::{MatchAction, TeamSide};
                let entry = MatchAction {
                    id: action_id.clone(),
                    turn: *turn,
                    player: player.clone(),
                    action: action.clone(),
                    player_display_name: player_display_name.clone(),
                    player_position: player_position.clone(),
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
                MatchReportDomainEvent::ActionDeleted {
                    action_id,
                    team_side,
                    ..
                },
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
            // Le tirage du MVP (carte 638) : l'action tirée est déjà rejouée
            // par l'`ActionRecorded` qui le précède, seul le tirage reste.
            (
                Some(MatchReportState::PreMatch(mut pm)),
                MatchReportDomainEvent::MvpDrawn {
                    action_id,
                    team_side,
                    designated,
                },
            ) => {
                pm.apply_mvp_drawn(drawn_mvp_of(action_id, *team_side, designated));
                MatchReportState::PreMatch(pm)
            }
            (
                Some(MatchReportState::ReadyToPublish(mut rtp)),
                MatchReportDomainEvent::MvpDrawn {
                    action_id,
                    team_side,
                    designated,
                },
            ) => {
                rtp.drawn_mvps
                    .push(drawn_mvp_of(action_id, *team_side, designated));
                rtp.version += 1;
                MatchReportState::ReadyToPublish(rtp)
            }
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::MatchReportCancelled { reason, .. },
            ) => MatchReportState::Cancelled(MatchReportCancelled {
                id: pm.id,
                reason: reason.clone(),
            }),
            (
                Some(MatchReportState::PreMatch(pm)),
                MatchReportDomainEvent::PostMatchRecorded {
                    home_gain,
                    away_gain,
                    home_no_stalling_bonus,
                    away_no_stalling_bonus,
                    home_fan_mod,
                    away_fan_mod,
                    summary_title,
                    summary_body,
                    ..
                },
            ) => MatchReportState::ReadyToPublish(MatchReportReadyToPublish::from_pre_match(
                &pm,
                *home_gain,
                *away_gain,
                NoStallingBonuses {
                    home: *home_no_stalling_bonus,
                    away: *away_no_stalling_bonus,
                },
                *home_fan_mod,
                *away_fan_mod,
                summary_title.clone(),
                summary_body.clone(),
            )),
            (
                Some(MatchReportState::ReadyToPublish(rtp)),
                MatchReportDomainEvent::MatchReportCancelled { reason, .. },
            ) => MatchReportState::Cancelled(MatchReportCancelled {
                id: rtp.id,
                reason: reason.clone(),
            }),
            (
                Some(MatchReportState::ReadyToPublish(rtp)),
                MatchReportDomainEvent::FanFactorRecorded {
                    home_fan_roll,
                    away_fan_roll,
                    home_dedicated_fans,
                    away_dedicated_fans,
                    ..
                },
            ) => {
                let mut updated = rtp;
                updated.home_fan_roll = Some(*home_fan_roll);
                updated.away_fan_roll = Some(*away_fan_roll);
                updated.home_dedicated_fans = *home_dedicated_fans;
                updated.away_dedicated_fans = *away_dedicated_fans;
                updated.version += 1;
                MatchReportState::ReadyToPublish(updated)
            }
            (
                Some(MatchReportState::ReadyToPublish(rtp)),
                MatchReportDomainEvent::TeamValuesRecorded { .. },
            ) => {
                let mut updated = rtp;
                updated.version += 1;
                MatchReportState::ReadyToPublish(updated)
            }
            (
                Some(MatchReportState::ReadyToPublish(rtp)),
                MatchReportDomainEvent::InducementsRecorded {
                    team_id,
                    purchases,
                    recorded_by,
                },
            ) => {
                let mut updated = rtp;
                updated.star_engagements.retain(|(tid, _)| tid != team_id);
                if team_id == &updated.home_team_id {
                    updated.home_inducements = Some(purchases.clone());
                    updated.home_inducements_recorded_by = Some(*recorded_by);
                } else {
                    updated.away_inducements = Some(purchases.clone());
                    updated.away_inducements_recorded_by = Some(*recorded_by);
                }
                updated.version += 1;
                MatchReportState::ReadyToPublish(updated)
            }
            (
                Some(MatchReportState::ReadyToPublish(rtp)),
                MatchReportDomainEvent::StarPlayerEngaged {
                    team_id,
                    star_player_uid,
                    ..
                },
            ) => {
                let mut updated = rtp;
                updated
                    .star_engagements
                    .push((team_id.clone(), star_player_uid.clone()));
                updated.version += 1;
                MatchReportState::ReadyToPublish(updated)
            }
            (
                Some(MatchReportState::ReadyToPublish(rtp)),
                MatchReportDomainEvent::TempPlayersInitialized { team_id, players },
            ) => {
                let mut updated = rtp;
                if team_id == &updated.home_team_id {
                    updated.home_temp_players = players.clone();
                } else {
                    updated.away_temp_players = players.clone();
                }
                updated.version += 1;
                MatchReportState::ReadyToPublish(updated)
            }
            (
                Some(MatchReportState::ReadyToPublish(rtp)),
                MatchReportDomainEvent::TempPlayersReset { team_id, .. },
            ) => {
                let mut updated = rtp;
                if team_id == &updated.home_team_id {
                    updated.home_temp_players = vec![];
                } else {
                    updated.away_temp_players = vec![];
                }
                updated.version += 1;
                MatchReportState::ReadyToPublish(updated)
            }
            (
                Some(MatchReportState::ReadyToPublish(rtp)),
                MatchReportDomainEvent::ActionRecorded {
                    action_id,
                    team_side,
                    turn,
                    player,
                    action,
                    player_display_name,
                    player_position,
                    ..
                },
            ) => {
                use crate::app::match_report::domain::value_objects::{MatchAction, TeamSide};
                let entry = MatchAction {
                    id: action_id.clone(),
                    turn: *turn,
                    player: player.clone(),
                    action: action.clone(),
                    player_display_name: player_display_name.clone(),
                    player_position: player_position.clone(),
                };
                let mut updated = rtp;
                match team_side {
                    TeamSide::Home => updated.home_actions.push(entry),
                    TeamSide::Away => updated.away_actions.push(entry),
                }
                updated.version += 1;
                MatchReportState::ReadyToPublish(updated)
            }
            (
                Some(MatchReportState::ReadyToPublish(rtp)),
                MatchReportDomainEvent::ActionDeleted {
                    action_id,
                    team_side,
                    ..
                },
            ) => {
                use crate::app::match_report::domain::value_objects::TeamSide;
                let mut updated = rtp;
                match team_side {
                    TeamSide::Home => updated.home_actions.retain(|a| &a.id != action_id),
                    TeamSide::Away => updated.away_actions.retain(|a| &a.id != action_id),
                }
                updated.version += 1;
                MatchReportState::ReadyToPublish(updated)
            }
            (
                Some(MatchReportState::ReadyToPublish(rtp)),
                MatchReportDomainEvent::PostMatchRecorded {
                    home_gain,
                    away_gain,
                    home_no_stalling_bonus,
                    away_no_stalling_bonus,
                    home_fan_mod,
                    away_fan_mod,
                    summary_title,
                    summary_body,
                    ..
                },
            ) => {
                let mut updated = rtp;
                updated.home_gain = *home_gain;
                updated.away_gain = *away_gain;
                updated.home_no_stalling_bonus = *home_no_stalling_bonus;
                updated.away_no_stalling_bonus = *away_no_stalling_bonus;
                updated.home_fan_mod = *home_fan_mod;
                updated.away_fan_mod = *away_fan_mod;
                updated.summary_title = summary_title.clone();
                updated.summary_body = summary_body.clone();
                updated.version += 1;
                MatchReportState::ReadyToPublish(updated)
            }
            (
                Some(MatchReportState::ReadyToPublish(rtp)),
                MatchReportDomainEvent::MatchReportPublished {
                    published_by,
                    published_at,
                },
            ) => MatchReportState::Published(MatchReportPublished::from_ready_to_publish(
                &rtp,
                *published_by,
                *published_at,
            )),
            // Correction : retour en état corrigeable. Le couple avec l'arête
            // ci-dessus suffit à rejouer un nombre quelconque d'allers-retours,
            // `rehydrate` n'étant qu'un pli sur le flux.
            (
                Some(MatchReportState::Published(published)),
                MatchReportDomainEvent::MatchReportUnpublished { unpublished_by, .. },
            ) => {
                let (rtp, _) = published
                    .unpublish(*unpublished_by, CorrectionEligibility::Eligible)
                    .map_err(|_| DomainError::InvalidEventSequence)?;
                MatchReportState::ReadyToPublish(rtp)
            }
            // Le changement de journée vaut pour les quatre états vivants
            // (carte 557) : la journée change, le reste de l'état non.
            (Some(vivant), MatchReportDomainEvent::RoundReassigned { round_id }) => {
                rejouer_la_journee(vivant, round_id.clone())?
            }
            _ => return Err(DomainError::InvalidEventSequence),
        });
    }

    state.ok_or(DomainError::EmptyEventStream)
}

/// Le tirage que `MvpDrawn` retient.
fn drawn_mvp_of(
    action_id: &ActionId,
    team_side: TeamSide,
    designated: &[DesignatedPlayer],
) -> DrawnMvp {
    DrawnMvp {
        action_id: action_id.clone(),
        team_side,
        designated: designated.to_vec(),
    }
}

fn rejouer_la_journee(
    state: MatchReportState,
    round_id: RoundId,
) -> Result<MatchReportState, DomainError> {
    Ok(match state {
        MatchReportState::Draft(mut d) => {
            d.round_id = round_id;
            d.version += 1;
            MatchReportState::Draft(d)
        }
        MatchReportState::PreMatch(mut pm) => {
            pm.round_id = round_id;
            pm.version += 1;
            MatchReportState::PreMatch(pm)
        }
        MatchReportState::ReadyToPublish(mut rtp) => {
            rtp.round_id = round_id;
            rtp.version += 1;
            MatchReportState::ReadyToPublish(rtp)
        }
        MatchReportState::Published(mut p) => {
            p.round_id = round_id;
            p.version += 1;
            MatchReportState::Published(p)
        }
        MatchReportState::Cancelled(_) => return Err(DomainError::InvalidEventSequence),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::{
        D3Roll, DedicatedFans, EntryMode, MatchReportOrigin,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{
        CompetitionId, MatchReportId, RoundId, SeasonId,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};

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
            mr_id,
            space_id,
            comp_id,
            season_id,
            round_id,
            home_id,
            away_id,
            coach_id,
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
            mr_id,
            space_id,
            comp_id,
            season_id,
            round_id,
            home_id,
            away_id,
            coach_id,
            MatchReportOrigin::Manual,
            None,
        )
        .unwrap();

        let new_home = TeamId::new();
        let new_away = TeamId::new();
        let (updated, event) = draft
            .update_selection(new_home, new_away, coach_id)
            .unwrap();

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
            mr_id,
            space_id,
            comp_id,
            season_id,
            round_id,
            home_id,
            away_id,
            coach_id,
            MatchReportOrigin::Manual,
            None,
        )
        .unwrap();

        let (pre_match, events) = draft.confirm_selection(coach_id, EntryMode::SoloEntry);
        assert_eq!(pre_match.id, mr_id);
        assert_eq!(pre_match.home_team_id, home_id);
        assert_eq!(pre_match.version, 3, "deux événements après la création");
        assert_eq!(pre_match.entry_mode, EntryMode::SoloEntry);
        assert_eq!(pre_match.started_by, coach_id);
        assert!(matches!(
            events.as_slice(),
            [
                MatchReportDomainEvent::SelectionConfirmed { confirmed_by },
                MatchReportDomainEvent::EntryModeChosen {
                    mode: EntryMode::SoloEntry,
                    started_by,
                },
            ] if *confirmed_by == coach_id && *started_by == coach_id
        ));
    }

    /// Ce que `confirm_selection` rend est ce que le rejeu de ses événements
    /// reconstruit : version, mode et démarreur (carte 631).
    #[test]
    fn confirm_selection_matches_the_replay_of_its_events() {
        let (draft, coach_id) = draft_by_competition();
        let created = created_event_of(&draft);
        let (pre_match, events) = draft.confirm_selection(coach_id, EntryMode::SoloEntry);

        let mut stream = vec![created];
        stream.extend(events);
        let MatchReportState::PreMatch(replayed) = rehydrate(stream).unwrap() else {
            panic!("attendu PreMatch");
        };
        assert_eq!(replayed.version, pre_match.version);
        assert_eq!(replayed.started_by, pre_match.started_by);
        assert_eq!(replayed.entry_mode, pre_match.entry_mode);
    }

    // ── mode de saisie et démarreur (carte 631) ──────────────────────────

    /// Un brouillon né du calendrier : `created_by` vaut un autre identifiant
    /// que celui du coach qui démarre, comme en production.
    fn draft_by_competition() -> (MatchReportDraft, CoachId) {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, creator) = test_ids();
        let (draft, _) = MatchReportDraft::create(
            mr_id,
            space_id,
            comp_id,
            season_id,
            round_id,
            home_id,
            away_id,
            creator,
            MatchReportOrigin::Pairing,
            None,
        )
        .unwrap();
        (draft, CoachId::new())
    }

    fn created_event_of(draft: &MatchReportDraft) -> MatchReportDomainEvent {
        created_event(
            draft.id,
            draft.space_id,
            draft.competition_id,
            draft.season_id,
            draft.round_id,
            draft.home_team_id,
            draft.away_team_id,
            draft.created_by,
        )
    }

    fn entry_mode_chosen(started_by: CoachId) -> MatchReportDomainEvent {
        MatchReportDomainEvent::EntryModeChosen {
            mode: EntryMode::SoloEntry,
            started_by,
        }
    }

    /// Un flux écrit avant la carte n'a pas d'`EntryModeChosen` : il se rejoue
    /// en « Je saisis tout », démarré par celui qui a confirmé — et non par
    /// `created_by`.
    #[test]
    fn an_old_stream_replays_as_solo_entry_started_by_the_confirmer() {
        let (draft, confirmer) = draft_by_competition();
        let events = vec![
            created_event_of(&draft),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: confirmer,
            },
        ];
        let MatchReportState::PreMatch(pm) = rehydrate(events).unwrap() else {
            panic!("attendu PreMatch");
        };
        assert_eq!(pm.entry_mode, EntryMode::SoloEntry);
        assert_eq!(pm.started_by, confirmer);
        assert_ne!(pm.started_by, pm.created_by);
        assert_eq!(pm.version, 2);
    }

    /// `EntryModeChosen` remplace ce que la confirmation a posé.
    #[test]
    fn a_new_stream_replays_the_chosen_mode() {
        let (draft, confirmer) = draft_by_competition();
        let starter = CoachId::new();
        let events = vec![
            created_event_of(&draft),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: confirmer,
            },
            entry_mode_chosen(starter),
        ];
        let MatchReportState::PreMatch(pm) = rehydrate(events).unwrap() else {
            panic!("attendu PreMatch");
        };
        assert_eq!(pm.entry_mode, EntryMode::SoloEntry);
        assert_eq!(pm.started_by, starter);
        assert_eq!(pm.version, 3);
    }

    /// « Chacun son équipe » se rejoue tel qu'il a été choisi (carte 652).
    #[test]
    fn a_team_entry_choice_replays_as_team_entry() {
        let (draft, confirmer) = draft_by_competition();
        let events = vec![
            created_event_of(&draft),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: confirmer,
            },
            MatchReportDomainEvent::EntryModeChosen {
                mode: EntryMode::TeamEntry,
                started_by: confirmer,
            },
        ];
        let MatchReportState::PreMatch(pm) = rehydrate(events).unwrap() else {
            panic!("attendu PreMatch");
        };
        assert_eq!(pm.entry_mode, EntryMode::TeamEntry);
        assert_eq!(pm.started_by, confirmer);
    }

    /// Le démarreur traverse la publication et la dépublication, comme
    /// `created_by`.
    #[test]
    fn the_starter_survives_publication_and_unpublication() {
        let (draft, confirmer) = draft_by_competition();
        let starter = CoachId::new();
        let mut events = vec![
            created_event_of(&draft),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: confirmer,
            },
            entry_mode_chosen(starter),
            post_match_event(confirmer),
            published_event(confirmer),
        ];
        let MatchReportState::Published(p) = rehydrate(events.clone()).unwrap() else {
            panic!("attendu Published");
        };
        assert_eq!(
            (p.started_by, p.entry_mode),
            (starter, EntryMode::SoloEntry)
        );

        events.push(unpublished_event(confirmer));
        let MatchReportState::ReadyToPublish(rtp) = rehydrate(events).unwrap() else {
            panic!("attendu ReadyToPublish");
        };
        assert_eq!(
            (rtp.started_by, rtp.entry_mode),
            (starter, EntryMode::SoloEntry)
        );
        assert_eq!(rtp.into_pre_match().started_by, starter);
    }

    /// Un flux ancien garde son démarreur jusqu'à la publication.
    #[test]
    fn an_old_stream_keeps_its_starter_once_published() {
        let (draft, confirmer) = draft_by_competition();
        let events = vec![
            created_event_of(&draft),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: confirmer,
            },
            post_match_event(confirmer),
            published_event(confirmer),
        ];
        let MatchReportState::Published(p) = rehydrate(events).unwrap() else {
            panic!("attendu Published");
        };
        assert_eq!(
            (p.started_by, p.entry_mode),
            (confirmer, EntryMode::SoloEntry)
        );
    }

    /// Le mode se choisit au démarrage, nulle part ailleurs : ni sur un
    /// brouillon, ni après le match.
    #[test]
    fn entry_mode_chosen_outside_pre_match_is_refused() {
        let (draft, coach) = draft_by_competition();
        let on_draft = vec![created_event_of(&draft), entry_mode_chosen(coach)];
        assert_eq!(
            rehydrate(on_draft).unwrap_err(),
            DomainError::InvalidEventSequence
        );

        let after_match = vec![
            created_event_of(&draft),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: coach,
            },
            post_match_event(coach),
            entry_mode_chosen(coach),
        ];
        assert_eq!(
            rehydrate(after_match).unwrap_err(),
            DomainError::InvalidEventSequence
        );
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
            mr_id,
            space_id,
            comp_id,
            season_id,
            round_id,
            home_id,
            away_id,
            coach_id,
            MatchReportOrigin::Manual,
            None,
        )
        .unwrap();
        let (pm, _) = draft.confirm_selection(coach_id, EntryMode::SoloEntry);
        pm
    }

    #[test]
    fn record_fan_factor_emet_evenement() {
        let pm = make_pre_match();
        let (_, event) = pm.record_fan_factor(
            D3Roll::try_new(2).unwrap(),
            D3Roll::try_new(1).unwrap(),
            DedicatedFans::default(),
            DedicatedFans::default(),
            CoachId::new(),
        );
        assert!(matches!(
            event,
            MatchReportDomainEvent::FanFactorRecorded { .. }
        ));
    }

    #[test]
    fn record_fan_factor_met_a_jour_les_champs() {
        let pm = make_pre_match();
        let (updated, _) = pm.record_fan_factor(
            D3Roll::try_new(3).unwrap(),
            D3Roll::try_new(1).unwrap(),
            DedicatedFans::default(),
            DedicatedFans::default(),
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
            created_event(
                mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            ),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: coach_id,
            },
            MatchReportDomainEvent::FanFactorRecorded {
                home_fan_roll: D3Roll::try_new(2).unwrap(),
                away_fan_roll: D3Roll::try_new(3).unwrap(),
                home_dedicated_fans: DedicatedFans::default(),
                away_dedicated_fans: DedicatedFans::default(),
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
            created_event(
                mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            ),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: coach_id,
            },
            MatchReportDomainEvent::FanFactorRecorded {
                home_fan_roll: D3Roll::try_new(1).unwrap(),
                away_fan_roll: D3Roll::try_new(1).unwrap(),
                home_dedicated_fans: DedicatedFans::default(),
                away_dedicated_fans: DedicatedFans::default(),
                recorded_by: coach_id,
            },
            MatchReportDomainEvent::FanFactorRecorded {
                home_fan_roll: D3Roll::try_new(3).unwrap(),
                away_fan_roll: D3Roll::try_new(2).unwrap(),
                home_dedicated_fans: DedicatedFans::default(),
                away_dedicated_fans: DedicatedFans::default(),
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

    #[test]
    fn rehydrate_ready_to_publish_then_published_yields_published_state() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let events = vec![
            created_event(
                mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            ),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: coach_id,
            },
            MatchReportDomainEvent::PostMatchRecorded {
                home_gain: crate::app::match_report::domain::value_objects::MatchGain::try_new(
                    10_000,
                )
                .unwrap(),
                away_gain: crate::app::match_report::domain::value_objects::MatchGain::try_new(
                    5_000,
                )
                .unwrap(),
                home_no_stalling_bonus: Default::default(),
                away_no_stalling_bonus: Default::default(),
                home_fan_mod:
                    crate::app::match_report::domain::value_objects::FanFactorMod::try_new(1)
                        .unwrap(),
                away_fan_mod:
                    crate::app::match_report::domain::value_objects::FanFactorMod::try_new(-1)
                        .unwrap(),
                summary_title: None,
                summary_body: None,
                recorded_by: coach_id,
            },
            MatchReportDomainEvent::MatchReportPublished {
                published_by: coach_id,
                published_at: chrono::Utc::now(),
            },
        ];
        let state = rehydrate(events).unwrap();
        match state {
            MatchReportState::Published(p) => {
                assert_eq!(p.id, mr_id);
                assert_eq!(p.published_by, coach_id);
            }
            _ => panic!("attendu Published"),
        }
    }

    // ── correction : dépublication ───────────────────────────────────────

    fn post_match_event(coach_id: CoachId) -> MatchReportDomainEvent {
        use crate::app::match_report::domain::value_objects::{FanFactorMod, MatchGain};
        MatchReportDomainEvent::PostMatchRecorded {
            home_gain: MatchGain::try_new(10_000).unwrap(),
            away_gain: MatchGain::try_new(5_000).unwrap(),
            home_no_stalling_bonus: Default::default(),
            away_no_stalling_bonus: Default::default(),
            home_fan_mod: FanFactorMod::try_new(1).unwrap(),
            away_fan_mod: FanFactorMod::try_new(-1).unwrap(),
            summary_title: None,
            summary_body: None,
            recorded_by: coach_id,
        }
    }

    fn published_event(coach_id: CoachId) -> MatchReportDomainEvent {
        MatchReportDomainEvent::MatchReportPublished {
            published_by: coach_id,
            published_at: chrono::Utc::now(),
        }
    }

    fn unpublished_event(coach_id: CoachId) -> MatchReportDomainEvent {
        MatchReportDomainEvent::MatchReportUnpublished {
            unpublished_by: coach_id,
            unpublished_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn rehydrate_published_then_unpublished_yields_ready_to_publish_with_flag() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let events = vec![
            created_event(
                mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            ),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: coach_id,
            },
            post_match_event(coach_id),
            published_event(coach_id),
            unpublished_event(coach_id),
        ];

        match rehydrate(events).unwrap() {
            MatchReportState::ReadyToPublish(rtp) => {
                assert_eq!(rtp.id, mr_id);
                assert!(rtp.was_published_before);
            }
            _ => panic!("attendu ReadyToPublish"),
        }
    }

    /// Le nombre de corrections n'est pas limité : `rehydrate` n'étant qu'un pli
    /// sur le flux, l'alternance doit se rejouer sans cas particulier.
    #[test]
    fn rehydrate_supporte_trois_cycles_publier_depublier() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let mut events = vec![
            created_event(
                mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            ),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: coach_id,
            },
            post_match_event(coach_id),
        ];
        for _ in 0..3 {
            events.push(published_event(coach_id));
            events.push(unpublished_event(coach_id));
        }

        match rehydrate(events).unwrap() {
            MatchReportState::ReadyToPublish(rtp) => assert!(rtp.was_published_before),
            _ => panic!("attendu ReadyToPublish"),
        }
    }

    /// Le drapeau doit survivre à l'édition qui suit la dépublication. Il vit sur
    /// `ReadyToPublish` seul : `rehydrate` y mute l'état en place, sans jamais
    /// repasser par `PreMatch`.
    #[test]
    fn le_drapeau_survit_a_l_edition_apres_depublication() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let events = vec![
            created_event(
                mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            ),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: coach_id,
            },
            post_match_event(coach_id),
            published_event(coach_id),
            unpublished_event(coach_id),
            // le coach corrige les gains, puis resaisit l'après-match
            post_match_event(coach_id),
        ];

        match rehydrate(events).unwrap() {
            MatchReportState::ReadyToPublish(rtp) => assert!(rtp.was_published_before),
            _ => panic!("attendu ReadyToPublish"),
        }
    }

    #[test]
    fn un_rapport_jamais_publie_ne_porte_pas_le_drapeau() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let events = vec![
            created_event(
                mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            ),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: coach_id,
            },
            post_match_event(coach_id),
        ];

        match rehydrate(events).unwrap() {
            MatchReportState::ReadyToPublish(rtp) => assert!(!rtp.was_published_before),
            _ => panic!("attendu ReadyToPublish"),
        }
    }

    #[test]
    fn depublier_un_rapport_non_publie_est_une_sequence_invalide() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let events = vec![
            created_event(
                mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            ),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: coach_id,
            },
            post_match_event(coach_id),
            unpublished_event(coach_id),
        ];

        assert!(matches!(
            rehydrate(events),
            Err(DomainError::InvalidEventSequence)
        ));
    }

    // ── annulation : pairing supprimé ────────────────────────────────────

    /// Les trois états non publiés doivent produire un événement d'annulation
    /// portant les deux équipes — c'est la seule source d'information du
    /// listener `teams`, l'état `Cancelled` ne retenant qu'un id et une raison.
    #[test]
    fn annuler_un_brouillon_porte_les_deux_equipes() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let events = vec![created_event(
            mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
        )];

        let MatchReportState::Draft(draft) = rehydrate(events).unwrap() else {
            panic!("attendu Draft");
        };

        assert!(matches!(
            draft.cancel("Pairing supprimé".to_string()),
            MatchReportDomainEvent::MatchReportCancelled {
                home_team_id: Some(h),
                away_team_id: Some(a),
                ..
            } if h == home_id && a == away_id
        ));
    }

    #[test]
    fn annuler_un_rapport_confirme_porte_les_deux_equipes() {
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

        let MatchReportState::PreMatch(pm) = rehydrate(events).unwrap() else {
            panic!("attendu PreMatch");
        };

        assert!(matches!(
            pm.cancel("Pairing supprimé".to_string()),
            MatchReportDomainEvent::MatchReportCancelled {
                home_team_id: Some(h),
                away_team_id: Some(a),
                ..
            } if h == home_id && a == away_id
        ));
    }

    #[test]
    fn annuler_un_rapport_pret_a_publier_porte_les_deux_equipes() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let events = vec![
            created_event(
                mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            ),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: coach_id,
            },
            post_match_event(coach_id),
        ];

        let MatchReportState::ReadyToPublish(rtp) = rehydrate(events).unwrap() else {
            panic!("attendu ReadyToPublish");
        };

        assert!(matches!(
            rtp.cancel("Pairing supprimé".to_string()),
            MatchReportDomainEvent::MatchReportCancelled {
                home_team_id: Some(h),
                away_team_id: Some(a),
                ..
            } if h == home_id && a == away_id
        ));
    }

    fn cancelled_event(home_id: TeamId, away_id: TeamId) -> MatchReportDomainEvent {
        MatchReportDomainEvent::MatchReportCancelled {
            reason: "Pairing supprimé".to_string(),
            journeymen: vec![],
            home_team_id: Some(home_id),
            away_team_id: Some(away_id),
            pairing_id: None,
        }
    }

    #[test]
    fn rehydrate_annulation_depuis_les_trois_etats_non_publies() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let created = created_event(
            mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
        );
        let confirmed = MatchReportDomainEvent::SelectionConfirmed {
            confirmed_by: coach_id,
        };

        let flux_par_etat = vec![
            (
                "Draft",
                vec![created.clone(), cancelled_event(home_id, away_id)],
            ),
            (
                "PreMatch",
                vec![
                    created.clone(),
                    confirmed.clone(),
                    cancelled_event(home_id, away_id),
                ],
            ),
            (
                "ReadyToPublish",
                vec![
                    created,
                    confirmed,
                    post_match_event(coach_id),
                    cancelled_event(home_id, away_id),
                ],
            ),
        ];

        for (etat, events) in flux_par_etat {
            match rehydrate(events) {
                Ok(MatchReportState::Cancelled(c)) => assert_eq!(c.id, mr_id),
                _ => panic!("annulation depuis {etat} : attendu Cancelled"),
            }
        }
    }

    /// Les annulations persistées avant l'ajout des ids d'équipes n'ont pas ces
    /// champs : la rehydratation de l'existant ne doit pas casser.
    #[test]
    fn rehydrate_une_annulation_a_l_ancien_format() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let created = created_event(
            mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
        );
        let ancien_format = serde_json::json!({
            "type": "MatchReportCancelled",
            "reason": "Pairing supprimé",
        });

        let cancelled: MatchReportDomainEvent =
            serde_json::from_value(ancien_format).expect("ancien format désérialisable");

        match rehydrate(vec![created, cancelled]) {
            Ok(MatchReportState::Cancelled(c)) => assert_eq!(c.id, mr_id),
            _ => panic!("attendu Cancelled"),
        }
    }

    // ── Le changement de journée (carte 557) ─────────────────────────────────

    fn brouillon_et_journee() -> (Vec<MatchReportDomainEvent>, RoundId) {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let created = created_event(
            mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
        );
        (vec![created], RoundId::new())
    }

    fn journee_de(state: &MatchReportState) -> RoundId {
        match state {
            MatchReportState::Draft(d) => d.round_id.clone(),
            MatchReportState::PreMatch(pm) => pm.round_id.clone(),
            MatchReportState::ReadyToPublish(rtp) => rtp.round_id.clone(),
            MatchReportState::Published(p) => p.round_id.clone(),
            MatchReportState::Cancelled(_) => panic!("annulé"),
        }
    }

    #[test]
    fn rehydrate_round_reassigned_change_la_journee_d_un_brouillon() {
        let (mut events, nouvelle) = brouillon_et_journee();
        events.push(MatchReportDomainEvent::RoundReassigned {
            round_id: nouvelle.clone(),
        });

        let state = rehydrate(events).unwrap();

        assert!(matches!(state, MatchReportState::Draft(_)));
        assert_eq!(journee_de(&state), nouvelle);
    }

    /// Le même événement vaut pour les quatre états vivants : ici le rapport
    /// publié, qui est le cas de la carte — et le plus lourd de conséquences.
    #[test]
    fn rehydrate_round_reassigned_change_la_journee_d_un_rapport_publie() {
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let nouvelle = RoundId::new();
        let events = vec![
            created_event(
                mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id,
            ),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: coach_id,
            },
            MatchReportDomainEvent::PostMatchRecorded {
                home_gain: crate::app::match_report::domain::value_objects::MatchGain::try_new(
                    10_000,
                )
                .unwrap(),
                away_gain: crate::app::match_report::domain::value_objects::MatchGain::try_new(
                    5_000,
                )
                .unwrap(),
                home_no_stalling_bonus: Default::default(),
                away_no_stalling_bonus: Default::default(),
                home_fan_mod:
                    crate::app::match_report::domain::value_objects::FanFactorMod::try_new(1)
                        .unwrap(),
                away_fan_mod:
                    crate::app::match_report::domain::value_objects::FanFactorMod::try_new(-1)
                        .unwrap(),
                summary_title: None,
                summary_body: None,
                recorded_by: coach_id,
            },
            MatchReportDomainEvent::MatchReportPublished {
                published_by: coach_id,
                published_at: chrono::Utc::now(),
            },
            MatchReportDomainEvent::RoundReassigned {
                round_id: nouvelle.clone(),
            },
        ];

        let state = rehydrate(events).unwrap();

        assert!(matches!(state, MatchReportState::Published(_)));
        assert_eq!(journee_de(&state), nouvelle);
    }

    #[test]
    fn reassign_round_rend_l_evenement_et_la_version_attendue() {
        let (events, nouvelle) = brouillon_et_journee();
        let state = rehydrate(events).unwrap();

        let (version, event) = state
            .reassign_round(nouvelle.clone())
            .unwrap()
            .expect("journée différente");

        assert_eq!(version, 1);
        assert!(
            matches!(event, MatchReportDomainEvent::RoundReassigned { round_id } if round_id == nouvelle)
        );
    }

    /// Rien à écrire quand la journée est déjà la bonne : un événement vide
    /// dans le flux n'apprendrait rien.
    #[test]
    fn reassign_round_vers_la_meme_journee_ne_produit_rien() {
        let (events, _) = brouillon_et_journee();
        let state = rehydrate(events).unwrap();
        let courante = journee_de(&state);

        assert!(state.reassign_round(courante).unwrap().is_none());
    }

    #[test]
    fn reassign_round_est_refuse_sur_un_rapport_annule() {
        let (mut events, nouvelle) = brouillon_et_journee();
        events.push(MatchReportDomainEvent::MatchReportCancelled {
            reason: "test".into(),
            journeymen: vec![],
            home_team_id: None,
            away_team_id: None,
            pairing_id: None,
        });
        let state = rehydrate(events).unwrap();

        assert!(matches!(
            state.reassign_round(nouvelle),
            Err(DomainError::ReportCancelled)
        ));
    }

    // ── carte 626 : annuler les coups de pouce ───────────────────────────

    /// Un avant-match où le domicile a validé un achat et engagé une vedette.
    fn avant_match_valide() -> (Vec<MatchReportDomainEvent>, CoachId) {
        use crate::app::match_report::domain::value_objects::{
            InducementCost, InducementPurchase, InducementQty,
        };
        use crate::app::shared_kernel::bloodbowl::inducement_definition::InducementId;
        let (mr_id, space_id, comp_id, season_id, round_id, home_id, away_id, coach_id) =
            test_ids();
        let star = InducementId("MORG".into());
        let events = vec![
            created_event(
                mr_id,
                space_id,
                comp_id,
                season_id,
                round_id,
                home_id.clone(),
                away_id,
                coach_id,
            ),
            MatchReportDomainEvent::SelectionConfirmed {
                confirmed_by: coach_id,
            },
            MatchReportDomainEvent::InducementsRecorded {
                team_id: home_id.clone(),
                purchases: vec![InducementPurchase {
                    uid: star.clone(),
                    qty: InducementQty::try_new(1).unwrap(),
                    unit_cost: InducementCost::try_new(100).unwrap(),
                }],
                recorded_by: coach_id,
            },
            MatchReportDomainEvent::StarPlayerEngaged {
                team_id: home_id,
                star_player_uid: star,
                recorded_by: coach_id,
            },
        ];
        (events, coach_id)
    }

    #[test]
    fn rehydrate_inducements_cancelled_efface_les_achats() {
        let (mut events, coach_id) = avant_match_valide();
        events.push(MatchReportDomainEvent::InducementsCancelled {
            cancelled_by: coach_id,
        });
        let MatchReportState::PreMatch(pm) = rehydrate(events).unwrap() else {
            panic!("attendu PreMatch");
        };
        assert!(pm.home_inducements.is_none());
        assert!(pm.away_inducements.is_none());
        assert!(pm.star_engagements.is_empty());
        assert_eq!(pm.version, 5);
    }

    /// R3 : en `ReadyToPublish`, la dépense de trésorerie est figée.
    #[test]
    fn rehydrate_inducements_cancelled_apres_l_etape_5_est_invalide() {
        let (mut events, coach_id) = avant_match_valide();
        events.push(post_match_event(coach_id));
        events.push(MatchReportDomainEvent::InducementsCancelled {
            cancelled_by: coach_id,
        });
        assert_eq!(
            rehydrate(events).unwrap_err(),
            DomainError::InvalidEventSequence
        );
    }

    // ── carte 628 : le récapitulatif ─────────────────────────────────────

    /// Décision 5 : l'auteur se reconstruit au rejeu, et l'annulation l'efface.
    #[test]
    fn rehydrate_garde_l_auteur_de_la_validation_puis_l_efface() {
        let (mut events, coach_id) = avant_match_valide();
        let MatchReportState::PreMatch(pm) = rehydrate(events.clone()).unwrap() else {
            panic!("attendu PreMatch");
        };
        assert_eq!(pm.home_inducements_recorded_by, Some(coach_id));
        assert_eq!(pm.away_inducements_recorded_by, None);
        events.push(MatchReportDomainEvent::InducementsCancelled {
            cancelled_by: coach_id,
        });
        let MatchReportState::PreMatch(pm) = rehydrate(events).unwrap() else {
            panic!("attendu PreMatch");
        };
        assert_eq!(pm.home_inducements_recorded_by, None);
    }

    /// L'auteur survit au passage en `ReadyToPublish`, qui ne propose jamais
    /// l'annulation (R3).
    #[test]
    fn l_auteur_survit_a_l_etape_5_qui_ne_propose_plus_l_annulation() {
        let (mut events, coach_id) = avant_match_valide();
        assert!(rehydrate(events.clone()).unwrap().can_cancel_inducements());
        events.push(post_match_event(coach_id));
        let state = rehydrate(events).unwrap();
        assert!(!state.can_cancel_inducements());
        let MatchReportState::ReadyToPublish(rtp) = state else {
            panic!("attendu ReadyToPublish");
        };
        assert_eq!(rtp.home_inducements_recorded_by, Some(coach_id));
    }

    // ── carte 637 : le rejeu ignore la règle du MVP ──────────────────────

    fn old_mvp(coach_id: CoachId, id: &str, turn: u8) -> MatchReportDomainEvent {
        use crate::app::match_report::domain::value_objects::{
            ActionId, ActionPlayer, MatchActionType, TeamSide, TurnNumber,
        };
        use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
        MatchReportDomainEvent::ActionRecorded {
            action_id: ActionId(id.into()),
            team_side: TeamSide::Home,
            turn: TurnNumber::try_new(turn).unwrap(),
            player: ActionPlayer::Regular(PlayerId::new()),
            action: MatchActionType::Mvp,
            player_display_name: format!("Joueur {id}"),
            player_position: String::new(),
            recorded_by: coach_id,
        }
    }

    /// Un rapport saisi avant la règle : **deux** MVP au domicile, aux tours 7
    /// et 3. Il se rejoue à l'identique, publié compris — la règle ne vaut
    /// qu'à la saisie.
    #[test]
    fn un_ancien_rapport_a_deux_mvp_hors_tour_16_se_rejoue() {
        let (mut events, coach_id) = avant_match_valide();
        events.push(old_mvp(coach_id, "mvp-1", 7));
        events.push(old_mvp(coach_id, "mvp-2", 3));
        events.push(post_match_event(coach_id));
        events.push(published_event(coach_id));
        let MatchReportState::Published(published) = rehydrate(events).unwrap() else {
            panic!("attendu Published");
        };
        let turns: Vec<u8> = published
            .home_actions
            .iter()
            .map(|a| a.turn.value())
            .collect();
        assert_eq!(turns, vec![7, 3]);
    }

    /// En correction, l'étape refuse un troisième MVP à ce rapport : on
    /// supprime d'abord.
    #[test]
    fn un_ancien_rapport_a_mvp_refuse_un_mvp_de_plus() {
        use crate::app::match_report::domain::value_objects::{ActionId, ActionPlayer, TeamSide};
        use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
        let (mut events, coach_id) = avant_match_valide();
        events.push(old_mvp(coach_id, "mvp-1", 7));
        let MatchReportState::PreMatch(pm) = rehydrate(events).unwrap() else {
            panic!("attendu PreMatch");
        };
        let refus = pm.record_mvp(
            TeamSide::Home,
            ActionPlayer::Regular(PlayerId::new()),
            "Nouveau".into(),
            String::new(),
            ActionId("mvp-3".into()),
            coach_id,
        );
        assert_eq!(refus.unwrap_err(), DomainError::MvpAlreadyRecorded);
    }

    // ── carte 638 : le tirage au sort, rejoué ────────────────────────────

    fn drawn_events(coach_id: CoachId) -> Vec<MatchReportDomainEvent> {
        use crate::app::match_report::domain::mvp_draw::DesignatedPlayer;
        use crate::app::match_report::domain::value_objects::ActionPlayer;
        use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
        let designated: Vec<DesignatedPlayer> = (0..6)
            .map(|i| DesignatedPlayer {
                player: ActionPlayer::Regular(PlayerId::new()),
                display_name: format!("Nain {i}"),
                position: String::new(),
            })
            .collect();
        vec![
            old_mvp(coach_id, "tire", 16),
            MatchReportDomainEvent::MvpDrawn {
                action_id: ActionId("tire".into()),
                team_side: TeamSide::Home,
                designated,
            },
        ]
    }

    /// `MvpDrawn` se rejoue : l'agrégat retient le tirage, et le MVP tiré ne
    /// se supprime pas.
    #[test]
    fn le_tirage_se_rejoue_et_reste_definitif() {
        let (mut events, coach_id) = avant_match_valide();
        events.extend(drawn_events(coach_id));
        let MatchReportState::PreMatch(pm) = rehydrate(events).unwrap() else {
            panic!("attendu PreMatch");
        };
        let drawn = pm
            .drawn_mvp(&ActionId("tire".into()))
            .expect("tirage rejoué");
        assert_eq!(drawn.designated.len(), 6);
        assert_eq!(
            pm.delete_action(&ActionId("tire".into()), coach_id)
                .unwrap_err(),
            DomainError::DrawnMvpIsFinal
        );
    }

    /// Le tirage survit à la publication et à la dépublication : en
    /// correction, il reste définitif.
    #[test]
    fn le_tirage_survit_a_la_correction() {
        let (mut events, coach_id) = avant_match_valide();
        events.extend(drawn_events(coach_id));
        events.push(post_match_event(coach_id));
        events.push(published_event(coach_id));
        events.push(unpublished_event(coach_id));
        let MatchReportState::ReadyToPublish(rtp) = rehydrate(events).unwrap() else {
            panic!("attendu ReadyToPublish");
        };
        let refus = rtp
            .into_pre_match()
            .delete_action(&ActionId("tire".into()), coach_id);
        assert_eq!(refus.unwrap_err(), DomainError::DrawnMvpIsFinal);
    }

    /// Un rapport publié sans aucun MVP, d'avant la règle, se rejoue tel quel :
    /// le MVP obligatoire vaut à la saisie, jamais au rejeu.
    #[test]
    fn un_ancien_rapport_sans_mvp_se_rejoue() {
        let (mut events, coach_id) = avant_match_valide();
        events.push(post_match_event(coach_id));
        events.push(published_event(coach_id));
        let MatchReportState::Published(published) = rehydrate(events).unwrap() else {
            panic!("attendu Published");
        };
        assert!(published.home_actions.is_empty() && published.away_actions.is_empty());
    }
}

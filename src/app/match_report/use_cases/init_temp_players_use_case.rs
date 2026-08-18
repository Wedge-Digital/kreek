use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{
    TeamSide, TempPlayer, TempPlayerId, TempPlayerKind,
};
use crate::app::match_report::ports::{IPlayerDataPort, ITeamDataPort};
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;

#[derive(Debug)]
pub struct InitTempPlayersCommand {
    pub match_report_id: MatchReportId,
    pub team_id: TeamId,
}

#[derive(Debug)]
pub enum InitTempPlayersError {
    NotFound,
    NotInPreMatchPhase,
    JourneymanPositionUnavailable(String),
    PlayerCountUnavailable(String),
    Repository(String),
}

pub async fn execute(
    cmd: InitTempPlayersCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    player_data: &dyn IPlayerDataPort,
) -> Result<(), InitTempPlayersError> {
    let mr_id = cmd.match_report_id.to_string();
    let pm = load_pm(repo, &mr_id).await?;

    let pm = reset_if_needed(pm, &cmd.team_id, repo, &mr_id).await?;

    let stars = collect_stars(&pm, &cmd.team_id);
    let mercs = collect_mercs(&pm, &cmd.team_id);
    let journeymen = collect_journeymen(&cmd.team_id, team_data, player_data).await?;

    let players = stars.into_iter().chain(mercs).chain(journeymen).collect();
    let (_, event) = pm.init_temp_players(&cmd.team_id, players);

    let version = pm.version;
    repo.append(&mr_id, &event, version)
        .await
        .map(|_| ())
        .map_err(|e| InitTempPlayersError::Repository(e.to_string()))
}

async fn load_pm(
    repo: &dyn IMatchReportRepository,
    mr_id: &str,
) -> Result<
    crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch,
    InitTempPlayersError,
> {
    let state = repo
        .find_by_id(mr_id)
        .await
        .map_err(|e| InitTempPlayersError::Repository(e.to_string()))?
        .ok_or(InitTempPlayersError::NotFound)?;
    match state {
        MatchReportState::PreMatch(pm) => Ok(pm),
        MatchReportState::ReadyToPublish(rtp) => Ok(rtp.into_pre_match()),
        _ => Err(InitTempPlayersError::NotInPreMatchPhase),
    }
}

async fn reset_if_needed(
    pm: crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch,
    team_id: &TeamId,
    repo: &dyn IMatchReportRepository,
    mr_id: &str,
) -> Result<
    crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch,
    InitTempPlayersError,
> {
    let side = if team_id == &pm.home_team_id {
        TeamSide::Home
    } else {
        TeamSide::Away
    };
    if pm.temp_players_for(side).is_empty() {
        return Ok(pm);
    }
    let (updated, reset_event) = pm.reset_temp_players(team_id);
    let version = updated.version - 1;
    repo.append(mr_id, &reset_event, version)
        .await
        .map_err(|e| InitTempPlayersError::Repository(e.to_string()))?;
    Ok(updated)
}

fn collect_stars(
    pm: &crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch,
    team_id: &TeamId,
) -> Vec<TempPlayer> {
    pm.star_player_uids_for(team_id)
        .into_iter()
        .map(|uid| TempPlayer {
            id: TempPlayerId(ulid::Ulid::new().to_string()),
            team_id: team_id.clone(),
            kind: TempPlayerKind::StarPlayer {
                ref_uid: uid.0.clone(),
                position_uid: String::new(),
            },
            display_name: Some(uid.0),
        })
        .collect()
}

fn collect_mercs(
    pm: &crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch,
    team_id: &TeamId,
) -> Vec<TempPlayer> {
    pm.purchases_for(team_id)
        .iter()
        .filter(|p| p.uid.0.starts_with("MERCO:"))
        .flat_map(|p| {
            let position_uid = p.uid.0.splitn(3, ':').nth(1).unwrap_or("").to_string();
            (0..p.qty.into_inner()).map(move |_| TempPlayer {
                id: TempPlayerId(ulid::Ulid::new().to_string()),
                team_id: team_id.clone(),
                kind: TempPlayerKind::Mercenary {
                    position_uid: position_uid.clone(),
                },
                display_name: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
    use crate::app::match_report::domain::value_objects::{
        DedicatedFans, InducementCost, InducementPurchase, InducementQty, MatchReportOrigin,
        TeamValue,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{
        CompetitionId, MatchReportId, RoundId, SeasonId,
    };
    use crate::app::shared_kernel::bloodbowl::inducement_definition::InducementId;
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};

    fn make_pm() -> MatchReportPreMatch {
        MatchReportPreMatch {
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
            home_team_value: Some(TeamValue::try_new(1000).unwrap()),
            away_team_value: Some(TeamValue::try_new(1000).unwrap()),
            home_inducements: None,
            away_inducements: None,
            star_engagements: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
            home_actions: vec![],
            away_actions: vec![],
            version: 1,
        }
    }

    #[test]
    fn collect_stars_creates_one_per_engagement() {
        let mut pm = make_pm();
        let uid = InducementId("MORG_N_THORG".into());
        pm.star_engagements
            .push((pm.home_team_id.clone(), uid.clone()));
        let stars = collect_stars(&pm, &pm.home_team_id.clone());
        assert_eq!(stars.len(), 1);
        assert!(
            matches!(&stars[0].kind, TempPlayerKind::StarPlayer { ref_uid, .. } if ref_uid == "MORG_N_THORG")
        );
        assert_eq!(stars[0].display_name, Some("MORG_N_THORG".into()));
    }

    #[test]
    fn collect_mercs_creates_qty_per_purchase() {
        let mut pm = make_pm();
        pm.home_inducements = Some(vec![InducementPurchase {
            uid: InducementId("MERCO:blitzeur:base".into()),
            qty: InducementQty::try_new(2).unwrap(),
            unit_cost: InducementCost::try_new(130).unwrap(),
        }]);
        let mercs = collect_mercs(&pm, &pm.home_team_id.clone());
        assert_eq!(mercs.len(), 2);
        assert!(
            matches!(&mercs[0].kind, TempPlayerKind::Mercenary { position_uid } if position_uid == "blitzeur")
        );
    }

    #[test]
    fn collect_mercs_extracts_position_uid_from_encoded_uid() {
        let mut pm = make_pm();
        pm.home_inducements = Some(vec![InducementPurchase {
            uid: InducementId("MERCO:witch-elf:lvl1".into()),
            qty: InducementQty::try_new(1).unwrap(),
            unit_cost: InducementCost::try_new(180).unwrap(),
        }]);
        let mercs = collect_mercs(&pm, &pm.home_team_id.clone());
        assert_eq!(mercs.len(), 1);
        assert!(
            matches!(&mercs[0].kind, TempPlayerKind::Mercenary { position_uid } if position_uid == "witch-elf")
        );
    }

    #[test]
    fn collect_mercs_ignores_non_mercenary_purchases() {
        let mut pm = make_pm();
        pm.home_inducements = Some(vec![InducementPurchase {
            uid: InducementId("BRIBE".into()),
            qty: InducementQty::try_new(1).unwrap(),
            unit_cost: InducementCost::try_new(100).unwrap(),
        }]);
        let mercs = collect_mercs(&pm, &pm.home_team_id.clone());
        assert!(mercs.is_empty());
    }
}

async fn collect_journeymen(
    team_id: &TeamId,
    team_data: &dyn ITeamDataPort,
    player_data: &dyn IPlayerDataPort,
) -> Result<Vec<TempPlayer>, InitTempPlayersError> {
    let count = player_data
        .count_available_players(&team_id.to_string())
        .await
        .map_err(|e| InitTempPlayersError::PlayerCountUnavailable(e))?;
    let n = 11usize.saturating_sub(count);
    if n == 0 {
        return Ok(vec![]);
    }
    let pos = team_data
        .find_journeyman_position(&team_id.to_string())
        .await
        .ok_or_else(|| InitTempPlayersError::JourneymanPositionUnavailable(team_id.to_string()))?;
    Ok((0..n)
        .map(|_| TempPlayer {
            id: TempPlayerId(ulid::Ulid::new().to_string()),
            team_id: team_id.clone(),
            kind: TempPlayerKind::Journeyman {
                position_uid: pos.position_uid.clone(),
            },
            display_name: None,
        })
        .collect())
}

use crate::app::competitions::domain::group_repository_port::{GroupWithTeams, IGroupRepository};
use crate::app::competitions::domain::match_day::generate_round_pairings;
use crate::app::competitions::domain::match_day::Pairing;
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::shared_kernel::app_events::competitions_app_events::CompetitionsAppEvent;
use crate::app::shared_kernel::common_types::EventId;
use crate::common::services::event_bus::event_bus::EventBus;
use std::collections::HashSet;

#[derive(Debug)]
pub enum GenerateError {
    MatchDayNotFound,
    IsRestDay,
    NoGroups,
    Repository(String),
}

pub async fn execute(
    match_day_id: &str,
    season_id: &str,
    space_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    group_repo: &dyn IGroupRepository,
    team_port: &dyn ITeamInfoPort,
    app_event_bus: &EventBus,
) -> Result<(), GenerateError> {
    let match_day = match_day_repo
        .find_by_id(match_day_id)
        .await
        .map_err(|e| GenerateError::Repository(e.to_string()))?
        .ok_or(GenerateError::MatchDayNotFound)?;

    if match_day.is_rest() {
        return Err(GenerateError::IsRestDay);
    }

    let groups = group_repo
        .find_groups(season_id)
        .await
        .map_err(|e| GenerateError::Repository(e.to_string()))?;

    let groups = if groups.is_empty() {
        let enrolled = team_port
            .find_enrolled_teams(season_id)
            .await
            .map_err(|e| GenerateError::Repository(e))?;
        if enrolled.is_empty() {
            return Err(GenerateError::NoGroups);
        }
        vec![GroupWithTeams {
            group_id: "default".to_string(),
            group_name: "Toutes les équipes".to_string(),
            position: 0,
            team_ids: enrolled.iter().map(|t| t.team_id.clone()).collect(),
        }]
    } else {
        groups
    };

    match_day_repo
        .clear_pairings(match_day_id)
        .await
        .map_err(|e| GenerateError::Repository(e.to_string()))?;

    let all_days = match_day_repo
        .find_by_season(season_id)
        .await
        .map_err(|e| GenerateError::Repository(e.to_string()))?;

    let mut already_played: HashSet<(String, String)> = HashSet::new();
    for day in &all_days {
        if day.id == match_day_id {
            continue;
        }
        for p in &day.pairings {
            let pair = if p.home_team_id < p.away_team_id {
                (p.home_team_id.clone(), p.away_team_id.clone())
            } else {
                (p.away_team_id.clone(), p.home_team_id.clone())
            };
            already_played.insert(pair);
        }
    }

    for group in &groups {
        let pairings = generate_round_pairings(&group.team_ids, &already_played);

        for (home, away) in pairings {
            let pairing = Pairing {
                id: ulid::Ulid::new().to_string(),
                home_team_id: home.clone(),
                away_team_id: away.clone(),
            };
            match_day_repo
                .save_pairing(match_day_id, &pairing)
                .await
                .map_err(|e| GenerateError::Repository(e.to_string()))?;

            let _ = app_event_bus.send(
                CompetitionsAppEvent::PairingCreated {
                    event_id: EventId::new(),
                    pairing_id: pairing.id.clone(),
                    season_id: season_id.to_string(),
                    round_id: match_day_id.to_string(),
                    home_team_id: home.clone(),
                    away_team_id: away.clone(),
                    space_id: space_id.to_string(),
                }
                .to_enveloppe(),
            );

            let norm = if home < away {
                (home, away)
            } else {
                (away, home)
            };
            already_played.insert(norm);
        }
    }

    Ok(())
}

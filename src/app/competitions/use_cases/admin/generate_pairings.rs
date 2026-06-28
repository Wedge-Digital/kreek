use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::group_repository_port::{GroupWithTeams, IGroupRepository};
use crate::app::competitions::domain::match_day::generate_round_pairings;
use crate::app::competitions::domain::match_day::Pairing;
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::shared_kernel::common_types::{EventId, PairingId};
use crate::app::shared_kernel::team::TeamId;
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
    competition_id: &str,
    space_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    group_repo: &dyn IGroupRepository,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
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

    let existing_pairings: Vec<String> = match_day
        .pairings
        .iter()
        .map(|p| p.id.to_string())
        .collect();

    match_day_repo
        .clear_pairings(match_day_id)
        .await
        .map_err(|e| GenerateError::Repository(e.to_string()))?;

    for pid in &existing_pairings {
        let _ = event_bus.send(
            CompetitionsDomainEvent::PairingDeleted {
                event_id: EventId::new(),
                pairing_id: pid.clone(),
            }
            .to_enveloppe(),
        );
    }

    let all_days = match_day_repo
        .find_by_season(season_id)
        .await
        .map_err(|e| GenerateError::Repository(e.to_string()))?;

    let mut already_played: HashSet<(String, String)> = HashSet::new();
    for day in &all_days {
        if day.id.to_string() == match_day_id {
            continue;
        }
        for p in &day.pairings {
            let home = p.home_team_id.to_string();
            let away = p.away_team_id.to_string();
            let pair = if home < away { (home, away) } else { (away, home) };
            already_played.insert(pair);
        }
    }

    for group in &groups {
        let pairings = generate_round_pairings(&group.team_ids, &already_played);

        for (home, away) in pairings {
            let pairing = Pairing {
                id: PairingId::new(),
                home_team_id: TeamId::try_new(&home).expect("valid team id"),
                away_team_id: TeamId::try_new(&away).expect("valid team id"),
            };
            match_day_repo
                .save_pairing(match_day_id, &pairing)
                .await
                .map_err(|e| GenerateError::Repository(e.to_string()))?;

            let _ = event_bus.send(
                CompetitionsDomainEvent::PairingCreated {
                    event_id: EventId::new(),
                    pairing_id: pairing.id.to_string(),
                    competition_id: competition_id.to_string(),
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

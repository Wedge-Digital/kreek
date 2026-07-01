use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::group_repository_port::{GroupWithTeams, IGroupRepository};
use crate::app::competitions::domain::match_day::generate_round_pairings;
use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::ports::{ITeamInfoPort, TeamInfoDto};
use crate::app::shared_kernel::common_types::{EventId, PairingId};
use crate::app::shared_kernel::team::TeamId;
use crate::common::services::event_bus::event_bus::EventBus;
use std::collections::{HashMap, HashSet};

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

    let team_display = load_display_map(season_id, team_port).await?;

    let groups = group_repo
        .find_groups(season_id)
        .await
        .map_err(|e| GenerateError::Repository(e.to_string()))?;

    let groups = if groups.is_empty() {
        if team_display.is_empty() {
            return Err(GenerateError::NoGroups);
        }
        vec![GroupWithTeams {
            group_id: "default".to_string(),
            group_name: "Toutes les équipes".to_string(),
            position: 0,
            team_ids: team_display.keys().cloned().collect(),
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

    let mut already_played = build_played_set(&all_days, match_day_id);

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

            emit_pairing_created(
                &home,
                &away,
                &pairing,
                competition_id,
                season_id,
                space_id,
                &match_day,
                &team_display,
                event_bus,
            );

            let norm = if home < away { (home, away) } else { (away, home) };
            already_played.insert(norm);
        }
    }

    Ok(())
}

async fn load_display_map(
    season_id: &str,
    team_port: &dyn ITeamInfoPort,
) -> Result<HashMap<String, TeamInfoDto>, GenerateError> {
    let enrolled = team_port
        .find_enrolled_teams(season_id)
        .await
        .map_err(GenerateError::Repository)?;
    Ok(enrolled.into_iter().map(|t| (t.team_id.clone(), t)).collect())
}

fn build_played_set(days: &[MatchDay], exclude_id: &str) -> HashSet<(String, String)> {
    let mut played = HashSet::new();
    for day in days {
        if day.id.to_string() == exclude_id {
            continue;
        }
        for p in &day.pairings {
            let home = p.home_team_id.to_string();
            let away = p.away_team_id.to_string();
            let pair = if home < away { (home, away) } else { (away, home) };
            played.insert(pair);
        }
    }
    played
}

fn emit_pairing_created(
    home: &str,
    away: &str,
    pairing: &Pairing,
    competition_id: &str,
    season_id: &str,
    space_id: &str,
    match_day: &MatchDay,
    team_display: &HashMap<String, TeamInfoDto>,
    event_bus: &EventBus,
) {
    let empty = TeamInfoDto {
        team_id: String::new(),
        team_name: String::new(),
        coach_name: String::new(),
        roster_name: String::new(),
        logo_url: None,
    };
    let home_info = team_display.get(home).unwrap_or(&empty);
    let away_info = team_display.get(away).unwrap_or(&empty);

    let _ = event_bus.send(
        CompetitionsDomainEvent::PairingCreated {
            event_id: EventId::new(),
            pairing_id: pairing.id.to_string(),
            competition_id: competition_id.to_string(),
            season_id: season_id.to_string(),
            round_id: match_day.id.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            space_id: space_id.to_string(),
            home_team_name: home_info.team_name.clone(),
            home_roster_name: home_info.roster_name.clone(),
            home_coach_name: home_info.coach_name.clone(),
            home_logo_url: home_info.logo_url.clone(),
            away_team_name: away_info.team_name.clone(),
            away_roster_name: away_info.roster_name.clone(),
            away_coach_name: away_info.coach_name.clone(),
            away_logo_url: away_info.logo_url.clone(),
            round_name: match_day.name.to_string(),
            round_position: match_day.position.into_inner(),
            round_date_start: match_day.date_start.as_ref().map(|d| d.to_string()),
            round_date_end: match_day.date_end.as_ref().map(|d| d.to_string()),
            round_day_type: match_day.day_type.as_str().to_string(),
        }
        .to_enveloppe(),
    );
}

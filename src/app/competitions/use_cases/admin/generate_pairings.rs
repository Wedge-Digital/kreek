use crate::app::competitions::domain::group_repository_port::IGroupRepository;
use crate::app::competitions::domain::match_day::generate_round_pairings;
use crate::app::competitions::domain::match_day::Pairing;
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
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
    match_day_repo: &dyn IMatchDayRepository,
    group_repo: &dyn IGroupRepository,
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

    tracing::info!(
        "generate_pairings: {} groups found for season {season_id}",
        groups.len()
    );
    for g in &groups {
        tracing::info!(
            "  group '{}': {} teams {:?}",
            g.group_name, g.team_ids.len(), g.team_ids
        );
    }

    if groups.is_empty() {
        return Err(GenerateError::NoGroups);
    }

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

    tracing::info!("generate_pairings: {} already played pairs", already_played.len());

    for group in &groups {
        let pairings = generate_round_pairings(&group.team_ids, &already_played);
        tracing::info!(
            "generate_pairings: group '{}' -> {} pairings generated",
            group.group_name, pairings.len()
        );

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

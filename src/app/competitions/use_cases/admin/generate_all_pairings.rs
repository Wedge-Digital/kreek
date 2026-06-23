use crate::app::competitions::domain::group_repository_port::IGroupRepository;
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::competitions::use_cases::admin::generate_pairings;

#[derive(Debug)]
pub enum GenerateAllError {
    Repository(String),
    Generate(generate_pairings::GenerateError),
}

pub async fn execute(
    season_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    group_repo: &dyn IGroupRepository,
    team_port: &dyn ITeamInfoPort,
) -> Result<(), GenerateAllError> {
    let days = match_day_repo
        .find_by_season(season_id)
        .await
        .map_err(|e| GenerateAllError::Repository(e.to_string()))?;

    for day in &days {
        if day.is_rest() {
            continue;
        }
        generate_pairings::execute(&day.id, season_id, match_day_repo, group_repo, team_port)
            .await
            .map_err(GenerateAllError::Generate)?;
    }

    Ok(())
}

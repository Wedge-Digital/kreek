use crate::app::competitions::domain::group_repository_port::{GroupRepositoryError, IGroupRepository};
use crate::app::competitions::ports::ITeamInfoPort;
use rand::seq::SliceRandom;
use rand::SeedableRng;

#[derive(Debug)]
pub enum DrawError {
    NoGroups,
    NoTeams,
    Repository(String),
    Port(String),
}

pub async fn execute(
    season_id: &str,
    group_repo: &dyn IGroupRepository,
    team_port: &dyn ITeamInfoPort,
) -> Result<(), DrawError> {
    let groups = group_repo
        .find_groups(season_id)
        .await
        .map_err(|e| DrawError::Repository(e.to_string()))?;

    if groups.is_empty() {
        return Err(DrawError::NoGroups);
    }

    let enrolled = team_port
        .find_enrolled_teams(season_id)
        .await
        .map_err(DrawError::Port)?;

    if enrolled.is_empty() {
        return Err(DrawError::NoTeams);
    }

    group_repo
        .reset_assignments(season_id)
        .await
        .map_err(|e| DrawError::Repository(e.to_string()))?;

    let mut team_ids: Vec<&str> = enrolled.iter().map(|t| t.team_id.as_str()).collect();
    let mut rng = rand::rngs::StdRng::from_os_rng();
    team_ids.shuffle(&mut rng);

    let group_ids: Vec<&str> = groups.iter().map(|g| g.group_id.as_str()).collect();

    let assignments: Vec<(String, String)> = team_ids
        .iter()
        .enumerate()
        .map(|(i, team_id)| {
            let group_id = group_ids[i % group_ids.len()];
            (group_id.to_string(), team_id.to_string())
        })
        .collect();

    group_repo
        .save_assignments(&assignments)
        .await
        .map_err(|e| DrawError::Repository(e.to_string()))?;

    Ok(())
}

pub fn distribute(team_count: usize, group_count: usize) -> Vec<Vec<usize>> {
    if group_count == 0 {
        return vec![];
    }
    let mut groups: Vec<Vec<usize>> = (0..group_count).map(|_| Vec::new()).collect();
    for i in 0..team_count {
        groups[i % group_count].push(i);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distribute_6_teams_2_groups() {
        let result = distribute(6, 2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 3);
        assert_eq!(result[1].len(), 3);
    }

    #[test]
    fn distribute_7_teams_2_groups() {
        let result = distribute(7, 2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 4);
        assert_eq!(result[1].len(), 3);
    }

    #[test]
    fn distribute_0_teams() {
        let result = distribute(0, 2);
        assert_eq!(result.len(), 2);
        assert!(result[0].is_empty());
        assert!(result[1].is_empty());
    }

    #[test]
    fn distribute_0_groups() {
        let result = distribute(6, 0);
        assert!(result.is_empty());
    }
}

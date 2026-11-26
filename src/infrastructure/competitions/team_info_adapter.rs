use crate::app::competitions::ports::{
    CoachTeamDto, ITeamInfoPort, TeamEnrollmentDto, TeamInfoDto,
};
use crate::app::teams::ports::{ITeamRepository, MyTeamRow};
use async_trait::async_trait;
use std::sync::Arc;

pub struct TeamInfoAdapter {
    team_repo: Arc<dyn ITeamRepository>,
}

impl TeamInfoAdapter {
    pub fn new(team_repo: Arc<dyn ITeamRepository>) -> Self {
        Self { team_repo }
    }
}

#[async_trait]
impl ITeamInfoPort for TeamInfoAdapter {
    async fn find_enrolled_teams(&self, season_id: &str) -> Result<Vec<TeamInfoDto>, String> {
        let rows = self
            .team_repo
            .find_enrolled_for_season(season_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .map(|r| TeamInfoDto {
                team_id: r.team_id,
                team_name: r.team_name,
                coach_id: r.coach_id,
                coach_name: r.coach_name,
                roster_name: r.roster_name,
                logo_url: r.logo_url,
            })
            .collect())
    }

    /// **Aucune requête neuve** : l'agrégat `Team` porte déjà sa compétition et
    /// sa saison, remplies par `TeamEnrolled`. Les deux valent `None` sur un
    /// brouillon non soumis, et l'appelant y lit une liste vide.
    async fn find_team_enrollment(
        &self,
        team_id: &str,
    ) -> Result<Option<TeamEnrollmentDto>, String> {
        let Some(team) = self
            .team_repo
            .find_by_id(team_id)
            .await
            .map_err(|e| e.to_string())?
        else {
            return Ok(None);
        };
        Ok(match (team.competition_id, team.season_id) {
            (Some(c), Some(s)) => Some(TeamEnrollmentDto {
                competition_id: c.to_string(),
                season_id: s.to_string(),
            }),
            _ => None,
        })
    }

    async fn find_team_names(&self, team_ids: &[String]) -> Result<Vec<TeamInfoDto>, String> {
        let mut out = Vec::with_capacity(team_ids.len());
        for id in team_ids {
            if let Ok(Some(team)) = self.team_repo.find_by_id(id).await {
                out.push(TeamInfoDto {
                    team_id: id.clone(),
                    team_name: team.name.to_string(),
                    coach_id: team.coach_id.to_string(),
                    coach_name: team.coach_name.clone(),
                    roster_name: team.roster_name.to_string(),
                    logo_url: team.logo_url.clone(),
                });
            }
        }
        Ok(out)
    }

    /// **Aucune requête neuve** : `find_by_coach_and_space` rend déjà les
    /// équipes du coach dans l'espace, tous statuts confondus ; on n'en garde
    /// que les inscrites.
    async fn find_coach_enrolled_teams(
        &self,
        coach_id: &str,
        space_id: &str,
    ) -> Result<Vec<CoachTeamDto>, String> {
        let rows = self
            .team_repo
            .find_by_coach_and_space(coach_id, space_id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(enrolled_only(rows))
    }
}

/// Le statut que `team_proj` porte pour une équipe inscrite — celui
/// qu'écrit `TeamEnrolled`, et que filtre déjà `find_enrolled_for_season`.
const ENROLLED: &str = "Enrolled";

/// Les équipes inscrites seules : ni brouillon, ni renvoyée, ni refusée.
fn enrolled_only(rows: Vec<MyTeamRow>) -> Vec<CoachTeamDto> {
    rows.into_iter()
        .filter(|r| r.status == ENROLLED)
        .map(|r| CoachTeamDto {
            team_id: r.team_id,
            team_name: r.team_name,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::teams::domain::team::{Team, TeamDomainEvent};
    use crate::app::teams::ports::{
        RepositoryError, TeamCardRow, TeamEnrollmentRow, TreasuryMovementRow,
    };

    fn my_team(id: &str, status: &str) -> MyTeamRow {
        MyTeamRow {
            team_id: id.into(),
            team_name: format!("Équipe {id}"),
            roster_name: "R".into(),
            logo_url: None,
            status: status.into(),
            game_phase: None,
        }
    }

    fn statuses() -> Vec<MyTeamRow> {
        vec![
            my_team("inscrite", "Enrolled"),
            my_team("brouillon", "PendingEnrollment"),
            my_team("renvoyee", "Dismissed"),
            my_team("refusee", "Rejected"),
        ]
    }

    /// Les équipes du coach, ou une panne. Seule la lecture par coach sert.
    struct FakeTeams(Option<Vec<MyTeamRow>>);

    #[async_trait]
    impl ITeamRepository for FakeTeams {
        async fn find_by_coach_and_space(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Vec<MyTeamRow>, RepositoryError> {
            match &self.0 {
                Some(rows) => Ok(rows
                    .iter()
                    .map(|r| my_team(&r.team_id, &r.status))
                    .collect()),
                None => Err(RepositoryError::ConcurrentWrite),
            }
        }
        async fn append(
            &self,
            _: &str,
            _: &TeamDomainEvent,
            _: u64,
        ) -> Result<u64, RepositoryError> {
            unimplemented!("hors de l'adapter")
        }
        async fn append_batch(
            &self,
            _: &str,
            _: &[TeamDomainEvent],
            _: u64,
        ) -> Result<u64, RepositoryError> {
            unimplemented!("hors de l'adapter")
        }
        async fn find_space_id(&self, _: &str) -> Result<Option<String>, RepositoryError> {
            unimplemented!("hors de l'adapter")
        }
        async fn find_coach_ids_in_space(
            &self,
            _: &[&str],
            _: &str,
        ) -> Result<Vec<String>, RepositoryError> {
            unimplemented!("hors de l'adapter")
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<Team>, RepositoryError> {
            unimplemented!("hors de l'adapter")
        }
        async fn find_by_season_and_status(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Vec<TeamEnrollmentRow>, RepositoryError> {
            unimplemented!("hors de l'adapter")
        }
        async fn find_enrolled_for_season(
            &self,
            _: &str,
        ) -> Result<Vec<TeamCardRow>, RepositoryError> {
            unimplemented!("hors de l'adapter")
        }
        async fn list_treasury_movements(
            &self,
            _: &str,
        ) -> Result<Vec<TreasuryMovementRow>, RepositoryError> {
            unimplemented!("hors de l'adapter")
        }
    }

    #[test]
    fn enrolled_only_keeps_the_enrolled_teams() {
        let kept = enrolled_only(statuses());
        assert_eq!(
            kept,
            vec![CoachTeamDto {
                team_id: "inscrite".into(),
                team_name: "Équipe inscrite".into(),
            }]
        );
    }

    #[tokio::test]
    async fn find_coach_enrolled_teams_filters_on_enrollment() {
        let adapter = TeamInfoAdapter::new(Arc::new(FakeTeams(Some(statuses()))));
        let teams = adapter.find_coach_enrolled_teams("coach", "space").await;
        let ids: Vec<String> = teams.unwrap().into_iter().map(|t| t.team_id).collect();
        assert_eq!(ids, vec!["inscrite".to_string()]);
    }

    #[tokio::test]
    async fn find_coach_enrolled_teams_reports_a_repository_failure() {
        let adapter = TeamInfoAdapter::new(Arc::new(FakeTeams(None)));
        assert!(adapter
            .find_coach_enrolled_teams("coach", "space")
            .await
            .is_err());
    }
}

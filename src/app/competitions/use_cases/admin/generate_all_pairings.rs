use crate::app::competitions::domain::group_repository_port::IGroupRepository;
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::competitions::use_cases::admin::generate_pairings;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub enum GenerateAllError {
    Repository(String),
    Generate(generate_pairings::GenerateError),
}

/// `skipped_round_names` : journées déjà pourvues d'appariements (générés
/// précédemment ou ajoutés manuellement) — jamais régénérées automatiquement,
/// pour ne pas écraser des matchs déjà saisis (il faut les vider explicitement
/// via "Vider les matchs" avant de pouvoir régénérer cette journée).
/// `skipped_group_names` : poules sans effectif suffisant (< 2 équipes
/// enrôlées assignées) — dédupliquées car réévaluées à chaque journée.
#[derive(Debug, Default)]
pub struct GenerateAllOutcome {
    pub skipped_team_names: Vec<String>,
    pub skipped_round_names: Vec<String>,
    pub skipped_group_names: Vec<String>,
}

pub async fn execute(
    season_id: &str,
    competition_id: &str,
    space_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    group_repo: &dyn IGroupRepository,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
) -> Result<GenerateAllOutcome, GenerateAllError> {
    let days = match_day_repo
        .find_by_season(season_id)
        .await
        .map_err(|e| GenerateAllError::Repository(e.to_string()))?;

    let mut skipped_team_names: Vec<String> = Vec::new();
    let mut skipped_round_names: Vec<String> = Vec::new();
    let mut skipped_group_names: Vec<String> = Vec::new();
    for day in &days {
        if day.is_rest() {
            continue;
        }
        let result = generate_pairings::execute(&day.id.to_string(), season_id, competition_id, space_id, match_day_repo, group_repo, team_port, event_bus).await;
        match result {
            Ok(outcome) => {
                skipped_team_names.extend(outcome.skipped_team_names);
                skipped_group_names.extend(outcome.skipped_group_names);
            }
            Err(generate_pairings::GenerateError::PairingsAlreadyExist) => {
                skipped_round_names.push(day.name.to_string());
            }
            Err(e) => return Err(GenerateAllError::Generate(e)),
        }
    }

    skipped_team_names.sort();
    skipped_team_names.dedup();
    skipped_group_names.sort();
    skipped_group_names.dedup();
    Ok(GenerateAllOutcome { skipped_team_names, skipped_round_names, skipped_group_names })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::group_repository_port::{GroupRepositoryError, GroupWithTeams};
    use crate::app::competitions::domain::match_day::{MatchDay, MatchDayName, MatchDayPosition, MatchDayType, Pairing};
    use crate::app::competitions::domain::match_day_repository_port::{MatchDayRepositoryError, PairingDisplayDto};
    use crate::app::competitions::ports::TeamInfoDto;
    use crate::app::shared_kernel::common_types::{MatchId, PairingId, SeasonId};
    use crate::app::shared_kernel::team::TeamId;
    use async_trait::async_trait;
    use std::collections::HashMap;

    struct FakeMatchDayRepo(HashMap<String, MatchDay>);
    #[async_trait]
    impl IMatchDayRepository for FakeMatchDayRepo {
        async fn find_by_season(&self, _: &str) -> Result<Vec<MatchDay>, MatchDayRepositoryError> {
            let mut days: Vec<MatchDay> = self.0.values().cloned().collect();
            days.sort_by_key(|d| d.position.into_inner());
            Ok(days)
        }
        async fn find_by_id(&self, id: &str) -> Result<Option<MatchDay>, MatchDayRepositoryError> { Ok(self.0.get(id).cloned()) }
        async fn save_match_day(&self, _: &MatchDay) -> Result<(), MatchDayRepositoryError> { Ok(()) }
        async fn delete_match_day(&self, _: &str) -> Result<(), MatchDayRepositoryError> { Ok(()) }
        async fn save_pairing(&self, _: &str, _: &Pairing, _: &crate::app::competitions::domain::match_day_repository_port::NewPairingProjection) -> Result<(), MatchDayRepositoryError> { Ok(()) }
        async fn find_pairing_id(&self, _: &str, _: &str, _: &str) -> Result<Option<String>, MatchDayRepositoryError> { Ok(None) }
        async fn delete_pairing(&self, _: &str) -> Result<(), MatchDayRepositoryError> { Ok(()) }
        async fn ensure_match_days_from_structure(&self, _: &str, _: &[(String, String, String, Option<String>, Option<String>)]) -> Result<(), MatchDayRepositoryError> { Ok(()) }
        async fn list_resultats(&self, _: &str, _: Option<i32>, _: u32) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> { Ok(vec![]) }
        async fn list_calendrier(&self, _: &str, _: Option<i32>, _: u32) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> { Ok(vec![]) }
    }

    struct FakeGroupRepo;
    #[async_trait]
    impl IGroupRepository for FakeGroupRepo {
        async fn find_groups(&self, _: &str) -> Result<Vec<GroupWithTeams>, GroupRepositoryError> { Ok(vec![]) }
        async fn save_assignments(&self, _: &[(String, String)]) -> Result<(), GroupRepositoryError> { Ok(()) }
        async fn reset_assignments(&self, _: &str) -> Result<(), GroupRepositoryError> { Ok(()) }
        async fn assign_team(&self, _: &str, _: &str) -> Result<(), GroupRepositoryError> { Ok(()) }
        async fn unassign_team(&self, _: &str) -> Result<(), GroupRepositoryError> { Ok(()) }
        async fn ensure_groups_from_structure(&self, _: &str, _: &[(String, String)]) -> Result<(), GroupRepositoryError> { Ok(()) }
    }

    struct FakeTeamInfoPort(Vec<TeamInfoDto>);
    #[async_trait]
    impl ITeamInfoPort for FakeTeamInfoPort {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> { Ok(self.0.clone()) }
        async fn find_team_names(&self, _: &[String]) -> Result<Vec<TeamInfoDto>, String> { Ok(vec![]) }
    }

    fn dto(id: &str) -> TeamInfoDto {
        TeamInfoDto { team_id: id.into(), team_name: format!("Team {id}"), coach_id: String::new(), coach_name: String::new(), roster_name: String::new(), logo_url: None }
    }

    fn day(position: i32, pairings: Vec<Pairing>) -> MatchDay {
        MatchDay {
            id: MatchId::new(), season_id: SeasonId::new(),
            name: MatchDayName::try_new(format!("Journée {position}")).unwrap(),
            day_type: MatchDayType::FixedDate, date_start: None, date_end: None,
            position: MatchDayPosition::try_new(position).unwrap(), pairings,
        }
    }

    #[tokio::test]
    async fn skips_days_with_existing_pairings_but_keeps_generating_others() {
        let home = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let away = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
        let existing = Pairing { id: PairingId::new(), home_team_id: TeamId::try_new(home).unwrap(), away_team_id: TeamId::try_new(away).unwrap() };

        let empty_day = day(0, vec![]);
        let filled_day = day(1, vec![existing]);
        let mut days = HashMap::new();
        days.insert(empty_day.id.to_string(), empty_day);
        days.insert(filled_day.id.to_string(), filled_day);
        let match_day_repo = FakeMatchDayRepo(days);
        let group_repo = FakeGroupRepo;
        let team_port = FakeTeamInfoPort(vec![dto(home), dto(away)]);
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();

        let outcome = execute("s1", "c1", "sp1", &match_day_repo, &group_repo, &team_port, &event_bus)
            .await
            .expect("ne doit pas échouer globalement à cause d'une journée déjà pourvue");

        assert_eq!(outcome.skipped_round_names, vec!["Journée 1".to_string()]);
    }
}

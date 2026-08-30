use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::group_repository_port::{GroupWithTeams, IGroupRepository};
use crate::app::competitions::domain::match_day::generate_round_pairings;
use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::ports::{ITeamInfoPort, TeamInfoDto};
use crate::app::competitions::use_cases::admin::team_enrollment::{
    build_new_pairing_projection, filter_enrolled_team_ids, load_enrolled_teams, resolve_team_names,
};
use crate::app::shared_kernel::bloodbowl::ids::PairingId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::EventId;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub enum GenerateError {
    MatchDayNotFound,
    IsRestDay,
    PairingsAlreadyExist,
    NoGroups,
    Repository(String),
}

/// `skipped_team_names` : équipes présentes dans un groupe/poule mais non
/// `Enrolled` pour la saison — jamais appariées (BR : pas de pairing pour une
/// équipe non enrôlée, donc pas d'event `PairingCreated` la concernant).
/// `skipped_group_names` : poules avec moins de 2 équipes enrôlées assignées
/// — aucun appariement possible, sans quoi la génération réussirait
/// silencieusement à 0 rencontre pour cette poule (BR : signaler explicitement
/// plutôt que de laisser l'admin croire que la génération a échoué).
#[derive(Debug, Default)]
pub struct GenerateOutcome {
    pub skipped_team_names: Vec<String>,
    pub skipped_group_names: Vec<String>,
}

#[tracing::instrument(skip_all, fields(match_day_id = ?match_day_id))]
pub async fn execute(
    match_day_id: &str,
    season_id: &str,
    competition_id: &str,
    space_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    group_repo: &dyn IGroupRepository,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
) -> Result<GenerateOutcome, GenerateError> {
    let match_day = match_day_repo
        .find_by_id(match_day_id)
        .await
        .map_err(|e| GenerateError::Repository(e.to_string()))?
        .ok_or(GenerateError::MatchDayNotFound)?;

    if match_day.is_rest() {
        return Err(GenerateError::IsRestDay);
    }
    if !match_day.pairings.is_empty() {
        return Err(GenerateError::PairingsAlreadyExist);
    }

    let team_display = load_enrolled_teams(season_id, team_port)
        .await
        .map_err(GenerateError::Repository)?;
    let groups = load_groups(season_id, group_repo, &team_display).await?;

    let all_days = match_day_repo
        .find_by_season(season_id)
        .await
        .map_err(|e| GenerateError::Repository(e.to_string()))?;
    let mut already_played = build_played_set(&all_days, match_day_id);

    let mut skipped_team_ids: Vec<String> = Vec::new();
    let mut skipped_group_names: Vec<String> = Vec::new();
    for group in &groups {
        let (filtered_ids, skipped) = filter_enrolled_team_ids(&group.team_ids, &team_display);
        skipped_team_ids.extend(skipped);
        if filtered_ids.len() < 2 {
            skipped_group_names.push(group.group_name.clone());
            continue;
        }
        generate_and_save_group_pairings(
            &filtered_ids,
            &mut already_played,
            match_day_id,
            competition_id,
            season_id,
            space_id,
            &match_day,
            &team_display,
            match_day_repo,
            event_bus,
        )
        .await?;
    }

    let skipped_team_names = resolve_team_names(skipped_team_ids, team_port).await;
    Ok(GenerateOutcome {
        skipped_team_names,
        skipped_group_names,
    })
}

async fn load_groups(
    season_id: &str,
    group_repo: &dyn IGroupRepository,
    team_display: &HashMap<String, TeamInfoDto>,
) -> Result<Vec<GroupWithTeams>, GenerateError> {
    let groups = group_repo
        .find_groups(season_id)
        .await
        .map_err(|e| GenerateError::Repository(e.to_string()))?;

    if !groups.is_empty() {
        return Ok(groups);
    }
    if team_display.is_empty() {
        return Err(GenerateError::NoGroups);
    }
    Ok(vec![GroupWithTeams {
        group_id: "default".to_string(),
        group_name: "Toutes les équipes".to_string(),
        position: 0,
        team_ids: team_display.keys().cloned().collect(),
    }])
}

#[allow(clippy::too_many_arguments)]
async fn generate_and_save_group_pairings(
    team_ids: &[String],
    already_played: &mut HashSet<(String, String)>,
    match_day_id: &str,
    competition_id: &str,
    season_id: &str,
    space_id: &str,
    match_day: &MatchDay,
    team_display: &HashMap<String, TeamInfoDto>,
    match_day_repo: &dyn IMatchDayRepository,
    event_bus: &EventBus,
) -> Result<(), GenerateError> {
    let pairings = generate_round_pairings(team_ids, already_played);

    for (home, away) in pairings {
        let pairing = Pairing {
            id: PairingId::new(),
            home_team_id: TeamId::try_new(&home).expect("valid team id"),
            away_team_id: TeamId::try_new(&away).expect("valid team id"),
        };
        let projection =
            build_new_pairing_projection(&home, &away, season_id, match_day, team_display);
        match_day_repo
            .save_pairing(match_day_id, &pairing, &projection)
            .await
            .map_err(|e| GenerateError::Repository(e.to_string()))?;

        emit_pairing_created(
            &home,
            &away,
            &pairing,
            competition_id,
            season_id,
            space_id,
            match_day,
            team_display,
            event_bus,
        );

        let norm = if home < away {
            (home, away)
        } else {
            (away, home)
        };
        already_played.insert(norm);
    }
    Ok(())
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
            let pair = if home < away {
                (home, away)
            } else {
                (away, home)
            };
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
    // Invariant garanti par le filtrage fait avant l'appel à generate_round_pairings :
    // home/away ne peuvent être ici que des ids déjà présents dans team_display.
    let home_info = team_display
        .get(home)
        .expect("home team filtré comme enrôlé avant appariement");
    let away_info = team_display
        .get(away)
        .expect("away team filtré comme enrôlé avant appariement");

    emettre(
        event_bus,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::group_repository_port::GroupRepositoryError;
    use crate::app::competitions::domain::match_day::{
        MatchDayName, MatchDayPosition, MatchDayType,
    };
    use crate::app::competitions::domain::match_day_repository_port::{
        MatchDayRepositoryError, PairingDisplayDto,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
    use async_trait::async_trait;

    struct FakeMatchDayRepo(MatchDay);
    #[async_trait]
    impl IMatchDayRepository for FakeMatchDayRepo {
        async fn find_by_season(&self, _: &str) -> Result<Vec<MatchDay>, MatchDayRepositoryError> {
            Ok(vec![self.0.clone()])
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<MatchDay>, MatchDayRepositoryError> {
            Ok(Some(self.0.clone()))
        }
        async fn save_match_day(&self, _: &MatchDay) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn delete_match_day(&self, _: &str) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn save_pairing(
            &self,
            _: &str,
            _: &Pairing,
            _: &crate::app::competitions::domain::match_day_repository_port::NewPairingProjection,
        ) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn find_pairing_id(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<Option<String>, MatchDayRepositoryError> {
            Ok(None)
        }
        async fn delete_pairing(&self, _: &str) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn ensure_match_days_from_structure(
            &self,
            _: &str,
            _: &[(String, String, String, Option<String>, Option<String>)],
        ) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn list_resultats(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_calendrier(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_team_matches(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_latest_completed_results(
            &self,
            _: &str,
            _: i64,
        ) -> Result<
            Vec<crate::app::competitions::domain::match_day_repository_port::LatestResultDto>,
            MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
    }

    struct FakeGroupRepo;
    #[async_trait]
    impl IGroupRepository for FakeGroupRepo {
        async fn find_groups(&self, _: &str) -> Result<Vec<GroupWithTeams>, GroupRepositoryError> {
            Ok(vec![])
        }
        async fn save_assignments(
            &self,
            _: &[(String, String)],
        ) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn reset_assignments(&self, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn assign_team(&self, _: &str, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn unassign_team(&self, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn ensure_groups_from_structure(
            &self,
            _: &str,
            _: &[(String, String)],
        ) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
    }

    struct FakeTeamInfoPort;
    #[async_trait]
    impl ITeamInfoPort for FakeTeamInfoPort {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_names(&self, _: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_enrollment(
            &self,
            _: &str,
        ) -> Result<Option<crate::app::competitions::ports::TeamEnrollmentDto>, String> {
            Ok(None)
        }
    }

    struct FakeGroupRepoWithEmptyGroup(&'static str);
    #[async_trait]
    impl IGroupRepository for FakeGroupRepoWithEmptyGroup {
        async fn find_groups(&self, _: &str) -> Result<Vec<GroupWithTeams>, GroupRepositoryError> {
            Ok(vec![GroupWithTeams {
                group_id: "g1".to_string(),
                group_name: self.0.to_string(),
                position: 0,
                team_ids: vec![],
            }])
        }
        async fn save_assignments(
            &self,
            _: &[(String, String)],
        ) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn reset_assignments(&self, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn assign_team(&self, _: &str, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn unassign_team(&self, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn ensure_groups_from_structure(
            &self,
            _: &str,
            _: &[(String, String)],
        ) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
    }

    struct FakeTeamInfoPortWithEnrolled(Vec<TeamInfoDto>);
    #[async_trait]
    impl ITeamInfoPort for FakeTeamInfoPortWithEnrolled {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(self.0.clone())
        }
        async fn find_team_names(&self, _: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_enrollment(
            &self,
            _: &str,
        ) -> Result<Option<crate::app::competitions::ports::TeamEnrollmentDto>, String> {
            Ok(None)
        }
    }

    fn match_day_with_pairings(pairings: Vec<Pairing>) -> MatchDay {
        MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 1".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(0).unwrap(),
            pairings,
        }
    }

    #[tokio::test]
    async fn refuses_when_match_day_already_has_pairings() {
        let existing = Pairing {
            id: PairingId::new(),
            home_team_id: TeamId::try_new("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap(),
            away_team_id: TeamId::try_new("01ARZ3NDEKTSV4RRFFQ69G5FAW").unwrap(),
        };
        let match_day_repo = FakeMatchDayRepo(match_day_with_pairings(vec![existing]));
        let group_repo = FakeGroupRepo;
        let team_port = FakeTeamInfoPort;
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();

        let result = execute(
            "d1",
            "s1",
            "c1",
            "sp1",
            &match_day_repo,
            &group_repo,
            &team_port,
            &event_bus,
        )
        .await;

        assert!(matches!(result, Err(GenerateError::PairingsAlreadyExist)));
    }

    #[tokio::test]
    async fn proceeds_when_match_day_has_no_pairings() {
        let match_day_repo = FakeMatchDayRepo(match_day_with_pairings(vec![]));
        let group_repo = FakeGroupRepo;
        let team_port = FakeTeamInfoPort;
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();

        let result = execute(
            "d1",
            "s1",
            "c1",
            "sp1",
            &match_day_repo,
            &group_repo,
            &team_port,
            &event_bus,
        )
        .await;

        // pas de groupes ni d'équipes enrôlées -> NoGroups, mais surtout PAS PairingsAlreadyExist
        assert!(matches!(result, Err(GenerateError::NoGroups)));
    }

    #[tokio::test]
    async fn reports_group_with_fewer_than_two_teams_as_skipped_instead_of_silent_success() {
        let match_day_repo = FakeMatchDayRepo(match_day_with_pairings(vec![]));
        let group_repo = FakeGroupRepoWithEmptyGroup("Poule 1");
        let team_port = FakeTeamInfoPortWithEnrolled(vec![TeamInfoDto {
            team_id: "t1".into(),
            team_name: "Team 1".into(),
            coach_id: String::new(),
            coach_name: String::new(),
            roster_name: String::new(),
            logo_url: None,
        }]);
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();

        let outcome = execute(
            "d1",
            "s1",
            "c1",
            "sp1",
            &match_day_repo,
            &group_repo,
            &team_port,
            &event_bus,
        )
        .await
        .expect("ne doit pas échouer, juste signaler la poule ignorée");

        assert_eq!(outcome.skipped_group_names, vec!["Poule 1".to_string()]);
    }
}

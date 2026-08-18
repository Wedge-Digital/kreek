use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::ports::{ITeamInfoPort, TeamInfoDto};
use crate::app::competitions::use_cases::admin::team_enrollment::{
    build_new_pairing_projection, load_enrolled_teams, resolve_team_names,
};
use crate::app::shared_kernel::bloodbowl::ids::PairingId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::EventId;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;
use std::collections::HashMap;

#[derive(Debug)]
pub enum AddMatchError {
    RoundNotFound,
    InvalidTeamId,
    TeamsNotEnrolled(Vec<String>),
    Repository(String),
}

/// Crée un pairing manuel — refuse (BR : pas de pairing pour une équipe non
/// enrôlée, donc pas d'event `PairingCreated`) si l'une des deux équipes n'est
/// pas `Enrolled` pour la saison, plutôt que d'émettre un event à métadonnées vides.
#[tracing::instrument(skip_all, fields(round_id = ?round_id))]
pub async fn execute(
    round_id: &str,
    season_id: &str,
    competition_id: &str,
    space_id: &str,
    home_team_id: &str,
    away_team_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
) -> Result<(), AddMatchError> {
    let match_day = match_day_repo
        .find_by_id(round_id)
        .await
        .map_err(|e| AddMatchError::Repository(e.to_string()))?
        .ok_or(AddMatchError::RoundNotFound)?;

    let team_display = load_enrolled_teams(season_id, team_port)
        .await
        .map_err(AddMatchError::Repository)?;

    ensure_both_enrolled(home_team_id, away_team_id, &team_display, team_port).await?;

    let pairing = Pairing {
        id: PairingId::new(),
        home_team_id: TeamId::try_new(home_team_id).map_err(|_| AddMatchError::InvalidTeamId)?,
        away_team_id: TeamId::try_new(away_team_id).map_err(|_| AddMatchError::InvalidTeamId)?,
    };
    let projection = build_new_pairing_projection(
        home_team_id,
        away_team_id,
        season_id,
        &match_day,
        &team_display,
    );
    match_day_repo
        .save_pairing(round_id, &pairing, &projection)
        .await
        .map_err(|e| AddMatchError::Repository(e.to_string()))?;

    emit_pairing_created(
        home_team_id,
        away_team_id,
        &pairing,
        competition_id,
        season_id,
        space_id,
        &match_day,
        &team_display,
        event_bus,
    );
    Ok(())
}

async fn ensure_both_enrolled(
    home_team_id: &str,
    away_team_id: &str,
    team_display: &HashMap<String, TeamInfoDto>,
    team_port: &dyn ITeamInfoPort,
) -> Result<(), AddMatchError> {
    let mut missing = Vec::new();
    if !team_display.contains_key(home_team_id) {
        missing.push(home_team_id.to_string());
    }
    if !team_display.contains_key(away_team_id) {
        missing.push(away_team_id.to_string());
    }
    if missing.is_empty() {
        return Ok(());
    }
    let names = resolve_team_names(missing, team_port).await;
    Err(AddMatchError::TeamsNotEnrolled(names))
}

#[allow(clippy::too_many_arguments)]
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
    let home_info = team_display
        .get(home)
        .expect("home team vérifié enrôlé avant émission");
    let away_info = team_display
        .get(away)
        .expect("away team vérifié enrôlé avant émission");

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
    use crate::app::competitions::domain::match_day::{
        MatchDayName, MatchDayPosition, MatchDayType,
    };
    use crate::app::shared_kernel::bloodbowl::ids::MatchId;
    use crate::common::services::event_bus::event_bus::new_bus;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct FakeMatchDayRepo(MatchDay);
    #[async_trait]
    impl IMatchDayRepository for FakeMatchDayRepo {
        async fn find_by_season(
            &self,
            _: &str,
        ) -> Result<
            Vec<MatchDay>,
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
        async fn find_by_id(
            &self,
            _: &str,
        ) -> Result<
            Option<MatchDay>,
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(Some(self.0.clone()))
        }
        async fn save_match_day(
            &self,
            _: &MatchDay,
        ) -> Result<
            (),
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(())
        }
        async fn delete_match_day(
            &self,
            _: &str,
        ) -> Result<
            (),
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(())
        }
        async fn save_pairing(
            &self,
            _: &str,
            _: &Pairing,
            _: &crate::app::competitions::domain::match_day_repository_port::NewPairingProjection,
        ) -> Result<
            (),
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(())
        }
        async fn find_pairing_id(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<
            Option<String>,
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(None)
        }
        async fn delete_pairing(
            &self,
            _: &str,
        ) -> Result<
            (),
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(())
        }
        async fn ensure_match_days_from_structure(
            &self,
            _: &str,
            _: &[(String, String, String, Option<String>, Option<String>)],
        ) -> Result<
            (),
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(())
        }
        async fn list_resultats(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<
            Vec<crate::app::competitions::domain::match_day_repository_port::PairingDisplayDto>,
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
        async fn list_calendrier(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<
            Vec<crate::app::competitions::domain::match_day_repository_port::PairingDisplayDto>,
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
        async fn list_latest_completed_results(
            &self,
            _: &str,
            _: i64,
        ) -> Result<
            Vec<crate::app::competitions::domain::match_day_repository_port::LatestResultDto>,
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
    }

    struct FakeTeamInfoPort(Mutex<Vec<TeamInfoDto>>);
    #[async_trait]
    impl ITeamInfoPort for FakeTeamInfoPort {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(self.0.lock().unwrap().clone())
        }
        async fn find_team_names(&self, team_ids: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![TeamInfoDto {
                team_id: team_ids[0].clone(),
                team_name: format!("Équipe {}", team_ids[0]),
                coach_id: String::new(),
                coach_name: String::new(),
                roster_name: String::new(),
                logo_url: None,
            }])
        }
    }

    fn dto(id: &str, name: &str) -> TeamInfoDto {
        TeamInfoDto {
            team_id: id.into(),
            team_name: name.into(),
            coach_id: "coach".into(),
            coach_name: "Coach".into(),
            roster_name: "Roster".into(),
            logo_url: None,
        }
    }

    fn sample_match_day() -> MatchDay {
        MatchDay {
            id: MatchId::new(),
            season_id: crate::app::shared_kernel::bloodbowl::ids::SeasonId::new(),
            name: MatchDayName::try_new("Journée 1".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(0).unwrap(),
            pairings: vec![],
        }
    }

    #[tokio::test]
    async fn refuses_when_a_team_is_not_enrolled() {
        let match_day_repo = FakeMatchDayRepo(sample_match_day());
        let team_port = FakeTeamInfoPort(Mutex::new(vec![dto("home", "Home Team")]));
        let event_bus = new_bus();

        let result = execute(
            "r1",
            "s1",
            "c1",
            "sp1",
            "home",
            "away",
            &match_day_repo,
            &team_port,
            &event_bus,
        )
        .await;

        assert!(matches!(result, Err(AddMatchError::TeamsNotEnrolled(_))));
    }

    #[tokio::test]
    async fn succeeds_and_emits_real_names_when_both_enrolled() {
        let home_id = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let away_id = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
        let match_day_repo = FakeMatchDayRepo(sample_match_day());
        let team_port = FakeTeamInfoPort(Mutex::new(vec![
            dto(home_id, "Home Team"),
            dto(away_id, "Away Team"),
        ]));
        let event_bus = new_bus();
        let mut rx = event_bus.subscribe();

        let result = execute(
            "r1",
            "s1",
            "c1",
            "sp1",
            home_id,
            away_id,
            &match_day_repo,
            &team_port,
            &event_bus,
        )
        .await;
        assert!(result.is_ok());

        let envelope = rx
            .try_recv()
            .expect("un event PairingCreated doit être émis");
        let event: CompetitionsDomainEvent = serde_json::from_value(envelope.payload).unwrap();
        let CompetitionsDomainEvent::PairingCreated {
            home_team_name,
            away_team_name,
            round_name,
            ..
        } = event
        else {
            panic!("mauvais type d'event");
        };
        assert_eq!(home_team_name, "Home Team");
        assert_eq!(away_team_name, "Away Team");
        assert_eq!(round_name, "Journée 1");
    }
}

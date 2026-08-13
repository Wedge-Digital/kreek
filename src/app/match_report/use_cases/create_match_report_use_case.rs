use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::MatchReportOrigin;
use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportAppEvent;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, EventId, SpaceId};
use crate::common::services::event_bus::event_bus::EventBus;

pub struct CreateMatchReportCommand {
    pub space_id: SpaceId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub created_by: CoachId,
    pub origin: MatchReportOrigin,
    pub pairing_id: Option<String>,
}

#[derive(Debug)]
pub enum CreateMatchReportError {
    SameTeam,
    Repository(String),
}

pub async fn execute(
    cmd: CreateMatchReportCommand,
    repo: &dyn IMatchReportRepository,
    app_event_bus: &EventBus,
) -> Result<MatchReportId, CreateMatchReportError> {
    if let Ok(Some(existing_id)) = repo
        .find_id_by_round_and_teams(
            &cmd.round_id.to_string(),
            &cmd.home_team_id.to_string(),
            &cmd.away_team_id.to_string(),
        )
        .await
    {
        let mr_id = MatchReportId::try_new(&existing_id)
            .map_err(|e| CreateMatchReportError::Repository(e.to_string()))?;

        return confirm_existing(mr_id, cmd.created_by, repo, app_event_bus).await;
    }

    let id = MatchReportId::new();

    let (draft, event) = MatchReportDraft::create(
        id,
        cmd.space_id,
        cmd.competition_id,
        cmd.season_id,
        cmd.round_id,
        cmd.home_team_id,
        cmd.away_team_id,
        cmd.created_by,
        cmd.origin,
        cmd.pairing_id,
    )
    .map_err(|_| CreateMatchReportError::SameTeam)?;

    repo.append(&id.to_string(), &event, 0)
        .await
        .map_err(|e| CreateMatchReportError::Repository(e.to_string()))?;

    // Saisie manuelle : le coach vient de choisir lui-même compétition/journée/équipes
    // via le formulaire — inutile de lui faire revalider la même sélection une seconde
    // fois (contrairement à l'origine Pairing, où le draft est créé en tâche de fond par
    // pairing_created_listener avant même que le coach n'ait vu l'écran de sélection).
    if cmd.origin == MatchReportOrigin::Manual {
        confirm_draft(draft, cmd.created_by, repo, app_event_bus).await?;
    }

    Ok(id)
}

async fn confirm_draft(
    draft: MatchReportDraft,
    confirmed_by: CoachId,
    repo: &dyn IMatchReportRepository,
    app_event_bus: &EventBus,
) -> Result<(), CreateMatchReportError> {
    let mr_id_str = draft.id.to_string();
    let space_id = draft.space_id.to_string();
    let home_id = draft.home_team_id.to_string();
    let away_id = draft.away_team_id.to_string();

    let (pre_match, confirm_event) = draft.confirm_selection(confirmed_by);

    repo.append(&mr_id_str, &confirm_event, pre_match.version - 1)
        .await
        .map_err(|e| CreateMatchReportError::Repository(e.to_string()))?;

    let _ = app_event_bus.send(
        MatchReportAppEvent::MatchReportConfirmed {
            event_id: EventId::new(),
            match_report_id: mr_id_str,
            home_team_id: home_id,
            away_team_id: away_id,
            space_id,
            pairing_id: pre_match.pairing_id.clone(),
        }
        .to_enveloppe(),
    );
    Ok(())
}

async fn confirm_existing(
    mr_id: MatchReportId,
    confirmed_by: CoachId,
    repo: &dyn IMatchReportRepository,
    app_event_bus: &EventBus,
) -> Result<MatchReportId, CreateMatchReportError> {
    let mr_id_str = mr_id.to_string();

    let state = repo
        .find_by_id(&mr_id_str)
        .await
        .map_err(|e| CreateMatchReportError::Repository(e.to_string()))?
        .ok_or_else(|| CreateMatchReportError::Repository("rapport introuvable".into()))?;

    match state {
        MatchReportState::Draft(draft) => {
            confirm_draft(draft, confirmed_by, repo, app_event_bus).await?;
            Ok(mr_id)
        }
        MatchReportState::PreMatch(_) => Ok(mr_id),
        MatchReportState::ReadyToPublish(_) => Ok(mr_id),
        MatchReportState::Published(_) => Ok(mr_id),
        MatchReportState::Cancelled(_) => {
            Err(CreateMatchReportError::Repository("rapport annulé".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::events::MatchReportDomainEvent;
    use crate::app::match_report::domain::match_report_repository_port::{
        MatchActionRow, RepositoryError,
    };
    use crate::app::match_report::domain::match_report_state::rehydrate;
    use crate::app::match_report::domain::value_objects::TeamSide;
    use crate::common::services::event_bus::event_bus::new_bus;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeMatchReportRepo {
        events: Mutex<HashMap<String, Vec<MatchReportDomainEvent>>>,
    }

    #[async_trait]
    impl IMatchReportRepository for FakeMatchReportRepo {
        /// Doublure : le contrôle d'appartenance est exercé par les tests de
        /// handler, sur une vraie base.
        async fn find_space_id(&self, _: &str) -> Result<Option<String>, RepositoryError> {
            Ok(None)
        }

        async fn append(
            &self,
            match_report_id: &str,
            event: &MatchReportDomainEvent,
            _expected_version: u64,
        ) -> Result<u64, RepositoryError> {
            let mut events = self.events.lock().unwrap();
            let entry = events.entry(match_report_id.to_string()).or_default();
            entry.push(event.clone());
            Ok(entry.len() as u64)
        }
        async fn find_by_id(
            &self,
            match_report_id: &str,
        ) -> Result<Option<MatchReportState>, RepositoryError> {
            let events = self.events.lock().unwrap();
            match events.get(match_report_id) {
                Some(evs) => Ok(Some(rehydrate(evs.clone()).expect("rehydrate"))),
                None => Ok(None),
            }
        }
        async fn append_many(
            &self,
            _: &str,
            _: Vec<MatchReportDomainEvent>,
            _: u64,
        ) -> Result<u64, RepositoryError> {
            unimplemented!("non utilisé par ces tests")
        }
        async fn find_id_by_pairing(&self, _: &str) -> Result<Option<String>, RepositoryError> {
            Ok(None)
        }
        async fn find_phases_by_pairings(
            &self,
            _: &[String],
        ) -> Result<Vec<(String, String)>, RepositoryError> {
            Ok(vec![])
        }
        async fn find_id_by_round_and_teams(
            &self,
            round_id: &str,
            team_a: &str,
            team_b: &str,
        ) -> Result<Option<String>, RepositoryError> {
            let events = self.events.lock().unwrap();
            for (id, evs) in events.iter() {
                let Some(MatchReportDomainEvent::MatchReportCreated {
                    round_id: r,
                    home_team_id,
                    away_team_id,
                    ..
                }) = evs.first()
                else {
                    continue;
                };
                let (r, h, a) = (
                    r.to_string(),
                    home_team_id.to_string(),
                    away_team_id.to_string(),
                );
                let same_teams = (h == team_a && a == team_b) || (h == team_b && a == team_a);
                if r == round_id && same_teams {
                    return Ok(Some(id.clone()));
                }
            }
            Ok(None)
        }
        async fn find_actions_by_match_and_side(
            &self,
            _: &str,
            _: TeamSide,
        ) -> Result<Vec<MatchActionRow>, RepositoryError> {
            Ok(vec![])
        }
    }

    fn sample_cmd(origin: MatchReportOrigin) -> CreateMatchReportCommand {
        CreateMatchReportCommand {
            space_id: SpaceId::new(),
            competition_id: CompetitionId::new(),
            season_id: SeasonId::new(),
            round_id: RoundId::new(),
            home_team_id: TeamId::new(),
            away_team_id: TeamId::new(),
            created_by: CoachId::new(),
            origin,
            pairing_id: None,
        }
    }

    #[tokio::test]
    async fn manual_origin_is_auto_confirmed_to_pre_match_in_one_step() {
        let repo = FakeMatchReportRepo::default();
        let bus = new_bus();

        let id = execute(sample_cmd(MatchReportOrigin::Manual), &repo, &bus)
            .await
            .unwrap();

        let state = repo.find_by_id(&id.to_string()).await.unwrap().unwrap();
        assert!(matches!(state, MatchReportState::PreMatch(_)));
    }

    #[tokio::test]
    async fn pairing_origin_stays_in_draft_until_explicitly_confirmed() {
        let repo = FakeMatchReportRepo::default();
        let bus = new_bus();

        let id = execute(sample_cmd(MatchReportOrigin::Pairing), &repo, &bus)
            .await
            .unwrap();

        let state = repo.find_by_id(&id.to_string()).await.unwrap().unwrap();
        assert!(matches!(state, MatchReportState::Draft(_)));
    }

    #[tokio::test]
    async fn calling_execute_again_for_the_same_round_and_teams_confirms_the_existing_draft() {
        let repo = FakeMatchReportRepo::default();
        let bus = new_bus();
        let cmd = sample_cmd(MatchReportOrigin::Pairing);
        let round_id = cmd.round_id;
        let home = cmd.home_team_id;
        let away = cmd.away_team_id;
        let coach = cmd.created_by;

        let first_id = execute(cmd, &repo, &bus).await.unwrap();
        let state = repo
            .find_by_id(&first_id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(state, MatchReportState::Draft(_)));

        let second_cmd = CreateMatchReportCommand {
            space_id: SpaceId::new(),
            competition_id: CompetitionId::new(),
            season_id: SeasonId::new(),
            round_id,
            home_team_id: home,
            away_team_id: away,
            created_by: coach,
            origin: MatchReportOrigin::Pairing,
            pairing_id: None,
        };
        let second_id = execute(second_cmd, &repo, &bus).await.unwrap();

        assert_eq!(first_id, second_id);
        let state = repo
            .find_by_id(&first_id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(state, MatchReportState::PreMatch(_)));
    }
}

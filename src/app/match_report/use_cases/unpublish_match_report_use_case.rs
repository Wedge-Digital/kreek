use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_published::MatchReportPublished;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{CorrectionBlocker, CorrectionEligibility};
use crate::app::match_report::ports::{IPlayerDataPort, ITeamDataPort};
use crate::app::match_report::use_cases::correction_eligibility_service;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct UnpublishMatchReportCommand {
    pub match_report_id: MatchReportId,
    pub unpublished_by: CoachId,
}

#[derive(Debug)]
pub enum UnpublishMatchReportError {
    NotFound,
    NotPublished,
    NotEligible(CorrectionBlocker),
    Repository(String),
}

/// Ramène un rapport publié en état corrigeable.
///
/// Symétrique de `publish_match_report_use_case`, y compris la convention
/// `version - 1` sur l'append. L'app event bus n'apparaît pas ici : le use case
/// émet un domain event sur le bus interne, le publisher fait la conversion.
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: UnpublishMatchReportCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    player_data: &dyn IPlayerDataPort,
    bus: &EventBus,
) -> Result<(), UnpublishMatchReportError> {
    let mr_id = cmd.match_report_id.to_string();
    let published = load_published(repo, &mr_id).await?;
    let eligibility =
        eligibility_of(&published, &cmd.match_report_id, team_data, player_data).await;

    let (rtp, event) = published
        .unpublish(cmd.unpublished_by, eligibility)
        .map_err(to_error)?;

    persist_and_announce(repo, bus, &mr_id, &event, rtp.version - 1).await
}

/// Persiste l'événement puis l'annonce sur le bus **interne** du BC. C'est le
/// publisher qui le convertira en app events — le use case ne connaît pas
/// l'app event bus.
async fn persist_and_announce(
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
    mr_id: &str,
    event: &MatchReportDomainEvent,
    version: u64,
) -> Result<(), UnpublishMatchReportError> {
    repo.append(mr_id, event, version)
        .await
        .map_err(|e| UnpublishMatchReportError::Repository(e.to_string()))?;
    emettre(bus, event.to_enveloppe(mr_id));
    Ok(())
}

async fn eligibility_of(
    published: &MatchReportPublished,
    match_report_id: &MatchReportId,
    team_data: &dyn ITeamDataPort,
    player_data: &dyn IPlayerDataPort,
) -> CorrectionEligibility {
    correction_eligibility_service::evaluate(
        &published.home_team_id,
        &published.away_team_id,
        match_report_id,
        team_data,
        player_data,
    )
    .await
}

async fn load_published(
    repo: &dyn IMatchReportRepository,
    mr_id: &str,
) -> Result<MatchReportPublished, UnpublishMatchReportError> {
    let state = repo
        .find_by_id(mr_id)
        .await
        .map_err(|e| UnpublishMatchReportError::Repository(e.to_string()))?
        .ok_or(UnpublishMatchReportError::NotFound)?;

    match state {
        MatchReportState::Published(p) => Ok(p),
        _ => Err(UnpublishMatchReportError::NotPublished),
    }
}

fn to_error(e: DomainError) -> UnpublishMatchReportError {
    match e {
        DomainError::CorrectionNotAllowed(blocker) => {
            UnpublishMatchReportError::NotEligible(blocker)
        }
        other => UnpublishMatchReportError::Repository(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::match_report::domain::match_report_repository_port::{
        MatchActionRow, RepositoryError,
    };
    use crate::app::match_report::domain::value_objects::{
        CorrectionEligibility, DedicatedFans, FanFactorMod, MatchGain, MatchReportOrigin, TeamSide,
    };
    use crate::app::match_report::ports::{
        JourneymanPositionDto, PositionCountDto, RosterPositionDto, TeamInfoDto,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RoundId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::SpaceId;
    use crate::common::services::event_bus::event_bus::new_bus;
    use std::sync::Mutex;

    // ── Fakes ─────────────────────────────────────────────────────────────

    struct FakeRepo {
        state: Option<MatchReportState>,
        appended: Mutex<Vec<MatchReportDomainEvent>>,
    }

    impl FakeRepo {
        fn with(state: Option<MatchReportState>) -> Self {
            Self {
                state,
                appended: Mutex::new(vec![]),
            }
        }
    }

    #[async_trait::async_trait]
    impl IMatchReportRepository for FakeRepo {
        /// Doublure : le contrôle d'appartenance est exercé par les tests de
        /// handler, sur une vraie base.
        async fn find_space_id(&self, _: &str) -> Result<Option<String>, RepositoryError> {
            Ok(None)
        }

        async fn append(
            &self,
            _: &str,
            event: &MatchReportDomainEvent,
            _: u64,
        ) -> Result<u64, RepositoryError> {
            self.appended.lock().unwrap().push(event.clone());
            Ok(1)
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<MatchReportState>, RepositoryError> {
            // `MatchReportState` n'est pas `Clone` : on rejoue l'état stocké en
            // le reconstruisant, ce qui suffit aux assertions de ces tests.
            Ok(match &self.state {
                Some(MatchReportState::Published(p)) => {
                    Some(MatchReportState::Published(p.clone()))
                }
                Some(MatchReportState::ReadyToPublish(r)) => {
                    Some(MatchReportState::ReadyToPublish(r.clone()))
                }
                _ => None,
            })
        }
        async fn append_many(
            &self,
            _: &str,
            _: Vec<MatchReportDomainEvent>,
            _: u64,
        ) -> Result<u64, RepositoryError> {
            Ok(1)
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
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<Option<String>, RepositoryError> {
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

    struct FakeTeamData {
        in_improvement: bool,
    }
    #[async_trait::async_trait]
    impl ITeamDataPort for FakeTeamData {
        async fn is_team_in_player_improvement(&self, _: &str) -> Result<bool, String> {
            Ok(self.in_improvement)
        }
        async fn is_team_ready_to_play(&self, _: &str) -> Result<bool, String> {
            Ok(false)
        }
        async fn is_coach_of_team(&self, _: &str, _: &str) -> Result<bool, String> {
            Ok(true)
        }
        async fn find_team_info(&self, _: &str) -> Option<TeamInfoDto> {
            None
        }
        async fn find_team_value(&self, _: &str) -> Option<u32> {
            None
        }
        async fn find_team_treasury(&self, _: &str) -> Option<u32> {
            None
        }
        async fn find_journeyman_position(&self, _: &str) -> Option<JourneymanPositionDto> {
            None
        }
        async fn find_roster_positions(&self, _: &str) -> Vec<RosterPositionDto> {
            vec![]
        }
    }

    struct FakePlayerData {
        spp_spent: bool,
    }
    #[async_trait::async_trait]
    impl IPlayerDataPort for FakePlayerData {
        async fn has_spent_spp_since_match(&self, _: &str, _: &str) -> Result<bool, String> {
            Ok(self.spp_spent)
        }
        async fn count_available_players(&self, _: &str) -> Result<usize, String> {
            Ok(11)
        }
        async fn find_player_display(&self, _: &str) -> Option<String> {
            None
        }
        async fn find_player_position(&self, _: &str) -> Option<String> {
            None
        }
        async fn find_player_counts_by_position(&self, _: &str) -> Vec<PositionCountDto> {
            vec![]
        }
    }

    // ── Fixtures ──────────────────────────────────────────────────────────

    fn published_state() -> MatchReportState {
        MatchReportState::Published(MatchReportPublished {
            home_inducement_spending: Default::default(),
            away_inducement_spending: Default::default(),
            id: MatchReportId::new(),
            space_id: SpaceId::new(),
            competition_id: CompetitionId::new(),
            season_id: SeasonId::new(),
            round_id: RoundId::new(),
            home_team_id: TeamId::new(),
            away_team_id: TeamId::new(),
            created_by: CoachId::new(),
            origin: MatchReportOrigin::Manual,
            pairing_id: None,
            home_fan_roll: None,
            away_fan_roll: None,
            home_dedicated_fans: DedicatedFans::default(),
            away_dedicated_fans: DedicatedFans::default(),
            home_inducements: None,
            away_inducements: None,
            star_engagements: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
            home_actions: vec![],
            away_actions: vec![],
            version: 7,
            home_gain: MatchGain::try_new(10_000).unwrap(),
            away_gain: MatchGain::try_new(5_000).unwrap(),
            home_fan_mod: FanFactorMod::try_new(1).unwrap(),
            away_fan_mod: FanFactorMod::try_new(-1).unwrap(),
            summary_title: None,
            summary_body: None,
            published_by: CoachId::new(),
            published_at: chrono::Utc::now(),
        })
    }

    fn command() -> UnpublishMatchReportCommand {
        UnpublishMatchReportCommand {
            match_report_id: MatchReportId::new(),
            unpublished_by: CoachId::new(),
        }
    }

    async fn run(
        state: Option<MatchReportState>,
        eligible: bool,
        spp_spent: bool,
    ) -> (Result<(), UnpublishMatchReportError>, FakeRepo) {
        let repo = FakeRepo::with(state);
        let bus = new_bus();
        let result = execute(
            command(),
            &repo,
            &FakeTeamData {
                in_improvement: eligible,
            },
            &FakePlayerData { spp_spent },
            &bus,
        )
        .await;
        (result, repo)
    }

    // ── Tests ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn rapport_introuvable_donne_not_found() {
        let (result, _) = run(None, true, false).await;
        assert!(matches!(result, Err(UnpublishMatchReportError::NotFound)));
    }

    #[tokio::test]
    async fn rapport_non_publie_donne_not_published() {
        let MatchReportState::Published(p) = published_state() else {
            unreachable!()
        };
        let (rtp, _) = p
            .unpublish(CoachId::new(), CorrectionEligibility::Eligible)
            .unwrap();
        let (result, _) = run(Some(MatchReportState::ReadyToPublish(rtp)), true, false).await;
        assert!(matches!(
            result,
            Err(UnpublishMatchReportError::NotPublished)
        ));
    }

    #[tokio::test]
    async fn spp_deja_depenses_donnent_not_eligible_avec_le_motif() {
        let (result, repo) = run(Some(published_state()), true, true).await;
        assert!(matches!(
            result,
            Err(UnpublishMatchReportError::NotEligible(
                CorrectionBlocker::SppAlreadySpent {
                    side: TeamSide::Home
                }
            ))
        ));
        assert!(
            repo.appended.lock().unwrap().is_empty(),
            "rien ne doit être appendé"
        );
    }

    #[tokio::test]
    async fn phase_avancee_donne_not_eligible() {
        let (result, _) = run(Some(published_state()), false, false).await;
        assert!(matches!(
            result,
            Err(UnpublishMatchReportError::NotEligible(
                CorrectionBlocker::PhaseAdvanced {
                    side: TeamSide::Home
                }
            ))
        ));
    }

    #[tokio::test]
    async fn succes_appende_l_evenement_de_depublication() {
        let (result, repo) = run(Some(published_state()), true, false).await;
        assert!(result.is_ok());

        let appended = repo.appended.lock().unwrap();
        assert_eq!(appended.len(), 1);
        assert!(matches!(
            appended[0],
            MatchReportDomainEvent::MatchReportUnpublished { .. }
        ));
    }

    #[tokio::test]
    async fn succes_emet_sur_le_bus_interne() {
        let repo = FakeRepo::with(Some(published_state()));
        let bus = new_bus();
        let mut rx = bus.subscribe();

        execute(
            command(),
            &repo,
            &FakeTeamData {
                in_improvement: true,
            },
            &FakePlayerData { spp_spent: false },
            &bus,
        )
        .await
        .unwrap();

        let envelope = rx.try_recv().expect("un événement doit être publié");
        assert_eq!(envelope.event_type, "MatchReportUnpublished");
    }
}

//! La validation d'un camp, en « Chacun son équipe » (carte 659-A, épic E22).
//!
//! Chaque coach valide son équipe ; la seconde validation publie :
//! `TeamSideValidated` et `MatchReportPublished` partent dans le même
//! `append_many`. La règle est celle du domaine (`validate_side`) ; ce use
//! case ne fait que charger, appender et émettre. Le `MatchReportPublished`
//! émis est celui du mode A, produit par la même `publish()` : ses
//! consommateurs ne voient pas la différence.
//!
//! **Le réessai sur conflit** (carte 653), sans garde : le domaine refuse de
//! valider deux fois le même camp (`SideValidated`). Un conflit perdu contre
//! l'autre camp se rejoue donc sans risque — et si l'autre vient de valider,
//! le réessai publie.
//!
//! Le camp vient de l'appelant, qui le tient du droit d'accès (le camp du
//! coach, ou celui que choisit l'admin) : jamais d'un formulaire de coach.

use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::team_validation::SideValidationOutcome;
use crate::app::match_report::domain::value_objects::TeamSide;
use crate::app::match_report::use_cases::conflict_retry::{lost_the_race, settle, ConflictAware};
use crate::app::match_report::use_cases::event_emission::emit_all;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct ValidateTeamSideCommand {
    pub match_report_id: MatchReportId,
    pub team_side: TeamSide,
    pub validated_by: CoachId,
}

#[derive(Debug)]
pub enum ValidateTeamSideError {
    NotFound,
    /// Un brouillon, un rapport publié ou annulé : rien à valider.
    NotInCompatibleState,
    Domain(DomainError),
    /// Une autre écriture est passée entre le chargement et l'append, et le
    /// réessai ne l'a pas résolue.
    Conflict,
    Internal(String),
}

impl ValidateTeamSideError {
    fn from_repository(e: RepositoryError) -> Self {
        match e {
            RepositoryError::ConcurrentWrite => Self::Conflict,
            other => Self::Internal(other.to_string()),
        }
    }
}

impl ConflictAware for ValidateTeamSideError {
    fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict)
    }
}

/// Où en est le rapport une fois le camp validé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStage {
    /// L'autre camp n'a pas encore validé.
    AwaitingOtherSide,
    /// La seconde validation : le rapport est publié.
    Published,
}

const COMMAND: &str = "validate_side";

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ValidateTeamSideCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
) -> Result<ValidationStage, ValidateTeamSideError> {
    let first = validate_once(&cmd, repo, bus).await;
    if !lost_the_race(COMMAND, &first) {
        return first;
    }
    settle(COMMAND, validate_once(&cmd, repo, bus).await)
}

/// Un essai : relire l'agrégat, le laisser décider, appender, émettre.
async fn validate_once(
    cmd: &ValidateTeamSideCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
) -> Result<ValidationStage, ValidateTeamSideError> {
    let mr_id = cmd.match_report_id.to_string();
    let state = repo
        .find_by_id(&mr_id)
        .await
        .map_err(ValidateTeamSideError::from_repository)?
        .ok_or(ValidateTeamSideError::NotFound)?;
    let (version, events, stage) = decide(state, cmd)?;
    repo.append_many(&mr_id, events.clone(), version)
        .await
        .map_err(ValidateTeamSideError::from_repository)?;
    emit_all(bus, &mr_id, &events);
    Ok(stage)
}

/// Ce qu'un essai écrit : la version lue, les événements, et l'étape.
type Decision = (u64, Vec<MatchReportDomainEvent>, ValidationStage);

fn decide(
    state: MatchReportState,
    cmd: &ValidateTeamSideCommand,
) -> Result<Decision, ValidateTeamSideError> {
    let (side, by) = (cmd.team_side, cmd.validated_by);
    let decided = match state {
        MatchReportState::PreMatch(pm) => pm
            .validate_side(side, by)
            .map(|(_, event)| (pm.version, vec![event], ValidationStage::AwaitingOtherSide)),
        MatchReportState::ReadyToPublish(rtp) => rtp
            .validate_side(side, by)
            .map(|(outcome, events)| (rtp.version, events, stage_of(&outcome))),
        _ => return Err(ValidateTeamSideError::NotInCompatibleState),
    };
    decided.map_err(ValidateTeamSideError::Domain)
}

fn stage_of(outcome: &SideValidationOutcome) -> ValidationStage {
    match outcome {
        SideValidationOutcome::Validated(_) => ValidationStage::AwaitingOtherSide,
        SideValidationOutcome::Published(_) => ValidationStage::Published,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
    use crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish;
    use crate::app::match_report::domain::team_post_match::{SidePostMatch, TeamPostMatch};
    use crate::app::match_report::domain::value_objects::{
        EntryMode, FanFactorMod, MatchGain, NoStallingBonus,
    };
    use crate::app::match_report::use_cases::test_doubles::{
        pre_match, with_mvps, RecordingRepository, ScriptedRepository,
    };
    use crate::common::services::event_bus::event_bus::new_bus;

    fn entry(side: TeamSide) -> TeamPostMatch {
        TeamPostMatch {
            side,
            post_match: SidePostMatch {
                gain: MatchGain::try_new(40).unwrap(),
                no_stalling_bonus: NoStallingBonus::new(false),
                fan_mod: FanFactorMod::try_new(0).unwrap(),
                paragraph: None,
                recorded_by: CoachId::new(),
            },
            title: None,
        }
    }

    /// Un avant-match « Chacun son équipe » où ce camp a son après-match.
    fn waiting_on(side: TeamSide) -> MatchReportPreMatch {
        let mut pm = with_mvps(pre_match());
        pm.entry_mode = EntryMode::TeamEntry;
        pm.apply_team_post_match(&entry(side))
    }

    fn ready() -> MatchReportReadyToPublish {
        waiting_on(TeamSide::Home)
            .apply_team_post_match(&entry(TeamSide::Away))
            .close_post_match()
            .unwrap()
    }

    fn command(side: TeamSide, by: CoachId) -> ValidateTeamSideCommand {
        ValidateTeamSideCommand {
            match_report_id: MatchReportId::new(),
            team_side: side,
            validated_by: by,
        }
    }

    #[tokio::test]
    async fn the_first_side_validates_in_pre_match_and_emits_one_event() {
        let repo =
            RecordingRepository::holding(MatchReportState::PreMatch(waiting_on(TeamSide::Away)));
        let bus = new_bus();
        let mut rx = bus.subscribe();
        let stage = execute(command(TeamSide::Away, CoachId::new()), &repo, &bus)
            .await
            .unwrap();
        assert_eq!(stage, ValidationStage::AwaitingOtherSide);
        assert!(matches!(
            repo.appended().as_slice(),
            [MatchReportDomainEvent::TeamSideValidated {
                team_side: TeamSide::Away,
                ..
            }]
        ));
        assert_eq!(rx.try_recv().unwrap().event_type, "TeamSideValidated");
    }

    /// R9 : la seconde validation publie, dans un seul append, et le second
    /// validateur est celui qui publie.
    #[tokio::test]
    async fn the_second_validation_publishes_in_one_append() {
        let rtp = ready().apply_side_validated(TeamSide::Away, CoachId::new());
        let repo = RecordingRepository::holding(MatchReportState::ReadyToPublish(rtp));
        let (bus, second) = (new_bus(), CoachId::new());
        let mut rx = bus.subscribe();
        let stage = execute(command(TeamSide::Home, second), &repo, &bus)
            .await
            .unwrap();
        assert_eq!(stage, ValidationStage::Published);
        let appended = repo.appended();
        let [MatchReportDomainEvent::TeamSideValidated { .. }, MatchReportDomainEvent::MatchReportPublished { published_by, .. }] =
            appended.as_slice()
        else {
            panic!("attendu validation puis publication : {appended:?}");
        };
        assert_eq!(*published_by, second);
        assert_eq!(rx.try_recv().unwrap().event_type, "TeamSideValidated");
        assert_eq!(rx.try_recv().unwrap().event_type, "MatchReportPublished");
    }

    /// L'autre camp a validé entre-temps : le réessai relit, et publie.
    #[tokio::test]
    async fn a_conflict_with_the_other_side_is_retried_and_publishes() {
        let before = ready();
        let after = before
            .clone()
            .apply_side_validated(TeamSide::Away, CoachId::new());
        let repo = ScriptedRepository::reading(vec![
            MatchReportState::ReadyToPublish(before),
            MatchReportState::ReadyToPublish(after),
        ])
        .refusing(1);
        let stage = execute(command(TeamSide::Home, CoachId::new()), &repo, &new_bus())
            .await
            .unwrap();
        assert_eq!(stage, ValidationStage::Published);
        assert_eq!(repo.append_attempts(), 2);
        assert_eq!(repo.appended().len(), 2);
    }

    #[tokio::test]
    async fn a_domain_refusal_writes_nothing() {
        let repo =
            RecordingRepository::holding(MatchReportState::PreMatch(waiting_on(TeamSide::Home)));
        let result = execute(command(TeamSide::Away, CoachId::new()), &repo, &new_bus()).await;
        assert!(matches!(
            result,
            Err(ValidateTeamSideError::Domain(
                DomainError::PostMatchMissing(TeamSide::Away)
            ))
        ));
        assert!(repo.appended().is_empty());
    }

    #[tokio::test]
    async fn a_published_report_has_nothing_to_validate() {
        let (published, _) = ready().publish(CoachId::new());
        let repo = RecordingRepository::holding(MatchReportState::Published(published));
        let result = execute(command(TeamSide::Home, CoachId::new()), &repo, &new_bus()).await;
        assert!(matches!(
            result,
            Err(ValidateTeamSideError::NotInCompatibleState)
        ));
    }

    #[tokio::test]
    async fn an_unknown_report_is_not_found() {
        let repo = ScriptedRepository::reading(vec![]);
        let result = execute(command(TeamSide::Home, CoachId::new()), &repo, &new_bus()).await;
        assert!(matches!(result, Err(ValidateTeamSideError::NotFound)));
    }

    /// Seul un conflit de version se rejoue ; toute autre panne reste interne.
    #[test]
    fn only_a_concurrent_write_is_a_conflict() {
        let conflict = ValidateTeamSideError::from_repository(RepositoryError::ConcurrentWrite);
        assert!(conflict.is_conflict());
        let broken =
            ValidateTeamSideError::from_repository(RepositoryError::Rehydration("x".into()));
        assert!(matches!(broken, ValidateTeamSideError::Internal(_)));
        assert!(!broken.is_conflict());
    }
}

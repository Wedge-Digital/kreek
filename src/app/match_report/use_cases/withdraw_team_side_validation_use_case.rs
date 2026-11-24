//! Le retrait de la validation d'un camp, en « Chacun son équipe » (carte
//! 659-B, épic E22).
//!
//! Un coach qui a validé trop tôt retire sa validation, corrige son camp et le
//! revalide, tant que le rapport n'est pas publié (§ 12.5). La règle est celle
//! du domaine (`withdraw_side_validation`) ; ce use case ne fait que charger,
//! appender et émettre.
//!
//! **Le réessai sur conflit** (carte 653), sans garde : le domaine refuse de
//! retirer la validation d'un camp qui n'est pas validé (`SideNotValidated`).
//! Un conflit perdu contre l'autre camp se rejoue donc sans risque ; un
//! conflit perdu contre la publication rend `NotInCompatibleState`.
//!
//! Le camp vient de l'appelant, qui le tient du droit d'accès (le camp du
//! coach, ou celui que choisit l'admin) : jamais d'un formulaire de coach.

use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::TeamSide;
use crate::app::match_report::use_cases::conflict_retry::{lost_the_race, settle, ConflictAware};
use crate::app::match_report::use_cases::event_emission::emit_all;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct WithdrawTeamSideValidationCommand {
    pub match_report_id: MatchReportId,
    pub team_side: TeamSide,
    pub withdrawn_by: CoachId,
}

#[derive(Debug)]
pub enum WithdrawTeamSideValidationError {
    NotFound,
    /// Un brouillon, un rapport publié ou annulé : rien à retirer.
    NotInCompatibleState,
    Domain(DomainError),
    /// Une autre écriture est passée entre le chargement et l'append, et le
    /// réessai ne l'a pas résolue.
    Conflict,
    Internal(String),
}

impl WithdrawTeamSideValidationError {
    fn from_repository(e: RepositoryError) -> Self {
        match e {
            RepositoryError::ConcurrentWrite => Self::Conflict,
            other => Self::Internal(other.to_string()),
        }
    }
}

impl ConflictAware for WithdrawTeamSideValidationError {
    fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict)
    }
}

const COMMAND: &str = "withdraw_side_validation";

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: WithdrawTeamSideValidationCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
) -> Result<(), WithdrawTeamSideValidationError> {
    let first = withdraw_once(&cmd, repo, bus).await;
    if !lost_the_race(COMMAND, &first) {
        return first;
    }
    settle(COMMAND, withdraw_once(&cmd, repo, bus).await)
}

/// Un essai : relire l'agrégat, le laisser décider, appender, émettre.
async fn withdraw_once(
    cmd: &WithdrawTeamSideValidationCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
) -> Result<(), WithdrawTeamSideValidationError> {
    let mr_id = cmd.match_report_id.to_string();
    let state = repo
        .find_by_id(&mr_id)
        .await
        .map_err(WithdrawTeamSideValidationError::from_repository)?
        .ok_or(WithdrawTeamSideValidationError::NotFound)?;
    let (version, event) = decide(state, cmd)?;
    let events = vec![event];
    repo.append_many(&mr_id, events.clone(), version)
        .await
        .map_err(WithdrawTeamSideValidationError::from_repository)?;
    emit_all(bus, &mr_id, &events);
    Ok(())
}

/// La version lue, et l'événement du retrait.
fn decide(
    state: MatchReportState,
    cmd: &WithdrawTeamSideValidationCommand,
) -> Result<(u64, MatchReportDomainEvent), WithdrawTeamSideValidationError> {
    let (side, by) = (cmd.team_side, cmd.withdrawn_by);
    let decided = match state {
        MatchReportState::PreMatch(pm) => pm
            .withdraw_side_validation(side, by)
            .map(|(_, event)| (pm.version, event)),
        MatchReportState::ReadyToPublish(rtp) => rtp
            .withdraw_side_validation(side, by)
            .map(|(_, event)| (rtp.version, event)),
        _ => return Err(WithdrawTeamSideValidationError::NotInCompatibleState),
    };
    decided.map_err(WithdrawTeamSideValidationError::Domain)
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

    fn command(side: TeamSide, by: CoachId) -> WithdrawTeamSideValidationCommand {
        WithdrawTeamSideValidationCommand {
            match_report_id: MatchReportId::new(),
            team_side: side,
            withdrawn_by: by,
        }
    }

    #[tokio::test]
    async fn a_validation_is_withdrawn_in_pre_match_and_emits_one_event() {
        let pm = waiting_on(TeamSide::Away).apply_side_validated(TeamSide::Away, CoachId::new());
        let repo = RecordingRepository::holding(MatchReportState::PreMatch(pm));
        let bus = new_bus();
        let mut rx = bus.subscribe();
        let by = CoachId::new();
        execute(command(TeamSide::Away, by), &repo, &bus)
            .await
            .unwrap();
        let appended = repo.appended();
        let [MatchReportDomainEvent::TeamSideValidationWithdrawn {
            team_side: TeamSide::Away,
            withdrawn_by,
        }] = appended.as_slice()
        else {
            panic!("attendu un retrait : {appended:?}");
        };
        assert_eq!(*withdrawn_by, by);
        let emitted = rx.try_recv().unwrap().event_type;
        assert_eq!(emitted, "TeamSideValidationWithdrawn");
    }

    #[tokio::test]
    async fn a_validation_is_withdrawn_in_ready_to_publish() {
        let rtp = ready().apply_side_validated(TeamSide::Home, CoachId::new());
        let repo = RecordingRepository::holding(MatchReportState::ReadyToPublish(rtp));
        execute(command(TeamSide::Home, CoachId::new()), &repo, &new_bus())
            .await
            .unwrap();
        assert_eq!(repo.appended().len(), 1);
    }

    /// L'autre camp a écrit entre-temps : le réessai relit, et retire.
    #[tokio::test]
    async fn a_conflict_with_the_other_side_is_retried() {
        let before = ready().apply_side_validated(TeamSide::Home, CoachId::new());
        let after = before.clone().apply_team_post_match(&entry(TeamSide::Away));
        let repo = ScriptedRepository::reading(vec![
            MatchReportState::ReadyToPublish(before),
            MatchReportState::ReadyToPublish(after),
        ])
        .refusing(1);
        execute(command(TeamSide::Home, CoachId::new()), &repo, &new_bus())
            .await
            .unwrap();
        assert_eq!(repo.append_attempts(), 2);
        assert_eq!(repo.appended().len(), 1);
    }

    #[tokio::test]
    async fn a_side_not_validated_writes_nothing() {
        let repo =
            RecordingRepository::holding(MatchReportState::PreMatch(waiting_on(TeamSide::Home)));
        let result = execute(command(TeamSide::Home, CoachId::new()), &repo, &new_bus()).await;
        assert!(matches!(
            result,
            Err(WithdrawTeamSideValidationError::Domain(
                DomainError::SideNotValidated(TeamSide::Home)
            ))
        ));
        assert!(repo.appended().is_empty());
    }

    #[tokio::test]
    async fn a_published_report_has_nothing_to_withdraw() {
        let (published, _) = ready().publish(CoachId::new());
        let repo = RecordingRepository::holding(MatchReportState::Published(published));
        let result = execute(command(TeamSide::Home, CoachId::new()), &repo, &new_bus()).await;
        assert!(matches!(
            result,
            Err(WithdrawTeamSideValidationError::NotInCompatibleState)
        ));
    }

    #[tokio::test]
    async fn an_unknown_report_is_not_found() {
        let repo = ScriptedRepository::reading(vec![]);
        let result = execute(command(TeamSide::Home, CoachId::new()), &repo, &new_bus()).await;
        assert!(matches!(
            result,
            Err(WithdrawTeamSideValidationError::NotFound)
        ));
    }

    /// Seul un conflit de version se rejoue ; toute autre panne reste interne.
    #[test]
    fn only_a_concurrent_write_is_a_conflict() {
        let conflict =
            WithdrawTeamSideValidationError::from_repository(RepositoryError::ConcurrentWrite);
        assert!(conflict.is_conflict());
        let broken = WithdrawTeamSideValidationError::from_repository(
            RepositoryError::Rehydration("x".into()),
        );
        assert!(matches!(
            broken,
            WithdrawTeamSideValidationError::Internal(_)
        ));
        assert!(!broken.is_conflict());
    }
}

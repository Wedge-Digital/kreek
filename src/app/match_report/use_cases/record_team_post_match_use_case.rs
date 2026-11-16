//! L'après-match d'un camp, en « Chacun son équipe » (carte 658, épic E22).
//!
//! Chaque coach enregistre les gains, le bonus, la modification de fans et le
//! paragraphe de son équipe ; le domicile y ajoute le titre. Le second camp
//! ferme l'après-match : `PostMatchClosed` part dans le même append. La règle
//! est celle du domaine (`record_team_post_match`) ; ce use case ne fait que
//! charger, appender et émettre.
//!
//! **Le réessai sur conflit** (carte 653), gardé : le domaine ne refuse pas
//! qu'un camp ressaisisse son après-match, il le remplace. Un réessai aveugle
//! écraserait donc en silence ce qu'un autre — l'admin, un second onglet —
//! vient d'écrire **pour le même camp**. Le réessai compare l'après-match de
//! son camp à la première lecture : inchangé, seul l'autre camp a écrit, et
//! rejouer est sûr ; changé, c'est un vrai conflit, rendu au client.
//!
//! Le camp vient de la route, que `garde_camp` a vérifiée : jamais du
//! formulaire.

use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::team_post_match::{
    SidePostMatch, TeamPostMatch, TeamPostMatchOutcome,
};
use crate::app::match_report::use_cases::conflict_retry::{lost_the_race, settle, ConflictAware};
use crate::app::match_report::use_cases::event_emission::emit_all;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct RecordTeamPostMatchCommand {
    pub match_report_id: MatchReportId,
    pub entry: TeamPostMatch,
}

#[derive(Debug)]
pub enum RecordTeamPostMatchError {
    NotFound,
    NotInCompatibleState,
    Domain(DomainError),
    /// Une autre écriture est passée entre le chargement et l'append, et le
    /// réessai ne l'a pas résolue.
    Conflict,
    Internal(String),
}

impl RecordTeamPostMatchError {
    fn from_repository(e: RepositoryError) -> Self {
        match e {
            RepositoryError::ConcurrentWrite => Self::Conflict,
            other => Self::Internal(other.to_string()),
        }
    }
}

/// Où en est l'après-match une fois celui du camp enregistré.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostMatchStage {
    /// L'autre camp n'a pas encore le sien.
    AwaitingOtherSide,
    /// Les deux camps l'ont : le rapport est prêt à publier.
    ReadyToPublish,
}

impl ConflictAware for RecordTeamPostMatchError {
    fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict)
    }
}

/// L'après-match du camp tel qu'un essai l'a lu.
type Seen = Option<SidePostMatch>;

const COMMAND: &str = "record_team_post_match";

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RecordTeamPostMatchCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
) -> Result<PostMatchStage, RecordTeamPostMatchError> {
    let (first, seen) = record_once(&cmd, repo, bus, None).await;
    if !lost_the_race(COMMAND, &first) {
        return first;
    }
    settle(COMMAND, record_once(&cmd, repo, bus, Some(&seen)).await.0)
}

/// Un essai : relire l'agrégat, vérifier au réessai que son camp n'a pas
/// bougé, le laisser décider, appender, émettre. Rend aussi ce qu'il a lu du
/// camp, pour le réessai.
async fn record_once(
    cmd: &RecordTeamPostMatchCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
    expected: Option<&Seen>,
) -> (Result<PostMatchStage, RecordTeamPostMatchError>, Seen) {
    let mr_id = cmd.match_report_id.to_string();
    let state = match load(repo, &mr_id).await {
        Ok(state) => state,
        Err(e) => return (Err(e), None),
    };
    let seen = side_of(&state, cmd);
    if expected.is_some_and(|before| *before != seen) {
        return (Err(RecordTeamPostMatchError::Conflict), seen);
    }
    (write(state, cmd, repo, bus).await, seen)
}

async fn load(
    repo: &dyn IMatchReportRepository,
    mr_id: &str,
) -> Result<MatchReportState, RecordTeamPostMatchError> {
    repo.find_by_id(mr_id)
        .await
        .map_err(RecordTeamPostMatchError::from_repository)?
        .ok_or(RecordTeamPostMatchError::NotFound)
}

fn side_of(state: &MatchReportState, cmd: &RecordTeamPostMatchCommand) -> Seen {
    match state {
        MatchReportState::PreMatch(pm) => pm.post_match_of(cmd.entry.side),
        MatchReportState::ReadyToPublish(rtp) => rtp.post_match_of(cmd.entry.side),
        _ => None,
    }
}

async fn write(
    state: MatchReportState,
    cmd: &RecordTeamPostMatchCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
) -> Result<PostMatchStage, RecordTeamPostMatchError> {
    let mr_id = cmd.match_report_id.to_string();
    let (version, events, stage) = decide(state, cmd)?;
    repo.append_many(&mr_id, events.clone(), version)
        .await
        .map_err(RecordTeamPostMatchError::from_repository)?;
    emit_all(bus, &mr_id, &events);
    Ok(stage)
}

/// Ce qu'un essai écrit : la version lue, les événements à appender, et où en
/// sera l'après-match.
type Decision = (u64, Vec<MatchReportDomainEvent>, PostMatchStage);

/// Dans l'avant-match, le domaine dit si le camp ferme l'après-match ; en
/// correction, il l'était déjà.
fn decide(
    state: MatchReportState,
    cmd: &RecordTeamPostMatchCommand,
) -> Result<Decision, RecordTeamPostMatchError> {
    let entry = cmd.entry.clone();
    let decided = match state {
        MatchReportState::PreMatch(pm) => pm
            .record_team_post_match(entry)
            .map(|(outcome, events)| (pm.version, events, stage_of(&outcome))),
        MatchReportState::ReadyToPublish(rtp) => rtp
            .record_team_post_match(entry)
            .map(|(_, event)| (rtp.version, vec![event], PostMatchStage::ReadyToPublish)),
        _ => return Err(RecordTeamPostMatchError::NotInCompatibleState),
    };
    decided.map_err(RecordTeamPostMatchError::Domain)
}

fn stage_of(outcome: &TeamPostMatchOutcome) -> PostMatchStage {
    match outcome {
        TeamPostMatchOutcome::Pending(_) => PostMatchStage::AwaitingOtherSide,
        TeamPostMatchOutcome::Closed(_) => PostMatchStage::ReadyToPublish,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
    use crate::app::match_report::domain::value_objects::{
        EntryMode, FanFactorMod, MatchGain, NoStallingBonus, TeamSide,
    };
    use crate::app::match_report::use_cases::test_doubles::{
        pre_match, with_mvps, RecordingRepository, ScriptedRepository,
    };
    use crate::app::shared_kernel::identity::ids::CoachId;
    use crate::common::services::event_bus::event_bus::new_bus;

    fn team_entry() -> MatchReportPreMatch {
        let mut pm = with_mvps(pre_match());
        pm.entry_mode = EntryMode::TeamEntry;
        pm
    }

    fn entry(side: TeamSide, kpo: u32) -> TeamPostMatch {
        TeamPostMatch {
            side,
            post_match: SidePostMatch {
                gain: MatchGain::try_new(kpo).unwrap(),
                no_stalling_bonus: NoStallingBonus::new(false),
                fan_mod: FanFactorMod::try_new(0).unwrap(),
                paragraph: None,
                recorded_by: CoachId::new(),
            },
            title: None,
        }
    }

    fn command(side: TeamSide, kpo: u32) -> RecordTeamPostMatchCommand {
        RecordTeamPostMatchCommand {
            match_report_id: MatchReportId::new(),
            entry: entry(side, kpo),
        }
    }

    /// L'avant-match où ce camp a déjà son après-match.
    fn with_side(pm: &MatchReportPreMatch, side: TeamSide, kpo: u32) -> MatchReportPreMatch {
        pm.clone().apply_team_post_match(&entry(side, kpo))
    }

    #[tokio::test]
    async fn the_first_side_records_and_emits_one_event() {
        let repo = RecordingRepository::holding(MatchReportState::PreMatch(team_entry()));
        let bus = new_bus();
        let mut rx = bus.subscribe();
        let stage = execute(command(TeamSide::Home, 60), &repo, &bus)
            .await
            .unwrap();
        assert_eq!(stage, PostMatchStage::AwaitingOtherSide);
        assert!(matches!(
            repo.appended().as_slice(),
            [MatchReportDomainEvent::TeamPostMatchRecorded { .. }]
        ));
        assert_eq!(rx.try_recv().unwrap().event_type, "TeamPostMatchRecorded");
    }

    #[tokio::test]
    async fn the_second_side_appends_the_closing_with_it() {
        let pm = with_side(&team_entry(), TeamSide::Home, 60);
        let repo = RecordingRepository::holding(MatchReportState::PreMatch(pm));
        let stage = execute(command(TeamSide::Away, 40), &repo, &new_bus())
            .await
            .unwrap();
        assert_eq!(stage, PostMatchStage::ReadyToPublish);
        assert!(matches!(
            repo.appended().as_slice(),
            [
                MatchReportDomainEvent::TeamPostMatchRecorded { .. },
                MatchReportDomainEvent::PostMatchClosed
            ]
        ));
    }

    /// L'autre camp a écrit entre-temps : le réessai relit, et ferme
    /// l'après-match.
    #[tokio::test]
    async fn a_conflict_with_the_other_side_is_retried() {
        let before = team_entry();
        let after = with_side(&before, TeamSide::Home, 60);
        let repo = ScriptedRepository::reading(vec![
            MatchReportState::PreMatch(before),
            MatchReportState::PreMatch(after),
        ])
        .refusing(1);
        execute(command(TeamSide::Away, 40), &repo, &new_bus())
            .await
            .unwrap();
        assert_eq!(repo.append_attempts(), 2);
        assert_eq!(repo.appended().len(), 2);
    }

    /// Le même camp a écrit entre-temps : rejouer l'écraserait en silence.
    #[tokio::test]
    async fn a_conflict_on_the_same_side_is_not_overwritten() {
        let before = team_entry();
        let after = with_side(&before, TeamSide::Away, 25);
        let repo = ScriptedRepository::reading(vec![
            MatchReportState::PreMatch(before),
            MatchReportState::PreMatch(after),
        ])
        .refusing(1);
        let result = execute(command(TeamSide::Away, 40), &repo, &new_bus()).await;
        assert!(matches!(result, Err(RecordTeamPostMatchError::Conflict)));
        assert_eq!(repo.append_attempts(), 1);
        assert!(repo.appended().is_empty());
    }

    #[tokio::test]
    async fn a_domain_refusal_writes_nothing() {
        let mut pm = team_entry();
        pm.away_actions.clear();
        let repo = RecordingRepository::holding(MatchReportState::PreMatch(pm));
        let result = execute(command(TeamSide::Away, 40), &repo, &new_bus()).await;
        assert!(matches!(
            result,
            Err(RecordTeamPostMatchError::Domain(DomainError::MvpMissing(
                TeamSide::Away
            )))
        ));
        assert!(repo.appended().is_empty());
    }

    /// Seul un conflit de version se rejoue ; toute autre panne reste interne.
    #[test]
    fn only_a_concurrent_write_is_a_conflict() {
        let conflict = RecordTeamPostMatchError::from_repository(RepositoryError::ConcurrentWrite);
        assert!(conflict.is_conflict());
        let broken =
            RecordTeamPostMatchError::from_repository(RepositoryError::Rehydration("x".into()));
        assert!(matches!(broken, RecordTeamPostMatchError::Internal(_)));
        assert!(!broken.is_conflict());
    }
}

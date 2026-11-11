//! L'étape MVP, saisie manuelle (carte 637).
//!
//! Sur le modèle de `record_action_use_case` : le nom et le poste du joueur
//! sont résolus au moment présent, un rapport prêt à publier repasse en
//! avant-match (correction), et un conflit d'écriture se relaie. La règle —
//! tour 16, un MVP par camp — est celle du domaine (`record_mvp`).

use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{ActionId, ActionPlayer, TeamSide};
use crate::app::match_report::ports::IPlayerDataPort;
use crate::app::match_report::use_cases::conflict_retry::{lost_the_race, settle, ConflictAware};
use crate::app::match_report::use_cases::event_emission::emit_all;
use crate::app::match_report::use_cases::record_action_use_case::{
    resolve_player_info, RecordActionError,
};
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct RecordMvpCommand {
    pub match_report_id: MatchReportId,
    pub team_side: TeamSide,
    pub player: ActionPlayer,
    pub recorded_by: CoachId,
}

#[derive(Debug)]
pub enum RecordMvpError {
    NotFound,
    NotInPreMatchPhase,
    PlayerNotFound(String),
    TempPlayerNotFound(String),
    Domain(DomainError),
    /// Une autre écriture est passée entre le chargement et l'append (carte 623).
    Conflict,
    Repository(String),
}

impl RecordMvpError {
    pub(crate) fn from_repository(e: RepositoryError) -> Self {
        match e {
            RepositoryError::ConcurrentWrite => Self::Conflict,
            other => Self::Repository(other.to_string()),
        }
    }

    /// La résolution du joueur est celle des actions : seuls ses deux refus
    /// peuvent en sortir.
    pub(crate) fn from_resolution(e: RecordActionError) -> Self {
        match e {
            RecordActionError::PlayerNotFound(id) => Self::PlayerNotFound(id),
            RecordActionError::TempPlayerNotFound(id) => Self::TempPlayerNotFound(id),
            other => Self::Repository(format!("{other:?}")),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RecordMvpCommand,
    repo: &dyn IMatchReportRepository,
    player_data: &dyn IPlayerDataPort,
    bus: &EventBus,
) -> Result<(), RecordMvpError> {
    let action_id = ActionId(ulid::Ulid::new().to_string());
    // Carte 653 : une course perdue contre l'autre camp se rejoue une fois. Un
    // MVP que le même camp vient d'inscrire sort alors en `MvpAlreadyRecorded`.
    let first = record_once(&cmd, &action_id, repo, player_data, bus).await;
    if !lost_the_race(COMMAND, &first) {
        return first;
    }
    settle(
        COMMAND,
        record_once(&cmd, &action_id, repo, player_data, bus).await,
    )
}

const COMMAND: &str = "record_mvp";

/// Partagé avec le tirage au sort, dont l'erreur est la même.
impl ConflictAware for RecordMvpError {
    fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict)
    }
}

/// Un essai : relire l'agrégat, le laisser décider, appender à la version lue.
async fn record_once(
    cmd: &RecordMvpCommand,
    action_id: &ActionId,
    repo: &dyn IMatchReportRepository,
    player_data: &dyn IPlayerDataPort,
    bus: &EventBus,
) -> Result<(), RecordMvpError> {
    let mr_id = cmd.match_report_id.to_string();
    let pm = load_pre_match(&mr_id, repo).await?;
    let (display_name, position) =
        resolve_player_info(&cmd.player, cmd.team_side, &pm, player_data)
            .await
            .map_err(RecordMvpError::from_resolution)?;
    // Le refus d'un second MVP vient du domaine — le use case ne fait que le porter.
    let (_, event) = pm
        .record_mvp(
            cmd.team_side,
            cmd.player.clone(),
            display_name,
            position,
            action_id.clone(),
            cmd.recorded_by.clone(),
        )
        .map_err(RecordMvpError::Domain)?;
    repo.append(&mr_id, &event, pm.version)
        .await
        .map_err(RecordMvpError::from_repository)?;
    // Le MVP sort du BC comme une action (carte 643) : un conflit n'émet rien.
    emit_all(bus, &mr_id, std::slice::from_ref(&event));
    Ok(())
}

/// Un rapport prêt à publier repasse en avant-match : la correction d'un MVP
/// est une saisie comme une autre.
pub(crate) async fn load_pre_match(
    mr_id: &str,
    repo: &dyn IMatchReportRepository,
) -> Result<MatchReportPreMatch, RecordMvpError> {
    let state = repo
        .find_by_id(mr_id)
        .await
        .map_err(RecordMvpError::from_repository)?;
    match state.ok_or(RecordMvpError::NotFound)? {
        MatchReportState::PreMatch(pm) => Ok(pm),
        MatchReportState::ReadyToPublish(rtp) => Ok(rtp.into_pre_match()),
        _ => Err(RecordMvpError::NotInPreMatchPhase),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::events::MatchReportDomainEvent;
    use crate::app::match_report::domain::value_objects::{
        MatchAction, MatchActionType, TurnNumber,
    };
    use crate::app::match_report::use_cases::test_doubles::{
        pre_match, KnownPlayers, RecordingRepository, ScriptedRepository,
    };
    use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
    use crate::common::services::event_bus::event_bus::new_bus;

    fn command(side: TeamSide) -> RecordMvpCommand {
        RecordMvpCommand {
            match_report_id: MatchReportId::new(),
            team_side: side,
            player: ActionPlayer::Regular(PlayerId::new()),
            recorded_by: CoachId::new(),
        }
    }

    #[tokio::test]
    async fn le_mvp_est_appende_au_tour_16() {
        let repo = RecordingRepository::holding(MatchReportState::PreMatch(pre_match()));
        execute(command(TeamSide::Away), &repo, &KnownPlayers, &new_bus())
            .await
            .unwrap();
        let appended = repo.appended();
        let [MatchReportDomainEvent::ActionRecorded {
            turn,
            action,
            team_side,
            ..
        }] = appended.as_slice()
        else {
            panic!("attendu un seul ActionRecorded : {appended:?}");
        };
        assert_eq!(turn.value(), 16);
        assert_eq!(*team_side, TeamSide::Away);
        assert!(matches!(action, MatchActionType::Mvp));
    }

    #[tokio::test]
    async fn un_second_mvp_est_refuse_sans_ecriture() {
        let mut pm = pre_match();
        pm.home_actions.push(MatchAction {
            id: ActionId("ancien".into()),
            turn: TurnNumber::try_new(7).unwrap(),
            player: ActionPlayer::Regular(PlayerId::new()),
            action: MatchActionType::Mvp,
            player_display_name: "Borin".into(),
            player_position: String::new(),
        });
        let repo = RecordingRepository::holding(MatchReportState::PreMatch(pm));
        let r = execute(command(TeamSide::Home), &repo, &KnownPlayers, &new_bus()).await;
        assert!(matches!(
            r,
            Err(RecordMvpError::Domain(DomainError::MvpAlreadyRecorded))
        ));
        assert!(repo.appended().is_empty());
    }

    fn reads(second: MatchReportPreMatch) -> Vec<MatchReportState> {
        vec![
            MatchReportState::PreMatch(pre_match()),
            MatchReportState::PreMatch(second),
        ]
    }

    /// Carte 653 : deux conflits de suite sont relayés, après deux appends.
    #[tokio::test]
    async fn two_conflicts_are_relayed() {
        let repo = ScriptedRepository::reading(reads(pre_match())).refusing(2);
        let bus = new_bus();
        let mut subscriber = bus.subscribe();
        let r = execute(command(TeamSide::Home), &repo, &KnownPlayers, &bus).await;
        assert!(matches!(r, Err(RecordMvpError::Conflict)), "{:?}", r.err());
        assert_eq!(repo.append_attempts(), 2);
        assert!(subscriber.try_recv().is_err(), "un conflit n'émet rien");
    }

    /// Carte 653 : l'autre camp a écrit pendant ce temps ; le MVP passe au
    /// réessai.
    #[tokio::test]
    async fn a_lost_race_against_the_other_side_is_retried() {
        let mut other_side_wrote = pre_match();
        other_side_wrote.away_actions.push(mvp_at(16, "visiteur"));
        let repo = ScriptedRepository::reading(reads(other_side_wrote)).refusing(1);
        execute(command(TeamSide::Home), &repo, &KnownPlayers, &new_bus())
            .await
            .expect("le réessai passe");
        assert_eq!(repo.appended().len(), 1);
    }

    /// Carte 653 : la course est perdue contre un MVP **du même camp**. Relu,
    /// le domaine refuse le second — `MvpAlreadyRecorded`, pas un 409, et rien
    /// n'est écrasé.
    #[tokio::test]
    async fn a_same_side_mvp_written_meanwhile_is_refused_by_the_domain() {
        let mut same_side_wrote = pre_match();
        same_side_wrote.home_actions.push(mvp_at(16, "domicile"));
        let repo = ScriptedRepository::reading(reads(same_side_wrote)).refusing(1);
        let r = execute(command(TeamSide::Home), &repo, &KnownPlayers, &new_bus()).await;
        assert!(
            matches!(
                r,
                Err(RecordMvpError::Domain(DomainError::MvpAlreadyRecorded))
            ),
            "{:?}",
            r.err()
        );
        assert!(repo.appended().is_empty());
    }

    fn mvp_at(turn: u8, id: &str) -> MatchAction {
        MatchAction {
            id: ActionId(id.into()),
            turn: TurnNumber::try_new(turn).unwrap(),
            player: ActionPlayer::Regular(PlayerId::new()),
            action: MatchActionType::Mvp,
            player_display_name: "Borin".into(),
            player_position: String::new(),
        }
    }

    /// Carte 643 : le MVP saisi sort du BC comme une action, une fois appendé.
    #[tokio::test]
    async fn a_recorded_mvp_is_emitted_after_append() {
        let repo = RecordingRepository::holding(MatchReportState::PreMatch(pre_match()));
        let bus = new_bus();
        let mut subscriber = bus.subscribe();

        execute(command(TeamSide::Home), &repo, &KnownPlayers, &bus)
            .await
            .expect("MVP saisi");

        assert_eq!(subscriber.try_recv().unwrap().event_type, "ActionRecorded");
        assert!(subscriber.try_recv().is_err());
    }
}

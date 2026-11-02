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
use crate::app::match_report::use_cases::record_action_use_case::{
    resolve_player_info, RecordActionError,
};
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;

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
) -> Result<(), RecordMvpError> {
    let mr_id = cmd.match_report_id.to_string();
    let pm = load_pre_match(&mr_id, repo).await?;
    let (display_name, position) =
        resolve_player_info(&cmd.player, cmd.team_side, &pm, player_data)
            .await
            .map_err(RecordMvpError::from_resolution)?;
    let action_id = ActionId(ulid::Ulid::new().to_string());
    // Le refus d'un second MVP vient du domaine — le use case ne fait que le porter.
    let (_, event) = pm
        .record_mvp(
            cmd.team_side,
            cmd.player,
            display_name,
            position,
            action_id,
            cmd.recorded_by,
        )
        .map_err(RecordMvpError::Domain)?;
    repo.append(&mr_id, &event, pm.version)
        .await
        .map_err(RecordMvpError::from_repository)?;
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
        pre_match, ConflictingRepository, KnownPlayers, RecordingRepository,
    };
    use crate::app::shared_kernel::bloodbowl::ids::PlayerId;

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
        execute(command(TeamSide::Away), &repo, &KnownPlayers)
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
        let r = execute(command(TeamSide::Home), &repo, &KnownPlayers).await;
        assert!(matches!(
            r,
            Err(RecordMvpError::Domain(DomainError::MvpAlreadyRecorded))
        ));
        assert!(repo.appended().is_empty());
    }

    #[tokio::test]
    async fn un_conflit_d_ecriture_est_relaye() {
        let repo = ConflictingRepository::holding(MatchReportState::PreMatch(pre_match()));
        let r = execute(command(TeamSide::Home), &repo, &KnownPlayers).await;
        assert!(matches!(r, Err(RecordMvpError::Conflict)), "{:?}", r.err());
    }
}

use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::ports::ITeamDataPort;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct UpdateMatchSelectionCommand {
    pub match_report_id: MatchReportId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub confirmed_by: CoachId,
}

#[derive(Debug)]
pub enum UpdateMatchSelectionError {
    NotFound,
    NotInDraftPhase,
    SameTeam,
    TeamNotAvailable(String),
    /// Une autre écriture est passée entre le chargement et l'append (carte 623).
    Conflict,
    Repository(String),
}

impl UpdateMatchSelectionError {
    /// `ConcurrentWrite` garde son sens : sans cette conversion, il devenait un
    /// `Repository` comme un autre, et le contrôleur rendait 500 (carte 623).
    fn from_repository(e: RepositoryError) -> Self {
        match e {
            RepositoryError::ConcurrentWrite => Self::Conflict,
            other => Self::Repository(other.to_string()),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: UpdateMatchSelectionCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    event_bus: &EventBus,
) -> Result<MatchReportId, UpdateMatchSelectionError> {
    let mr_id_str = cmd.match_report_id.to_string();

    let state = repo
        .find_by_id(&mr_id_str)
        .await
        .map_err(UpdateMatchSelectionError::from_repository)?
        .ok_or(UpdateMatchSelectionError::NotFound)?;

    let mut draft = match state {
        MatchReportState::Draft(d) => d,
        _ => return Err(UpdateMatchSelectionError::NotInDraftPhase),
    };

    let selection_changed =
        draft.home_team_id != cmd.home_team_id || draft.away_team_id != cmd.away_team_id;

    if selection_changed {
        let (updated, event) = draft
            .update_selection(cmd.home_team_id, cmd.away_team_id, cmd.confirmed_by)
            .map_err(|_| UpdateMatchSelectionError::SameTeam)?;

        repo.append(&mr_id_str, &event, updated.version - 1)
            .await
            .map_err(UpdateMatchSelectionError::from_repository)?;

        draft = updated;
    }

    let home_ready = team_data
        .is_team_ready_to_play(&draft.home_team_id.to_string())
        .await
        .map_err(|e| UpdateMatchSelectionError::Repository(e))?;
    if !home_ready {
        return Err(UpdateMatchSelectionError::TeamNotAvailable(
            draft.home_team_id.to_string(),
        ));
    }

    let away_ready = team_data
        .is_team_ready_to_play(&draft.away_team_id.to_string())
        .await
        .map_err(|e| UpdateMatchSelectionError::Repository(e))?;
    if !away_ready {
        return Err(UpdateMatchSelectionError::TeamNotAvailable(
            draft.away_team_id.to_string(),
        ));
    }

    let (pre_match, confirm_event) = draft.confirm_selection(cmd.confirmed_by);

    repo.append(&mr_id_str, &confirm_event, pre_match.version - 1)
        .await
        .map_err(UpdateMatchSelectionError::from_repository)?;

    // Le domain event, sur le bus du BC : c'est le publisher qui en fait
    // `MatchReportConfirmed` pour `teams` et `competitions` (carte 352).
    emettre(event_bus, confirm_event.to_enveloppe(&mr_id_str));

    Ok(cmd.match_report_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
    use crate::app::match_report::domain::value_objects::MatchReportOrigin;
    use crate::app::match_report::use_cases::test_doubles::{ConflictingRepository, ReadyTeams};
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RoundId, SeasonId};
    use crate::app::shared_kernel::identity::ids::SpaceId;
    use crate::common::services::event_bus::event_bus::new_bus;

    fn draft() -> MatchReportDraft {
        MatchReportDraft {
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
            version: 1,
        }
    }

    async fn run(draft: MatchReportDraft, home: TeamId, away: TeamId) -> UpdateMatchSelectionError {
        let repo = ConflictingRepository::holding(MatchReportState::Draft(draft));
        let cmd = UpdateMatchSelectionCommand {
            match_report_id: MatchReportId::new(),
            home_team_id: home,
            away_team_id: away,
            confirmed_by: CoachId::new(),
        };
        execute(cmd, &repo, &ReadyTeams, &new_bus())
            .await
            .expect_err("l'append doit échouer")
    }

    /// Sélection inchangée : le seul append est celui de la confirmation.
    #[tokio::test]
    async fn un_conflit_sur_la_confirmation_est_relaye() {
        let d = draft();
        let (home, away) = (d.home_team_id.clone(), d.away_team_id.clone());
        let erreur = run(d, home, away).await;
        assert!(
            matches!(erreur, UpdateMatchSelectionError::Conflict),
            "{erreur:?}"
        );
    }

    /// Sélection changée : le premier append, celui du changement, échoue.
    #[tokio::test]
    async fn un_conflit_sur_le_changement_d_equipes_est_relaye() {
        let erreur = run(draft(), TeamId::new(), TeamId::new()).await;
        assert!(
            matches!(erreur, UpdateMatchSelectionError::Conflict),
            "{erreur:?}"
        );
    }
}

use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::EntryMode;
use crate::app::match_report::ports::ITeamDataPort;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct UpdateMatchSelectionCommand {
    pub match_report_id: MatchReportId,
    /// Les équipes demandées par le formulaire, domicile puis visiteur.
    ///
    /// `None` confirme les équipes du brouillon. **Ignorées pour un match du
    /// calendrier**, pour tous, admin compris (carte 633, règle R4) : c'est
    /// `MatchReportDraft::keeps_its_teams` qui en décide.
    pub requested_teams: Option<(TeamId, TeamId)>,
    pub confirmed_by: CoachId,
    /// Le mode de saisie choisi par celui qui démarre (carte 631).
    pub entry_mode: EntryMode,
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
    let mut draft = load_draft(&cmd.match_report_id.to_string(), repo).await?;

    let requested = cmd.requested_teams.filter(|_| !draft.keeps_its_teams());
    if let Some((home, away)) = requested {
        draft = change_selection(draft, home, away, cmd.confirmed_by, repo).await?;
    }

    ensure_ready_to_play(&draft.home_team_id, team_data).await?;
    ensure_ready_to_play(&draft.away_team_id, team_data).await?;
    confirm(draft, cmd.confirmed_by, cmd.entry_mode, repo, event_bus).await?;
    Ok(cmd.match_report_id)
}

async fn load_draft(
    mr_id: &str,
    repo: &dyn IMatchReportRepository,
) -> Result<MatchReportDraft, UpdateMatchSelectionError> {
    let state = repo
        .find_by_id(mr_id)
        .await
        .map_err(UpdateMatchSelectionError::from_repository)?
        .ok_or(UpdateMatchSelectionError::NotFound)?;
    match state {
        MatchReportState::Draft(d) => Ok(d),
        _ => Err(UpdateMatchSelectionError::NotInDraftPhase),
    }
}

/// Change la sélection du brouillon, si celle demandée en diffère.
async fn change_selection(
    draft: MatchReportDraft,
    home: TeamId,
    away: TeamId,
    updated_by: CoachId,
    repo: &dyn IMatchReportRepository,
) -> Result<MatchReportDraft, UpdateMatchSelectionError> {
    if draft.home_team_id == home && draft.away_team_id == away {
        return Ok(draft);
    }
    let (updated, event) = draft
        .update_selection(home, away, updated_by)
        .map_err(|_| UpdateMatchSelectionError::SameTeam)?;
    repo.append(&draft.id.to_string(), &event, updated.version - 1)
        .await
        .map_err(UpdateMatchSelectionError::from_repository)?;
    Ok(updated)
}

async fn ensure_ready_to_play(
    team_id: &TeamId,
    team_data: &dyn ITeamDataPort,
) -> Result<(), UpdateMatchSelectionError> {
    let ready = team_data
        .is_team_ready_to_play(&team_id.to_string())
        .await
        .map_err(UpdateMatchSelectionError::Repository)?;
    if ready {
        Ok(())
    } else {
        Err(UpdateMatchSelectionError::TeamNotAvailable(
            team_id.to_string(),
        ))
    }
}

/// `SelectionConfirmed` et `EntryModeChosen` dans le **même** `append_many` :
/// un démarrage n'existe pas sans son mode (carte 631).
///
/// Les domain events partent sur le bus du BC : c'est le publisher qui fait de
/// `SelectionConfirmed` un `MatchReportConfirmed` pour `teams` et
/// `competitions` (carte 352) ; `EntryModeChosen` ne sort pas du BC.
async fn confirm(
    draft: MatchReportDraft,
    confirmed_by: CoachId,
    entry_mode: EntryMode,
    repo: &dyn IMatchReportRepository,
    event_bus: &EventBus,
) -> Result<(), UpdateMatchSelectionError> {
    let mr_id = draft.id.to_string();
    let expected_version = draft.version;
    let (_, events) = draft.confirm_selection(confirmed_by, entry_mode);
    repo.append_many(&mr_id, events.clone(), expected_version)
        .await
        .map_err(UpdateMatchSelectionError::from_repository)?;
    for event in &events {
        emettre(event_bus, event.to_enveloppe(&mr_id));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::events::MatchReportDomainEvent;
    use crate::app::match_report::domain::value_objects::MatchReportOrigin;
    use crate::app::match_report::use_cases::test_doubles::{
        ConflictingRepository, ReadyTeams, RecordingRepository,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RoundId, SeasonId};
    use crate::app::shared_kernel::identity::ids::SpaceId;
    use crate::common::services::event_bus::event_bus::new_bus;

    fn draft_from(origin: MatchReportOrigin) -> MatchReportDraft {
        MatchReportDraft {
            id: MatchReportId::new(),
            space_id: SpaceId::new(),
            competition_id: CompetitionId::new(),
            season_id: SeasonId::new(),
            round_id: RoundId::new(),
            home_team_id: TeamId::new(),
            away_team_id: TeamId::new(),
            created_by: CoachId::new(),
            origin,
            pairing_id: None,
            version: 1,
        }
    }

    fn draft() -> MatchReportDraft {
        draft_from(MatchReportOrigin::Manual)
    }

    fn command(requested_teams: Option<(TeamId, TeamId)>) -> UpdateMatchSelectionCommand {
        UpdateMatchSelectionCommand {
            match_report_id: MatchReportId::new(),
            requested_teams,
            confirmed_by: CoachId::new(),
            entry_mode: EntryMode::SoloEntry,
        }
    }

    async fn run(draft: MatchReportDraft, home: TeamId, away: TeamId) -> UpdateMatchSelectionError {
        let repo = ConflictingRepository::holding(MatchReportState::Draft(draft));
        execute(command(Some((home, away))), &repo, &ReadyTeams, &new_bus())
            .await
            .expect_err("l'append doit échouer")
    }

    /// Ce que le use case a appendu, en démarrant ce brouillon avec ces équipes.
    async fn appended_by(
        draft: MatchReportDraft,
        requested_teams: Option<(TeamId, TeamId)>,
    ) -> Vec<MatchReportDomainEvent> {
        let repo = RecordingRepository::holding(MatchReportState::Draft(draft));
        execute(command(requested_teams), &repo, &ReadyTeams, &new_bus())
            .await
            .expect("le démarrage doit réussir");
        repo.appended()
    }

    fn changes_selection(events: &[MatchReportDomainEvent]) -> bool {
        events
            .iter()
            .any(|e| matches!(e, MatchReportDomainEvent::SelectionUpdated { .. }))
    }

    /// Sélection inchangée : le seul append est celui de la confirmation.
    #[tokio::test]
    async fn un_conflit_sur_la_confirmation_est_relaye() {
        let d = draft();
        let (home, away) = (d.home_team_id, d.away_team_id);
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

    /// Carte 633 — un match du calendrier démarre avec **ses** équipes : celles
    /// envoyées sont ignorées, et le démarrage a lieu quand même.
    #[tokio::test]
    async fn a_scheduled_draft_ignores_the_requested_teams() {
        let events = appended_by(
            draft_from(MatchReportOrigin::Pairing),
            Some((TeamId::new(), TeamId::new())),
        )
        .await;
        assert!(!changes_selection(&events), "{events:?}");
        assert!(
            matches!(
                events.as_slice(),
                [
                    MatchReportDomainEvent::SelectionConfirmed { .. },
                    MatchReportDomainEvent::EntryModeChosen { .. }
                ]
            ),
            "{events:?}"
        );
    }

    /// Sans équipes demandées, le brouillon démarre tel qu'il est.
    #[tokio::test]
    async fn no_requested_teams_confirms_the_draft_as_it_is() {
        let events = appended_by(draft(), None).await;
        assert!(!changes_selection(&events), "{events:?}");
        assert_eq!(events.len(), 2, "{events:?}");
    }

    /// Un brouillon hors calendrier d'avant la carte 552 garde sa sélection.
    #[tokio::test]
    async fn a_manual_draft_still_changes_its_teams() {
        let events = appended_by(draft(), Some((TeamId::new(), TeamId::new()))).await;
        assert!(changes_selection(&events), "{events:?}");
    }
}

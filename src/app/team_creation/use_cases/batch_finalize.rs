use crate::app::shared_kernel::common_types::{Entity, EventId};
use crate::app::shared_kernel::staff::StaffKind;
use crate::app::shared_kernel::staff_counts::{
    ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
};
use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::roster::{AcquisitionMode, PlayerId, SkillId};
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::domain_event::TeamCreationDomainEvent;
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};
use crate::lib::services::event_bus::event_bus::EventBus;
use crate::app::shared_kernel::common_types::EntityId;

pub struct SkillAssignment {
    pub player_id: PlayerId,
    pub skill_id:  SkillId,
    pub mode:      AcquisitionMode,
    pub spp_cost:  u8,
}

pub struct BatchFinalizeCommand {
    pub team_id:     EntityId,
    pub space_id:    String,
    pub coach_name:  String,
    pub assignments: Vec<SkillAssignment>,
}

pub enum BatchFinalizeError {
    TeamNotFound,
    Domain(Vec<DomainError>),
    Repository(RepositoryError),
}

fn count_staff(team: &RosterSelectedTeam, kind: StaffKind) -> u8 {
    team.hired_staff().iter().filter(|s| s.kind == kind).count() as u8
}

pub async fn execute(
    cmd: BatchFinalizeCommand,
    repo: &dyn ITeamRosterRepository,
    bus: &EventBus,
) -> Result<(), BatchFinalizeError> {
    let mut team = repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(BatchFinalizeError::Repository)?
        .ok_or(BatchFinalizeError::TeamNotFound)?;

    let mut errors = Vec::new();
    for a in &cmd.assignments {
        if let Err(e) = team.spend_spp(&a.player_id, a.skill_id.clone(), a.mode, a.spp_cost) {
            errors.push(e);
        }
    }
    if !errors.is_empty() {
        return Err(BatchFinalizeError::Domain(errors));
    }

    team.validate_for_submission()
        .map_err(BatchFinalizeError::Domain)?;

    repo.save(&team, &cmd.space_id)
        .await
        .map_err(BatchFinalizeError::Repository)?;

    repo.mark_submitted(&team.get_id())
        .await
        .map_err(BatchFinalizeError::Repository)?;

    let base      = team.base_infos();
    let rerolls   = RerollCount::new(team.reroll_count()).unwrap_or_default();
    let apos      = ApothecaryCount::new(count_staff(&team, StaffKind::Apothecary)).unwrap_or_default();
    let assistants = AssistantCount::new(count_staff(&team, StaffKind::CoachAssistant)).unwrap_or_default();
    let cheers    = CheerleaderCount::new(count_staff(&team, StaffKind::Cheerleaders)).unwrap_or_default();
    let fans      = count_staff(&team, StaffKind::FansFactor);

    let event = TeamCreationDomainEvent::TeamSubmitted {
        event_id:     EventId::new(),
        team_id:      team.get_id().to_string(),
        space_id:     cmd.space_id,
        team_name:    base.name().clone().into_inner(),
        roster_id:    team.roster.id.0.clone(),
        roster_name:  team.roster.name.0.clone(),
        coach_id:     base.coach_id().to_string(),
        coach_name:   cmd.coach_name,
        treasury:     team.remaining_budget().unwrap_or(0),
        rerolls,
        apothecaries: apos,
        assistants,
        cheerleaders: cheers,
        fans_factor:  fans,
    };
    let _ = bus.send(event.to_enveloppe());

    Ok(())
}

pub fn domain_error_message(e: &DomainError) -> &'static str {
    match e {
        DomainError::InsufficientSpp        => "Budget SPP insuffisant pour cette attribution.",
        DomainError::SkillAlreadyAcquired   => "Cette compétence est déjà acquise.",
        DomainError::PlayerNotFoundInTeam   => "Joueur introuvable dans l'équipe.",
        DomainError::InsufficientPlayerCount => "Au moins 11 joueurs sont requis.",
        DomainError::LeagueNotSelected      => "Sélectionnez une ligue avant de soumettre.",
        _                                   => "Action impossible.",
    }
}

use crate::app::shared_kernel::app_events::team_creation_app_events::{
    AcquiredSkillPayload, PlayerPayload,
};
use crate::app::shared_kernel::common_types::{Entity, EventId};
use crate::app::shared_kernel::staff::StaffKind;
use crate::app::shared_kernel::staff_counts::{
    ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
};
use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::roster::{AcquisitionMode, PlayerId, SkillId};
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::domain_event::TeamCreationDomainEvent;
use crate::app::team_creation::ports::{IReferenceDataPort, ITeamRosterRepository, RepositoryError};
use crate::common::services::event_bus::event_bus::EventBus;
use crate::app::shared_kernel::common_types::EntityId;

pub struct SkillAssignment {
    pub player_id: PlayerId,
    pub skill_id:  SkillId,
    pub mode:      AcquisitionMode,
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
    ref_data: &dyn IReferenceDataPort,
    bus: &EventBus,
) -> Result<(), BatchFinalizeError> {
    let mut team = repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(BatchFinalizeError::Repository)?
        .ok_or(BatchFinalizeError::TeamNotFound)?;

    let mut errors = Vec::new();
    for a in &cmd.assignments {
        let roster_line_id = team
            .hired_players()
            .iter()
            .find(|p| p.instance_id == a.player_id)
            .map(|p| p.definition.id.0.clone())
            .unwrap_or_default();

        let mode_str = match a.mode {
            AcquisitionMode::Chosen => "chosen",
            AcquisitionMode::Random => "random",
        };

        let spp_cost = ref_data
            .resolve_skill_cost(&roster_line_id, &a.skill_id.0, mode_str)
            .map(|c| c.spp_cost)
            .unwrap_or(0);

        if let Err(e) = team.spend_spp(&a.player_id, a.skill_id.clone(), a.mode, spp_cost) {
            errors.push(e);
        }
    }
    if !errors.is_empty() {
        return Err(BatchFinalizeError::Domain(errors));
    }

    team.assign_missing_jerseys();

    team.validate_for_submission()
        .map_err(BatchFinalizeError::Domain)?;

    repo.save(&team, &cmd.space_id)
        .await
        .map_err(BatchFinalizeError::Repository)?;

    repo.mark_submitted(&team.get_id())
        .await
        .map_err(BatchFinalizeError::Repository)?;

    let base       = team.base_infos();
    let rerolls    = RerollCount::new(team.reroll_count()).unwrap_or_default();
    let apos       = ApothecaryCount::new(count_staff(&team, StaffKind::Apothecary)).unwrap_or_default();
    let assistants = AssistantCount::new(count_staff(&team, StaffKind::CoachAssistant)).unwrap_or_default();
    let cheers     = CheerleaderCount::new(count_staff(&team, StaffKind::Cheerleaders)).unwrap_or_default();
    let fans       = count_staff(&team, StaffKind::FansFactor);

    let players: Vec<PlayerPayload> = team.hired_players().iter().map(|p| PlayerPayload {
        instance_id:     p.instance_id.0.clone(),
        roster_line_id:  p.definition.id.0.clone(),
        position_name:   p.definition.name.0.clone(),
        personal_name:   p.personal_name.clone(),
        jersey:          p.jersey.map(|j| j.0),
        acquired_skills: p.acquired_skills.iter().map(|a| AcquiredSkillPayload {
            skill_id: a.skill_id.0.clone(),
            mode:     match a.mode {
                AcquisitionMode::Chosen => "Chosen".to_string(),
                AcquisitionMode::Random => "Random".to_string(),
            },
            spp_cost: a.spp_cost,
        }).collect(),
    }).collect();

    let event = TeamCreationDomainEvent::TeamSubmitted {
        event_id:     EventId::new(),
        team_id:      team.get_id().to_string(),
        space_id:     cmd.space_id,
        team_name:    base.name().clone().into_inner(),
        roster_id:    team.roster.id.0.clone(),
        roster_name:  team.roster.name.0.clone(),
        coach_id:     base.coach_id().to_string(),
        coach_name:   cmd.coach_name,
        logo_url:     base.logo_url().map(|img| img.as_ref().to_string()),
        treasury:     team.remaining_budget().unwrap_or(0),
        rerolls,
        apothecaries: apos,
        assistants,
        cheerleaders: cheers,
        fans_factor:  fans,
        players,
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

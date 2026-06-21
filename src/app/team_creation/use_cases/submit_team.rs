use crate::app::shared_kernel::app_events::team_creation_app_events::{
    AcquiredSkillPayload, PlayerPayload,
};
use crate::app::shared_kernel::common_types::Entity;
use crate::app::shared_kernel::common_types::EventId;
use crate::app::shared_kernel::staff::StaffKind;
use crate::app::shared_kernel::staff_counts::{
    ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
};
use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::roster::AcquisitionMode;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::domain_event::TeamCreationDomainEvent;
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};
use crate::app::team_creation::use_cases::commands::SubmitTeamCommand;
use crate::common::services::event_bus::event_bus::EventBus;

pub enum SubmitTeamError {
    TeamNotFound,
    Domain(Vec<DomainError>),
    Repository(RepositoryError),
}

fn count_staff(team: &RosterSelectedTeam, kind: StaffKind) -> u8 {
    team.hired_staff().iter().filter(|s| s.kind == kind).count() as u8
}

pub async fn execute(
    cmd: SubmitTeamCommand,
    team_repo: &dyn ITeamRosterRepository,
    bus: &EventBus,
) -> Result<(), SubmitTeamError> {
    let mut team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(SubmitTeamError::Repository)?
        .ok_or(SubmitTeamError::TeamNotFound)?;

    team.validate_for_submission()
        .map_err(SubmitTeamError::Domain)?;

    team.assign_missing_jerseys();

    team_repo
        .mark_submitted(&team.get_id())
        .await
        .map_err(SubmitTeamError::Repository)?;

    let base         = team.base_infos();
    let rerolls      = RerollCount::new(team.reroll_count()).unwrap_or_default();
    let apothecaries = ApothecaryCount::new(count_staff(&team, StaffKind::Apothecary)).unwrap_or_default();
    let assistants   = AssistantCount::new(count_staff(&team, StaffKind::CoachAssistant)).unwrap_or_default();
    let cheerleaders = CheerleaderCount::new(count_staff(&team, StaffKind::Cheerleaders)).unwrap_or_default();
    let fans_factor  = count_staff(&team, StaffKind::FansFactor);

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
        event_id:       EventId::new(),
        team_id:        team.get_id().to_string(),
        space_id:       cmd.space_id,
        competition_id: cmd.competition_id,
        season_id:      cmd.season_id,
        team_name:      base.name().clone().into_inner(),
        roster_id:    team.roster.id.0.clone(),
        roster_name:  team.roster.name.0.clone(),
        coach_id:     base.coach_id().to_string(),
        coach_name:   cmd.coach_name,
        logo_url:     base.logo_url().map(|img| img.as_ref().to_string()),
        treasury:     team.remaining_budget().unwrap_or(0),
        rerolls,
        apothecaries,
        assistants,
        cheerleaders,
        fans_factor,
        players,
    };
    let _ = bus.send(event.to_enveloppe());

    Ok(())
}

pub fn domain_error_message(e: &DomainError) -> &'static str {
    match e {
        DomainError::InsufficientPlayerCount => {
            "Vous devez engager au moins 11 joueurs pour soumettre votre équipe."
        }
        DomainError::LeagueNotSelected => {
            "Veuillez sélectionner une ligue avant de soumettre."
        }
        _ => "Action impossible.",
    }
}

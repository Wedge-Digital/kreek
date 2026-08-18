use crate::app::players::domain::error::DomainError;
use crate::app::players::domain::value_objects::SkillName;
use crate::app::players::ports::{IPlayerRepository, ISkillCatalogPort, RepositoryError};
use crate::app::players::use_cases::commands::PurchaseSkillCommand;
use crate::app::players::use_cases::improvement_cost_service::{
    resolve_skill_cost, ImprovementCostError,
};
use crate::common::services::event_bus::event_bus::EventBus;

pub enum PurchaseSkillError {
    PlayerNotFound,
    Cost(ImprovementCostError),
    Domain(DomainError),
    Repository(RepositoryError),
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: PurchaseSkillCommand,
    player_repo: &dyn IPlayerRepository,
    catalog: &dyn ISkillCatalogPort,
    event_bus: &EventBus,
) -> Result<(), PurchaseSkillError> {
    let player = player_repo
        .find_by_id(&cmd.player_id)
        .await
        .map_err(PurchaseSkillError::Repository)?
        .ok_or(PurchaseSkillError::PlayerNotFound)?;

    let skill = catalog
        .find_skill(cmd.skill_id.as_ref())
        .ok_or(PurchaseSkillError::Cost(
            ImprovementCostError::SkillNotFound,
        ))?;

    let level = player.next_improvement_level();
    let (cost, value_delta) = resolve_skill_cost(
        catalog,
        player.roster_line_id.as_ref(),
        cmd.skill_id.as_ref(),
        cmd.mode,
        level,
    )
    .map_err(PurchaseSkillError::Cost)?;

    let category_css = category_css(&skill.category);
    let skill_name = SkillName::try_new(skill.name).expect("nom déjà validé côté référentiel");
    let event = player
        .purchase_skill(
            cmd.skill_id,
            skill_name,
            category_css.to_string(),
            cmd.mode,
            cost,
            value_delta,
        )
        .map_err(PurchaseSkillError::Domain)?;

    player_repo
        .append(&player.id, &player.team_id, &event, player.version + 1)
        .await
        .map_err(PurchaseSkillError::Repository)?;

    let _ = event_bus.send(event.to_enveloppe(&player.id.0));

    Ok(())
}

/// Même mapping catégorie → classe CSS que `player_table.rs`/`team_created_listener.rs`.
fn category_css(category: &str) -> &'static str {
    match category {
        "GENERAL" => "type-general",
        "STRENGTH" => "type-strength",
        "AGILITY" => "type-agility",
        "PASSING" => "type-passing",
        "MUTATION" => "type-mutation",
        _ => "type-general",
    }
}

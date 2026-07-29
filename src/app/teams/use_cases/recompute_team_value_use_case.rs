use crate::app::teams::domain::team::TeamDomainEvent;
use crate::app::teams::ports::{
    IJourneymanTypePort, IPlayerValuePort, IRosterCatalogPort, ITeamRepository,
};
use crate::app::teams::use_cases::team_value_service::resolve_team_value;

#[derive(Debug)]
pub enum RecomputeTeamValueError {
    TeamNotFound,
    Repository(String),
}

/// Recalcule la valeur d'une équipe et l'appende en valeur absolue.
///
/// L'événement est appendu **même si la valeur n'a pas changé** : la suite des
/// `TeamValueRecomputed` est l'historique de progression de la TV sur la saison,
/// information qu'aucune autre trace ne porte.
pub async fn execute(
    team_id: &str,
    repo: &dyn ITeamRepository,
    player_value_port: &dyn IPlayerValuePort,
    roster_catalog_port: &dyn IRosterCatalogPort,
    journeyman_type_port: &dyn IJourneymanTypePort,
) -> Result<(), RecomputeTeamValueError> {
    let team = repo
        .find_by_id(team_id)
        .await
        .map_err(|e| RecomputeTeamValueError::Repository(e.to_string()))?
        .ok_or(RecomputeTeamValueError::TeamNotFound)?;

    let value = resolve_team_value(
        &team,
        player_value_port,
        roster_catalog_port,
        journeyman_type_port,
    )
    .await;

    let event = TeamDomainEvent::TeamValueRecomputed { value };
    repo.append(team_id, &event, team.version)
        .await
        .map(|_| ())
        .map_err(|e| RecomputeTeamValueError::Repository(e.to_string()))
}

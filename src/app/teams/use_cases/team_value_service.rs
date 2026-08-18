use crate::app::teams::domain::team::Team;
use crate::app::teams::domain::team_value::{compute_team_value, TeamValueInputs, ValuedPlayer};
use crate::app::teams::domain::value_objects::Kpo;
use crate::app::teams::ports::{IJourneymanTypePort, IRosterCatalogPort, ISquadPort};

/// Recalcule la valeur d'une équipe à partir de son effectif réel et des prix
/// du corpus de référence.
///
/// C'est ici que s'arrête la connaissance des ports : le domaine reçoit des
/// `TeamValueInputs`, jamais un DTO. Aucun handler, aucun template ne voit
/// `SquadMemberDto` ni `RosterCatalogDto`.
// arch:no-instrument — service : recalcule une valeur, appelé par le use case qui la persiste
pub async fn resolve_team_value(
    team: &Team,
    squad_port: &dyn ISquadPort,
    roster_catalog_port: &dyn IRosterCatalogPort,
    journeyman_type_port: &dyn IJourneymanTypePort,
) -> Kpo {
    let roster_id = team.roster_id.0.as_str();
    let players = load_players(team, squad_port).await;
    let journeyman = journeyman_type_port.journeyman_type_for_roster(roster_id);
    let roster = roster_catalog_port.find_catalog(roster_id);

    let inputs = build_inputs(team, players, &journeyman, roster.as_ref());
    compute_team_value(&inputs)
}

async fn load_players(team: &Team, port: &dyn ISquadPort) -> Vec<ValuedPlayer> {
    port.find_squad(&team.id.to_string())
        .await
        .into_iter()
        .map(|p| ValuedPlayer {
            value_kpo: Kpo(p.value_kpo),
            available_for_next_match: p.available_for_next_match,
        })
        .collect()
}

/// Un roster introuvable dans le corpus donne des prix nuls plutôt qu'une
/// panique : la TV sera incomplète, l'application reste debout.
fn build_inputs(
    team: &Team,
    players: Vec<ValuedPlayer>,
    journeyman: &crate::app::teams::ports::JourneymanTypeDto,
    roster: Option<&crate::app::teams::ports::RosterCatalogDto>,
) -> TeamValueInputs {
    TeamValueInputs {
        players,
        rerolls: team.rerolls,
        reroll_price: Kpo(roster.map(|r| r.reroll_base_cost).unwrap_or(0)),
        apothecaries: team.apothecaries,
        apothecary_price: Kpo(roster.map(|r| r.staff_price("APOTHECARY")).unwrap_or(0)),
        assistants: team.assistants,
        assistant_price: Kpo(roster
            .map(|r| r.staff_price("COACH_ASSISTANTS"))
            .unwrap_or(0)),
        cheerleaders: team.cheerleaders,
        cheerleader_price: Kpo(roster.map(|r| r.staff_price("CHEERLEADERS")).unwrap_or(0)),
        journeyman_price: Kpo(journeyman.price_kpo),
    }
}

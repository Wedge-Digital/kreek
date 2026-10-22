//! Le prochain adversaire d'une équipe, pour la page « Imprimer » (carte 599).
//!
//! Lecture seule : l'inscription de l'équipe, les matchs de sa saison, et la
//! règle du domaine (`next_opponent`, R2 et R12). Ne pas avoir de prochain
//! match **n'est pas une erreur** : c'est une réponse, que la page affiche.

use crate::app::competitions::domain::match_day::{MatchDayPosition, Pairing};
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, PairingDisplayDto,
};
use crate::app::competitions::domain::next_opponent::{
    next_opponent, NextOpponent, PairingStatus, ScheduledPairing,
};
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;

#[derive(Debug)]
pub enum NextOpponentError {
    Port(String),
}

impl std::fmt::Display for NextOpponentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Port(cause) => write!(f, "port : {cause}"),
        }
    }
}

/// Ce que la page affiche : l'adversaire, et la rencontre dont il vient — elle
/// porte déjà les noms et la journée, dénormalisés par la projection.
pub enum NextOpponentLookup {
    Found {
        opponent: NextOpponent,
        pairing: Box<PairingDisplayDto>,
    },
    NotEnrolled,
    NoUpcomingMatch,
}

// arch:no-instrument — service de lecture : cherche le prochain adversaire, sans intention métier
pub async fn find(
    team_info: &dyn ITeamInfoPort,
    match_days: &dyn IMatchDayRepository,
    team_id: &str,
) -> Result<NextOpponentLookup, NextOpponentError> {
    let Some(inscription) = team_info
        .find_team_enrollment(team_id)
        .await
        .map_err(NextOpponentError::Port)?
    else {
        return Ok(NextOpponentLookup::NotEnrolled);
    };
    let rows = match_days
        .list_team_matches(&inscription.season_id, team_id)
        .await
        .map_err(|e| NextOpponentError::Port(e.to_string()))?;
    Ok(lookup(team_id, rows))
}

fn lookup(team_id: &str, rows: Vec<PairingDisplayDto>) -> NextOpponentLookup {
    let Ok(team) = TeamId::try_new(team_id) else {
        return NextOpponentLookup::NoUpcomingMatch;
    };
    let scheduled: Vec<ScheduledPairing> = rows.iter().filter_map(scheduled).collect();
    let Some(opponent) = next_opponent(&team, &scheduled) else {
        return NextOpponentLookup::NoUpcomingMatch;
    };
    match rows
        .into_iter()
        .find(|r| r.round_id == opponent.round_id.to_string())
    {
        Some(pairing) => NextOpponentLookup::Found {
            opponent,
            pairing: Box::new(pairing),
        },
        None => NextOpponentLookup::NoUpcomingMatch,
    }
}

/// Une ligne de projection illisible est ignorée : elle ne peut pas désigner
/// un adversaire qu'on saurait imprimer.
fn scheduled(row: &PairingDisplayDto) -> Option<ScheduledPairing> {
    Some(ScheduledPairing {
        round_id: MatchId::try_new(&row.round_id).ok()?,
        round_position: MatchDayPosition::try_new(row.round_position).ok()?,
        pairing: Pairing {
            id: PairingId::try_new(&row.pairing_id).ok()?,
            home_team_id: TeamId::try_new(&row.home_team_id).ok()?,
            away_team_id: TeamId::try_new(&row.away_team_id).ok()?,
        },
        status: PairingStatus::from_projection(&row.match_status),
    })
}

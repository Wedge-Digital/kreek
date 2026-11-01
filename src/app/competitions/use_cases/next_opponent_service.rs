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
    upcoming_opponents, NextOpponent, PairingStatus, ScheduledPairing,
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

/// Un match à venir : l'adversaire, et la rencontre dont il vient — elle
/// porte déjà les noms et la journée, dénormalisés par la projection.
pub struct UpcomingMatch {
    pub opponent: NextOpponent,
    pub pairing: PairingDisplayDto,
}

/// Ce que la page affiche.
pub enum NextOpponentLookup {
    /// Les matchs à venir dans l'ordre de R12, **jamais vide** : le premier est
    /// le prochain adversaire, les suivants se choisissent à la main.
    Found {
        upcoming: Vec<UpcomingMatch>,
    },
    NotEnrolled,
    NoUpcomingMatch,
    /// Une rencontre de la saison est illisible — un statut inconnu, un
    /// identifiant invalide. On ne devine pas l'adversaire : sauter la ligne
    /// ferait imprimer celui du match suivant.
    Unreadable,
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
    let Some(scheduled) = rows.iter().map(readable).collect::<Option<Vec<_>>>() else {
        return NextOpponentLookup::Unreadable;
    };
    let mut rows = rows;
    let upcoming: Vec<UpcomingMatch> = upcoming_opponents(&team, &scheduled)
        .into_iter()
        .filter_map(|opponent| with_pairing(opponent, &mut rows))
        .collect();
    if upcoming.is_empty() {
        NextOpponentLookup::NoUpcomingMatch
    } else {
        NextOpponentLookup::Found { upcoming }
    }
}

/// La rencontre de la projection qui porte ce match — une par journée pour
/// une équipe.
fn with_pairing(
    opponent: NextOpponent,
    rows: &mut Vec<PairingDisplayDto>,
) -> Option<UpcomingMatch> {
    let round_id = opponent.round_id.to_string();
    let index = rows.iter().position(|r| r.round_id == round_id)?;
    Some(UpcomingMatch {
        opponent,
        pairing: rows.swap_remove(index),
    })
}

/// La rencontre lue, ou `None` journalisé : une ligne illisible ne disparaît
/// pas en silence.
fn readable(row: &PairingDisplayDto) -> Option<ScheduledPairing> {
    let read = scheduled(row);
    if read.is_none() {
        tracing::error!(
            pairing_id = %row.pairing_id,
            match_status = %row.match_status,
            "rencontre illisible : le prochain adversaire n'est pas déterminé"
        );
    }
    read
}

fn scheduled(row: &PairingDisplayDto) -> Option<ScheduledPairing> {
    Some(ScheduledPairing {
        round_id: MatchId::try_new(&row.round_id).ok()?,
        round_position: MatchDayPosition::try_new(row.round_position).ok()?,
        pairing: Pairing {
            id: PairingId::try_new(&row.pairing_id).ok()?,
            home_team_id: TeamId::try_new(&row.home_team_id).ok()?,
            away_team_id: TeamId::try_new(&row.away_team_id).ok()?,
        },
        status: PairingStatus::from_projection(&row.match_status)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ligne(home: &TeamId, away: &TeamId, position: i32, status: &str) -> PairingDisplayDto {
        PairingDisplayDto {
            pairing_id: PairingId::new().to_string(),
            round_id: MatchId::new().to_string(),
            round_name: format!("Journée {position}"),
            round_position: position,
            round_date_start: None,
            round_date_end: None,
            round_day_type: "fixed_date".into(),
            home_team_id: home.to_string(),
            home_team_name: "A".into(),
            home_roster_name: "R".into(),
            home_coach_name: String::new(),
            home_logo_url: None,
            home_initials: "A".into(),
            away_team_id: away.to_string(),
            away_team_name: "B".into(),
            away_roster_name: "R".into(),
            away_coach_name: String::new(),
            away_logo_url: None,
            away_initials: "B".into(),
            match_status: status.into(),
            home_score: None,
            away_score: None,
            home_casualties: None,
            away_casualties: None,
            match_report_url: None,
        }
    }

    #[test]
    fn le_match_joue_est_ecarte_et_le_suivant_retenu() {
        let (moi, a, b) = (TeamId::new(), TeamId::new(), TeamId::new());
        let rows = vec![
            ligne(&moi, &a, 1, "completed"),
            ligne(&b, &moi, 2, "upcoming"),
        ];
        match lookup(&moi.to_string(), rows) {
            NextOpponentLookup::Found { upcoming } => {
                assert_eq!(upcoming.len(), 1);
                assert_eq!(upcoming[0].opponent.team_id, b);
                assert_eq!(upcoming[0].pairing.round_name, "Journée 2");
            }
            _ => panic!("l'adversaire de la journée 2"),
        }
    }

    /// Chaque match à venir garde sa rencontre : le coach choisit la journée.
    #[test]
    fn chaque_match_a_venir_porte_sa_journee() {
        let (moi, a, b) = (TeamId::new(), TeamId::new(), TeamId::new());
        let rows = vec![
            ligne(&moi, &a, 4, "upcoming"),
            ligne(&b, &moi, 2, "upcoming"),
        ];
        let NextOpponentLookup::Found { upcoming } = lookup(&moi.to_string(), rows) else {
            panic!("deux matchs à venir");
        };
        let journees: Vec<&str> = upcoming
            .iter()
            .map(|m| m.pairing.round_name.as_str())
            .collect();
        assert_eq!(journees, vec!["Journée 2", "Journée 4"]);
    }

    /// Un statut inconnu n'est pas un match joué : la recherche le dit, au
    /// lieu d'imprimer l'adversaire du match d'après.
    #[test]
    fn une_rencontre_illisible_rend_la_recherche_illisible() {
        let (moi, a, b) = (TeamId::new(), TeamId::new(), TeamId::new());
        let rows = vec![
            ligne(&moi, &a, 1, "postponed"),
            ligne(&b, &moi, 2, "upcoming"),
        ];
        assert!(matches!(
            lookup(&moi.to_string(), rows),
            NextOpponentLookup::Unreadable
        ));
    }
}

//! L'option « prochain adversaire » de la page « Imprimer » (carte 599).
//!
//! **C'est `competitions` qui la sert** : il possède les journées et leurs
//! rencontres. La page de `teams` la compose par un `hx-get`, sans rien savoir
//! du calendrier. La valeur du bouton radio porte l'équipe adverse ; la page
//! la met dans le **chemin** de l'aperçu, où `space_scope` la contrôle.
//!
//! Comme pour les matchs d'une équipe, seul le `team_id` est dans l'URL : la
//! saison se résout depuis l'inscription.

use crate::app::competitions::domain::match_day_repository_port::PairingDisplayDto;
use crate::app::competitions::domain::next_opponent::{NextOpponent, Venue};
use crate::app::competitions::use_cases::next_opponent_service::{
    self, NextOpponentLookup, UpcomingMatch,
};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// Un match à venir, tel que l'option le propose.
pub struct UpcomingMatchVm {
    pub other_team_id: String,
    pub team_name: String,
    pub roster_name: String,
    pub round_id: String,
    pub round_label: String,
    pub opponent_is_home: bool,
}

impl UpcomingMatchVm {
    /// « Journée 6, 25/10/2026 : Les Tailleurs (Granitiers) »
    pub fn choice_label(&self) -> String {
        format!(
            "{} : {} ({})",
            self.round_label, self.team_name, self.roster_name
        )
    }
}

pub enum NextOpponentOptionVm {
    /// `next` est le prochain match (R12) ; `choices` les liste tous, lui
    /// compris, pour en choisir un autre — vide quand il n'y en a qu'un.
    Available {
        next: UpcomingMatchVm,
        choices: Vec<UpcomingMatchVm>,
    },
    Unavailable {
        reason: &'static str,
    },
}

impl NextOpponentOptionVm {
    pub fn from_lookup(lookup: NextOpponentLookup) -> Self {
        match lookup {
            NextOpponentLookup::Found { upcoming } => Self::available(upcoming),
            NextOpponentLookup::NotEnrolled => Self::Unavailable {
                reason: "L'équipe n'est inscrite dans aucune compétition.",
            },
            NextOpponentLookup::NoUpcomingMatch => Self::Unavailable {
                reason: "Aucun match à venir pour mon équipe dans cette compétition.",
            },
            NextOpponentLookup::Unreadable => Self::Unavailable {
                reason: "Calendrier illisible : l'adversaire ne peut pas être déterminé.",
            },
        }
    }

    /// `upcoming` n'est jamais vide (`NextOpponentLookup::Found`).
    fn available(upcoming: Vec<UpcomingMatch>) -> Self {
        let mut choices: Vec<UpcomingMatchVm> = upcoming.iter().map(upcoming_match).collect();
        let next = upcoming_match(&upcoming[0]);
        if choices.len() < 2 {
            choices.clear();
        }
        Self::Available { next, choices }
    }
}

fn upcoming_match(m: &UpcomingMatch) -> UpcomingMatchVm {
    let (opponent, pairing): (&NextOpponent, &PairingDisplayDto) = (&m.opponent, &m.pairing);
    let opponent_is_home = opponent.opponent_venue == Venue::Home;
    let (team_name, roster_name) = if opponent_is_home {
        (&pairing.home_team_name, &pairing.home_roster_name)
    } else {
        (&pairing.away_team_name, &pairing.away_roster_name)
    };
    UpcomingMatchVm {
        other_team_id: opponent.team_id.to_string(),
        team_name: team_name.clone(),
        roster_name: roster_name.clone(),
        round_id: opponent.round_id.to_string(),
        round_label: round_label(pairing),
        opponent_is_home,
    }
}

/// « Journée 6, 25/10/2026 » — la date quand la journée en a une.
fn round_label(pairing: &PairingDisplayDto) -> String {
    match pairing.round_date_start.as_deref().and_then(date_fr) {
        Some(date) => format!("{}, {date}", pairing.round_name),
        None => pairing.round_name.clone(),
    }
}

/// `2026-10-25` (ou un horodatage qui commence ainsi) → `25/10/2026`.
fn date_fr(iso: &str) -> Option<String> {
    let jour = iso.get(..10)?;
    let mut parts = jour.split('-');
    let (a, m, j) = (parts.next()?, parts.next()?, parts.next()?);
    Some(format!("{j}/{m}/{a}"))
}

#[derive(Template)]
#[template(path = "widgets/competitions-print-next-opponent.html")]
pub struct NextOpponentOptionTemplate {
    pub vm: NextOpponentOptionVm,
}

impl IntoResponse for NextOpponentOptionTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("print next opponent render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn get_print_next_opponent(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let ctx = &state.competitions;
    match next_opponent_service::find(
        ctx.team_info_port.as_ref(),
        ctx.match_day_repository.as_ref(),
        &team_id,
    )
    .await
    {
        Ok(lookup) => NextOpponentOptionTemplate {
            vm: NextOpponentOptionVm::from_lookup(lookup),
        }
        .into_response(),
        Err(e) => {
            tracing::error!("print next opponent {team_id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_date_iso_devient_francaise() {
        assert_eq!(date_fr("2026-10-25"), Some("25/10/2026".to_string()));
        assert_eq!(
            date_fr("2026-10-25T00:00:00Z"),
            Some("25/10/2026".to_string())
        );
        assert_eq!(date_fr("demain"), None);
    }

    #[test]
    fn sans_match_l_option_dit_pourquoi() {
        let vm = NextOpponentOptionVm::from_lookup(NextOpponentLookup::NoUpcomingMatch);
        assert!(
            matches!(vm, NextOpponentOptionVm::Unavailable { reason } if reason.contains("Aucun match"))
        );
    }

    #[test]
    fn un_calendrier_illisible_grise_l_option() {
        let vm = NextOpponentOptionVm::from_lookup(NextOpponentLookup::Unreadable);
        assert!(
            matches!(vm, NextOpponentOptionVm::Unavailable { reason } if reason.contains("illisible"))
        );
    }
}

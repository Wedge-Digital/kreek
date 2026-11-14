//! Le renvoi d'un coach arrivé au match avant le coup d'envoi, en « Chacun son
//! équipe » (carte 656, § 12.3 de E22) : `/live`, `step3` et `step4` le
//! ramènent à son avant-match ou à ses coups de pouce.
//!
//! La question « le match a-t-il commencé ? » est celle du domaine
//! (`has_kicked_off`), posée par `kick_off_service` ; ce module ne fait que la
//! traduire en redirection.

use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::value_objects::TeamSide;
use crate::app::match_report::use_cases::kick_off_service::{
    kick_off_detour, KickOffDetour, KickOffPorts,
};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use axum::response::{IntoResponse, Redirect, Response};

/// La redirection à faire avant le coup d'envoi pour le camp `side`, `None`
/// une fois le match commencé — et toujours en « Je saisis tout ».
pub async fn kick_off_redirect(
    state: &AppState,
    (space_id, mr_id): (&str, &str),
    pm: &MatchReportPreMatch,
    side: TeamSide,
) -> Option<Response> {
    let ports = KickOffPorts::from_context(&state.match_report);
    let detour = kick_off_detour(pm, side, &ports).await?;
    Some(Redirect::to(&detour_url(space_id, mr_id, &detour)).into_response())
}

fn detour_url(space_id: &str, mr_id: &str, detour: &KickOffDetour) -> String {
    let routes = AppRoutes::default().match_report;
    match detour {
        // `/step2` mène chaque coach à l'avant-match de son camp (carte 654).
        KickOffDetour::PreMatch => routes.step2(space_id, mr_id),
        KickOffDetour::Inducements { team_id } => routes.inducements(space_id, mr_id, team_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avant_son_jet_le_coach_retourne_a_l_avant_match() {
        let url = detour_url("espace", "rapport", &KickOffDetour::PreMatch);
        assert!(url.ends_with("/rapport/step2"), "{url}");
    }

    #[test]
    fn apres_son_jet_le_coach_va_a_ses_coups_de_pouce() {
        let detour = KickOffDetour::Inducements {
            team_id: "equipe".to_string(),
        };
        let url = detour_url("espace", "rapport", &detour);
        assert!(url.contains("equipe"), "{url}");
    }
}

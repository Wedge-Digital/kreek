//! Le tirage au sort du MVP (carte 638) : `POST /{id}/mvp/{side}/draw`.
//!
//! Six couples `player_id`/`player_type`. La règle — six désignés, distincts,
//! un MVP par camp — est celle du domaine. Le générateur est créé ici,
//! `StdRng::from_os_rng()`, comme pour le tirage du calendrier : le use case
//! le reçoit, et les tests lui passent une graine fixe.
//!
//! Le résultat se montre sur la page même du camp : on y lit le tiré et ses
//! désignés avant de passer à la suite.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::domain::value_objects::ActionPlayer;
use crate::app::match_report::io::web::mvp_step_controller::{
    error_response, parse_side, side_slug,
};
use crate::app::match_report::io::web::record_action_controller::build_player;
use crate::app::match_report::use_cases::draw_mvp_use_case::{self, DrawMvpCommand};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::Form;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::Deserialize;

/// Les désignés, un couple par joueur, dans l'ordre de leur désignation.
#[derive(Deserialize)]
pub struct DrawMvpForm {
    #[serde(default)]
    pub player_id: Vec<String>,
    #[serde(default)]
    pub player_type: Vec<String>,
}

pub async fn post_mvp_draw(
    auth_session: AuthSession,
    Path((space_id, mr_id, side)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Form(form): Form<DrawMvpForm>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(cmd) = build_command(&mr_id, &side, &form, user.id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let page_url =
        AppRoutes::default()
            .match_report
            .mvp(&space_id, &mr_id, side_slug(cmd.team_side));
    let mut rng = StdRng::from_os_rng();
    let (repo, players) = (
        &state.match_report.match_report_repo,
        &state.match_report.player_data,
    );
    match draw_mvp_use_case::execute(cmd, repo.as_ref(), players.as_ref(), &mut rng).await {
        Ok(()) => (StatusCode::OK, [("HX-Redirect", page_url)]).into_response(),
        Err(e) => error_response(e),
    }
}

fn build_command(
    mr_id: &str,
    side: &str,
    form: &DrawMvpForm,
    recorded_by: CoachId,
) -> Option<DrawMvpCommand> {
    Some(DrawMvpCommand {
        match_report_id: MatchReportId::try_new(mr_id).ok()?,
        team_side: parse_side(side)?,
        designated: build_designated(form)?,
        recorded_by,
    })
}

/// Un couple incomplet, ou un joueur illisible, est une requête mal formée.
/// Leur nombre, lui, est l'affaire du domaine.
fn build_designated(form: &DrawMvpForm) -> Option<Vec<ActionPlayer>> {
    if form.player_id.len() != form.player_type.len() {
        return None;
    }
    form.player_id
        .iter()
        .zip(&form.player_type)
        .map(|(id, kind)| build_player(id, kind))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::PlayerId;

    fn form(ids: Vec<String>, types: Vec<&str>) -> DrawMvpForm {
        DrawMvpForm {
            player_id: ids,
            player_type: types.into_iter().map(str::to_string).collect(),
        }
    }

    #[test]
    fn six_couples_donnent_six_designes() {
        let ids: Vec<String> = (0..6).map(|_| PlayerId::new().to_string()).collect();
        let designated = build_designated(&form(ids, vec!["regular"; 6])).unwrap();
        assert_eq!(designated.len(), 6);
    }

    /// Cinq désignés passent le contrôleur : c'est le domaine qui les refuse.
    #[test]
    fn le_nombre_n_est_pas_l_affaire_du_controleur() {
        let ids: Vec<String> = (0..5).map(|_| PlayerId::new().to_string()).collect();
        assert_eq!(
            build_designated(&form(ids, vec!["regular"; 5])).map(|d| d.len()),
            Some(5)
        );
    }

    #[test]
    fn un_couple_incomplet_est_mal_forme() {
        let ids: Vec<String> = (0..2).map(|_| PlayerId::new().to_string()).collect();
        assert!(build_designated(&form(ids, vec!["regular"])).is_none());
    }

    #[test]
    fn seuls_home_et_away_se_tirent() {
        let f = form(vec![], vec![]);
        assert!(build_command("01HZ0000000000000000000000", "step5", &f, CoachId::new()).is_none());
    }
}

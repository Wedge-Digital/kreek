use crate::app::match_report::io::web::actions_step_controller::{get_step3, get_step4};
use crate::app::match_report::io::web::cancel_inducements_controller::post_cancel_inducements;
use crate::app::match_report::io::web::garde_saisie::{garde_camp, garde_saisie, garde_suivi};
use crate::app::match_report::io::web::inducements_controller::{
    get_inducements, post_inducements,
};
use crate::app::match_report::io::web::live_match_controller::get_live_match;
use crate::app::match_report::io::web::match_selection_controller::{
    create_match_report, edit_match_report, from_pairing, new_match_report, update_match_selection,
};
use crate::app::match_report::io::web::mvp_draw_controller::post_mvp_draw;
use crate::app::match_report::io::web::mvp_step_controller::{get_mvp_step, post_mvp_step};
use crate::app::match_report::io::web::pre_match_controller::{get_pre_match, post_pre_match};
use crate::app::match_report::io::web::recap_controller::{
    get_recap, post_publish, post_unpublish,
};
use crate::app::match_report::io::web::record_action_controller::{
    delete_action, post_action_step3, post_action_step4,
};
use crate::app::match_report::io::web::step5_controller::{get_step5, post_step5};
use crate::app::match_report::io::web::team_pre_match_controller::{
    get_team_pre_match, post_team_pre_match,
};
use crate::app::match_report::io::web::widgets::action_log_widget::get_action_log;
use crate::app::match_report::io::web::widgets::action_panel_widget::{
    get_action_panel_step3, get_action_panel_step4,
};
use crate::app::match_report::io::web::widgets::inducements_wait_widget::get_inducements_wait;
use crate::app::match_report::io::web::widgets::live_score_widget::get_live_score;
use crate::app::match_report::io::web::widgets::mercenary_selector_widget::get_mercenary_selector;
use crate::app::match_report::io::web::widgets::opponent_fan_factor_widget::get_opponent_fan_factor;
use crate::app::match_report::io::web::widgets::print_inducements_widget::{
    get_print_inducements, get_print_inducements_with,
};
use crate::app::match_report::io::web::widgets::print_match_sheet_widget::{
    get_print_match_sheet, get_print_match_sheet_with,
};
use crate::app::match_report::io::web::widgets::temp_player_selector_widget::{
    get_temp_players_step3, get_temp_players_step4,
};
use crate::app::match_report::io::web::widgets::turn_selector_widget::{
    get_turn_selector_step3, get_turn_selector_step4,
};
use crate::app::match_report::routes::path;
use crate::state::AppState;
use axum::extract::Request;
use axum::response::IntoResponse;
use axum::routing::Route;
use axum::{
    middleware::from_fn_with_state,
    routing::{delete, get, post},
    Router,
};
use std::convert::Infallible;
use tower::{Layer, Service};

/// Quatre groupes, et les frontières entre eux sont des décisions (cartes 622,
/// 634 et 652). Le quatrième, `routes_de_camp()`, est décrit avec sa fonction.
///
/// `routes_de_saisie()` porte l'écriture d'un compte rendu et les écrans qui
/// la préparent : `garde_saisie` y est posée **une fois** — admins, et en « Je
/// saisis tout » le démarreur seul — et couvre du même coup ce qu'on ajoutera
/// au groupe.
///
/// `routes_de_suivi()` porte ce que l'autre coach lit sans écrire :
/// `garde_suivi`, admins et coachs des deux équipes.
///
/// `routes_ouvertes()` garde ce qui n'en relève pas : le hors-calendrier, les
/// feuilles imprimables, l'arrivée depuis le calendrier, et le récapitulatif,
/// qui se garde lui-même.
///
/// **`router` prend l'état**, comme celui de `teams` : `from_fn_with_state`
/// exige une valeur à la construction.
pub fn router(state: AppState) -> Router<AppState> {
    let suivi = from_fn_with_state(state.clone(), garde_suivi);
    let saisie = from_fn_with_state(state.clone(), garde_saisie);
    let camp = from_fn_with_state(state, garde_camp);
    routes_ouvertes()
        .merge(routes_de_suivi().route_layer(suivi))
        .merge(routes_de_saisie().route_layer(saisie))
        .merge(garder_si_non_vide(routes_de_camp(), camp))
}

/// `route_layer` sur un routeur sans route **panique** (axum 0.8) : le groupe
/// de camp naît vide, et le serveur ne démarrerait pas.
fn garder_si_non_vide<S, L>(routes: Router<S>, layer: L) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    L: Layer<Route> + Clone + Send + Sync + 'static,
    L::Service: Service<Request> + Clone + Send + Sync + 'static,
    <L::Service as Service<Request>>::Response: IntoResponse + 'static,
    <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
    <L::Service as Service<Request>>::Future: Send + 'static,
{
    if !routes.has_routes() {
        return routes;
    }
    routes.route_layer(layer)
}

/// Les routes dont le chemin désigne un camp — `side`, ou un `team_id` du
/// match —, gardées par `garde_camp` (carte 652, R3 de E22) : en « Chacun son
/// équipe », chaque coach n'y écrit que dans le sien. Né vide à la carte 652,
/// les cartes 654 à 659 y rangent leurs routes, en les retirant de
/// `routes_de_saisie()`. En « Je saisis tout », `can_write_side` y rend la
/// règle de `can_write_report` : rien n'y change.
fn routes_de_camp() -> Router<AppState> {
    Router::new()
        // L'avant-match d'un camp, en « Chacun son équipe » (carte 654).
        .route(
            path::MATCH_REPORT_STEP2_SIDE,
            get(get_team_pre_match).post(post_team_pre_match),
        )
        // Les coups de pouce d'une équipe, ses mercenaires et l'attente de
        // l'autre équipe (carte 655) : chacun n'achète que pour la sienne.
        .route(
            path::MATCH_REPORT_INDUCEMENTS,
            get(get_inducements).post(post_inducements),
        )
        .route(
            path::MATCH_REPORT_MERCENARY_SELECTOR,
            get(get_mercenary_selector),
        )
        .route(
            path::MATCH_REPORT_INDUCEMENTS_WAIT,
            get(get_inducements_wait),
        )
}

fn routes_ouvertes() -> Router<AppState> {
    Router::new()
        // L'impression (épic E16) : lecture seule.
        .route(path::PRINT_INDUCEMENTS, get(get_print_inducements))
        .route(
            path::PRINT_INDUCEMENTS_WITH,
            get(get_print_inducements_with),
        )
        .route(path::PRINT_MATCH_SHEET, get(get_print_match_sheet))
        .route(
            path::PRINT_MATCH_SHEET_WITH,
            get(get_print_match_sheet_with),
        )
        .route(
            path::MATCH_REPORT_NEW,
            get(new_match_report).post(create_match_report),
        )
        .route(path::MATCH_REPORT_FROM_PAIRING, get(from_pairing))
        .route(path::MATCH_REPORT_RECAP, get(get_recap))
        .route(path::MATCH_REPORT_RECAP_PUBLISH, post(post_publish))
        .route(path::MATCH_REPORT_RECAP_UNPUBLISH, post(post_unpublish))
}

/// La lecture d'un compte rendu en cours, ouverte à l'autre coach (carte 634).
fn routes_de_suivi() -> Router<AppState> {
    Router::new()
        // Le match en direct (carte 636) : les gestes d'écriture y suivent le
        // droit d'écrire, la lecture non.
        .route(path::MATCH_REPORT_LIVE, get(get_live_match))
        // Le journal des deux camps (carte 635) : le ✕ y suit le droit
        // d'écrire, la lecture non.
        .route(path::MATCH_REPORT_ACTION_LOG, get(get_action_log))
        // Le score du direct (carte 646) : un widget, rechargé sur le signal.
        .route(path::MATCH_REPORT_LIVE_SCORE, get(get_live_score))
        // Le fan factor de l'adversaire, en « Chacun son équipe » (carte 654) :
        // un widget en lecture seule, rechargé sur le signal.
        .route(
            path::MATCH_REPORT_STEP2_FAN_FACTOR,
            get(get_opponent_fan_factor),
        )
}

fn routes_de_saisie() -> Router<AppState> {
    Router::new()
        .route(
            path::MATCH_REPORT_EDIT,
            get(edit_match_report).post(update_match_selection),
        )
        .route(
            path::MATCH_REPORT_STEP2,
            get(get_pre_match).post(post_pre_match),
        )
        .route(
            path::MATCH_REPORT_CANCEL_INDUCEMENTS,
            post(post_cancel_inducements),
        )
        .route(path::MATCH_REPORT_STEP3, get(get_step3))
        .route(path::MATCH_REPORT_STEP4, get(get_step4))
        // L'étape MVP d'un camp (carte 637).
        .route(
            path::MATCH_REPORT_MVP,
            get(get_mvp_step).post(post_mvp_step),
        )
        // Le tirage au sort du MVP (carte 638).
        .route(path::MATCH_REPORT_MVP_DRAW, post(post_mvp_draw))
        .route(path::MATCH_REPORT_STEP5, get(get_step5).post(post_step5))
        .route(
            path::MATCH_REPORT_STEP3_TURN_SELECTOR,
            get(get_turn_selector_step3),
        )
        .route(
            path::MATCH_REPORT_STEP4_TURN_SELECTOR,
            get(get_turn_selector_step4),
        )
        .route(
            path::MATCH_REPORT_STEP3_TEMP_PLAYERS,
            get(get_temp_players_step3),
        )
        .route(
            path::MATCH_REPORT_STEP4_TEMP_PLAYERS,
            get(get_temp_players_step4),
        )
        .route(
            path::MATCH_REPORT_STEP3_ACTION_PANEL,
            get(get_action_panel_step3),
        )
        .route(
            path::MATCH_REPORT_STEP4_ACTION_PANEL,
            get(get_action_panel_step4),
        )
        .route(path::MATCH_REPORT_STEP3_ACTIONS, post(post_action_step3))
        .route(path::MATCH_REPORT_STEP4_ACTIONS, post(post_action_step4))
        .route(path::MATCH_REPORT_ACTION, delete(delete_action))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::StatusCode;
    use axum::middleware::{from_fn, Next};
    use axum::response::Response;
    use tower::ServiceExt;

    /// Une couche témoin : elle répond à la place de la route.
    async fn temoin(_: Request, _: Next) -> Response {
        StatusCode::IM_A_TEAPOT.into_response()
    }

    /// Le groupe de camp naît vide : le garder ne doit pas paniquer.
    #[test]
    fn an_empty_group_gets_no_route_layer() {
        let routes = garder_si_non_vide(Router::<()>::new(), from_fn(temoin));
        assert!(!routes.has_routes());
    }

    /// Les quatre groupes se fusionnent sans chemin en conflit : axum panique
    /// au démarrage sinon (carte 655, qui a déplacé les coups de pouce).
    #[test]
    fn the_four_groups_merge_without_conflict() {
        let routes = routes_ouvertes()
            .merge(routes_de_suivi())
            .merge(routes_de_saisie())
            .merge(routes_de_camp());
        assert!(routes.has_routes());
    }

    #[tokio::test]
    async fn a_non_empty_group_is_guarded() {
        let routes = Router::<()>::new().route("/camp", get(|| async { "ouvert" }));
        let routes = garder_si_non_vide(routes, from_fn(temoin));
        let request = Request::builder().uri("/camp").body(Body::empty()).unwrap();
        let response = routes.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::IM_A_TEAPOT);
    }
}

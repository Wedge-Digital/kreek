//! Le MVP de l'adversaire, en « Chacun son équipe » (carte 657) :
//! `GET /{id}/mvp/{side}/opponent`, derrière `garde_suivi`.
//!
//! L'étape MVP d'un camp le charge pour l'autre camp, et le recharge sur le
//! signal du match : un MVP saisi ou tiré part comme une action, et réveille
//! la page sans toucher aux désignés du coach.
//!
//! **Un rapport qui n'est plus en cours rend 204**, comme le fan factor de
//! l'adversaire : htmx ne remplace rien.

use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::value_objects::TeamSide;
use crate::app::match_report::io::web::mvp_step_controller::{names_sentence, parse_side};
use crate::app::match_report::io::web::widgets::opponent_fan_factor_widget::load_in_progress;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// Le MVP désigné : « Skritt », et parmi qui s'il a été tiré au sort.
pub struct OpponentMvpVm {
    pub player_name: String,
    pub drawn_among: Option<String>,
}

impl OpponentMvpVm {
    /// `None` tant que le camp n'a pas son MVP.
    pub fn from_domain(pm: &MatchReportPreMatch, side: TeamSide) -> Option<Self> {
        let action = *pm.mvps_of(side).first()?;
        Some(Self {
            player_name: action.player_display_name.clone(),
            drawn_among: pm
                .drawn_mvp(&action.id)
                .map(|d| names_sentence(&d.designated_names())),
        })
    }
}

#[derive(Template)]
#[template(path = "opponent-mvp-widget.html")]
pub struct OpponentMvpTemplate {
    pub team_name: String,
    pub mvp: Option<OpponentMvpVm>,
}

impl IntoResponse for OpponentMvpTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("opponent-mvp-widget.html: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn get_opponent_mvp(
    Path((_space_id, mr_id, side)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Some(side) = parse_side(&side) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match load_in_progress(&state, &mr_id).await {
        Ok(Some(pm)) => build(&state, &pm, side).await.into_response(),
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Err(status) => status.into_response(),
    }
}

async fn build(state: &AppState, pm: &MatchReportPreMatch, side: TeamSide) -> OpponentMvpTemplate {
    let team_id = pm.team_id_of(side).to_string();
    let info = state.match_report.team_data.find_team_info(&team_id).await;
    OpponentMvpTemplate {
        team_name: info.map(|i| i.team_name).unwrap_or_default(),
        mvp: OpponentMvpVm::from_domain(pm, side),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(mvp: Option<OpponentMvpVm>) -> String {
        OpponentMvpTemplate {
            team_name: "London Rats".into(),
            mvp,
        }
        .render()
        .unwrap()
    }

    #[test]
    fn the_vm_reads_the_mvp_of_the_side() {
        use crate::app::match_report::use_cases::test_doubles::{pre_match, with_mvps};
        assert!(OpponentMvpVm::from_domain(&pre_match(), TeamSide::Away).is_none());
        let vm = OpponentMvpVm::from_domain(&with_mvps(pre_match()), TeamSide::Away).unwrap();
        assert_eq!(vm.player_name, "MVP mvp-away");
        assert_eq!(vm.drawn_among, None);
    }

    #[test]
    fn a_chosen_mvp_is_named() {
        let html = render(Some(OpponentMvpVm {
            player_name: "Skritt".into(),
            drawn_among: None,
        }));
        assert!(html.contains("London Rats"), "{html}");
        assert!(html.contains("MVP : Skritt"), "{html}");
        assert!(!html.contains("tiré au sort"), "{html}");
        assert!(html.contains(r#"hx-disinherit="*""#), "{html}");
    }

    #[test]
    fn a_drawn_mvp_says_among_whom() {
        let html = render(Some(OpponentMvpVm {
            player_name: "Skritt".into(),
            drawn_among: Some("Skritt et Queek".into()),
        }));
        assert!(
            html.contains("tiré au sort parmi Skritt et Queek"),
            "{html}"
        );
    }

    #[test]
    fn a_missing_mvp_is_in_progress() {
        let html = render(None);
        assert!(html.contains("en cours"), "{html}");
        assert!(html.contains("choisit son MVP"), "{html}");
        assert!(!html.contains("MVP :"), "{html}");
    }
}

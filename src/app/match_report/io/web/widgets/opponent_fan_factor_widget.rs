//! Le fan factor de l'adversaire, en « Chacun son équipe » (carte 654) :
//! `GET /{id}/step2/{side}/fan-factor`, derrière `garde_suivi`.
//!
//! L'avant-match d'un camp le charge pour l'autre camp, et le recharge sur le
//! signal du match : le jet de l'adversaire apparaît sans recharger la page,
//! et le formulaire du coach, lui, n'est jamais remplacé.
//!
//! **Un rapport qui n'est plus en cours rend 204**, comme le score du direct :
//! htmx ne remplace rien.

use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::TeamSide;
use crate::app::match_report::io::web::mvp_step_controller::parse_side;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// Un jet lancé : « 1 + 2 = 3, lancé par Morgane ».
pub struct RecordedFanFactorVm {
    pub dedicated_fans: u32,
    pub roll: u8,
    pub total: u32,
    /// Le nom de qui a lancé ; absent si le port ne le connaît pas — la page
    /// n'invente pas d'auteur.
    pub recorded_by: Option<String>,
}

impl RecordedFanFactorVm {
    /// `None` tant que le camp n'a pas lancé son jet.
    pub fn from_domain(
        pm: &MatchReportPreMatch,
        side: TeamSide,
        recorded_by: Option<String>,
    ) -> Option<Self> {
        Some(Self {
            dedicated_fans: pm.dedicated_fans_of(side).into_inner(),
            roll: pm.fan_roll_of(side)?.value(),
            total: pm.fan_factor_of(side)?,
            recorded_by,
        })
    }
}

#[derive(Template)]
#[template(path = "opponent-fan-factor-widget.html")]
pub struct OpponentFanFactorTemplate {
    pub team_name: String,
    pub recorded: Option<RecordedFanFactorVm>,
}

impl IntoResponse for OpponentFanFactorTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("opponent-fan-factor-widget.html: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn get_opponent_fan_factor(
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

/// L'agrégat d'un rapport en cours ou prêt à publier ; `None` pour tout
/// autre état. Partagé avec l'avant-match d'un camp.
pub(crate) async fn load_in_progress(
    state: &AppState,
    mr_id: &str,
) -> Result<Option<MatchReportPreMatch>, StatusCode> {
    match state.match_report.match_report_repo.find_by_id(mr_id).await {
        Ok(Some(MatchReportState::PreMatch(pm))) => Ok(Some(pm)),
        Ok(Some(MatchReportState::ReadyToPublish(rtp))) => Ok(Some(rtp.into_pre_match())),
        Ok(Some(_)) => Ok(None),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("opponent fan factor find_by_id {mr_id}: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn build(
    state: &AppState,
    pm: &MatchReportPreMatch,
    side: TeamSide,
) -> OpponentFanFactorTemplate {
    let ctx = &state.match_report;
    let team_id = pm.team_id_of(side).to_string();
    let author = pm.fan_factor_recorded_by.of(side).map(|id| id.to_string());
    let (info, recorded_by) = tokio::join!(ctx.team_data.find_team_info(&team_id), async {
        match &author {
            Some(id) => ctx.coach_data.find_coach_name(id).await,
            None => None,
        }
    });
    OpponentFanFactorTemplate {
        team_name: info.map(|i| i.team_name).unwrap_or_default(),
        recorded: RecordedFanFactorVm::from_domain(pm, side, recorded_by),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(recorded: Option<RecordedFanFactorVm>) -> String {
        OpponentFanFactorTemplate {
            team_name: "London Rats".into(),
            recorded,
        }
        .render()
        .unwrap()
    }

    #[test]
    fn a_rolled_fan_factor_says_who_rolled_it() {
        let html = render(Some(RecordedFanFactorVm {
            dedicated_fans: 1,
            roll: 2,
            total: 3,
            recorded_by: Some("Morgane".into()),
        }));
        assert!(html.contains("London Rats"), "{html}");
        assert!(html.contains("1 + 2 = 3"), "{html}");
        assert!(html.contains("lancé par Morgane"), "{html}");
        assert!(html.contains(r#"hx-disinherit="*""#), "{html}");
    }

    /// Un auteur inconnu du port ne s'invente pas.
    #[test]
    fn an_unknown_author_is_not_named() {
        let html = render(Some(RecordedFanFactorVm {
            dedicated_fans: 0,
            roll: 3,
            total: 3,
            recorded_by: None,
        }));
        assert!(html.contains("0 + 3 = 3"), "{html}");
        assert!(!html.contains("lancé par"), "{html}");
    }

    #[test]
    fn a_missing_roll_is_awaited() {
        let html = render(None);
        assert!(html.contains("en attente"), "{html}");
        assert!(!html.contains(" = "), "{html}");
    }
}

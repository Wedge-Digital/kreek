//! L'attente de l'autre équipe, en « Chacun son équipe » (carte 655) :
//! `GET /{id}/inducements/{team_id}/wait`, derrière `garde_camp`.
//!
//! La page d'attente le charge, et le recharge sur le signal du match. Tant que
//! l'équipe attend — l'outsider le favori (R5 de E22), le favori qui a fini
//! l'outsider —, il dit qui est attendu et où en est son avant-match.
//!
//! **Quand l'attente prend fin, il rouvre la page** par `HX-Location` sur
//! `#app-content` : la page de l'équipe est remplacée par ses achats, ou par le
//! récapitulatif, sans recharger le navigateur. Un `HX-Refresh` aurait détruit
//! le toast qui arrive au même moment.
//!
//! **Un rapport qui n'est plus en cours rend 204**, comme le fan factor de
//! l'adversaire : htmx ne remplace rien.

use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::value_objects::TeamSide;
use crate::app::match_report::io::web::inducements_controller::{
    fill_team_values, view_for, InducementsView,
};
use crate::app::match_report::io::web::widgets::opponent_fan_factor_widget::{
    load_in_progress, RecordedFanFactorVm,
};
use crate::app::match_report::ports::{ICoachDataPort, ITeamDataPort};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "inducements-wait-widget.html")]
pub struct InducementsWaitWidgetTemplate {
    /// Le coach attendu, ou son équipe si le port ne le connaît pas — la page
    /// n'invente pas de nom.
    pub waited_label: String,
    pub waited_coach: Option<String>,
    pub waited_team_name: String,
    /// L'équipe attendue a-t-elle lancé son fan factor ? Sinon, elle n'achète
    /// pas encore, et l'écran ne le prétend pas.
    pub waited_has_rolled: bool,
    /// Le favori qui a fini ses achats et attend l'outsider.
    pub own_is_done: bool,
    pub own_fan_factor: Option<RecordedFanFactorVm>,
}

impl IntoResponse for InducementsWaitWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("inducements-wait-widget.html: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn get_inducements_wait(
    Path((space_id, mr_id, team_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Ok(team_id) = TeamId::try_new(&team_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let pm = match load_with_team_values(&state, &mr_id).await {
        Ok(pm) => pm,
        Err(response) => return response,
    };
    match view_for(&pm, &team_id) {
        InducementsView::Wait(waited) => {
            let ctx = &state.match_report;
            let ports = (ctx.team_data.as_ref(), ctx.coach_data.as_ref());
            build(ports, &pm, &team_id, waited).await.into_response()
        }
        _ => reopen_response(&page_url(&space_id, &mr_id, &team_id)),
    }
}

/// Le rapport en cours, ses valeurs d'équipe relues s'il n'en porte pas. Sans
/// elles, personne n'a la main : 204, rien n'est remplacé.
async fn load_with_team_values(
    state: &AppState,
    mr_id: &str,
) -> Result<MatchReportPreMatch, Response> {
    let mut pm = match load_in_progress(state, mr_id).await {
        Ok(Some(pm)) => pm,
        Ok(None) => return Err(StatusCode::NO_CONTENT.into_response()),
        Err(status) => return Err(status.into_response()),
    };
    fill_team_values(&mut pm, state.match_report.team_data.as_ref()).await;
    require_team_values(pm)
}

/// Sans valeurs d'équipe, ni favori ni outsider : `topdog_team_id` paniquerait.
fn require_team_values(pm: MatchReportPreMatch) -> Result<MatchReportPreMatch, Response> {
    if pm.awaits_team_values() {
        return Err(StatusCode::NO_CONTENT.into_response());
    }
    Ok(pm)
}

fn page_url(space_id: &str, mr_id: &str, team_id: &TeamId) -> String {
    AppRoutes::default()
        .match_report
        .inducements(space_id, mr_id, &team_id.to_string())
}

/// L'attente est finie : la page de l'équipe se rouvre dans le contenu
/// applicatif, comme une navigation du menu.
pub(crate) fn reopen_response(page_url: &str) -> Response {
    let location = serde_json::json!({
        "path": page_url,
        "target": "#app-content",
        "select": "#app-content",
    });
    (StatusCode::OK, [("HX-Location", location.to_string())]).into_response()
}

/// Les deux ports que lit le widget : les équipes, et le nom de leurs coachs.
type WaitPorts<'a> = (&'a dyn ITeamDataPort, &'a dyn ICoachDataPort);

async fn build(
    ports: WaitPorts<'_>,
    pm: &MatchReportPreMatch,
    team_id: &TeamId,
    waited: &TeamId,
) -> InducementsWaitWidgetTemplate {
    let waited_id = waited.to_string();
    let (info, coach) = tokio::join!(
        ports.0.find_team_info(&waited_id),
        coach_name_of(ports, &waited_id),
    );
    let waited_team_name = info.map(|i| i.team_name).unwrap_or_default();
    InducementsWaitWidgetTemplate {
        waited_label: coach.clone().unwrap_or_else(|| waited_team_name.clone()),
        waited_coach: coach,
        waited_team_name,
        waited_has_rolled: pm.fan_roll_of(side_of(pm, waited)).is_some(),
        own_is_done: pm.has_recorded_inducements(team_id),
        own_fan_factor: RecordedFanFactorVm::from_domain(pm, side_of(pm, team_id), None),
    }
}

/// Le nom du coach d'une équipe ; `None` si l'un des deux ports l'ignore.
async fn coach_name_of((team_data, coach_data): WaitPorts<'_>, team_id: &str) -> Option<String> {
    match team_data.find_team_coach(team_id).await {
        Ok(Some(coach_id)) => coach_data.find_coach_name(&coach_id).await,
        Ok(None) => None,
        Err(e) => {
            tracing::warn!("attente des coups de pouce : coach de {team_id} illisible : {e}");
            None
        }
    }
}

/// Le camp d'une équipe du match — `garde_camp` a vérifié qu'elle en est.
fn side_of(pm: &MatchReportPreMatch, team_id: &TeamId) -> TeamSide {
    if team_id == &pm.home_team_id {
        TeamSide::Home
    } else {
        TeamSide::Away
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(waited_coach: Option<&str>, waited_has_rolled: bool, own_is_done: bool) -> String {
        let waited_team_name = "Nains de Fer".to_string();
        InducementsWaitWidgetTemplate {
            waited_label: waited_coach
                .map(str::to_string)
                .unwrap_or_else(|| waited_team_name.clone()),
            waited_coach: waited_coach.map(str::to_string),
            waited_team_name,
            waited_has_rolled,
            own_is_done,
            own_fan_factor: Some(RecordedFanFactorVm {
                dedicated_fans: 1,
                roll: 2,
                total: 3,
                recorded_by: None,
            }),
        }
        .render()
        .unwrap()
    }

    /// L'outsider attend le favori : le coach est nommé, l'écran vouvoie, et
    /// le widget s'isole de la page hôte.
    #[test]
    fn the_underdog_waits_for_the_named_topdog_coach() {
        let html = render(Some("Yann"), true, false);
        assert!(html.contains("Yann achète ses coups de pouce"), "{html}");
        assert!(html.contains("Votre budget"), "{html}");
        assert!(html.contains("après Yann"), "{html}");
        assert!(html.contains("1 + 2 = 3"), "{html}");
        assert!(html.contains(r#"hx-disinherit="*""#), "{html}");
        assert!(!html.contains(" tu "), "{html}");
    }

    /// Un coach inconnu du port ne s'invente pas : l'équipe le remplace.
    #[test]
    fn an_unknown_coach_is_replaced_by_the_team() {
        let html = render(None, true, false);
        assert!(
            html.contains("Nains de Fer : coups de pouce en cours"),
            "{html}"
        );
        assert!(!html.contains("achète ses coups de pouce"), "{html}");
    }

    /// Le favori qui n'a pas lancé son fan factor n'achète pas encore : l'écran
    /// ne le prétend pas.
    #[test]
    fn a_topdog_without_fan_factor_is_not_said_to_buy() {
        let html = render(Some("Yann"), false, false);
        assert!(
            html.contains("Yann n'a pas encore lancé son fan factor"),
            "{html}"
        );
        assert!(!html.contains("achète ses coups de pouce"), "{html}");
        assert!(!html.contains("en cours"), "{html}");
    }

    /// Le favori qui a fini attend l'outsider : ses achats sont validés.
    #[test]
    fn the_topdog_who_finished_waits_for_the_recap() {
        let html = render(Some("Morgane"), true, true);
        assert!(html.contains("Morgane achète ses coups de pouce"), "{html}");
        assert!(html.contains("Vos coups de pouce sont validés"), "{html}");
        assert!(html.contains("récapitulatif"), "{html}");
    }

    /// La fin de l'attente rouvre la page dans le contenu applicatif.
    #[test]
    fn the_end_of_the_wait_reopens_the_page_in_place() {
        let response = reopen_response("/app/SP/match-report/MR/inducements/TM");
        assert_eq!(response.status(), StatusCode::OK);
        let location = response.headers()["HX-Location"].to_str().unwrap();
        let location: serde_json::Value = serde_json::from_str(location).unwrap();
        assert_eq!(location["path"], "/app/SP/match-report/MR/inducements/TM");
        assert_eq!(location["target"], "#app-content");
        assert_eq!(location["select"], "#app-content");
    }

    // ── Ce que le widget lit des ports (carte 655) ───────────────────────────

    use crate::app::match_report::domain::value_objects::D3Roll;
    use crate::app::match_report::use_cases::test_doubles::{pre_match, NamedCoaches, NamedTeams};

    /// Un avant-match où le domicile attend l'extérieur, nommé « Rats ».
    fn waiting_home() -> (MatchReportPreMatch, NamedTeams) {
        let pm = pre_match();
        let away = pm.away_team_id.to_string();
        let teams = NamedTeams::default()
            .named(&away, "Rats")
            .coached(&away, "C-AWAY");
        (pm, teams)
    }

    async fn built(pm: &MatchReportPreMatch, teams: &NamedTeams) -> InducementsWaitWidgetTemplate {
        let coaches = NamedCoaches::default().named("C-AWAY", "Morgane");
        build((teams, &coaches), pm, &pm.home_team_id, &pm.away_team_id).await
    }

    #[tokio::test]
    async fn the_waited_team_is_named_by_its_coach() {
        let (pm, teams) = waiting_home();
        let vm = built(&pm, &teams).await;
        assert_eq!(vm.waited_label, "Morgane");
        assert_eq!(vm.waited_coach.as_deref(), Some("Morgane"));
        assert_eq!(vm.waited_team_name, "Rats");
    }

    /// Le port ne connaît pas le coach : la vue reprend le nom de l'équipe, elle
    /// n'invente pas de coach.
    #[tokio::test]
    async fn a_team_without_coach_is_named_by_itself() {
        let (pm, _) = waiting_home();
        let teams = NamedTeams::default().named(&pm.away_team_id.to_string(), "Rats");
        let vm = built(&pm, &teams).await;
        assert_eq!(vm.waited_label, "Rats");
        assert_eq!(vm.waited_coach, None);
    }

    /// Une lecture du coach en échec n'est pas une page en erreur.
    #[tokio::test]
    async fn an_unreadable_coach_falls_back_on_the_team() {
        let (pm, teams) = waiting_home();
        let vm = built(&pm, &teams.coach_unreadable()).await;
        assert_eq!(vm.waited_label, "Rats");
        assert_eq!(vm.waited_coach, None);
    }

    /// Un coach que le port des coachs ignore : même repli.
    #[tokio::test]
    async fn a_coach_without_name_falls_back_on_the_team() {
        let (pm, teams) = waiting_home();
        let vm = build(
            (&teams, &NamedCoaches::default()),
            &pm,
            &pm.home_team_id,
            &pm.away_team_id,
        )
        .await;
        assert_eq!(vm.waited_label, "Rats");
    }

    /// Le jet et les achats lus sont ceux du bon camp — l'attendu pour le jet,
    /// l'équipe de la page pour ses achats et son fan factor.
    #[tokio::test]
    async fn rolls_and_purchases_are_read_on_the_right_side() {
        let (mut pm, teams) = waiting_home();
        assert!(!built(&pm, &teams).await.waited_has_rolled);
        pm.away_fan_roll = Some(D3Roll::try_new(2).unwrap());
        pm.home_inducements = Some(vec![]);
        let vm = built(&pm, &teams).await;
        assert!(vm.waited_has_rolled);
        assert!(vm.own_is_done);
        assert!(vm.own_fan_factor.is_none(), "le domicile n'a pas lancé");
    }

    #[test]
    fn side_of_reads_the_team_s_side() {
        let pm = pre_match();
        assert_eq!(side_of(&pm, &pm.home_team_id), TeamSide::Home);
        assert_eq!(side_of(&pm, &pm.away_team_id), TeamSide::Away);
    }

    /// Sans valeurs d'équipe, ni favori ni outsider : 204, au lieu de la
    /// panique de `topdog_team_id`.
    #[test]
    fn without_team_values_the_widget_replaces_nothing() {
        let mut pm = pre_match();
        pm.away_team_value = None;
        let response = require_team_values(pm).unwrap_err();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(require_team_values(pre_match()).is_ok());
    }

    #[test]
    fn the_page_url_is_the_team_s_inducements() {
        let team = TeamId::new();
        assert_eq!(
            page_url("SP", "MR", &team),
            format!("/app/SP/match-report/MR/inducements/{team}")
        );
    }
}

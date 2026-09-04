use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{D3Roll, DedicatedFans};
use crate::app::match_report::use_cases::record_fan_factor_use_case;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;

// ── Template ─────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "pre-match.html")]
pub struct PreMatchTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
    pub home_team_name: String,
    pub away_team_name: String,
    pub home_initials: String,
    pub away_initials: String,
    pub home_coach_name: String,
    pub away_coach_name: String,
    pub home_roster_name: String,
    pub away_roster_name: String,
    pub home_logo_url: Option<String>,
    pub away_logo_url: Option<String>,
    pub home_team_context_url: String,
    pub away_team_context_url: String,
    pub form_action: String,
    pub fan_factor_already_recorded: bool,
    /// Les jets enregistrés, s'il y en a (carte 494).
    ///
    /// Le gabarit en fait la valeur initiale des deux champs — vides sinon.
    /// Ils valaient 2 en dur, si bien qu'un rapport vierge proposait un jet
    /// qu'on n'avait pas fait, et qu'un rapport déjà saisi affichait 2 au lieu
    /// du sien, sous un bandeau « déjà enregistré » qui disait le contraire.
    pub home_fan_roll: Option<u8>,
    pub away_fan_roll: Option<u8>,
}

impl IntoResponse for PreMatchTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

fn initials_from(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

// ── GET ───────────────────────────────────────────────────────────────────────

pub async fn get_pre_match(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(_user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let mr_state = match state
        .match_report
        .match_report_repo
        .find_by_id(&match_report_id)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get_pre_match find_by_id {match_report_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Les trois états portent les jets. `ReadyToPublish` et `Published` rendaient
    // `true` en dur : c'est vrai de l'enregistrement, mais ça ne dit pas ce qui
    // a été enregistré — et le gabarit en a besoin.
    let (home_id, away_id, home_fan_roll, away_fan_roll) = match mr_state {
        MatchReportState::PreMatch(pm) => (
            pm.home_team_id.to_string(),
            pm.away_team_id.to_string(),
            pm.home_fan_roll,
            pm.away_fan_roll,
        ),
        MatchReportState::ReadyToPublish(rtp) => (
            rtp.home_team_id.to_string(),
            rtp.away_team_id.to_string(),
            rtp.home_fan_roll,
            rtp.away_fan_roll,
        ),
        MatchReportState::Published(p) => (
            p.home_team_id.to_string(),
            p.away_team_id.to_string(),
            p.home_fan_roll,
            p.away_fan_roll,
        ),
        MatchReportState::Draft(_) => {
            let url = AppRoutes::default()
                .match_report
                .edit_match_report(&space_id, &match_report_id);
            return Redirect::to(&url).into_response();
        }
        MatchReportState::Cancelled(_) => return StatusCode::GONE.into_response(),
    };
    let fan_factor_already_recorded = home_fan_roll.is_some() && away_fan_roll.is_some();

    let (home_info, away_info) = tokio::join!(
        state.match_report.team_data.find_team_info(&home_id),
        state.match_report.team_data.find_team_info(&away_id),
    );
    let home_info = home_info.unwrap_or_default();
    let away_info = away_info.unwrap_or_default();

    let base_url = AppRoutes::default()
        .teams
        .team_match_context_json(&space_id);
    let home_team_context_url = format!("{}?team_id={}", base_url, home_id);
    let away_team_context_url = format!("{}?team_id={}", base_url, away_id);
    let form_action = AppRoutes::default()
        .match_report
        .step2(&space_id, &match_report_id);

    PreMatchTemplate {
        app_routes: Default::default(),
        space_id,
        match_report_id,
        home_initials: initials_from(&home_info.team_name),
        away_initials: initials_from(&away_info.team_name),
        home_team_name: home_info.team_name,
        away_team_name: away_info.team_name,
        home_coach_name: home_info.coach_name,
        away_coach_name: away_info.coach_name,
        home_roster_name: home_info.roster_name,
        away_roster_name: away_info.roster_name,
        home_logo_url: home_info.logo_url,
        away_logo_url: away_info.logo_url,
        home_team_id: home_id,
        away_team_id: away_id,
        home_team_context_url,
        away_team_context_url,
        form_action,
        fan_factor_already_recorded,
        home_fan_roll: home_fan_roll.map(|r| r.value()),
        away_fan_roll: away_fan_roll.map(|r| r.value()),
    }
    .into_response()
}

// ── POST ──────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RecordFanFactorForm {
    pub home_fan_roll: u8,
    pub away_fan_roll: u8,
}

pub async fn post_pre_match(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<RecordFanFactorForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let home_fan_roll = match D3Roll::try_new(form.home_fan_roll) {
        Ok(r) => r,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let away_fan_roll = match D3Roll::try_new(form.away_fan_roll) {
        Ok(r) => r,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let mr_id = match MatchReportId::try_new(&match_report_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = record_fan_factor_use_case::RecordFanFactorCommand {
        match_report_id: mr_id,
        home_fan_roll,
        away_fan_roll,
        home_dedicated_fans: DedicatedFans::default(),
        away_dedicated_fans: DedicatedFans::default(),
        recorded_by: user.id,
    };

    let outcome = record_fan_factor_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        state.match_report.team_data.as_ref(),
        state.match_report.competition_data.as_ref(),
    )
    .await;

    match outcome {
        Ok(record_fan_factor_use_case::RecordFanFactorOutcome::RedirectToInducements {
            topdog_team_id,
        }) => {
            let url = AppRoutes::default().match_report.inducements(
                &space_id,
                &match_report_id,
                &topdog_team_id,
            );
            Redirect::to(&url).into_response()
        }
        Ok(record_fan_factor_use_case::RecordFanFactorOutcome::RedirectToStep3) => {
            let url = AppRoutes::default()
                .match_report
                .step3(&space_id, &match_report_id);
            Redirect::to(&url).into_response()
        }
        Err(record_fan_factor_use_case::RecordFanFactorError::NotFound) => {
            StatusCode::NOT_FOUND.into_response()
        }
        Err(record_fan_factor_use_case::RecordFanFactorError::NotInPreMatchPhase) => {
            StatusCode::CONFLICT.into_response()
        }
        Err(e) => {
            tracing::error!("post_pre_match: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un gabarit minimal : seuls les deux jets varient d'un cas à l'autre.
    fn gabarit(home: Option<u8>, away: Option<u8>) -> PreMatchTemplate {
        PreMatchTemplate {
            app_routes: AppRoutes::default(),
            space_id: "s".into(),
            match_report_id: "m".into(),
            home_team_id: "t-home".into(),
            away_team_id: "t-away".into(),
            home_team_name: "Domicile".into(),
            away_team_name: "Visiteur".into(),
            home_initials: "DO".into(),
            away_initials: "VI".into(),
            home_coach_name: "A".into(),
            away_coach_name: "B".into(),
            home_roster_name: "R".into(),
            away_roster_name: "R".into(),
            home_logo_url: None,
            away_logo_url: None,
            home_team_context_url: "/ctx/home".into(),
            away_team_context_url: "/ctx/away".into(),
            form_action: "/post".into(),
            fan_factor_already_recorded: home.is_some() && away.is_some(),
            home_fan_roll: home,
            away_fan_roll: away,
        }
    }

    /// **Le défaut d'origine.** Les deux champs portaient `value="2"` et
    /// l'`x-data` initialisait ses jets à 2 : un rapport neuf proposait un jet
    /// qu'on n'avait pas fait, et rien ne distinguait cette valeur d'une saisie.
    #[test]
    fn un_rapport_vierge_ne_propose_aucun_jet() {
        let html = gabarit(None, None).render().unwrap();
        assert!(
            !html.contains(r#"value="2""#),
            "aucun jet ne doit être proposé par défaut"
        );
        assert!(html.contains("homeRoll: ''"), "le jet domicile part vide");
        assert!(html.contains("awayRoll: ''"), "le jet visiteur part vide");
    }

    /// L'autre moitié : ce qui a été enregistré revient à l'écran. Les deux
    /// valeurs diffèrent à dessein — égales, le test passerait en confondant
    /// les deux champs.
    #[test]
    fn un_rapport_saisi_reaffiche_ses_jets() {
        let html = gabarit(Some(1), Some(3)).render().unwrap();
        assert!(html.contains("homeRoll: 1"), "le jet domicile enregistré");
        assert!(html.contains("awayRoll: 3"), "le jet visiteur enregistré");
        assert!(
            html.contains("déjà enregistré"),
            "le bandeau accompagne les jets"
        );
    }

    /// **`formatKpo` recomposait le nombre** depuis les milliers et les
    /// centaines, et le chiffre des dizaines n'apparaissait dans aucune de ses
    /// deux branches : 2075 s'affichait « 2 000 kPo ». Le gabarit ne doit plus
    /// porter cette arithmétique.
    #[test]
    fn le_gabarit_ne_recompose_plus_les_montants() {
        let html = gabarit(None, None).render().unwrap();
        assert!(
            !html.contains("Math.floor(v / 1000)"),
            "le montant s'imprime, il ne se recompose pas"
        );
        assert!(!html.contains("'00 kPo'"), "plus de centaines recollées");
        assert!(
            html.contains("return v + ' kPo';"),
            "la valeur, telle quelle"
        );
    }
}

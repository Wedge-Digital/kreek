use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::ports::{ICompetitionDataPort, ITeamDataPort};
use crate::app::match_report::domain::value_objects::{RosterPositionUid, TeamValue};
use crate::app::match_report::use_cases::record_inducements_use_case::{
    self, InducementPurchaseCmd, MercenaryLevel, MercenaryPurchaseCmd, RecordInducementsCommand,
    RecordInducementsOutcome,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::common_types::MatchReportId;
use crate::app::shared_kernel::inducement_definition::InducementId;
use crate::app::shared_kernel::team::TeamId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "inducements.html")]
pub struct InducementsTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub team_id: String,
    pub team_name: String,
    pub team_initials: String,
    pub order_label: String,
    pub budget: u32,
    pub inducement_selector_url:  String,
    pub mercenary_selector_url:   String,
    pub form_action:              String,
}

impl IntoResponse for InducementsTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── GET ───────────────────────────────────────────────────────────────────────

pub async fn get_inducements(
    auth_session: AuthSession,
    Path((space_id, match_report_id, team_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if auth_session.user.is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let mr_id = match MatchReportId::try_new(&match_report_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let team_id_vo = match TeamId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    match build_vm(
        mr_id,
        team_id_vo,
        &space_id,
        state.match_report.match_report_repo.as_ref(),
        state.match_report.team_data.as_ref(),
        state.match_report.competition_data.as_ref(),
        &AppRoutes::default(),
    )
    .await
    {
        Ok(tpl) => tpl.into_response(),
        Err(e) => e,
    }
}

async fn build_vm(
    mr_id: MatchReportId,
    team_id: TeamId,
    space_id: &str,
    repo: &dyn crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
    routes: &AppRoutes,
) -> Result<InducementsTemplate, Response> {
    let mut pm = load_pre_match_vm(repo, &mr_id.to_string()).await?;
    if pm.home_team_value.is_none() || pm.away_team_value.is_none() {
        let home_id = pm.home_team_id.to_string();
        let away_id = pm.away_team_id.to_string();
        let (home_raw, away_raw) = tokio::join!(
            team_data.find_team_value(&home_id),
            team_data.find_team_value(&away_id),
        );
        pm.home_team_value = home_raw.and_then(|v| TeamValue::try_new(v).ok());
        pm.away_team_value = away_raw.and_then(|v| TeamValue::try_new(v).ok());
    }
    let team_info = team_data
        .find_team_info(&team_id.to_string())
        .await
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
    let tier = competition_data
        .find_tier_rules_for_roster(&pm.season_id.to_string(), &team_info.roster_id)
        .await
        .unwrap_or_default();
    let treasury = team_data.find_team_treasury(&team_id.to_string()).await.unwrap_or(0);
    let budget = pm.inducement_budget_for(&team_id, treasury);
    let is_topdog = pm.topdog_team_id() == &team_id;
    let selector_url = build_selector_url(routes, &tier, &team_info.roster_id);
    let mercenary_url = routes.match_report.mercenary_selector(space_id, &mr_id.to_string(), &team_id.to_string());
    let form_action =
        routes.match_report.inducements(space_id, &mr_id.to_string(), &team_id.to_string());
    let initials = team_info
        .team_name
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    Ok(InducementsTemplate {
        app_routes: *routes,
        space_id: space_id.to_string(),
        match_report_id: mr_id.to_string(),
        team_id: team_id.to_string(),
        team_name: team_info.team_name,
        team_initials: initials,
        order_label: if is_topdog { "TopDog — achète en premier".to_string() } else { "Underdog — achète en second".to_string() },
        budget,
        inducement_selector_url:  selector_url,
        mercenary_selector_url:   mercenary_url,
        form_action,
    })
}

async fn load_pre_match_vm(
    repo: &dyn crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository,
    mr_id: &str,
) -> Result<crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch, Response>
{
    let state = repo
        .find_by_id(mr_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
    match state {
        MatchReportState::PreMatch(pm) => Ok(pm),
        MatchReportState::ReadyToPublish(rtp) => Ok(rtp.into_pre_match()),
        _ => Err(StatusCode::CONFLICT.into_response()),
    }
}

fn build_selector_url(
    routes: &AppRoutes,
    tier: &crate::app::match_report::ports::TierRulesDto,
    roster_id: &str,
) -> String {
    let inducement_csv: String = tier.allowed_inducements.iter().map(|s| s.uid.as_str()).collect::<Vec<_>>().join(",");
    let star_csv: String = tier.allowed_star_players.iter().map(|s| s.uid.as_str()).collect::<Vec<_>>().join(",");
    format!(
        "{}?allowed_inducement_uids={}&allowed_star_player_uids={}&roster_id={}",
        routes.references.inducement_selector(),
        inducement_csv,
        star_csv,
        roster_id,
    )
}

// ── POST ──────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct InducementsForm {
    #[serde(default)]
    pub intent:      String,
    #[serde(default)]
    pub selection:   String,
    #[serde(default)]
    pub mercenaries: String,
}

fn resolve_purchases_from_form(
    form: &InducementsForm,
) -> Result<(Vec<InducementPurchaseCmd>, Vec<MercenaryPurchaseCmd>), Response> {
    if form.intent == "pass" {
        return Ok((Vec::new(), Vec::new()));
    }
    let purchases = parse_purchases(&form.selection);
    let mercenary_purchases = parse_mercenaries(&form.mercenaries)?;
    Ok((purchases, mercenary_purchases))
}

#[derive(Deserialize)]
struct SelectionItem {
    uid: String,
    qty: u8,
}

pub async fn post_inducements(
    auth_session: AuthSession,
    Path((space_id, match_report_id, team_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Form(form): Form<InducementsForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let (mr_id, team_id_vo) = match parse_path_ids(&match_report_id, &team_id) {
        Ok(ids) => ids,
        Err(r) => return r,
    };
    let (purchases, mercenary_purchases) = match resolve_purchases_from_form(&form) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let cmd = RecordInducementsCommand {
        match_report_id: mr_id,
        team_id: team_id_vo,
        purchases,
        mercenary_purchases,
        recorded_by: user.id,
    };
    match record_inducements_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        state.match_report.team_data.as_ref(),
        state.match_report.competition_data.as_ref(),
        state.match_report.player_data.as_ref(),
    )
    .await
    {
        Ok(RecordInducementsOutcome::RedirectToInducements { next_team_id }) => {
            let url = AppRoutes::default().match_report.inducements(&space_id, &match_report_id, &next_team_id);
            Redirect::to(&url).into_response()
        }
        Ok(RecordInducementsOutcome::RedirectToStep3) => {
            let url = AppRoutes::default().match_report.step3(&space_id, &match_report_id);
            Redirect::to(&url).into_response()
        }
        Err(record_inducements_use_case::RecordInducementsError::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(record_inducements_use_case::RecordInducementsError::NotInPreMatchPhase) => StatusCode::CONFLICT.into_response(),
        Err(e) => {
            tracing::error!("post_inducements: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn parse_path_ids(
    match_report_id: &str,
    team_id: &str,
) -> Result<(MatchReportId, TeamId), Response> {
    let mr_id =
        MatchReportId::try_new(match_report_id).map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
    let team_id_vo =
        TeamId::try_new(team_id).map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
    Ok((mr_id, team_id_vo))
}

#[derive(serde::Deserialize)]
struct MercenaryItem {
    position_uid: String,
    level:        String,
}

fn parse_mercenaries(json: &str) -> Result<Vec<MercenaryPurchaseCmd>, Response> {
    let items: Vec<MercenaryItem> = serde_json::from_str(json).unwrap_or_default();
    items
        .into_iter()
        .filter(|m| !m.position_uid.is_empty())
        .map(|m| {
            let position_id = RosterPositionUid::try_new(&m.position_uid)
                .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
            let level = MercenaryLevel::try_from_str(&m.level)
                .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
            Ok(MercenaryPurchaseCmd { position_id, level })
        })
        .collect()
}

fn parse_purchases(selection_json: &str) -> Vec<InducementPurchaseCmd> {
    serde_json::from_str::<Vec<SelectionItem>>(selection_json)
        .unwrap_or_default()
        .into_iter()
        .filter(|item| !item.uid.is_empty())
        .map(|item| InducementPurchaseCmd { uid: InducementId(item.uid), qty: item.qty })
        .collect()
}

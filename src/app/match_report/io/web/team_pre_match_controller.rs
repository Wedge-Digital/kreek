//! L'avant-match d'un camp, en « Chacun son équipe » (carte 654, épic E22) :
//! `GET/POST /{id}/step2/{side}`, gardé par `garde_camp`.
//!
//! Chaque coach lance son seul jet ; celui de l'autre s'affiche en lecture
//! seule dans un widget rechargé sur le signal du match. Le camp vient de la
//! route, que la garde a vérifiée — jamais du formulaire. La page reprend le
//! gabarit de « Je saisis tout », `pre-match.html`, avec son `TeamEntryVm`.
//!
//! Un rapport qui n'est pas en « Chacun son équipe » renvoie à `/step2`.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::domain::user::User;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::value_objects::{D3Roll, EntryMode, TeamSide};
use crate::app::match_report::io::web::mvp_step_controller::{parse_side, side_slug};
use crate::app::match_report::io::web::pre_match_controller::{
    fan_factor_error_response, next_url, FrozenTeamValuesVm, PreMatchTemplate, SwitchSideVm,
    TeamEntryVm,
};
use crate::app::match_report::io::web::widgets::opponent_fan_factor_widget::load_in_progress;
use crate::app::match_report::ports::TeamInfoDto;
use crate::app::match_report::use_cases::match_report_access_service::{
    est_administrateur, AccesRapportDeps,
};
use crate::app::match_report::use_cases::record_fan_factor_use_case::next_step;
use crate::app::match_report::use_cases::record_team_fan_factor_use_case::{
    self, RecordTeamFanFactorCommand, TeamFanFactorDeps,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;

/// Les identifiants d'une page : espace, compte rendu.
struct PageIds<'a> {
    space_id: &'a str,
    mr_id: &'a str,
}

/// Ce que la page lit hors de l'agrégat.
struct SideFacts {
    home: TeamInfoDto,
    away: TeamInfoDto,
    /// L'admin peut saisir l'autre camp (§ 12.6 de E22).
    may_switch: bool,
    next_url: Option<String>,
}

pub async fn get_team_pre_match(
    auth_session: AuthSession,
    Path((space_id, mr_id, side)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(side) = parse_side(&side) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let ids = PageIds {
        space_id: &space_id,
        mr_id: &mr_id,
    };
    let pm = match load_team_entry(&state, &ids).await {
        Ok(pm) => pm,
        Err(response) => return response,
    };
    let facts = load_facts(&state, &user, &ids, &pm, side).await;
    build_page(&ids, &pm, side, facts).into_response()
}

/// Un rapport en cours en « Chacun son équipe » — ou prêt à publier, pour le
/// relire. Tout autre rapport renvoie à `/step2`, qui sait le traiter.
async fn load_team_entry(
    state: &AppState,
    ids: &PageIds<'_>,
) -> Result<MatchReportPreMatch, Response> {
    let found = load_in_progress(state, ids.mr_id).await;
    match found.map_err(IntoResponse::into_response)? {
        Some(pm) if pm.entry_mode == EntryMode::TeamEntry => Ok(pm),
        _ => Err(to_step2(ids)),
    }
}

fn to_step2(ids: &PageIds<'_>) -> Response {
    let url = AppRoutes::default()
        .match_report
        .step2(ids.space_id, ids.mr_id);
    Redirect::to(&url).into_response()
}

async fn load_facts(
    state: &AppState,
    user: &User,
    ids: &PageIds<'_>,
    pm: &MatchReportPreMatch,
    side: TeamSide,
) -> SideFacts {
    let team_data = state.match_report.team_data.as_ref();
    let (home_id, away_id) = (pm.home_team_id.to_string(), pm.away_team_id.to_string());
    let (home, away) = tokio::join!(
        team_data.find_team_info(&home_id),
        team_data.find_team_info(&away_id),
    );
    let deps = AccesRapportDeps::from_state(state);
    let competition_id = pm.competition_id.to_string();
    SideFacts {
        home: home.unwrap_or_default(),
        away: away.unwrap_or_default(),
        may_switch: est_administrateur(&deps, user, ids.space_id, &competition_id).await,
        next_url: next_url_once_rolled(state, ids, pm, side).await,
    }
}

/// La suite n'est proposée qu'une fois le jet du camp lancé.
async fn next_url_once_rolled(
    state: &AppState,
    ids: &PageIds<'_>,
    pm: &MatchReportPreMatch,
    side: TeamSide,
) -> Option<String> {
    pm.fan_roll_of(side)?;
    let ctx = &state.match_report;
    let team_id = pm.team_id_of(side).to_string();
    let outcome = next_step(
        pm,
        team_id,
        ctx.team_data.as_ref(),
        ctx.competition_data.as_ref(),
    )
    .await;
    Some(next_url(ids.space_id, ids.mr_id, outcome))
}

fn build_page(
    ids: &PageIds<'_>,
    pm: &MatchReportPreMatch,
    side: TeamSide,
    facts: SideFacts,
) -> PreMatchTemplate {
    let team_entry = team_entry_vm(ids, pm, side, &facts);
    let routes = AppRoutes::default().match_report;
    PreMatchTemplate {
        form_action: routes.step2_side(ids.space_id, ids.mr_id, side_slug(side)),
        home_fan_roll: pm.home_fan_roll.map(|r| r.value()),
        away_fan_roll: pm.away_fan_roll.map(|r| r.value()),
        frozen_values: FrozenTeamValuesVm::from_domain(pm),
        team_entry: Some(team_entry),
        ..PreMatchTemplate::base(
            ids.space_id,
            ids.mr_id,
            (pm.home_team_id.to_string(), pm.away_team_id.to_string()),
            facts.home,
            facts.away,
        )
    }
}

fn team_entry_vm(
    ids: &PageIds<'_>,
    pm: &MatchReportPreMatch,
    side: TeamSide,
    facts: &SideFacts,
) -> TeamEntryVm {
    let routes = AppRoutes::default().match_report;
    let opponent = side.opposite();
    TeamEntryVm {
        side: side_slug(side),
        own_team_name: team_name(facts, side).to_string(),
        own_roll: pm.fan_roll_of(side).map(|r| r.value()),
        own_total: pm.fan_factor_of(side),
        opponent_url: routes.step2_fan_factor(ids.space_id, ids.mr_id, side_slug(opponent)),
        next_url: facts.next_url.clone(),
        switch: facts.may_switch.then(|| SwitchSideVm {
            url: routes.step2_side(ids.space_id, ids.mr_id, side_slug(opponent)),
            team_name: team_name(facts, opponent).to_string(),
        }),
    }
}

fn team_name(facts: &SideFacts, side: TeamSide) -> &str {
    match side {
        TeamSide::Home => &facts.home.team_name,
        TeamSide::Away => &facts.away.team_name,
    }
}

// ── POST ──────────────────────────────────────────────────────────────────────

/// Le seul jet du camp : son camp vient de la route, jamais du formulaire.
#[derive(Deserialize)]
pub struct TeamFanFactorForm {
    pub fan_roll: u8,
}

pub async fn post_team_pre_match(
    auth_session: AuthSession,
    Path((space_id, mr_id, side)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Form(form): Form<TeamFanFactorForm>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(cmd) = build_command(&mr_id, &side, form.fan_roll, user.id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    match record_team_fan_factor_use_case::execute(cmd, &deps_of(&state)).await {
        Ok(outcome) => Redirect::to(&next_url(&space_id, &mr_id, outcome)).into_response(),
        Err(e) => fan_factor_error_response(e),
    }
}

fn deps_of(state: &AppState) -> TeamFanFactorDeps<'_> {
    let ctx = &state.match_report;
    TeamFanFactorDeps {
        repo: ctx.match_report_repo.as_ref(),
        team_data: ctx.team_data.as_ref(),
        competition_data: ctx.competition_data.as_ref(),
        bus: &ctx.event_bus,
    }
}

fn build_command(
    mr_id: &str,
    side: &str,
    fan_roll: u8,
    recorded_by: CoachId,
) -> Option<RecordTeamFanFactorCommand> {
    Some(RecordTeamFanFactorCommand {
        match_report_id: MatchReportId::try_new(mr_id).ok()?,
        team_side: parse_side(side)?,
        fan_roll: D3Roll::try_new(fan_roll).ok()?,
        recorded_by,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MR: &str = "01M45MDK4QNZWASD6QH0BYYYRS";

    #[test]
    fn the_side_comes_from_the_route() {
        let cmd = build_command(MR, "away", 2, CoachId::new()).unwrap();
        assert_eq!(cmd.team_side, TeamSide::Away);
        assert_eq!(cmd.fan_roll.value(), 2);
    }

    #[test]
    fn an_unknown_side_or_roll_is_refused() {
        assert!(build_command(MR, "both", 2, CoachId::new()).is_none());
        assert!(build_command(MR, "home", 4, CoachId::new()).is_none());
    }
}

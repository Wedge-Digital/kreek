use crate::app::auth::auth_backend::AuthSession;
use crate::app::references::routes::Routes as RefRoutes;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::io::web::set_player_identity::{
    build_skill_header_vm, SkillHeaderFragmentTemplate, SkillHeaderPlayerVm,
};
use crate::app::team_creation::io::web::spp_management::{build_summary, SppSummaryTemplate};
use crate::app::team_creation::routes::Routes;
use crate::app::shared_kernel::common_types::EntityId;
use crate::web::routes::Routes as WebRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

// ── View models ───────────────────────────────────────────────────────────────

pub struct FinalizePlayerRowVm {
    pub instance_id:          String,
    pub position_name:        String,
    pub personal_name:        String,
    pub jersey:               u8,
    pub base_skills:          String,
    pub skills_acquired_count: usize,
    pub set_identity_url:     String,
    pub spend_spp_url:        String,
    pub cancel_spp_base_url:  String,
    pub roster_line_id:       String,
    pub acquired_csv:         String,
}

pub struct FinalizeTeamHeaderVm {
    pub team_name:   String,
    pub roster_name: String,
    pub coach_id:    String,
    pub tv_kpo:      u32,
    pub treasury_kpo: u32,
    pub spp_pool:    u8,
    pub league_uid:  String,
    pub league_label: String,
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "finalize-team.html")]
pub struct FinalizeTeamTemplate {
    pub web_routes:    WebRoutes,
    pub team_routes:   Routes,
    pub ref_routes:    RefRoutes,
    pub space_id:      String,
    pub team_id:       String,
    pub header:        FinalizeTeamHeaderVm,
    pub players:       Vec<FinalizePlayerRowVm>,
    pub league_selector_url: String,
    pub set_league_url:      String,
    pub skill_header_url:    String,
}

impl IntoResponse for FinalizeTeamTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("finalize template render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Skill header GET ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SkillHeaderParams {
    pub player_id: String,
}

pub async fn skill_header(
    _auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    Query(params): Query<SkillHeaderParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let team = match state
        .team_creation
        .roster_repository
        .find_by_id(&team_entity_id)
        .await
    {
        Ok(Some(t)) => t,
        _ => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let routes = Routes::default();
    let spp_pool = team.spp_pool;
    let vm = build_skill_header_vm(&team, &params.player_id, &routes, &space_id, &team_id, spp_pool);

    SkillHeaderFragmentTemplate {
        player: vm,
        spp_pool,
    }
    .into_response()
}

// ── Main page GET ─────────────────────────────────────────────────────────────

pub async fn finalize_team(
    _auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let team = match state
        .team_creation
        .roster_repository
        .find_by_id(&team_entity_id)
        .await
    {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("finalize_team repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let routes = Routes::default();
    let ref_routes = RefRoutes::default();
    let ref_repo = state.references.repository.as_ref();

    let set_league_url = routes.finalize_team(&space_id, &team_id).replace("/finalize", "/league");
    let league_selector_url = ref_routes.league_selector(
        team.league_id.as_ref().map(|l| l.0.as_str()).unwrap_or(""),
        &routes.set_league(&space_id, &team_id),
    );

    let players: Vec<FinalizePlayerRowVm> = team
        .hired_players()
        .iter()
        .map(|p| {
            let base_skills = p
                .definition
                .id
                .0
                .split("__")
                .last()
                .and_then(|pos_uid| {
                    ref_repo
                        .find_position_by_uid(&p.definition.id.0)
                        .map(|pos| {
                            pos.skills
                                .iter()
                                .filter_map(|uid| ref_repo.find_skill_by_uid(uid))
                                .map(|s| s.name.clone())
                                .collect::<Vec<_>>()
                                .join(" · ")
                        })
                })
                .unwrap_or_default();

            FinalizePlayerRowVm {
                skills_acquired_count: p.acquired_skills.len(),
                set_identity_url: routes.set_player_identity(&space_id, &team_id, &p.instance_id.0),
                spend_spp_url: routes.spend_spp(&space_id, &team_id, &p.instance_id.0),
                cancel_spp_base_url: routes.cancel_spp_base(&space_id, &team_id, &p.instance_id.0),
                roster_line_id: p.definition.id.0.clone(),
                acquired_csv: p.acquired_skill_ids_csv(),
                instance_id: p.instance_id.0.clone(),
                position_name: p.definition.name.0.clone(),
                personal_name: p.personal_name.clone(),
                jersey: p.jersey.unwrap_or(0),
                base_skills,
            }
        })
        .collect();

    let tv_kpo = team.remaining_budget().map(|_| 0).unwrap_or(0);
    let treasury_kpo = team.remaining_budget().unwrap_or(0);

    let (league_uid, league_label) = team
        .league_id
        .as_ref()
        .and_then(|lid| {
            ref_repo
                .list_leagues()
                .iter()
                .find(|l| l.uid == lid.0)
                .map(|l| (l.uid.clone(), l.label.clone()))
        })
        .unwrap_or_default();

    let header = FinalizeTeamHeaderVm {
        team_name: team.base_infos().name().clone().into_inner(),
        roster_name: team.roster.name.0.clone(),
        coach_id: String::new(),
        tv_kpo,
        treasury_kpo,
        spp_pool: team.spp_pool,
        league_uid,
        league_label,
    };

    FinalizeTeamTemplate {
        web_routes: WebRoutes::default(),
        team_routes: routes.clone(),
        ref_routes,
        space_id: space_id.clone(),
        team_id: team_id.clone(),
        header,
        players,
        league_selector_url,
        set_league_url,
        skill_header_url: routes.skill_header(&space_id, &team_id),
    }
    .into_response()
}

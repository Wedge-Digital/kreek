use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::competitions::domain::competition_structure::CompetitionStructure;
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::competitions::io::web::admin::admin_page::render_admin_page;
use crate::app::auth::auth_backend::AuthSession;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::common_types::{CompetitionId, SeasonId};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "admin/summary.html")]
pub struct SummaryTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub competition_name: String,
    pub competition_logo: Option<String>,
    pub admin_names: Vec<String>,
    pub season_name: String,
    pub has_unfilled_spots: bool,
    pub remaining_spots: u32,
    pub total_spots: u32,
    pub has_rules: bool,
    pub ranking_points_label: String,
    pub bonus_label: Option<String>,
    pub tiers_label: Option<String>,
    pub rosters_preview: Vec<String>,
    pub rosters_extra: usize,
    pub has_structure: bool,
    pub groups_label: Option<String>,
    pub playoffs_label: Option<String>,
    pub dates_label: Option<String>,
    pub has_invitations: bool,
    pub access_mode_label: String,
    pub validation_label: String,
    pub invited_label: String,
    pub spots_label: Option<String>,
    pub spots_warn: bool,
    pub deadline_label: Option<String>,
}

impl IntoResponse for SummaryTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("summary_tab render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn summary_tab_fragment(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    render_admin_page(auth_session, &space_id, &competition_id, &season_id, "summary", &state).await
}

pub async fn build_summary_fragment(
    comp_id: &CompetitionId,
    season_id: &SeasonId,
    comp_repo: &dyn ICompetitionRepository,
    season_repo: &dyn ISeasonRepository,
    app_routes: AppRoutes,
    space_id: &str,
    competition_id: &str,
    season_id_str: &str,
) -> Option<SummaryTabTemplate> {
    let (base_info, season_info, rules, structure, invitations) = tokio::join!(
        comp_repo.find_base_info(comp_id),
        season_repo.find_base_info(season_id),
        season_repo.find_rules(season_id),
        season_repo.find_structure(season_id),
        season_repo.find_invitations(season_id),
    );

    let base = base_info.ok().flatten()?;
    let season_name = season_info.ok().flatten().map(|s| s.name).unwrap_or_default();
    let rules = rules.ok().flatten();
    let structure = structure.ok().flatten();
    let invitations = invitations.ok().flatten();

    let (has_rules, ranking_points_label, bonus_label, tiers_label, rosters_preview, rosters_extra) =
        build_rules_labels(&rules);
    let (has_structure, groups_label, playoffs_label, dates_label) =
        build_structure_labels(&structure);
    let (
        has_invitations,
        access_mode_label,
        validation_label,
        invited_label,
        spots_label,
        spots_warn,
        deadline_label,
        has_unfilled_spots,
        remaining_spots,
        total_spots,
    ) = build_invitations_labels(&invitations);

    Some(SummaryTabTemplate {
        app_routes,
        space_id: space_id.to_string(),
        competition_id: competition_id.to_string(),
        season_id: season_id_str.to_string(),
        competition_name: base.name,
        competition_logo: base.logo,
        admin_names: base.admin_names,
        season_name,
        has_unfilled_spots,
        remaining_spots,
        total_spots,
        has_rules,
        ranking_points_label,
        bonus_label,
        tiers_label,
        rosters_preview,
        rosters_extra,
        has_structure,
        groups_label,
        playoffs_label,
        dates_label,
        has_invitations,
        access_mode_label,
        validation_label,
        invited_label,
        spots_label,
        spots_warn,
        deadline_label,
    })
}

fn build_rules_labels(
    rules: &Option<CompetitionRules>,
) -> (bool, String, Option<String>, Option<String>, Vec<String>, usize) {
    let Some(r) = rules else {
        return (false, String::new(), None, None, vec![], 0);
    };
    let rr = &r.ranking_rules;
    let pts = format!(
        "Victoire = {} pts · Nul = {} pt · Défaite = {} pt",
        rr.win_points, rr.draw_points, rr.lose_points
    );
    let bonus: Option<String> = {
        let off = if rr.offensive_bonus.activated {
            Some(format!("+{} si ≥ {} TDs", rr.offensive_bonus.points, rr.offensive_bonus.diff_td))
        } else {
            None
        };
        let def = if rr.defensive_bonus.activated {
            Some(format!("+{} si ≤ 1 TD encaissé", rr.defensive_bonus.points))
        } else {
            None
        };
        match (off, def) {
            (Some(o), Some(d)) => Some(format!("Offensif ({o}) · Défensif ({d})")),
            (Some(o), None) => Some(format!("Offensif ({o})")),
            (None, Some(d)) => Some(format!("Défensif ({d})")),
            (None, None) => None,
        }
    };
    let tiers: Option<String> = if r.tiers.is_empty() {
        None
    } else {
        let names = r.tiers.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ");
        Some(format!("{} tier{} : {names}", r.tiers.len(), if r.tiers.len() > 1 { "s" } else { "" }))
    };
    let all_rosters: Vec<String> = r.tiers.iter().flat_map(|t| t.rosters.clone()).collect();
    let extra = all_rosters.len().saturating_sub(12);
    let preview = all_rosters.into_iter().take(12).collect();
    (true, pts, bonus, tiers, preview, extra)
}

fn build_structure_labels(
    structure: &Option<CompetitionStructure>,
) -> (bool, Option<String>, Option<String>, Option<String>) {
    let Some(s) = structure else {
        return (false, None, None, None);
    };
    let groups: Option<String> = if s.ranking_group.use_ranking_groups {
        let n = s.ranking_group.ranking_groups.len();
        Some(format!("{n} poule{}", if n > 1 { "s" } else { "" }))
    } else {
        None
    };
    let playoffs: Option<String> = if s.play_offs_phase.use_playoffs_phase {
        let q = s.play_offs_phase.qualified_team_per_pool;
        let third = if s.play_offs_phase.final_phase_match_for_third_place { " · Match 3e place" } else { "" };
        Some(format!("Top {q} par poule{third}"))
    } else {
        None
    };
    let dates: Option<String> = if s.schedule.use_schedule {
        Some(format!("{} → {}", s.schedule.schedule_start_date, s.schedule.schedule_end_date))
    } else {
        None
    };
    (true, groups, playoffs, dates)
}

fn build_invitations_labels(
    invitations: &Option<CompetitionInvitations>,
) -> (bool, String, String, String, Option<String>, bool, Option<String>, bool, u32, u32) {
    let Some(inv) = invitations else {
        return (false, String::new(), String::new(), String::new(), None, false, None, false, 0, 0);
    };
    let mode_label = match inv.access_mode.as_str() {
        "open" => "Inscription libre",
        _ => "Sur invitation (liste fermée)",
    }
    .to_string();
    let validation_label = if inv.requires_validation {
        "Oui (validation par les commissaires)"
    } else {
        "Non (acceptation automatique)"
    }
    .to_string();
    let count = inv.invited_coaches.len();
    let inv_label = format!("{count} coach{}", if count > 1 { "s" } else { "" });
    let (spots, spots_w, unfilled, remaining, total) = if let Some(max) = inv.max_participants {
        let c = count as u32;
        let warn = c < max;
        (Some(format!("{count} / {max} places")), warn, warn, if warn { max - c } else { 0 }, max)
    } else {
        (None, false, false, 0, 0)
    };
    let deadline = inv.registration_deadline.clone();
    (true, mode_label, validation_label, inv_label, spots, spots_w, deadline, unfilled, remaining, total)
}

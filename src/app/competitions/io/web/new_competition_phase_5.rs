use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::routes::Routes;
use crate::app::competitions::use_cases::finalize_competition::{FinalizeCompetitionCommand, FinalizeCompetitionError, execute as execute_finalize};
use crate::app::shared_kernel::common_types::{CompetitionId, SpaceId};
use crate::state::AppState;
use crate::web::app_layout::AppLayout;

/// All display data is pre-formatted in the handler — the template uses no filters.
#[derive(Template)]
#[template(path = "new-competition-phase-5.html")]
pub struct NewCompetitionPhase5Template {
    pub competition_routes:   Routes,
    pub space_id:             String,
    pub competition_id:       String,
    // General info
    pub competition_name:     String,
    pub competition_logo:     Option<String>,
    pub admin_names:          Vec<String>,
    // Unfilled spots warning
    pub has_unfilled_spots:   bool,
    pub remaining_spots:      u32,
    pub total_spots:          u32,
    // Rules section
    pub has_rules:            bool,
    pub ranking_points_label: String,
    pub bonus_label:          Option<String>,
    pub tiers_label:          Option<String>,
    pub rosters_preview:      Vec<String>,
    pub rosters_extra:        usize,
    // Structure section
    pub has_structure:        bool,
    pub groups_label:         Option<String>,
    pub playoffs_label:       Option<String>,
    pub dates_label:          Option<String>,
    // Invitations section
    pub has_invitations:      bool,
    pub access_mode_label:    String,
    pub invited_label:        String,
    pub spots_label:          Option<String>,
    pub spots_warn:           bool,
    pub deadline_label:       Option<String>,
}

impl IntoResponse for NewCompetitionPhase5Template {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_new_competition_phase_5(
    Path((space_id, competition_id)): Path<(String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let repo = state.competitions.competition_repository.as_ref();

    let (base_info, rules, structure, invitations) = tokio::join!(
        repo.find_base_info(&cid),
        repo.find_rules(&cid),
        repo.find_structure(&cid),
        repo.find_invitations(&cid),
    );

    let base = match base_info {
        Ok(Some(b)) => b,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("phase 5 find_base_info for {competition_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let rules       = rules.unwrap_or(None);
    let structure   = structure.unwrap_or(None);
    let invitations = invitations.unwrap_or(None);

    // ── Rules ────────────────────────────────────────────────────────────────
    let (has_rules, ranking_points_label, bonus_label, tiers_label, rosters_preview, rosters_extra) =
        if let Some(r) = &rules {
            let rr = &r.ranking_rules;
            let pts = format!(
                "Victoire = {} pts · Nul = {} pt · Défaite = {} pt",
                rr.win_points, rr.draw_points, rr.lose_points
            );

            let bonus: Option<String> = {
                let off = if rr.offensive_bonus.activated {
                    Some(format!("+{} si ≥ {} TDs", rr.offensive_bonus.points, rr.offensive_bonus.diff_td))
                } else { None };
                let def = if rr.defensive_bonus.activated {
                    Some(format!("+{} si ≤ 1 TD encaissé", rr.defensive_bonus.points))
                } else { None };
                match (off, def) {
                    (Some(o), Some(d)) => Some(format!("Offensif ({o}) · Défensif ({d})")),
                    (Some(o), None)    => Some(format!("Offensif ({o})")),
                    (None, Some(d))    => Some(format!("Défensif ({d})")),
                    (None, None)       => None,
                }
            };

            let tiers: Option<String> = if r.tiers.is_empty() { None } else {
                let names = r.tiers.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ");
                Some(format!("{} tier{} : {names}", r.tiers.len(), if r.tiers.len() > 1 { "s" } else { "" }))
            };

            let all_rosters: Vec<String> = r.tiers.iter()
                .flat_map(|t| t.rosters.clone())
                .collect();
            let extra   = all_rosters.len().saturating_sub(12);
            let preview = all_rosters.into_iter().take(12).collect::<Vec<_>>();

            (true, pts, bonus, tiers, preview, extra)
        } else {
            (false, String::new(), None, None, vec![], 0)
        };

    // ── Structure ────────────────────────────────────────────────────────────
    let (has_structure, groups_label, playoffs_label, dates_label) =
        if let Some(s) = &structure {
            let groups: Option<String> = if s.ranking_group.use_ranking_groups {
                let n = s.ranking_group.ranking_groups.len();
                Some(format!("{n} poule{}", if n > 1 { "s" } else { "" }))
            } else { None };

            let playoffs: Option<String> = if s.play_offs_phase.use_playoffs_phase {
                let q = s.play_offs_phase.qualified_team_per_pool;
                let third = if s.play_offs_phase.final_phase_match_for_third_place {
                    " · Match 3e place"
                } else { "" };
                Some(format!("Top {q} par poule{third}"))
            } else { None };

            let dates: Option<String> = if s.schedule.use_schedule {
                Some(format!("{} → {}", s.schedule.schedule_start_date, s.schedule.schedule_end_date))
            } else { None };

            (true, groups, playoffs, dates)
        } else {
            (false, None, None, None)
        };

    // ── Invitations ──────────────────────────────────────────────────────────
    let (has_invitations, access_mode_label, invited_label, spots_label, spots_warn, deadline_label,
         has_unfilled_spots, remaining_spots, total_spots) =
        if let Some(inv) = &invitations {
            let mode_label = match inv.access_mode.as_str() {
                "public_link" => "Lien public",
                "open"        => "Inscription libre",
                _             => "Sur invitation (liste fermée)",
            }.to_string();

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

            (true, mode_label, inv_label, spots, spots_w, deadline, unfilled, remaining, total)
        } else {
            (false, String::new(), String::new(), None, false, None, false, 0, 0)
        };

    let tmpl = NewCompetitionPhase5Template {
        competition_routes: Routes,
        space_id,
        competition_id,
        competition_name: base.name,
        competition_logo: base.logo,
        admin_names: base.admin_names,
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
        invited_label,
        spots_label,
        spots_warn,
        deadline_label,
    };

    if headers.contains_key("hx-request") {
        tmpl.into_response()
    } else {
        let content = tmpl.render().unwrap_or_default();
        AppLayout { content, routes: Default::default() }.into_response()
    }
}

// ── POST: finalisation ────────────────────────────────────────────────────────

pub async fn post_finalize_competition(
    auth_session: AuthSession,
    Path((space_id, competition_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return (StatusCode::UNAUTHORIZED, "Non authentifié.").into_response();
    };

    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, "Identifiant de compétition invalide.").into_response(),
    };

    let sid = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, "Identifiant d'espace invalide.").into_response(),
    };

    let cmd = FinalizeCompetitionCommand {
        competition_id: cid,
        space_id:       sid,
        finalized_by:   user.id,
    };

    match execute_finalize(cmd, state.competitions.competition_repository.as_ref(), &state.event_bus).await {
        Ok(()) => Response::builder()
            .header("HX-Redirect", Routes.all_competitions(&space_id))
            .body(Body::empty())
            .unwrap(),

        Err(FinalizeCompetitionError::CompetitionNotFound) =>
            (StatusCode::NOT_FOUND, "Compétition introuvable.").into_response(),

        Err(FinalizeCompetitionError::Database(_)) =>
            (StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne, veuillez réessayer.").into_response(),
    }
}
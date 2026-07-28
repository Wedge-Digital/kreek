use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::competition_invitations::AccessMode;
use crate::app::competitions::use_cases::finalize_competition::{
    execute as execute_finalize, FinalizeCompetitionCommand, FinalizeCompetitionError,
};
use crate::app::competitions::io::web::rules_labels::format_bonus_label;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use crate::state::AppState;
use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "new-competition-phase-5.html")]
pub struct NewCompetitionPhase5Template {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    // General info
    pub competition_name: String,
    pub competition_logo: Option<String>,
    pub admin_names: Vec<String>,
    pub season_name: String,
    // Unfilled spots warning
    pub has_unfilled_spots: bool,
    pub remaining_spots: u32,
    pub total_spots: u32,
    // Rules section
    pub has_rules: bool,
    pub ranking_points_label: String,
    pub bonus_label: Option<String>,
    pub tiers_label: Option<String>,
    pub rosters_preview: Vec<String>,
    pub rosters_extra: usize,
    // Structure section
    pub has_structure: bool,
    pub groups_label: Option<String>,
    pub playoffs_label: Option<String>,
    pub dates_label: Option<String>,
    // Invitations section
    pub has_invitations: bool,
    pub access_mode_label: String,
    pub validation_label: String,
    pub invited_label: String,
    pub spots_label: Option<String>,
    pub spots_warn: bool,
    pub deadline_label: Option<String>,
}

impl IntoResponse for NewCompetitionPhase5Template {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_new_competition_phase_5(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let comp_repo = state.competitions.competition_repository.as_ref();
    let season_repo = state.competitions.season_repository.as_ref();

    let (base_info, season_info, rules, structure, invitations) = tokio::join!(
        comp_repo.find_base_info(&cid),
        season_repo.find_base_info(&sid),
        season_repo.find_rules(&sid),
        season_repo.find_structure(&sid),
        season_repo.find_invitations(&sid),
    );

    let base = match base_info {
        Ok(Some(b)) => b,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("phase 5 find_base_info for {competition_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let season_name = match season_info {
        Ok(Some(s)) => s.name,
        Ok(None) => String::new(),
        Err(e) => {
            tracing::warn!("phase5 find_base_info season {season_id}: {e}");
            String::new()
        }
    };
    let rules = match rules {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("phase5 find_rules season {season_id}: {e}");
            None
        }
    };
    let structure = match structure {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("phase5 find_structure season {season_id}: {e}");
            None
        }
    };
    let invitations = match invitations {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("phase5 find_invitations season {season_id}: {e}");
            None
        }
    };

    // ── Rules ────────────────────────────────────────────────────────────────
    let (has_rules, ranking_points_label, bonus_label, tiers_label, rosters_preview, rosters_extra) =
        if let Some(r) = &rules {
            let rr = &r.ranking_rules;
            let pts = format!(
                "Victoire = {} pts · Nul = {} pt · Défaite = {} pt",
                rr.win_points, rr.draw_points, rr.lose_points
            );

            let bonus = format_bonus_label(rr);

            let tiers: Option<String> = if r.tiers.is_empty() {
                None
            } else {
                let names = r
                    .tiers
                    .iter()
                    .map(|t| t.name.as_ref())
                    .collect::<Vec<_>>()
                    .join(", ");
                Some(format!(
                    "{} tier{} : {names}",
                    r.tiers.len(),
                    if r.tiers.len() > 1 { "s" } else { "" }
                ))
            };

            let all_rosters: Vec<String> = r.tiers.iter().flat_map(|t| t.rosters.clone()).collect();
            let extra = all_rosters.len().saturating_sub(12);
            let preview = all_rosters.into_iter().take(12).collect::<Vec<_>>();

            (true, pts, bonus, tiers, preview, extra)
        } else {
            (false, String::new(), None, None, vec![], 0)
        };

    // ── Structure ────────────────────────────────────────────────────────────
    let (has_structure, groups_label, playoffs_label, dates_label) = if let Some(s) = &structure {
        let groups: Option<String> = if s.ranking_group.use_ranking_groups.0 {
            let n = s.ranking_group.ranking_groups.len();
            Some(format!("{n} poule{}", if n > 1 { "s" } else { "" }))
        } else {
            None
        };

        let playoffs: Option<String> = if s.play_offs_phase.use_playoffs_phase.0 {
            let q = s.play_offs_phase.qualified_team_per_pool;
            let third = if s.play_offs_phase.final_phase_match_for_third_place.0 {
                " · Match 3e place"
            } else {
                ""
            };
            Some(format!("Top {q} par poule{third}"))
        } else {
            None
        };

        let dates: Option<String> = if s.schedule.use_schedule.0 {
            Some(format!(
                "{} → {}",
                s.schedule.schedule_start_date, s.schedule.schedule_end_date
            ))
        } else {
            None
        };

        (true, groups, playoffs, dates)
    } else {
        (false, None, None, None)
    };

    // ── Invitations ──────────────────────────────────────────────────────────
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
    ) = if let Some(inv) = &invitations {
        let mode_label = match inv.access_mode {
            AccessMode::Open => "Inscription libre",
            AccessMode::Invitation => "Sur invitation (liste fermée)",
        }
        .to_string();

        let validation_label = if inv.requires_validation.0 {
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
            (
                Some(format!("{count} / {max} places")),
                warn,
                warn,
                if warn { max - c } else { 0 },
                max,
            )
        } else {
            (None, false, false, 0, 0)
        };

        let deadline = inv.registration_deadline.clone();

        (
            true, mode_label, validation_label, inv_label, spots, spots_w, deadline, unfilled,
            remaining, total,
        )
    } else {
        (
            false,
            String::new(),
            String::new(),
            String::new(),
            None,
            false,
            None,
            false,
            0,
            0,
        )
    };

    NewCompetitionPhase5Template {
        app_routes: AppRoutes::default(),
        space_id,
        competition_id,
        season_id,
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
    }
    .into_response()
}

pub async fn post_finalize_competition(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return (StatusCode::UNAUTHORIZED, "Non authentifié.").into_response();
    };

    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                "Identifiant de compétition invalide.",
            )
                .into_response()
        }
    };

    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Identifiant de saison invalide.").into_response()
        }
    };

    let space = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Identifiant d'espace invalide.").into_response()
        }
    };

    let cmd = FinalizeCompetitionCommand {
        competition_id: cid,
        season_id: sid,
        space_id: space,
        finalized_by: user.id,
    };

    match execute_finalize(
        cmd,
        state.competitions.season_repository.as_ref(),
        &state.event_bus,
    )
    .await
    {
        Ok(()) => Response::builder()
            .header(
                "HX-Redirect",
                AppRoutes::default().competitions.competition_detail(&space_id, &competition_id, &season_id),
            )
            .body(Body::empty())
            .unwrap(),

        Err(FinalizeCompetitionError::SeasonNotFound) => {
            (StatusCode::NOT_FOUND, "Saison introuvable.").into_response()
        }

        Err(FinalizeCompetitionError::Database(_)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Erreur interne, veuillez réessayer.",
        )
            .into_response(),
    }
}

use crate::app::players::domain::match_impact::{InjuryType, PlayerParticipationStatus, StatKind};
use crate::app::players::domain::player::{Player, PlayerId};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

// ── View models ───────────────────────────────────────────────────────────────

pub struct InjuryRowVm {
    pub label:               String,
    pub round_label:         String,
    pub opponent_team_name:  String,
}

pub struct StatAdjustmentRowVm {
    pub stat:  &'static str,
    pub malus: u8,
}

pub struct PlayerDebugVm {
    pub id:                          String,
    pub team_id:                     String,
    pub position_name:               String,
    pub roster_line_id:              String,
    pub jersey:                      String,
    pub spp:                         u32,
    pub value_kpo:                   u32,
    pub version:                     i32,
    pub participation_status:        &'static str,
    pub career_touchdowns:           u16,
    pub career_passes:               u16,
    pub career_interceptions:        u16,
    pub career_casualties:           u16,
    pub career_mvps:                 u16,
    pub career_fouls:                u16,
    pub career_persistent_injuries:  u16,
    pub injuries:                    Vec<InjuryRowVm>,
    pub stat_adjustments:            Vec<StatAdjustmentRowVm>,
}

fn participation_status_label(status: PlayerParticipationStatus) -> &'static str {
    match status {
        PlayerParticipationStatus::Available => "Available",
        PlayerParticipationStatus::MissingNextGame => "MissingNextGame",
        PlayerParticipationStatus::Retired => "Retired",
        PlayerParticipationStatus::Dead => "Dead",
    }
}

fn injury_label(injury: &InjuryType) -> String {
    match injury {
        InjuryType::Commotion => "Commotion".to_string(),
        InjuryType::Amoche => "Amoché".to_string(),
        InjuryType::BlessureSerieuse => "Blessure sérieuse".to_string(),
        InjuryType::Sequel { stat } => format!("Séquelle ({})", stat_label(*stat)),
        InjuryType::Mort => "Mort".to_string(),
    }
}

fn stat_label(stat: StatKind) -> &'static str {
    match stat {
        StatKind::Ma => "MA",
        StatKind::St => "ST",
        StatKind::Ag => "AG",
        StatKind::Pa => "PA",
        StatKind::Av => "AV",
    }
}

impl From<Player> for PlayerDebugVm {
    fn from(p: Player) -> Self {
        let injuries = p.injuries.iter().map(|r| InjuryRowVm {
            label:              injury_label(&r.injury_type),
            round_label:        r.context.round_label.clone(),
            opponent_team_name: r.context.opponent_team_name.clone(),
        }).collect();
        let stat_adjustments = p.stat_adjustments.iter().map(|a| StatAdjustmentRowVm {
            stat:  stat_label(a.stat),
            malus: a.malus.into_inner(),
        }).collect();

        Self {
            id:                         p.id.0,
            team_id:                    p.team_id.0,
            position_name:              p.position_name.to_string(),
            roster_line_id:             p.roster_line_id.as_ref().to_string(),
            jersey:                     p.jersey.map(|j| j.into_inner().to_string()).unwrap_or_else(|| "—".to_string()),
            spp:                        p.spp.0,
            value_kpo:                  p.value.0,
            version:                    p.version,
            participation_status:       participation_status_label(p.participation_status),
            career_touchdowns:          p.career_touchdowns.0,
            career_passes:              p.career_passes.0,
            career_interceptions:       p.career_interceptions.0,
            career_casualties:          p.career_casualties.0,
            career_mvps:                p.career_mvps.0,
            career_fouls:               p.career_fouls.0,
            career_persistent_injuries: p.career_persistent_injuries.0,
            injuries,
            stat_adjustments,
        }
    }
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "player-debug.html")]
pub struct PlayerDebugTemplate {
    pub vm: PlayerDebugVm,
}

impl IntoResponse for PlayerDebugTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("player_debug template render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn player_debug_controller(
    Path((_space_id, player_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let player = state.players.repository.find_by_id(&PlayerId(player_id)).await;
    match player {
        Ok(Some(p)) => PlayerDebugTemplate { vm: p.into() }.into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("player_debug_controller find_by_id: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

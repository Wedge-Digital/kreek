use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::domain::value_objects::{
    ActionId, ActionPlayer, InjuryType, MatchActionType, SequelStat, TeamSide, TempPlayerId,
    TurnNumber,
};
use crate::app::match_report::use_cases::delete_action_use_case::{self, DeleteActionCommand};
use crate::app::match_report::use_cases::record_action_use_case::{self, RecordActionCommand};
use crate::app::shared_kernel::bloodbowl::ids::{MatchReportId, PlayerId};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RecordActionForm {
    pub turn: u8,
    pub player_id: String,
    pub player_type: String,
    pub action_type: String,
    pub injury_type: Option<String>,
    pub sequel_stat: Option<String>,
}

pub async fn post_action_step3(
    auth_session: AuthSession,
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<RecordActionForm>,
) -> Response {
    let _ = space_id;
    post_action(auth_session, mr_id, TeamSide::Home, form, state).await
}

pub async fn post_action_step4(
    auth_session: AuthSession,
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<RecordActionForm>,
) -> Response {
    let _ = space_id;
    post_action(auth_session, mr_id, TeamSide::Away, form, state).await
}

async fn post_action(
    auth_session: AuthSession,
    mr_id: String,
    side: TeamSide,
    form: RecordActionForm,
    state: AppState,
) -> Response {
    let user = match auth_session.user {
        Some(u) => u,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let mr_id_vo = match MatchReportId::try_new(&mr_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let turn = match TurnNumber::try_new(form.turn) {
        Ok(t) => t,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let player = match build_player(&form.player_id, &form.player_type) {
        Some(p) => p,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    let action = match build_action_type(
        &form.action_type,
        form.injury_type.as_deref(),
        form.sequel_stat.as_deref(),
    ) {
        Some(a) => a,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    let cmd = RecordActionCommand {
        match_report_id: mr_id_vo,
        team_side: side,
        turn,
        player,
        action,
        recorded_by: user.id,
    };
    match record_action_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        state.match_report.player_data.as_ref(),
    )
    .await
    {
        Ok(outcome) => {
            let trigger = format!(
                r#"{{"actionRecorded": {{"action_id": "{}"}}}}"#,
                outcome.action_id
            );
            Response::builder()
                .header("HX-Trigger", trigger)
                .body(axum::body::Body::empty())
                .unwrap()
        }
        Err(e) => {
            tracing::error!("post_action: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_action(
    auth_session: AuthSession,
    Path((space_id, mr_id, action_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    let _ = space_id;
    let user = match auth_session.user {
        Some(u) => u,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let mr_id_vo = match MatchReportId::try_new(&mr_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let cmd = DeleteActionCommand {
        match_report_id: mr_id_vo,
        action_id: ActionId(action_id),
        deleted_by: user.id,
    };
    match delete_action_use_case::execute(cmd, state.match_report.match_report_repo.as_ref()).await
    {
        Ok(_) => Response::builder()
            .header("HX-Trigger", r#"{"actionDeleted": {}}"#)
            .body(axum::body::Body::empty())
            .unwrap(),
        Err(e) => {
            tracing::error!("delete_action: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn build_player(player_id: &str, player_type: &str) -> Option<ActionPlayer> {
    match player_type {
        "regular" => Some(ActionPlayer::Regular(PlayerId::try_new(player_id).ok()?)),
        "temp" => Some(ActionPlayer::Temp(TempPlayerId(player_id.to_string()))),
        _ => None,
    }
}

fn build_action_type(
    action_type: &str,
    injury: Option<&str>,
    sequel: Option<&str>,
) -> Option<MatchActionType> {
    match action_type {
        "TOUCHDOWN" => Some(MatchActionType::Touchdown),
        "PASSE" => Some(MatchActionType::Passe),
        "INTERCEPTION" => Some(MatchActionType::Interception),
        "AGRESSION" => Some(MatchActionType::Agression),
        "LANCER" => Some(MatchActionType::Lancer),
        "SORTIE" => Some(MatchActionType::Sortie),
        "MVP" => Some(MatchActionType::Mvp),
        // La Haine reste vide ici : sa saisie et sa validation arrivent avec la
        // carte 401, qui a besoin du port de référence que ce parseur n'a pas.
        "BLESSE" => Some(MatchActionType::Blesse {
            injury: build_injury(injury?, sequel)?,
            hatred: None,
            hatred_skill_uid: None,
        }),
        _ => None,
    }
}

fn build_injury(injury: &str, sequel: Option<&str>) -> Option<InjuryType> {
    match injury {
        "COMMOTION" => Some(InjuryType::Commotion),
        "AMOCHE" => Some(InjuryType::Amoche),
        "BLESSURE_SERIEUSE" => Some(InjuryType::BlessureSerieuse),
        "MORT" => Some(InjuryType::Mort),
        "SEQUEL" => Some(InjuryType::Sequel {
            stat: build_sequel(sequel?)?,
        }),
        _ => None,
    }
}

fn build_sequel(stat: &str) -> Option<SequelStat> {
    match stat {
        "AV" => Some(SequelStat::MinusAv),
        "MA" => Some(SequelStat::MinusMa),
        "PA" => Some(SequelStat::MinusPa),
        "AG" => Some(SequelStat::MinusAg),
        "ST" => Some(SequelStat::MinusSt),
        _ => None,
    }
}

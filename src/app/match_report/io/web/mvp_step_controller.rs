//! L'étape MVP, saisie manuelle (carte 637) : `GET/POST /{id}/mvp/{side}`.
//!
//! Domicile, puis visiteur, un joueur choisi directement. Le MVP s'inscrit au
//! journal comme une action `Mvp` au tour 16 ; la règle — un par camp — est
//! celle du domaine, la page ne fait que la montrer.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::mvp_draw::{DrawnMvp, DRAW_SIZE};
use crate::app::match_report::domain::value_objects::{MatchAction, TeamSide};
use crate::app::match_report::io::web::record_action_controller::build_player;
use crate::app::match_report::io::web::write_conflict::conflict_response;
use crate::app::match_report::ports::TeamInfoDto;
use crate::app::match_report::use_cases::record_mvp_use_case::{
    self, RecordMvpCommand, RecordMvpError,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;

/// Un MVP déjà saisi pour le camp, avec de quoi le supprimer — sauf s'il a
/// été tiré au sort : le tirage est définitif (carte 638).
pub struct MvpRowVm {
    pub player_name: String,
    pub player_position: String,
    pub delete_url: Option<String>,
    /// « Thorgrim, Borin, Durin, Balin, Nori et Gimli » pour un MVP tiré.
    pub drawn_among: Option<String>,
}

/// L'équipe dont c'est l'étape.
pub struct MvpTeamVm {
    pub name: String,
    pub initials: String,
    pub logo_url: Option<String>,
}

/// Le MVP du domicile, rappelé replié en tête de l'étape du visiteur.
pub struct HomeMvpRecapVm {
    pub team_name: String,
    pub player_names: Vec<String>,
}

/// Retour et suite de l'étape.
pub struct MvpNavVm {
    pub back_url: String,
    pub back_label: String,
    pub next_url: String,
    pub next_label: String,
}

#[derive(Template)]
#[template(path = "mvp-step.html")]
pub struct MvpStepTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub is_home: bool,
    pub team: MvpTeamVm,
    pub mvps: Vec<MvpRowVm>,
    pub home_recap: Option<HomeMvpRecapVm>,
    pub player_selector_url: String,
    pub temp_player_selector_url: String,
    pub post_url: String,
    /// Le tirage au sort, l'onglet coché par défaut (carte 638).
    pub draw_url: String,
    pub draw_size: usize,
    pub nav: MvpNavVm,
}

impl IntoResponse for MvpStepTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("mvp-step.html: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[derive(Deserialize)]
pub struct RecordMvpForm {
    pub player_id: String,
    pub player_type: String,
}

/// Les identifiants d'une page : espace, compte rendu.
struct PageIds<'a> {
    space_id: &'a str,
    mr_id: &'a str,
}

pub async fn get_mvp_step(
    Path((space_id, mr_id, side)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Some(side) = parse_side(&side) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let pm = match load_pre_match(&state, &mr_id).await {
        Ok(pm) => pm,
        Err(status) => return status.into_response(),
    };
    let ids = PageIds {
        space_id: &space_id,
        mr_id: &mr_id,
    };
    if let Some(redirect) = home_mvp_first(&ids, side, &pm) {
        return redirect;
    }
    let teams = load_teams(&state, &pm).await;
    build_template(&ids, side, &pm, &teams).into_response()
}

/// Le visiteur attend que le domicile ait son MVP (carte 638) : le domaine
/// dit quel camp en manque, la page renvoie à son étape.
fn home_mvp_first(ids: &PageIds<'_>, side: TeamSide, pm: &MatchReportPreMatch) -> Option<Response> {
    let home_missing = pm.first_side_without_mvp() == Some(TeamSide::Home);
    (side == TeamSide::Away && home_missing).then(|| {
        let url = AppRoutes::default()
            .match_report
            .mvp(ids.space_id, ids.mr_id, "home");
        Redirect::to(&url).into_response()
    })
}

/// Les deux équipes : celle de l'étape, et le domicile ou le visiteur que la
/// page nomme.
async fn load_teams(state: &AppState, pm: &MatchReportPreMatch) -> (TeamInfoDto, TeamInfoDto) {
    let (home_id, away_id) = (pm.home_team_id.to_string(), pm.away_team_id.to_string());
    let team_data = state.match_report.team_data.as_ref();
    let (home, away) = tokio::join!(
        team_data.find_team_info(&home_id),
        team_data.find_team_info(&away_id),
    );
    (home.unwrap_or_default(), away.unwrap_or_default())
}

pub async fn post_mvp_step(
    auth_session: AuthSession,
    Path((space_id, mr_id, side)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Form(form): Form<RecordMvpForm>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(cmd) = build_command(&mr_id, &side, &form, user.id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let next_url = next_url(&space_id, &mr_id, cmd.team_side);
    let repo = state.match_report.match_report_repo.as_ref();
    let (players, bus) = (
        &state.match_report.player_data,
        &state.match_report.event_bus,
    );
    match record_mvp_use_case::execute(cmd, repo, players.as_ref(), bus).await {
        Ok(()) => (StatusCode::OK, [("HX-Redirect", next_url)]).into_response(),
        Err(e) => error_response(e),
    }
}

pub(crate) fn parse_side(side: &str) -> Option<TeamSide> {
    match side {
        "home" => Some(TeamSide::Home),
        "away" => Some(TeamSide::Away),
        _ => None,
    }
}

pub(crate) fn side_slug(side: TeamSide) -> &'static str {
    match side {
        TeamSide::Home => "home",
        TeamSide::Away => "away",
    }
}

fn build_command(
    mr_id: &str,
    side: &str,
    form: &RecordMvpForm,
    recorded_by: crate::app::shared_kernel::identity::ids::CoachId,
) -> Option<RecordMvpCommand> {
    Some(RecordMvpCommand {
        match_report_id: MatchReportId::try_new(mr_id).ok()?,
        team_side: parse_side(side)?,
        player: build_player(&form.player_id, &form.player_type)?,
        recorded_by,
    })
}

/// Un compte rendu en cours, ou prêt à publier — l'étape sert aussi à la
/// correction, comme la saisie des actions.
async fn load_pre_match(state: &AppState, mr_id: &str) -> Result<MatchReportPreMatch, StatusCode> {
    let found = state.match_report.match_report_repo.find_by_id(mr_id).await;
    match found {
        Ok(Some(MatchReportState::PreMatch(pm))) => Ok(pm),
        Ok(Some(MatchReportState::ReadyToPublish(rtp))) => Ok(rtp.into_pre_match()),
        Ok(_) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("get_mvp_step find_by_id {mr_id}: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn build_template(
    ids: &PageIds<'_>,
    side: TeamSide,
    pm: &MatchReportPreMatch,
    (home, away): &(TeamInfoDto, TeamInfoDto),
) -> MvpStepTemplate {
    let routes = AppRoutes::default();
    let (team_info, team_id) = match side {
        TeamSide::Home => (home, pm.home_team_id.to_string()),
        TeamSide::Away => (away, pm.away_team_id.to_string()),
    };
    MvpStepTemplate {
        is_home: side == TeamSide::Home,
        team: team_vm(team_info),
        mvps: mvp_rows(ids, pm, side),
        home_recap: home_recap(side, pm, &home.team_name),
        player_selector_url: routes.players.match_player_selector(ids.space_id, &team_id),
        temp_player_selector_url: temp_players_url(ids, side),
        post_url: routes
            .match_report
            .mvp(ids.space_id, ids.mr_id, side_slug(side)),
        draw_url: routes
            .match_report
            .mvp_draw(ids.space_id, ids.mr_id, side_slug(side)),
        draw_size: DRAW_SIZE,
        nav: nav_for(ids, side, &away.team_name),
        app_routes: routes,
        space_id: ids.space_id.to_string(),
        match_report_id: ids.mr_id.to_string(),
    }
}

fn team_vm(info: &TeamInfoDto) -> MvpTeamVm {
    MvpTeamVm {
        name: info.team_name.clone(),
        initials: initials_from(&info.team_name),
        logo_url: info.logo_url.clone(),
    }
}

fn initials_from(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

fn mvp_rows(ids: &PageIds<'_>, pm: &MatchReportPreMatch, side: TeamSide) -> Vec<MvpRowVm> {
    pm.mvps_of(side)
        .into_iter()
        .map(|a| mvp_row(ids, a, pm.drawn_mvp(&a.id)))
        .collect()
}

/// Un MVP tiré au sort n'a pas de ✕ : le domaine refuse sa suppression.
fn mvp_row(ids: &PageIds<'_>, action: &MatchAction, drawn: Option<&DrawnMvp>) -> MvpRowVm {
    let routes = AppRoutes::default().match_report;
    MvpRowVm {
        player_name: action.player_display_name.clone(),
        player_position: action.player_position.clone(),
        delete_url: drawn
            .is_none()
            .then(|| routes.delete_action(ids.space_id, ids.mr_id, &action.id.0)),
        drawn_among: drawn.map(|d| names_sentence(&d.designated_names())),
    }
}

/// « A, B, C et D » : la liste des désignés, telle que l'écran la lit.
fn names_sentence(names: &[&str]) -> String {
    match names.split_last() {
        None => String::new(),
        Some((last, [])) => last.to_string(),
        Some((last, rest)) => format!("{} et {last}", rest.join(", ")),
    }
}

/// Au visiteur seulement : on ne revient pas en arrière pour vérifier le
/// domicile.
fn home_recap(side: TeamSide, pm: &MatchReportPreMatch, home_name: &str) -> Option<HomeMvpRecapVm> {
    (side == TeamSide::Away).then(|| HomeMvpRecapVm {
        team_name: home_name.to_string(),
        player_names: pm
            .mvps_of(TeamSide::Home)
            .iter()
            .map(|a| a.player_display_name.clone())
            .collect(),
    })
}

fn temp_players_url(ids: &PageIds<'_>, side: TeamSide) -> String {
    let routes = AppRoutes::default().match_report;
    match side {
        TeamSide::Home => routes.step3_temp_players(ids.space_id, ids.mr_id),
        TeamSide::Away => routes.step4_temp_players(ids.space_id, ids.mr_id),
    }
}

/// Domicile : retour au match en direct (carte 636), suite au visiteur. Visiteur : retour au
/// domicile, suite aux gains et fans.
fn nav_for(ids: &PageIds<'_>, side: TeamSide, away_name: &str) -> MvpNavVm {
    let routes = AppRoutes::default().match_report;
    let (back_url, back_label, next_label) = match side {
        TeamSide::Home => (
            routes.live(ids.space_id, ids.mr_id),
            "Match".to_string(),
            format!("MVP : {away_name}"),
        ),
        TeamSide::Away => (
            routes.mvp(ids.space_id, ids.mr_id, "home"),
            "MVP domicile".to_string(),
            "Gains et fans".to_string(),
        ),
    };
    MvpNavVm {
        back_url,
        back_label,
        next_url: next_url(ids.space_id, ids.mr_id, side),
        next_label,
    }
}

fn next_url(space_id: &str, mr_id: &str, side: TeamSide) -> String {
    let routes = AppRoutes::default().match_report;
    match side {
        TeamSide::Home => routes.mvp(space_id, mr_id, "away"),
        TeamSide::Away => routes.step5(space_id, mr_id),
    }
}

/// Les refus de l'étape MVP, saisie manuelle comme tirage au sort (cartes
/// 637 et 638).
pub(crate) fn error_response(e: RecordMvpError) -> Response {
    match e {
        RecordMvpError::Conflict => conflict_response(),
        RecordMvpError::Domain(err) => domain_refusal(err),
        RecordMvpError::NotFound => StatusCode::NOT_FOUND.into_response(),
        RecordMvpError::PlayerNotFound(_) | RecordMvpError::TempPlayerNotFound(_) => {
            tracing::warn!("post_mvp_step: joueur refusé ({e:?})");
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
        }
        other => {
            tracing::error!("post_mvp_step: {other:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Un refus du domaine se dit en 422, avec son motif. Le second MVP d'un camp
/// vient d'une page périmée : elle se recharge sur le MVP qui existe. Moins de
/// six désignés, un doublon : le motif seul (carte 638).
fn domain_refusal(err: DomainError) -> Response {
    let status = StatusCode::UNPROCESSABLE_ENTITY;
    match err {
        DomainError::MvpAlreadyRecorded => {
            (status, [("HX-Refresh", "true")], err.to_string()).into_response()
        }
        _ => (status, err.to_string()).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seuls_home_et_away_sont_des_camps() {
        assert_eq!(parse_side("home"), Some(TeamSide::Home));
        assert_eq!(parse_side("away"), Some(TeamSide::Away));
        assert_eq!(parse_side("step5"), None);
    }

    /// L'enchaînement de la carte : `/mvp/home` → `/mvp/away` → `step5`.
    #[test]
    fn le_domicile_mene_au_visiteur_puis_aux_gains() {
        let (space, mr) = ("S", "M");
        assert!(next_url(space, mr, TeamSide::Home).ends_with("/M/mvp/away"));
        assert!(next_url(space, mr, TeamSide::Away).ends_with("/M/step5"));
    }

    #[test]
    fn le_visiteur_revient_au_domicile() {
        let ids = PageIds {
            space_id: "S",
            mr_id: "M",
        };
        assert!(nav_for(&ids, TeamSide::Away, "Rats")
            .back_url
            .ends_with("/M/mvp/home"));
        assert!(nav_for(&ids, TeamSide::Home, "Rats")
            .back_url
            .ends_with("/M/live"));
    }

    #[test]
    fn les_designes_se_lisent_en_une_phrase() {
        assert_eq!(
            names_sentence(&["Thorgrim", "Borin", "Durin", "Balin", "Nori", "Gimli"]),
            "Thorgrim, Borin, Durin, Balin, Nori et Gimli"
        );
        assert_eq!(names_sentence(&["Gimli"]), "Gimli");
    }

    /// Un MVP tiré n'a pas de ✕ et dit parmi qui ; un MVP saisi à la main
    /// garde son ✕.
    #[test]
    fn un_mvp_tire_n_a_pas_de_croix() {
        use crate::app::match_report::domain::mvp_draw::DesignatedPlayer;
        use crate::app::match_report::domain::value_objects::{
            ActionId, ActionPlayer, MatchActionType, TurnNumber,
        };
        use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
        let ids = PageIds {
            space_id: "S",
            mr_id: "M",
        };
        let action = MatchAction {
            id: ActionId("tire".into()),
            turn: TurnNumber::LAST,
            player: ActionPlayer::Regular(PlayerId::new()),
            action: MatchActionType::Mvp,
            player_display_name: "Balin".into(),
            player_position: "Coureur".into(),
        };
        let designated = ["Thorgrim", "Balin"]
            .iter()
            .map(|n| DesignatedPlayer {
                player: ActionPlayer::Regular(PlayerId::new()),
                display_name: n.to_string(),
                position: String::new(),
            })
            .collect();
        let drawn = DrawnMvp {
            action_id: action.id.clone(),
            team_side: TeamSide::Home,
            designated,
        };
        let row = mvp_row(&ids, &action, Some(&drawn));
        assert_eq!(row.delete_url, None);
        assert_eq!(row.drawn_among.as_deref(), Some("Thorgrim et Balin"));
        assert!(mvp_row(&ids, &action, None).delete_url.is_some());
    }

    #[test]
    fn un_tirage_incomplet_est_refuse_en_422() {
        let r = error_response(RecordMvpError::Domain(DomainError::DrawNeedsSixPlayers(5)));
        assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(!r.headers().contains_key("HX-Refresh"));
    }

    #[test]
    fn un_second_mvp_recharge_la_page() {
        let r = error_response(RecordMvpError::Domain(DomainError::MvpAlreadyRecorded));
        assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(r.headers().contains_key("HX-Refresh"));
    }
}

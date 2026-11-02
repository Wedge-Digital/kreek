//! Le match en direct (carte 636) : `GET /{id}/live`.
//!
//! Le match se saisit depuis cet écran de journal : le score, le journal des
//! deux camps, « ＋ Nouvelle action » et « Fin du match → MVP ». La nouvelle
//! action demande l'équipe dans la page, puis ouvre l'étape de saisie de ce
//! camp (`step3` ou `step4`).
//!
//! L'autre coach ouvre le même écran, **sans aucun bouton d'écriture** : la
//! route vit dans `routes_de_suivi()`, et c'est `can_write_report` — jamais le
//! gabarit — qui décide de ce qui s'affiche.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::domain::user::User;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::io::web::widgets::action_log_widget::{load_entry, load_scope};
use crate::app::match_report::ports::TeamInfoDto;
use crate::app::match_report::use_cases::match_report_access_service::{
    can_write_report, AccesRapportDeps, EntryScope,
};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};

/// Une équipe du bandeau de score.
pub struct LiveTeamVm {
    pub name: String,
    pub initials: String,
    pub logo_url: Option<String>,
}

impl LiveTeamVm {
    fn from_info(info: &TeamInfoDto) -> Self {
        Self {
            name: info.team_name.clone(),
            initials: initials_from(&info.team_name),
            logo_url: info.logo_url.clone(),
        }
    }
}

/// Le score et les sorties, tels que le domaine les compte.
pub struct LiveScoreVm {
    pub home: u8,
    pub away: u8,
    pub home_cas: u8,
    pub away_cas: u8,
}

impl LiveScoreVm {
    pub fn from_domain(pm: &MatchReportPreMatch) -> Self {
        let (home, away) = pm.compute_score();
        let (home_cas, away_cas) = pm.compute_cas();
        Self {
            home,
            away,
            home_cas,
            away_cas,
        }
    }
}

/// Les gestes d'écriture : la saisie d'un camp, et la fin du match.
pub struct LiveWriteVm {
    pub home_step_url: String,
    pub away_step_url: String,
    pub mvp_url: String,
}

/// Ce que l'utilisateur a le droit de faire sur ce match, établi par le
/// service d'accès.
pub struct LiveAccess {
    pub can_write: bool,
    /// Le nom du démarreur, montré à qui suit ; `None` s'il est inconnu.
    pub starter_name: Option<String>,
}

#[derive(Template)]
#[template(path = "live-match.html")]
pub struct LiveMatchTemplate {
    pub app_routes: AppRoutes,
    pub home: LiveTeamVm,
    pub away: LiveTeamVm,
    pub score: LiveScoreVm,
    pub action_log_url: String,
    /// `None` pour qui suit : aucun bouton d'écriture n'est rendu.
    pub write: Option<LiveWriteVm>,
    pub starter_name: Option<String>,
    /// Le marqueur de présence (carte 644) : le layout déclare ce match.
    pub space_id: String,
    pub match_report_id: String,
    /// Le widget « qui suit, qui saisit » du canal temps réel (carte 644).
    pub presence_widget_url: String,
}

impl IntoResponse for LiveMatchTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("live-match.html: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

/// Les identifiants d'une page : espace, compte rendu.
pub struct LiveIds<'a> {
    pub space_id: &'a str,
    pub mr_id: &'a str,
}

pub async fn get_live_match(
    auth_session: AuthSession,
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let ids = LiveIds {
        space_id: &space_id,
        mr_id: &mr_id,
    };
    match render_live(&state, &user, &ids).await {
        Ok(template) => template.into_response(),
        Err(response) => response,
    }
}

async fn render_live(
    state: &AppState,
    user: &User,
    ids: &LiveIds<'_>,
) -> Result<LiveMatchTemplate, Response> {
    let pm = load_match(state, ids).await?;
    let (home, away) = load_teams(state, &pm).await;
    let access = load_access(state, user, ids).await?;
    let teams = (LiveTeamVm::from_info(&home), LiveTeamVm::from_info(&away));
    Ok(build_template(
        ids,
        teams,
        LiveScoreVm::from_domain(&pm),
        access,
    ))
}

/// Le match en cours, ou prêt à publier — l'écran sert aussi à la correction.
/// Un brouillon n'a pas commencé : il retourne à son démarrage ; un rapport
/// publié se lit sur son récapitulatif.
async fn load_match(state: &AppState, ids: &LiveIds<'_>) -> Result<MatchReportPreMatch, Response> {
    let repo = state.match_report.match_report_repo.as_ref();
    match repo.find_by_id(ids.mr_id).await {
        Ok(Some(found)) => in_progress(found, ids),
        Ok(None) => Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("get_live_match find_by_id {}: {e}", ids.mr_id);
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

/// Le match à suivre, ou l'écran où mène un rapport qui n'est pas en cours.
fn in_progress(
    found: MatchReportState,
    ids: &LiveIds<'_>,
) -> Result<MatchReportPreMatch, Response> {
    let routes = AppRoutes::default().match_report;
    match found {
        MatchReportState::PreMatch(pm) => Ok(pm),
        MatchReportState::ReadyToPublish(rtp) => Ok(rtp.into_pre_match()),
        MatchReportState::Draft(_) => {
            Err(redirect(routes.edit_match_report(ids.space_id, ids.mr_id)))
        }
        MatchReportState::Published(_) => Err(redirect(routes.recap(ids.space_id, ids.mr_id))),
        MatchReportState::Cancelled(_) => Err(StatusCode::GONE.into_response()),
    }
}

fn redirect(url: String) -> Response {
    Redirect::to(&url).into_response()
}

async fn load_teams(state: &AppState, pm: &MatchReportPreMatch) -> (TeamInfoDto, TeamInfoDto) {
    let (home_id, away_id) = (pm.home_team_id.to_string(), pm.away_team_id.to_string());
    let team_data = state.match_report.team_data.as_ref();
    let (home, away) = tokio::join!(
        team_data.find_team_info(&home_id),
        team_data.find_team_info(&away_id),
    );
    (home.unwrap_or_default(), away.unwrap_or_default())
}

/// Le droit d'écrire, et, pour qui suit seulement, le nom du démarreur.
async fn load_access(
    state: &AppState,
    user: &User,
    ids: &LiveIds<'_>,
) -> Result<LiveAccess, Response> {
    let scope = load_scope(state, ids.mr_id)
        .await
        .map_err(|s| s.into_response())?;
    let entry = load_entry(state, ids.mr_id)
        .await
        .map_err(|s| s.into_response())?;
    let deps = AccesRapportDeps::from_state(state);
    let can_write = can_write_report(&deps, user, ids.space_id, &scope, &entry).await;
    let starter_name = match can_write {
        true => None,
        false => starter_name(state, &entry).await,
    };
    Ok(LiveAccess {
        can_write,
        starter_name,
    })
}

async fn starter_name(state: &AppState, entry: &EntryScope) -> Option<String> {
    let starter = entry.started_by.as_deref()?;
    state.match_report.coach_data.find_coach_name(starter).await
}

/// L'écran, assemblé sans dépôt : c'est lui que les tests éprouvent.
pub fn build_template(
    ids: &LiveIds<'_>,
    (home, away): (LiveTeamVm, LiveTeamVm),
    score: LiveScoreVm,
    access: LiveAccess,
) -> LiveMatchTemplate {
    let routes = AppRoutes::default();
    LiveMatchTemplate {
        action_log_url: routes.match_report.action_log(ids.space_id, ids.mr_id),
        write: access.can_write.then(|| write_vm(ids)),
        starter_name: access.starter_name,
        space_id: ids.space_id.to_string(),
        match_report_id: ids.mr_id.to_string(),
        presence_widget_url: routes.web.realtime_match_presence(ids.space_id, ids.mr_id),
        app_routes: routes,
        home,
        away,
        score,
    }
}

fn write_vm(ids: &LiveIds<'_>) -> LiveWriteVm {
    let routes = AppRoutes::default().match_report;
    LiveWriteVm {
        home_step_url: routes.step3(ids.space_id, ids.mr_id),
        away_step_url: routes.step4(ids.space_id, ids.mr_id),
        mvp_url: routes.mvp(ids.space_id, ids.mr_id, "home"),
    }
}

fn initials_from(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDS: LiveIds<'static> = LiveIds {
        space_id: "espace",
        mr_id: "rapport",
    };

    fn team(name: &str) -> LiveTeamVm {
        LiveTeamVm {
            name: name.to_string(),
            initials: initials_from(name),
            logo_url: None,
        }
    }

    fn template(can_write: bool, starter_name: Option<&str>) -> LiveMatchTemplate {
        let score = LiveScoreVm {
            home: 2,
            away: 1,
            home_cas: 1,
            away_cas: 0,
        };
        let access = LiveAccess {
            can_write,
            starter_name: starter_name.map(str::to_string),
        };
        build_template(
            &IDS,
            (team("Nains de Fer"), team("London Rats")),
            score,
            access,
        )
    }

    #[test]
    fn who_writes_gets_both_sides_and_the_end_of_the_match() {
        let write = template(true, None).write.expect("les gestes d'écriture");
        assert!(write.home_step_url.ends_with("/rapport/step3"));
        assert!(write.away_step_url.ends_with("/rapport/step4"));
        assert!(write.mvp_url.ends_with("/rapport/mvp/home"));
    }

    #[test]
    fn who_follows_gets_no_write_gesture() {
        assert!(template(false, Some("Yann")).write.is_none());
    }

    #[test]
    fn the_log_is_the_shared_two_sided_log() {
        assert!(template(false, None)
            .action_log_url
            .ends_with("/rapport/action-log"));
    }

    /// Le gabarit, et non seulement le VM : sans droit, aucun bouton rendu.
    #[test]
    fn the_rendered_page_for_who_writes() {
        let html = template(true, None).render().unwrap();
        assert!(html.contains("Nouvelle action"));
        assert!(html.contains("Fin du match"));
        assert!(html.contains("live-team-pick"));
        assert!(!html.contains("saisit tout le match"));
    }

    #[test]
    fn the_rendered_page_for_who_follows() {
        let html = template(false, Some("Yann")).render().unwrap();
        assert!(html.contains("Yann saisit tout le match"));
        assert!(!html.contains("Nouvelle action"));
        assert!(!html.contains("Fin du match"));
        assert!(!html.contains("/step3"));
    }

    #[test]
    fn an_unknown_starter_is_not_named() {
        let html = template(false, None).render().unwrap();
        assert!(html.contains("Ce match est saisi par le coach qui l'a d"));
    }

    #[test]
    fn the_score_is_the_domain_score() {
        let html = template(true, None).render().unwrap();
        assert!(html.contains(r#"<span class="live-score-value">2</span>"#));
        assert!(html.contains(r#"<span class="live-score-value">1</span>"#));
    }
}

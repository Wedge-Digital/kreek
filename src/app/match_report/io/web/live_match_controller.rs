//! Le match en direct (carte 636) : `GET /{id}/live`.
//!
//! Le match se saisit depuis cet écran de journal : le score, le journal des
//! deux camps, « ＋ Nouvelle action » et « Fin du match → MVP ». La nouvelle
//! action demande l'équipe dans la page, puis ouvre l'étape de saisie de ce
//! camp (`step3` ou `step4`).
//!
//! L'autre coach ouvre le même écran, **sans aucun bouton d'écriture** : la
//! route vit dans `routes_de_suivi()`, et c'est le droit d'écrire par camp —
//! jamais le gabarit — qui décide de ce qui s'affiche.
//!
//! En « Chacun son équipe » (carte 656), chaque coach n'a qu'un bouton, celui
//! de son équipe, et la fin du match mène à son MVP ; l'admin garde le choix
//! d'équipe. Avant le coup d'envoi, le coach est renvoyé à son avant-match.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::domain::user::User;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{EntryMode, TeamSide};
use crate::app::match_report::io::web::kick_off_redirect::kick_off_redirect;
use crate::app::match_report::io::web::mvp_step_controller::side_slug;
use crate::app::match_report::io::web::widgets::action_log_widget::{load_entry, load_scope};
use crate::app::match_report::ports::TeamInfoDto;
use crate::app::match_report::use_cases::match_report_access_service::{
    sides_writable_by, AccesRapportDeps, EntryScope,
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

/// Les gestes d'écriture : la saisie d'un camp, et la fin du match.
pub struct LiveWriteVm {
    pub home_step_url: String,
    pub away_step_url: String,
    /// En « Chacun son équipe », la saisie de **son** camp, sans choix
    /// d'équipe (carte 656) ; `None` pour qui écrit les deux camps.
    pub own_step: Option<OwnStepVm>,
    pub mvp_url: String,
}

/// Le bouton unique d'un coach en « Chacun son équipe ».
pub struct OwnStepVm {
    pub url: String,
    pub team_name: String,
}

/// Ce que l'utilisateur a le droit de faire sur ce match, établi par le
/// service d'accès.
pub struct LiveAccess {
    pub can_write: bool,
    /// Le camp d'un coach en « Chacun son équipe » ; `None` pour l'admin,
    /// qui écrit les deux, et en « Je saisis tout ».
    pub own_side: Option<TeamSide>,
    /// « Chacun son équipe » : la présence s'affiche à tous (carte 656).
    pub team_entry: bool,
    /// Le nom du démarreur, montré à qui suit ; `None` s'il est inconnu.
    pub starter_name: Option<String>,
}

#[derive(Template)]
#[template(path = "live-match.html")]
pub struct LiveMatchTemplate {
    pub app_routes: AppRoutes,
    pub home: LiveTeamVm,
    pub away: LiveTeamVm,
    /// Le score est un widget (carte 646) : la page n'en garde que le
    /// conteneur, rechargé sur le signal du canal temps réel.
    pub live_score_url: String,
    pub action_log_url: String,
    /// `None` pour qui suit : aucun bouton d'écriture n'est rendu.
    pub write: Option<LiveWriteVm>,
    pub team_entry: bool,
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
    let access = load_access(state, user, ids).await?;
    if let Some(side) = access.own_side {
        let page = (ids.space_id, ids.mr_id);
        if let Some(detour) = kick_off_redirect(state, page, &pm, side).await {
            return Err(detour);
        }
    }
    let (home, away) = load_teams(state, &pm).await;
    let teams = (LiveTeamVm::from_info(&home), LiveTeamVm::from_info(&away));
    Ok(build_template(ids, teams, access))
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

/// Le droit d'écrire par camp, et, pour qui suit seulement, le nom du
/// démarreur.
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
    let sides = sides_writable_by(&deps, user, ids.space_id, &scope, &entry).await;
    let team_entry = EntryMode::from_written(&entry.entry_mode) == Some(EntryMode::TeamEntry);
    let starter_name = match sides.is_empty() && !team_entry {
        true => starter_name(state, &entry).await,
        false => None,
    };
    Ok(access_from(&sides, team_entry, starter_name))
}

/// Un seul camp en « Chacun son équipe » : c'est le sien.
fn access_from(sides: &[TeamSide], team_entry: bool, starter_name: Option<String>) -> LiveAccess {
    let own_side = match (team_entry, sides) {
        (true, [side]) => Some(*side),
        _ => None,
    };
    LiveAccess {
        can_write: !sides.is_empty(),
        own_side,
        team_entry,
        starter_name,
    }
}

async fn starter_name(state: &AppState, entry: &EntryScope) -> Option<String> {
    let starter = entry.started_by.as_deref()?;
    state.match_report.coach_data.find_coach_name(starter).await
}

/// L'écran, assemblé sans dépôt : c'est lui que les tests éprouvent.
pub fn build_template(
    ids: &LiveIds<'_>,
    (home, away): (LiveTeamVm, LiveTeamVm),
    access: LiveAccess,
) -> LiveMatchTemplate {
    let routes = AppRoutes::default();
    LiveMatchTemplate {
        live_score_url: routes.match_report.live_score(ids.space_id, ids.mr_id),
        action_log_url: routes.match_report.action_log(ids.space_id, ids.mr_id),
        write: access
            .can_write
            .then(|| write_vm(ids, access.own_side, (&home, &away))),
        team_entry: access.team_entry,
        starter_name: access.starter_name,
        space_id: ids.space_id.to_string(),
        match_report_id: ids.mr_id.to_string(),
        presence_widget_url: routes.web.realtime_match_presence(ids.space_id, ids.mr_id),
        app_routes: routes,
        home,
        away,
    }
}

fn write_vm(
    ids: &LiveIds<'_>,
    own_side: Option<TeamSide>,
    (home, away): (&LiveTeamVm, &LiveTeamVm),
) -> LiveWriteVm {
    let routes = AppRoutes::default().match_report;
    let home_step_url = routes.step3(ids.space_id, ids.mr_id);
    let away_step_url = routes.step4(ids.space_id, ids.mr_id);
    let own_step = own_side.map(|side| match side {
        TeamSide::Home => OwnStepVm {
            url: home_step_url.clone(),
            team_name: home.name.clone(),
        },
        TeamSide::Away => OwnStepVm {
            url: away_step_url.clone(),
            team_name: away.name.clone(),
        },
    });
    let mvp_side = side_slug(own_side.unwrap_or(TeamSide::Home));
    LiveWriteVm {
        mvp_url: routes.mvp(ids.space_id, ids.mr_id, mvp_side),
        home_step_url,
        away_step_url,
        own_step,
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
        let access = LiveAccess {
            can_write,
            own_side: None,
            team_entry: false,
            starter_name: starter_name.map(str::to_string),
        };
        build_template(&IDS, (team("Nains de Fer"), team("London Rats")), access)
    }

    /// « Chacun son équipe » : un coach du camp `sides`, ou l'admin.
    fn team_entry_template(sides: &[TeamSide]) -> LiveMatchTemplate {
        let access = access_from(sides, true, None);
        build_template(&IDS, (team("Nains de Fer"), team("London Rats")), access)
    }

    #[test]
    fn a_team_entry_coach_gets_only_his_side_and_his_mvp() {
        let page = team_entry_template(&[TeamSide::Away]);
        let write = page.write.as_ref().expect("les gestes d'écriture");
        let own = write.own_step.as_ref().expect("son camp");
        assert!(own.url.ends_with("/rapport/step4"));
        assert_eq!(own.team_name, "London Rats");
        assert!(write.mvp_url.ends_with("/rapport/mvp/away"));
        let html = page.render().unwrap();
        assert!(html.contains("Action des London Rats"), "{html}");
        assert!(!html.contains("live-team-pick"));
        assert!(!html.contains("/step3"));
    }

    /// § 12.6 de E22 : l'admin écrit les deux camps, il garde le choix.
    #[test]
    fn the_admin_keeps_the_team_choice_in_team_entry() {
        let page = team_entry_template(&TeamSide::BOTH);
        assert!(page.write.as_ref().unwrap().own_step.is_none());
        assert!(page.render().unwrap().contains("live-team-pick"));
    }

    /// Les deux coachs voient qui est sur le match, sans le bandeau « X
    /// saisit tout le match ».
    #[test]
    fn both_team_entry_coaches_see_the_presence() {
        let html = team_entry_template(&[TeamSide::Home]).render().unwrap();
        assert!(html.contains("live-presence"));
        assert!(html.contains("Chacun saisit les actions de son"), "{html}");
        assert!(!html.contains("saisit tout le match"));
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

    /// Le score est un widget (carte 646) : la page n'en rend que le
    /// conteneur ; ses chiffres sont éprouvés dans `live_score_widget`.
    #[test]
    fn the_page_keeps_only_the_score_container() {
        let html = template(false, None).render().unwrap();
        assert!(html.contains(r#"hx-get="/app/espace/match-report/rapport/live/score""#));
        assert!(!html.contains("live-score-value"));
    }

    /// Décision 8 de E23 : le signal et `realtimeResync` remplacent
    /// l'interrogation à intervalle fixe, pour qui suit comme pour qui saisit.
    #[test]
    fn the_live_page_never_polls() {
        for can_write in [true, false] {
            let html = template(can_write, None).render().unwrap();
            assert!(!html.contains("every "), "can_write={can_write}");
        }
    }

    /// Le journal et le score écoutent le signal de **ce** match, la
    /// resynchronisation, et la suppression locale.
    #[test]
    fn both_widgets_listen_to_this_match_and_to_resync() {
        let html = template(true, None).render().unwrap();
        let trigger = "load, actionDeleted from:body, \
                       matchChanged[detail.id=='rapport'] from:body, \
                       realtimeResync from:body";
        assert_eq!(html.matches(trigger).count(), 2, "{html}");
    }
}

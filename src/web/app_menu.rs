use crate::app::auth::auth_backend::AuthSession;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveSection {
    Competitions,
    MatchReport,
    CreateTeam,
    MyTeams,
}

#[derive(Template, Default)]
#[template(path = "app-menu.html")]
pub struct AppMenu {
    pub app_routes: AppRoutes,
    pub space_name: Option<String>,
    pub space_id: Option<String>,
    pub space_logo: Option<String>,
    pub active_section: Option<ActiveSection>,
    /// Gouverne la seule entrée de menu qui n'est pas offerte à tous.
    pub peut_administrer: bool,
}

/// Le compte qui administre tous les espaces, quel que soit son profil.
///
/// Codé en dur, et dans la couche hôte : c'est une notion d'exploitation de
/// kreek, pas une règle du BC `spaces`, qui est extractible et n'a pas à
/// connaître de compte privilégié. Si la valeur devait varier d'un
/// environnement à l'autre, elle passerait en configuration — pas avant.
const COMPTE_EXPLOITANT: &str = "Bagouze";

impl AppMenu {
    fn competitions_active(&self) -> bool {
        matches!(self.active_section, Some(ActiveSection::Competitions))
    }

    fn match_report_active(&self) -> bool {
        matches!(self.active_section, Some(ActiveSection::MatchReport))
    }

    fn create_team_active(&self) -> bool {
        matches!(self.active_section, Some(ActiveSection::CreateTeam))
    }

    fn my_teams_active(&self) -> bool {
        matches!(self.active_section, Some(ActiveSection::MyTeams))
    }
}

impl IntoResponse for AppMenu {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

fn path_parts(current_url: &str) -> Option<Vec<&str>> {
    let path = match current_url.find("://") {
        Some(i) => current_url[i + 3..]
            .find('/')
            .map(|j| &current_url[i + 3 + j..])?,
        None => current_url,
    };
    let path = path.split('?').next()?.split('#').next()?;
    Some(path.trim_start_matches('/').split('/').collect())
}

fn extract_space_id(current_url: &str) -> Option<String> {
    let parts = path_parts(current_url)?;
    // /app/{ulid}/... — le space_id est un ULID de 26 caractères
    if parts.len() >= 3 && parts[0] == "app" && parts[1].len() == 26 {
        Some(parts[1].to_string())
    } else {
        None
    }
}

fn extract_active_section(current_url: &str) -> Option<ActiveSection> {
    let parts = path_parts(current_url)?;
    if parts.len() >= 3 && parts[0] == "app" && parts[1].len() == 26 {
        match parts[2] {
            "competitions" => Some(ActiveSection::Competitions),
            "match-report" => Some(ActiveSection::MatchReport),
            "team" => match parts.get(3) {
                Some(&"list") => Some(ActiveSection::MyTeams),
                _ => Some(ActiveSection::CreateTeam),
            },
            _ => None,
        }
    } else {
        None
    }
}

pub async fn app_menu(
    auth_session: AuthSession,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (space_name, space_id, active_section, space_logo, peut_administrer) = {
        let result: Option<(String, String, Option<ActiveSection>, String, bool)> = async {
            let user = auth_session.user?;
            let current_url = headers
                .get("hx-current-url")
                .and_then(|v| v.to_str().ok())?;
            let sid = extract_space_id(current_url)?;
            let section = extract_active_section(current_url);
            let spaces = state
                .spaces
                .space_repository
                .find_by_coach_id(&user.id)
                .await
                .ok()?;
            let space = spaces.into_iter().find(|s| s.id == sid)?;
            let logo = space.logo.thumbnail(64, 64);

            // Le profil est relu à chaque rendu du menu, jamais mis en cache :
            // une rétrogradation doit faire disparaître l'entrée au
            // rafraîchissement suivant, pas à la reconnexion.
            let space_id_vo = SpaceId::try_new(&sid).ok()?;
            let profil = state
                .spaces
                .space_repository
                .find_member_profile(&user.id, &space_id_vo)
                .await
                .ok()?;
            let peut_administrer = profil == Some(SpaceProfile::SpaceAdmin)
                || user.coach_name.clone().into_inner() == COMPTE_EXPLOITANT;

            Some((space.name, sid, section, logo, peut_administrer))
        }
        .await;
        match result {
            Some((name, id, section, logo, admin)) => {
                (Some(name), Some(id), section, Some(logo), admin)
            }
            None => (None, None, None, None, false),
        }
    };

    AppMenu {
        space_name,
        space_id,
        space_logo,
        active_section,
        peut_administrer,
        ..Default::default()
    }
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::{extract_active_section, extract_space_id, ActiveSection};

    const ULID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

    // --- extract_space_id ---

    #[test]
    fn extrait_space_id_depuis_url_complete() {
        let id = extract_space_id(&format!("http://localhost:3000/app/{ULID}/home"));
        assert_eq!(id, Some(ULID.into()));
    }

    #[test]
    fn extrait_space_id_depuis_chemin_seul() {
        let id = extract_space_id(&format!("/app/{ULID}/home"));
        assert_eq!(id, Some(ULID.into()));
    }

    #[test]
    fn extrait_space_id_sur_toute_page_espace() {
        let id = extract_space_id(&format!("/app/{ULID}/competitions"));
        assert_eq!(id, Some(ULID.into()));
    }

    #[test]
    fn ignore_query_string() {
        let id = extract_space_id(&format!("/app/{ULID}/home?foo=bar"));
        assert_eq!(id, Some(ULID.into()));
    }

    #[test]
    fn ignore_fragment() {
        let id = extract_space_id(&format!("/app/{ULID}/home#section"));
        assert_eq!(id, Some(ULID.into()));
    }

    #[test]
    fn retourne_none_si_second_segment_pas_ulid() {
        let id = extract_space_id("/app/space/create");
        assert_eq!(id, None);
    }

    #[test]
    fn retourne_none_si_chemin_trop_court() {
        let id = extract_space_id("/app/home");
        assert_eq!(id, None);
    }

    #[test]
    fn retourne_none_sur_chemin_hors_app() {
        let id = extract_space_id("/auth/login");
        assert_eq!(id, None);
    }

    // --- extract_active_section ---

    #[test]
    fn section_competitions_sur_competition() {
        let s = extract_active_section(&format!("/app/{ULID}/competitions"));
        assert_eq!(s, Some(ActiveSection::Competitions));
    }

    #[test]
    fn section_create_team_sur_team_create() {
        let s = extract_active_section(&format!("/app/{ULID}/team/create"));
        assert_eq!(s, Some(ActiveSection::CreateTeam));
    }

    #[test]
    fn section_create_team_sur_team_build() {
        let s = extract_active_section(&format!(
            "/app/{ULID}/team/01ARZ3NDEKTSV4RRFFQ69G5FAV/build"
        ));
        assert_eq!(s, Some(ActiveSection::CreateTeam));
    }

    #[test]
    fn section_my_teams_sur_team_list() {
        let s = extract_active_section(&format!("/app/{ULID}/team/list"));
        assert_eq!(s, Some(ActiveSection::MyTeams));
    }

    #[test]
    fn section_none_sur_home() {
        let s = extract_active_section(&format!("/app/{ULID}/home"));
        assert_eq!(s, None);
    }

    #[test]
    fn section_none_hors_espace() {
        let s = extract_active_section("/auth/login");
        assert_eq!(s, None);
    }
}

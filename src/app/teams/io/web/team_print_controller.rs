//! La page « Imprimer » d'une équipe (carte 595).
//!
//! Elle remplace le `window.print()` de la fiche, qui imprimait la page web
//! telle quelle. La page est **sa propre vue d'impression** : options à gauche,
//! aperçu à droite ; à l'impression, le CSS ne laisse sortir que les feuilles.
//!
//! **Tout utilisateur connecté** peut imprimer (R16), comme il peut lire la
//! fiche. Une équipe qui n'est pas prête à jouer (R1) a une page qui
//! l'explique, plutôt qu'un 404 : on y arrive par un lien partagé ou un favori.

use crate::app::routes::AppRoutes;
use crate::app::teams::domain::team::Team;
use crate::app::teams::use_cases::team_print_service::{self, TeamPrintError};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub struct TeamPrintPageVm {
    pub team_name: String,
    pub competition_label: String,
    pub back_url: String,
    pub preview_url: String,
    /// Gabarit **avec `{other_team_id}` en place** : le script de la page y met
    /// l'équipe choisie. Précédent : `recruitment_add_journeyman_template`.
    pub preview_with_url_template: String,
    pub next_opponent_url: String,
    pub competitions_json_url: String,
    pub seasons_json_url: String,
    pub teams_json_url: String,
    pub competition_id: String,
    pub season_id: String,
}

impl TeamPrintPageVm {
    pub fn build(space_id: &str, team: &Team) -> Self {
        let routes = AppRoutes::default();
        let team_id = team.id.to_string();
        Self {
            team_name: team.name.to_string(),
            competition_label: competition_label(team),
            back_url: routes.teams.team_detail(space_id, &team_id),
            preview_url: routes.teams.team_print_preview(space_id, &team_id),
            preview_with_url_template: routes
                .teams
                .team_print_preview_with_template(space_id, &team_id),
            next_opponent_url: routes
                .competitions
                .team_print_next_opponent(space_id, &team_id),
            competitions_json_url: routes
                .competitions
                .competition_widget_json_competitions(space_id),
            seasons_json_url: routes
                .competitions
                .competition_widget_json_seasons(space_id),
            teams_json_url: routes.teams.team_selection_json(space_id),
            competition_id: team
                .competition_id
                .as_ref()
                .map(|c| c.to_string())
                .unwrap_or_default(),
            season_id: team
                .season_id
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_default(),
        }
    }
}

fn competition_label(team: &Team) -> String {
    match (&team.competition_name, &team.season_name) {
        (Some(c), Some(s)) => format!("{c} · {s}"),
        (Some(c), None) => c.clone(),
        _ => String::new(),
    }
}

#[derive(Template)]
#[template(path = "teams-print-page.html")]
pub struct TeamPrintPageTemplate {
    pub app_routes: AppRoutes,
    pub vm: TeamPrintPageVm,
}

#[derive(Template)]
#[template(path = "teams-print-unavailable.html")]
pub struct TeamPrintUnavailableTemplate {
    pub app_routes: AppRoutes,
    pub team_name: String,
    pub back_url: String,
}

fn rendre<T: Template>(template: T) -> Response {
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("teams print page render: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_team_print_page(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let repo = state.teams.team_repository.as_ref();
    match team_print_service::load_printable(repo, &team_id).await {
        Ok(team) => rendre(TeamPrintPageTemplate {
            app_routes: AppRoutes::default(),
            vm: TeamPrintPageVm::build(&space_id, &team),
        }),
        Err(TeamPrintError::NotPrintable { team_name }) => rendre(TeamPrintUnavailableTemplate {
            app_routes: AppRoutes::default(),
            team_name,
            back_url: AppRoutes::default().teams.team_detail(&space_id, &team_id),
        }),
        Err(TeamPrintError::TeamNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(TeamPrintError::Repository(cause)) => {
            tracing::error!("teams print page {team_id}: {cause}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::teams::io::web::tests::fixtures::equipe_de_test;

    const SPACE: &str = "01KZ1J5JER8K1EPZ3444X2H45S";

    #[test]
    fn la_page_porte_ses_urls_et_la_competition_de_l_equipe() {
        let team = equipe_de_test();
        let vm = TeamPrintPageVm::build(SPACE, &team);
        let team_id = team.id.to_string();
        assert!(vm
            .preview_url
            .ends_with(&format!("/teams/{team_id}/print/preview")));
        assert!(vm
            .preview_with_url_template
            .ends_with("/with/{other_team_id}"));
        assert!(vm.next_opponent_url.contains(&team_id));
        assert!(vm.teams_json_url.contains(SPACE));
        assert_eq!(vm.competition_id, "00000000000000000000000003");
        assert_eq!(vm.season_id, "00000000000000000000000004");
        assert_eq!(vm.competition_label, "Ligue de Condate · Saison 2025");
    }
}

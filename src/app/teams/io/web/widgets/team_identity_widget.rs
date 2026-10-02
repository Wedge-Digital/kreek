//! L'identité d'une équipe, en widget (carte 566) — logo, nom, statut,
//! roster · coach. Rendue comme le bloc d'équipe des Résultats, plus le badge
//! de statut de la fiche.
//!
//! **Il lit l'agrégat**, comme la fiche : le statut affiché est celui que
//! calcule le domaine, traduit par `status_display` — la même fonction que la
//! fiche, donc le même libellé.

use crate::app::teams::domain::team::Team;
use crate::app::teams::io::web::status_view_models::status_display;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub struct TeamIdentityVm {
    pub name: String,
    pub initials: String,
    pub logo_url: Option<String>,
    pub roster_name: String,
    pub coach_name: String,
    pub status_label: String,
    pub status_css_class: String,
}

impl TeamIdentityVm {
    pub fn from_domain(team: &Team) -> Self {
        let (status_label, status_css_class) = status_display(team);
        Self {
            name: team.name.to_string(),
            initials: team.initials.clone(),
            logo_url: team.logo_url.as_deref().map(|url| {
                crate::app::shared_kernel::identity::cloudinary::transform(
                    url,
                    "c_fill,w_96,h_96,q_auto,f_auto",
                )
            }),
            roster_name: team.roster_name.to_string(),
            coach_name: team.coach_name.clone(),
            status_label,
            status_css_class,
        }
    }
}

#[derive(Template)]
#[template(path = "widgets/team-identity.html")]
pub struct TeamIdentityWidgetTemplate {
    pub vm: TeamIdentityVm,
}

impl IntoResponse for TeamIdentityWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("team identity widget render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

/// Équipe introuvable : 404, et l'hôte garde ce qu'il affichait — htmx ne
/// remplace rien sur une erreur.
pub async fn team_identity_widget(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    match state.teams.team_repository.find_by_id(&team_id).await {
        Ok(Some(team)) => TeamIdentityWidgetTemplate {
            vm: TeamIdentityVm::from_domain(&team),
        }
        .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("team_identity_widget find_by_id {team_id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::teams::domain::team::{GamePhase, ParticipationStatus};
    use crate::app::teams::io::web::tests::fixtures::equipe_de_test;

    fn equipe_prete() -> Team {
        let mut team = equipe_de_test();
        team.participation_status = ParticipationStatus::Enrolled;
        team.game_phase = Some(GamePhase::ReadyToPlay);
        team
    }

    #[test]
    fn sans_logo_le_vm_porte_les_initiales() {
        let vm = TeamIdentityVm::from_domain(&equipe_prete());
        assert_eq!(vm.logo_url, None);
        assert_eq!(vm.initials, "LK");
        assert_eq!(vm.name, "Les Korrigans FC");
        assert_eq!(vm.roster_name, "Elfes Sylvestres");
        assert_eq!(vm.coach_name, "Colonel Castor");
    }

    #[test]
    fn avec_logo_le_vm_porte_l_image_transformee() {
        let mut team = equipe_prete();
        team.logo_url =
            Some("https://res.cloudinary.com/demo/image/upload/v1/korrigans.png".into());
        let logo = TeamIdentityVm::from_domain(&team).logo_url.unwrap();
        assert!(logo.contains("korrigans.png"), "{logo}");
        assert!(logo.contains("w_96"), "la vignette est réduite : {logo}");
    }

    #[test]
    fn le_statut_est_celui_de_la_fiche() {
        let team = equipe_prete();
        let vm = TeamIdentityVm::from_domain(&team);
        assert_eq!(
            (vm.status_label, vm.status_css_class),
            status_display(&team)
        );
    }

    #[test]
    fn le_rendu_porte_nom_coach_roster_et_badge() {
        let html = TeamIdentityWidgetTemplate {
            vm: TeamIdentityVm::from_domain(&equipe_prete()),
        }
        .render()
        .unwrap();
        assert!(html.contains("hx-disinherit=\"*\""));
        assert!(html.contains("Les Korrigans FC"));
        assert!(html.contains("Colonel Castor"));
        assert!(html.contains("Elfes Sylvestres"));
        assert!(html.contains("team-status-badge--ready"));
        assert!(html.contains("Prête à jouer"));
        assert!(html.contains("team-identity-logo--initials"));
    }

    /// Le coach est un texte libre, vide quand la création ne l'a pas reçu :
    /// ni span vide, ni séparateur qui pend après le roster.
    #[test]
    fn sans_coach_le_rendu_n_affiche_que_le_roster() {
        let mut vm = TeamIdentityVm::from_domain(&equipe_prete());
        vm.coach_name = String::new();
        let html = TeamIdentityWidgetTemplate { vm }.render().unwrap();
        assert!(html.contains("Elfes Sylvestres"));
        assert!(!html.contains("team-identity-coach"), "{html}");
    }
}

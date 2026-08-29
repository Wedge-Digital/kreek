use crate::app::auth::auth_backend::AuthSession;
use crate::app::ranking::io::web::builders::build_classement_groups;
use crate::app::ranking::use_cases::standings_service::tiebreak_order_of;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub struct ClassementRowVm {
    pub rank: u32,
    pub team_name: String,
    pub team_link: String,
    pub played: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    /// Les points **de match** seuls. Il ne s'affiche plus : la colonne « Pts »
    /// montre desormais `total`. Il reste parce que la difference entre les deux
    /// est ce que la colonne « Man. » explique.
    pub points: u32,
    /// `None` quand l'équipe n'a reçu aucun point manuel, sinon la valeur
    /// **déjà signée** — « +2 », « −1 ».
    ///
    /// **Une `Option`, jamais un zéro par convention.** Le gabarit doit
    /// distinguer « aucun point manuel » — un tiret — de « zéro point
    /// manuel », **qui n'existe pas** : `ManualPoints` refuse le zéro. L'option
    /// rend cette impossibilité dans le type plutôt que dans un commentaire.
    ///
    /// **Une chaîne et non un `i32`**, comme `bonus` du classement détaillé :
    /// Askama lie `m` en `&i32` dans un `if let`, et le comparer à un littéral
    /// demanderait un déréférencement que les gabarits du projet n'emploient
    /// pas. Le signe se pose donc dans le builder, une fois.
    pub manual: Option<String>,
    /// Points de match plus points manuels. **Signe** : un total peut etre
    /// negatif, et c'est un rang valide, pas une erreur.
    pub total: i32,
}

/// Un classement à afficher — soit l'unique classement de la saison
/// (`title: None`, saison sans poule ou une seule), soit le classement d'une
/// poule (`title: Some(nom)`) ou des équipes non assignées à une poule.
pub struct ClassementGroupVm {
    pub title: Option<String>,
    pub has_enrolled_teams: bool,
    pub rows: Vec<ClassementRowVm>,
}

pub struct ClassementWidgetVm {
    /// L'URL de la page de gestion. Portée par le VM plutôt que construite dans
    /// le gabarit : c'est le parti pris du projet, et `line.delete_url` du
    /// relevé suit le même.
    pub manual_points_url: String,
    /// Le bouton d'accès ne s'affiche qu'aux commissaires ; la page qu'il ouvre
    /// est consultable par tous. Le lien sur un point manuel, lui, est ouvert.
    pub can_manage: bool,
    pub rules_missing: bool,
    pub groups: Vec<ClassementGroupVm>,
}

#[derive(Template)]
#[template(path = "widgets/classement-widget.html")]
pub struct ClassementWidgetTemplate {
    pub vm: ClassementWidgetVm,
}

impl IntoResponse for ClassementWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("classement_widget render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn classement_widget(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if auth_session.user.is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let user_id = auth_session.user.as_ref().map(|u| u.id.to_string());
    let vm = build_vm(
        &state,
        &space_id,
        &competition_id,
        &season_id,
        user_id.as_deref(),
    )
    .await;
    ClassementWidgetTemplate { vm }.into_response()
}

async fn build_vm(
    state: &AppState,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    user_id: Option<&str>,
) -> ClassementWidgetVm {
    let (rules, teams, lines, groups, manual) = tokio::join!(
        state.ranking.competition_port.find_ranking_rules(season_id),
        state
            .ranking
            .competition_port
            .find_enrolled_teams(season_id),
        state
            .ranking
            .repository
            .find_latest_lines_for_season(season_id),
        state.ranking.competition_port.find_groups(season_id),
        // Cinquieme lecture, en parallele des quatre autres : le temps de
        // reponse ne bouge pas. Une erreur rend une carte vide plutot que de
        // faire echouer le classement -- un classement sans ses points manuels
        // est faux, mais un classement absent est pire.
        state
            .ranking
            .repository
            .find_manual_totals_for_season(season_id),
    );

    let rules_missing = rules.is_none();
    let order = tiebreak_order_of(&rules);
    let groups_vm = if rules_missing {
        vec![]
    } else {
        build_classement_groups(
            space_id,
            lines.unwrap_or_default(),
            &manual.unwrap_or_default(),
            &teams,
            &groups,
            &order,
        )
    };

    let routes = crate::app::routes::AppRoutes::default();
    ClassementWidgetVm {
        manual_points_url: routes
            .ranking
            .manual_points(space_id, competition_id, season_id),
        can_manage: match user_id {
            Some(id) => {
                crate::app::ranking::use_cases::manual_points::autorise(
                    state.ranking.admin_port.as_ref(),
                    id,
                    competition_id,
                    space_id,
                )
                .await
            }
            None => false,
        },
        rules_missing,
        groups: groups_vm,
    }
}

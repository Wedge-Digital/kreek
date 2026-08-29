use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::competition_repository_port::CompetitionBaseInfo;
use crate::app::competitions::io::web::admin::summary_tab::build_summary_fragment;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "admin-page.html")]
pub struct AdminPageTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub competition_name: String,
    pub season_name: String,
    pub admin_count: usize,
    pub has_groups: bool,
    pub active_tab: String,
    pub content: String,
}

impl IntoResponse for AdminPageTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("admin page template render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn admin_page(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    render_admin_page(
        auth_session,
        &space_id,
        &competition_id,
        &season_id,
        // Le Résumé est l'onglet d'accueil depuis la carte 419 : le tableau de
        // bord qui l'occupait a quitté l'administration.
        "summary",
        &state,
    )
    .await
}

/// Vérifie que l'utilisateur connecté est admin d'espace ou admin de la
/// compétition — condition d'accès à **toute** l'administration de
/// compétition. À appeler sur **chaque** route admin (page complète ET
/// fragment htmx), pas seulement sur le chargement de page complet : sans
/// quoi le chemin htmx (utilisé pour le changement d'onglet en SPA) contourne
/// le contrôle d'accès.
///
/// **La saison du chemin doit appartenir à la compétition du chemin** (carte
/// 416). Le droit est accordé par compétition ; sans ce contrôle,
/// l'administrateur de la compétition A pose son propre `competition_id` et le
/// `season_id` de la compétition B du même espace, et la garde le laisse passer
/// avant que le handler n'agisse sur B. `space_scope` ne le rattrape pas : il
/// vérifie que la saison appartient à l'**espace**, jamais à la compétition.
///
/// Le contrôle vit ici plutôt que dans chaque handler pour que la fonction soit
/// le seul endroit qui réponde « ce chemin est-il cohérent et m'est-il
/// permis ? ». Les appelants n'ont rien à y penser.
pub async fn require_admin_access(
    auth_session: &AuthSession,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    state: &AppState,
) -> Result<CompetitionBaseInfo, Response> {
    let Some(user) = &auth_session.user else {
        return Err(StatusCode::UNAUTHORIZED.into_response());
    };

    let comp_id = match CompetitionId::try_new(competition_id) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::BAD_REQUEST.into_response()),
    };

    let space_entity_id = match SpaceId::try_new(space_id) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::BAD_REQUEST.into_response()),
    };

    let is_space_admin = matches!(
        state
            .competitions
            .space_member_port
            .find_member_profile(&user.id, &space_entity_id)
            .await,
        Some(SpaceProfile::SpaceAdmin)
    );

    let comp_info = match state
        .competitions
        .competition_repository
        .find_base_info(&comp_id)
        .await
    {
        Ok(Some(info)) => info,
        Ok(None) => return Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("require_admin_access competition find: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };

    let user_id_str = user.id.to_string();
    let coach_name_str = user.coach_name.clone().into_inner();
    let is_comp_admin = comp_info.admin_ids.contains(&user_id_str)
        || comp_info.admin_names.contains(&coach_name_str);

    if !is_space_admin && !is_comp_admin {
        return Err(StatusCode::FORBIDDEN.into_response());
    }

    verifier_saison_de_la_competition(season_id, competition_id, state).await?;

    Ok(comp_info)
}

/// `404` et non `403` : une saison qui n'appartient pas à cette compétition est
/// hors du périmètre du chemin. Répondre `403` confirmerait son existence à qui
/// se contente d'essayer des identifiants.
async fn verifier_saison_de_la_competition(
    season_id: &str,
    competition_id: &str,
    state: &AppState,
) -> Result<(), Response> {
    // Un identifiant mal formé est une requête fautive, pas un refus de droit :
    // il n'a pas pu désigner quoi que ce soit.
    let Ok(season_entity_id) = SeasonId::try_new(season_id) else {
        return Err(StatusCode::BAD_REQUEST.into_response());
    };
    match state
        .competitions
        .season_repository
        .find_full(&season_entity_id)
        .await
    {
        Ok(Some(saison)) if saison.competition_id == competition_id => Ok(()),
        Ok(_) => Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("require_admin_access saison {season_id}: {e:?}");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

pub async fn render_admin_page(
    auth_session: AuthSession,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    active_tab: &str,
    state: &AppState,
) -> Response {
    let comp_info =
        match require_admin_access(&auth_session, space_id, competition_id, season_id, state).await
        {
            Ok(info) => info,
            Err(resp) => return resp,
        };

    let comp_id = match CompetitionId::try_new(competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let season_entity_id = match SeasonId::try_new(season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let season_name = state
        .competitions
        .season_repository
        .find_base_info(&season_entity_id)
        .await
        .ok()
        .flatten()
        .map(|i| i.name)
        .unwrap_or_default();

    let has_groups = state
        .competitions
        .season_repository
        .find_structure(&season_entity_id)
        .await
        .ok()
        .flatten()
        .map(|s| s.ranking_group.use_ranking_groups() && s.ranking_group.groups().len() > 1)
        .unwrap_or(false);

    let app_routes = AppRoutes::default();

    let content = match active_tab {
        "enrollments" => {
            let requires_validation = state
                .competitions
                .season_repository
                .find_invitations(&season_entity_id)
                .await
                .ok()
                .flatten()
                .map(|inv| inv.requires_validation.0)
                .unwrap_or(true);

            let tpl = super::enrollments_tab::EnrollmentsTabTemplate {
                app_routes,
                space_id: space_id.to_string(),
                competition_id: competition_id.to_string(),
                season_id: season_id.to_string(),
                requires_validation,
            };
            tpl.render().unwrap_or_default()
        }
        "groups" => {
            let tpl = super::groups_tab::GroupsTabTemplate {
                app_routes,
                space_id: space_id.to_string(),
                competition_id: competition_id.to_string(),
                season_id: season_id.to_string(),
            };
            tpl.render().unwrap_or_default()
        }
        "settings" => {
            let tpl = super::settings::settings_tab::SettingsTabTemplate {
                space_id: space_id.to_string(),
                competition_id: competition_id.to_string(),
                season_id: season_id.to_string(),
                general_url: app_routes.competitions.admin_settings_general(
                    space_id,
                    competition_id,
                    season_id,
                ),
                ranking_url: app_routes.competitions.admin_settings_ranking(
                    space_id,
                    competition_id,
                    season_id,
                ),
                pools_url: app_routes.competitions.admin_settings_pools(
                    space_id,
                    competition_id,
                    season_id,
                ),
                tiers_url: app_routes.competitions.admin_settings_tiers(
                    space_id,
                    competition_id,
                    season_id,
                ),
            };
            tpl.render().unwrap_or_default()
        }
        "schedule" => {
            let tpl = super::schedule_tab::ScheduleTabTemplate {
                app_routes,
                space_id: space_id.to_string(),
                competition_id: competition_id.to_string(),
                season_id: season_id.to_string(),
            };
            tpl.render().unwrap_or_default()
        }
        // **Le Résumé est le défaut**, et non un onglet nommé parmi d'autres :
        // il rendait le tableau de bord avant la carte 419. Un onglet inconnu —
        // signet périmé, URL forgée — atterrit donc sur l'accueil plutôt que sur
        // une page blanche.
        //
        // En dernière position, faute de quoi les branches suivantes
        // deviendraient inatteignables — ce que `-D unreachable-patterns`
        // refuserait, mais qu'il vaut mieux ne pas écrire.
        _ => {
            match build_summary_fragment(
                &comp_id,
                &season_entity_id,
                state.competitions.competition_repository.as_ref(),
                state.competitions.season_repository.as_ref(),
                state.competitions.reference_port.as_ref(),
                app_routes,
                space_id,
                competition_id,
                season_id,
            )
            .await
            {
                Some(tpl) => tpl.render().unwrap_or_default(),
                None => {
                    tracing::error!(
                        "admin_page summary build failed for {competition_id}/{season_id}"
                    );
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            }
        }
    };

    AdminPageTemplate {
        app_routes,
        space_id: space_id.to_string(),
        competition_id: competition_id.to_string(),
        season_id: season_id.to_string(),
        competition_name: comp_info.name,
        season_name,
        admin_count: comp_info.admin_ids.len(),
        has_groups,
        active_tab: active_tab.to_string(),
        content,
    }
    .into_response()
}

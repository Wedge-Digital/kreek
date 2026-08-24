//! Les deux mutations de la ligne de membre.
//!
//! Contrôleurs au sens strict : construire la commande, appeler le use case,
//! bâtir la réponse. Aucune règle ne se décide ici — « est-ce le dernier
//! administrateur ? » vit dans l'agrégat, et le grisage posé par le widget
//! n'est qu'une politesse. Ces deux endpoints sont directement atteignables,
//! et doivent refuser d'eux-mêmes.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::app::spaces::context::SpacesContext;
use crate::app::spaces::domain::membership::SpaceMembershipError;
use crate::app::spaces::io::web::builders::build_member_rows;
use crate::app::spaces::io::web::extractors::space_permissions::SpacePermissions;
use crate::app::spaces::use_cases::change_member_role_use_case::{
    self, ChangeMemberRoleCommand, ChangeMemberRoleError,
};
use crate::app::spaces::use_cases::remove_member_use_case::{
    self, RemoveMemberCommand, RemoveMemberError,
};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ChangeRoleForm {
    /// « SpaceAdmin » ou « SpaceUser » — la représentation de
    /// `SpaceProfile::as_str()`. Primitive assumée : c'est la frontière HTTP,
    /// où rien n'est encore validé.
    pub profile: String,
}

#[derive(Template)]
#[template(path = "widgets/_member-row.html")]
struct LigneTemplate {
    routes: crate::app::spaces::routes::Routes,
    space_id: String,
    membre: crate::app::spaces::io::web::builders::MemberRowVm,
}

/// Traduit une erreur du domaine en statut HTTP.
///
/// **409 et non 400** pour `DernierAdministrateur` : la requête est bien formée,
/// c'est l'*état* de l'espace qui la refuse, et cet état peut changer.
fn statut_metier(e: &SpaceMembershipError) -> StatusCode {
    match e {
        SpaceMembershipError::ActeurEstLaCible => StatusCode::FORBIDDEN,
        SpaceMembershipError::DernierAdministrateur => StatusCode::CONFLICT,
        SpaceMembershipError::PasMembre => StatusCode::NOT_FOUND,
    }
}

/// Fragment d'erreur HTML — jamais de JSON, ce sont des réponses HTMX.
fn erreur(statut: StatusCode, message: &str) -> Response {
    (
        statut,
        Html(format!(r#"<div class="sam-erreur">{message}</div>"#)),
    )
        .into_response()
}

fn cible(brut: &str) -> Result<CoachId, Response> {
    CoachId::try_new(brut).map_err(|_| erreur(StatusCode::BAD_REQUEST, "Coach inconnu."))
}

pub async fn change_member_role_controller(
    auth_session: AuthSession,
    perms: SpacePermissions,
    Path((_, coach_id)): Path<(String, String)>,
    State(ctx): State<SpacesContext>,
    Form(form): Form<ChangeRoleForm>,
) -> Response {
    let Some(acteur) = garde(&perms, auth_session) else {
        return StatusCode::FORBIDDEN.into_response();
    };
    let cible = match cible(&coach_id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let Ok(profil) = SpaceProfile::try_from(form.profile.as_str()) else {
        return erreur(StatusCode::BAD_REQUEST, "Profil inconnu.");
    };

    let cmd = ChangeMemberRoleCommand {
        space_id: perms.space_id,
        acteur,
        cible,
        nouveau_profil: profil,
    };
    match change_member_role_use_case::execute(cmd, ctx.space_repository.as_ref(), &ctx.event_bus)
        .await
    {
        Ok(_) => ligne_re_rendue(&ctx, &perms, &acteur, &cible).await,
        Err(ChangeMemberRoleError::Metier(e)) => erreur(statut_metier(&e), &libelle(&e)),
        Err(ChangeMemberRoleError::EspaceInconnu) => {
            erreur(StatusCode::NOT_FOUND, "Espace introuvable.")
        }
        Err(ChangeMemberRoleError::Database(m)) => {
            tracing::error!("change_member_role: {m}");
            erreur(StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne.")
        }
    }
}

pub async fn remove_member_controller(
    auth_session: AuthSession,
    perms: SpacePermissions,
    Path((_, coach_id)): Path<(String, String)>,
    State(ctx): State<SpacesContext>,
) -> Response {
    let Some(acteur) = garde(&perms, auth_session) else {
        return StatusCode::FORBIDDEN.into_response();
    };
    let cible = match cible(&coach_id) {
        Ok(id) => id,
        Err(r) => return r,
    };

    let cmd = RemoveMemberCommand {
        space_id: perms.space_id,
        acteur,
        cible,
    };
    match remove_member_use_case::execute(cmd, ctx.space_repository.as_ref(), &ctx.event_bus).await
    {
        // Corps vide : la ligne s'échange avec `outerHTML` et disparaît. Pas de
        // re-rendu de la liste — la ligne sait se supprimer.
        Ok(_) => (
            [("HX-Trigger", declencheur("memberRemoved", &cible))],
            Html(String::new()),
        )
            .into_response(),
        Err(RemoveMemberError::Metier(e)) => erreur(statut_metier(&e), &libelle(&e)),
        Err(RemoveMemberError::EspaceInconnu) => {
            erreur(StatusCode::NOT_FOUND, "Espace introuvable.")
        }
        Err(RemoveMemberError::Database(m)) => {
            tracing::error!("remove_member: {m}");
            erreur(StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne.")
        }
    }
}

fn garde(perms: &SpacePermissions, auth_session: AuthSession) -> Option<CoachId> {
    perms.is_admin().then_some(())?;
    auth_session.user.map(|u| u.id)
}

fn libelle(e: &SpaceMembershipError) -> String {
    match e {
        SpaceMembershipError::ActeurEstLaCible => "Vous ne pouvez pas agir sur votre propre ligne.",
        SpaceMembershipError::DernierAdministrateur => {
            "Un espace doit garder au moins un administrateur."
        }
        SpaceMembershipError::PasMembre => "Ce coach n'est pas membre de cet espace.",
    }
    .to_string()
}

fn declencheur(nom: &str, cible: &CoachId) -> String {
    format!(r#"{{"{nom}":{{"coach_id":"{cible}"}}}}"#)
}

/// Re-rend la ligne modifiée.
///
/// La liste est relue plutôt que reconstruite depuis le retour du use case : il
/// faut de toute façon le pseudo, l'email et les initiales, que le use case ne
/// rend pas. La relecture donne du même coup le compte d'administrateurs à jour,
/// dont `role_locked` dépend — rétrograder l'avant-dernier administrateur fige
/// le sélecteur du dernier, et c'est la seule chose que le client ne peut pas
/// déduire seul.
async fn ligne_re_rendue(
    ctx: &SpacesContext,
    perms: &SpacePermissions,
    acteur: &CoachId,
    cible: &CoachId,
) -> Response {
    let Ok(lignes) = ctx
        .space_repository
        .list_members_with_profile(&perms.space_id)
        .await
    else {
        return erreur(StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne.");
    };
    let Some(membre) = build_member_rows(lignes, acteur)
        .into_iter()
        .find(|m| m.coach_id == cible.to_string())
    else {
        return erreur(StatusCode::NOT_FOUND, "Ce coach n'est plus membre.");
    };

    let ligne = LigneTemplate {
        routes: crate::app::spaces::routes::Routes::default(),
        space_id: perms.space_id.to_string(),
        membre,
    };
    match ligne.render() {
        Ok(html) => (
            [("HX-Trigger", declencheur("memberRoleChanged", cible))],
            Html(html),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("re-rendu de ligne impossible: {e}");
            erreur(StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne.")
        }
    }
}

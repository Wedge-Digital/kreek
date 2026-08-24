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
use crate::app::spaces::io::web::builders::{build_member_rows, CandidateRowVm};
use crate::app::spaces::io::web::extractors::space_permissions::SpacePermissions;
use crate::app::spaces::use_cases::add_member_use_case::{self, AddMemberCommand, AddMemberError};
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
#[template(path = "widgets/_candidate-row.html")]
struct LigneCandidateTemplate {
    routes: crate::app::spaces::routes::Routes,
    space_id: String,
    candidat: CandidateRowVm,
}

#[derive(Template)]
#[template(path = "widgets/_member-row.html")]
struct LigneTemplate {
    routes: crate::app::spaces::routes::Routes,
    space_id: String,
    membre: crate::app::spaces::io::web::builders::MemberRowVm,
    reset_action: String,
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
        SpaceMembershipError::DejaMembre => StatusCode::CONFLICT,
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

#[derive(Deserialize)]
pub struct AddMemberForm {
    pub coach_id: String,
    pub profile: String,
    /// Une case décochée **n'est pas envoyée** par un formulaire HTML : absent
    /// vaut donc « ne pas prévenir », ce qui est le comportement voulu.
    #[serde(default)]
    pub notifier: bool,
}

/// Ajoute un coach déjà inscrit sur la plateforme.
///
/// Rend la **ligne candidate** re-rendue en « Déjà membre », plutôt que de la
/// retirer : le coach existe toujours dans l'annuaire, il est simplement devenu
/// membre. La faire disparaître laisserait croire à une suppression.
pub async fn add_member_controller(
    auth_session: AuthSession,
    perms: SpacePermissions,
    State(ctx): State<SpacesContext>,
    Form(form): Form<AddMemberForm>,
) -> Response {
    let Some(acteur) = garde(&perms, auth_session) else {
        return StatusCode::FORBIDDEN.into_response();
    };
    let nouveau = match cible(&form.coach_id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let Ok(profil) = SpaceProfile::try_from(form.profile.as_str()) else {
        return erreur(StatusCode::BAD_REQUEST, "Profil inconnu.");
    };

    let cmd = AddMemberCommand {
        space_id: perms.space_id,
        acteur,
        nouveau,
        profil,
        notifier: form.notifier.into(),
        space_url: ctx.host_layout.space_url(&perms.space_id.to_string()),
        app_url: ctx.host_layout.app_url(),
    };
    match add_member_use_case::execute(
        cmd,
        ctx.space_repository.as_ref(),
        ctx.user_cache_repository.as_ref(),
        ctx.email_service.as_ref(),
        &ctx.event_bus,
    )
    .await
    {
        Ok(_) => ligne_candidate_re_rendue(&ctx, &perms, &nouveau).await,
        Err(AddMemberError::Metier(e)) => erreur(statut_metier(&e), &libelle(&e)),
        Err(AddMemberError::EspaceInconnu) => erreur(StatusCode::NOT_FOUND, "Espace introuvable."),
        Err(AddMemberError::CoachInconnu) => {
            erreur(StatusCode::NOT_FOUND, "Ce coach est introuvable.")
        }
        Err(AddMemberError::Database(m)) => {
            tracing::error!("add_member: {m}");
            erreur(StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne.")
        }
    }
}

/// Re-rend la ligne candidate, qui porte désormais son badge.
///
/// Le `name` voyage dans `memberAdded` **pour une seule raison** : le journal de
/// session l'affiche depuis ce payload, sans relire. C'est ce qui masque le délai
/// d'alimentation du cache d'utilisateurs, alimenté par un app event asynchrone.
async fn ligne_candidate_re_rendue(
    ctx: &SpacesContext,
    perms: &SpacePermissions,
    nouveau: &CoachId,
) -> Response {
    let Ok(user) = ctx.user_cache_repository.find_user_by_id(nouveau).await else {
        return erreur(StatusCode::NOT_FOUND, "Ce coach est introuvable.");
    };
    let ligne = LigneCandidateTemplate {
        candidat: CandidateRowVm {
            coach_id: nouveau.to_string(),
            initials: crate::common::initials::initials(&user.name.to_string()),
            name: user.name.to_string(),
            email: user.email.as_ref().to_string(),
            est_membre: true,
        },
        routes: crate::app::spaces::routes::Routes::default(),
        space_id: perms.space_id.to_string(),
    };
    match ligne.render() {
        Ok(html) => (
            [(
                "HX-Trigger",
                format!(
                    r#"{{"memberAdded":{{"coach_id":"{}","name":"{}"}}}}"#,
                    nouveau, user.name
                ),
            )],
            Html(html),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("re-rendu de ligne candidate impossible: {e}");
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
        SpaceMembershipError::DejaMembre => "Ce coach est déjà membre de cet espace.",
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
        reset_action: ctx.host_layout.password_reset_action(),
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

//! Demande d'envoi d'un lien de réinitialisation, sans page ni redirection.
//!
//! Même opération que `/auth/forgot-password`, et **exactement les mêmes
//! droits** : le routeur de ce BC est fusionné hors du routeur protégé, donc
//! l'envoi d'un lien de réinitialisation est déjà public. Le lien part chez le
//! titulaire de l'adresse, ce qui rend l'opération inoffensive quel qu'en soit
//! le demandeur.
//!
//! Ce qui la distingue est sa **réponse** : `204`, sans corps. L'endpoint public
//! rend `HX-Redirect` vers la page « consultez vos emails », ce qui ferait
//! quitter l'application à un appelant qui l'invoque depuis une ligne de
//! tableau. Ici l'appelant gère son propre retour visuel — et ce BC n'a pas à
//! connaître la forme qu'il lui donnera.

use crate::app::auth::context::AuthContext;
use crate::app::auth::use_cases::send_reset_password_email::{
    execute, SendResetPasswordEmailCommand, SendResetPasswordEmailError,
};
use crate::app::shared_kernel::identity::coach_name::CoachName;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct DemandeReinitialisation {
    pub coach_name: String,
}

pub async fn post_reset_password_request(
    State(ctx): State<AuthContext>,
    Form(demande): Form<DemandeReinitialisation>,
) -> Response {
    let Ok(coach_name) = CoachName::try_new(&demande.coach_name) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    match execute(
        SendResetPasswordEmailCommand {
            coach_name,
            host_domain: ctx.host_domain.clone(),
        },
        ctx.user_repository.as_ref(),
        ctx.reset_token_repository.as_ref(),
        ctx.email_service.as_ref(),
    )
    .await
    {
        // Un pseudo inconnu rend `204` comme un pseudo connu : distinguer les
        // deux dirait à n'importe qui si un compte existe. C'est le choix déjà
        // fait par l'endpoint public, repris tel quel.
        Ok(()) | Err(SendResetPasswordEmailError::CoachNameNotFound) => {
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            tracing::error!("reset_password_request: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

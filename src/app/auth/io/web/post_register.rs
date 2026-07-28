use crate::app::auth::io::web::get_register::RegisterTemplate;
use crate::app::auth::io::web::get_register_success::RegisterFormPayload;
use crate::app::auth::routes::path;
use crate::app::auth::use_cases::register_new_acount;
use crate::app::auth::use_cases::register_new_acount::{RegisterCommand, RegisterError};
use crate::app::auth::context::AuthContext;
use axum::body::Body;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Form;

pub async fn post_register(
    State(ctx): State<AuthContext>,
    Form(payload): Form<RegisterFormPayload>,
) -> Response {
    let cmd = RegisterCommand {
        coach_name: payload.coach_name.clone(),
        email: payload.email.clone(),
        password: payload.password.clone(),
        password_confirm: payload.password_confirm.clone(),
    };

    match register_new_acount::execute(cmd, ctx.user_repository.as_ref(), &ctx.event_bus)
        .await
    {
        Ok(()) => Response::builder()
            .header("HX-Redirect", path::REGISTER_SUCCESS)
            .body(Body::empty())
            .unwrap(),

        Err(errors) => {
            let mut tmpl = RegisterTemplate {
                coach_name_value: payload.coach_name,
                email_value: payload.email,
                ..Default::default()
            };
            for e in errors {
                match e {
                    RegisterError::PasswordMismatch => {
                        tmpl.password_confirm_error =
                            Some("Les mots de passe ne correspondent pas".into())
                    }
                    RegisterError::PasswordTooShort => {
                        tmpl.password_error =
                            Some("Le mot de passe doit contenir au moins 8 caractères".into())
                    }
                    RegisterError::InvalidCoachName(err) => {
                        tmpl.coach_name_error = Some(err.to_string())
                    }
                    RegisterError::InvalidEmail(err) => tmpl.email_error = Some(err.to_string()),
                    RegisterError::CoachNameAlreadyTaken => {
                        tmpl.coach_name_error = Some("Ce nom de coach est déjà utilisé".into())
                    }
                    RegisterError::EmailAlreadyTaken => {
                        tmpl.email_error = Some("Cette adresse email est déjà utilisée".into())
                    }
                    RegisterError::PasswordHashError | RegisterError::Database(_) => {
                        tmpl.password_error = Some("Erreur interne, veuillez réessayer.".into())
                    }
                }
            }
            tmpl.into_response()
        }
    }
}

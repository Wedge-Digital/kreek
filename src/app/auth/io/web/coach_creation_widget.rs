//! Le formulaire de création de compte, rendu comme **fragment** pour un hôte.
//!
//! Ce BC ne sait pas où il sera affiché, et n'a pas à le savoir. Il rend ses
//! champs, valide, montre **ses** erreurs chez lui, et signale son succès par un
//! événement DOM. L'appelant n'a jamais à connaître ses règles ni ses messages —
//! c'est tout le bénéfice de la forme widget plutôt qu'un port.
//!
//! # Le contrat que rien ne vérifie
//!
//! En cas de succès, la réponse porte :
//!
//! ```text
//! HX-Trigger: {"accountCreated": {"coach_id": "01J…", "name": "NurgleFan"}}
//! ```
//!
//! Ce nom et ces deux clés franchissent une frontière de BC **par le
//! navigateur**. Ni le compilateur, ni `cargo test`, ni `check-arch` — un `grep`
//! aveugle aux chaînes littérales et aux attributs HTML — ne les voient. Les
//! renommer casse l'appelant en silence.
//!
//! Le harnais vérifie que l'en-tête est posé avec ces clés ; seul un test e2e
//! peut vérifier que quelqu'un les écoute.
//!
//! # Ce que ce widget ne rend pas
//!
//! Aucun sélecteur de profil : le rôle dans un espace est un concept de l'hôte,
//! que ce BC ne connaît pas.

use crate::app::auth::context::AuthContext;
use crate::app::auth::use_cases::create_account_without_password::{
    execute, CreateAccountError, CreateAccountWithoutPasswordCommand,
};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

#[derive(Template, Default)]
#[template(path = "widgets/coach-creation.html")]
pub struct CoachCreationWidget {
    pub pseudo: String,
    pub email: String,
    pub erreur: Option<String>,
}

impl IntoResponse for CoachCreationWidget {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("coach_creation_widget: rendu impossible: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

/// Pré-remplissage, décidé par l'appelant.
///
/// Deux champs ciblés plutôt qu'une chaîne à répartir : c'est l'appelant qui
/// sait ce que l'utilisateur cherchait. Faire trancher ce BC sur la présence
/// d'un `@` lui ferait deviner une intention qu'il n'observe pas.
#[derive(Deserialize, Default)]
pub struct CoachPrefill {
    #[serde(default)]
    pub pseudo: String,
    #[serde(default)]
    pub email: String,
}

pub async fn get_coach_creation_widget(Query(prefill): Query<CoachPrefill>) -> Response {
    CoachCreationWidget {
        pseudo: prefill.pseudo,
        email: prefill.email,
        erreur: None,
    }
    .into_response()
}

#[derive(Deserialize)]
pub struct CreateCoachForm {
    pub coach_name: String,
    pub email: String,
}

pub async fn post_coach_creation_widget(
    State(ctx): State<AuthContext>,
    Form(form): Form<CreateCoachForm>,
) -> Response {
    let cmd = CreateAccountWithoutPasswordCommand {
        coach_name: form.coach_name.clone(),
        email: form.email.clone(),
        host_domain: ctx.host_domain.clone(),
    };

    match execute(
        cmd,
        ctx.user_repository.as_ref(),
        ctx.reset_token_repository.as_ref(),
        ctx.email_service.as_ref(),
        &ctx.event_bus,
    )
    .await
    {
        Ok(id) => succes(&id.to_string(), &form.coach_name),
        Err(e) => CoachCreationWidget {
            pseudo: form.coach_name,
            email: form.email,
            erreur: Some(libelle(&e)),
        }
        .into_response(),
    }
}

/// Le fragment vidé, plus l'événement.
///
/// Le formulaire disparaît : le compte est créé, il n'y a plus rien à saisir.
/// L'appelant décide de ce qu'il met à la place.
fn succes(coach_id: &str, name: &str) -> Response {
    let declencheur =
        format!(r#"{{"accountCreated":{{"coach_id":"{coach_id}","name":"{name}"}}}}"#);
    ([("HX-Trigger", declencheur)], Html(String::new())).into_response()
}

fn libelle(e: &CreateAccountError) -> String {
    match e {
        CreateAccountError::PseudoDejaPris => "Ce pseudo est déjà pris.".into(),
        CreateAccountError::EmailDejaPris => "Cette adresse est déjà utilisée.".into(),
        CreateAccountError::PseudoInvalide(m) => format!("Pseudo invalide : {m}"),
        CreateAccountError::EmailInvalide(m) => format!("Adresse invalide : {m}"),
        CreateAccountError::EnvoiEmailImpossible(m) => {
            tracing::warn!("coach_creation_widget: envoi impossible: {m}");
            "L'e-mail n'a pas pu partir : le compte n'a pas été créé, réessaie.".into()
        }
        CreateAccountError::Database(m) => {
            tracing::error!("coach_creation_widget: {m}");
            "Erreur interne, réessaie plus tard.".into()
        }
    }
}

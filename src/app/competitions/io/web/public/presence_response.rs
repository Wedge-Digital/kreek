//! La route publique de réponse : le coach clique dans sa boîte mail, sans
//! connexion, et sa réponse est enregistrée.
//!
//! # Le seul handler de `competitions` sans `AuthSession`
//!
//! Aucun BC hors `auth` n'avait de route publique. Les neuf routeurs de BC vivent
//! dans `protected`, sous `require_auth` et `bypass_auth` : **une route ajoutée là
//! redirige vers `/auth/login`**, quel que soit son handler. D'où
//! `public_router()`, mergé dans `main.rs` à côté d'`auth`.
//!
//! Le verrou n'est pas ce commentaire — il n'a jamais arrêté personne — mais
//! `src/web/tests/test_route_publique_presence.rs`, qui demande la route **sans
//! cookie** contre le routeur de production. L'erreur qu'il empêche ne casserait
//! aucun autre test : les liens déjà partis cesseraient simplement de répondre, et
//! on l'apprendrait par un coach.
//!
//! # Deux routes littérales, et pas de verbe à parser
//!
//! `/presence/{token}/oui` et `/presence/{token}/non`. Un troisième verbe rend
//! `404` **par le routeur**, sans une ligne de code : pas de verbe à extraire, pas
//! de `match` à écrire, pas de branche à tester.
//!
//! # R4 — un `GET` qui enregistre, et ce que ça coûte
//!
//! Un lien visité par une machine enregistre une réponse : SafeLinks et certains
//! antivirus inspectent les URL. R4 assume ce risque et pose comme parade **le
//! bouton opposé sur la page**, non un second clic — qui renierait la promesse
//! « un clic suffit ».
//!
//! Le risque résiduel, écrit plutôt que tu : une prévisualisation automatique peut
//! enregistrer « oui » sans que le coach ait cliqué. Le bouton opposé le rattrape
//! s'il ouvre la page ; il ne le rattrape pas s'il ne l'ouvre jamais.
//!
//! # R26 — la page ne révèle jamais si un jeton a existé
//!
//! Jeton inconnu, mal formé, campagne introuvable, libellés manquants : une seule
//! page, et son gabarit ne porte **aucun** champ. Deux pages distinctes diraient
//! à qui essaie des jetons lesquels ont existé.

use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::presence_survey::{
    Motif, PresenceSurvey, Repondant, SurveyStatus, SurveyToken, Venue,
};
use crate::app::competitions::use_cases::presences::presence_landing_service::{
    self, LandingContext,
};
use crate::app::competitions::use_cases::presences::record_answer_use_case;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use time::macros::format_description;
use time::OffsetDateTime;

// ── Les gabarits ─────────────────────────────────────────────────────────────

/// « Présence confirmée » et « absence enregistrée » partagent leur gabarit : ils
/// ne diffèrent que par leurs mots — même encadré, même récapitulatif, même bouton
/// opposé. Les séparer aurait dupliqué un markup qui doit rester en phase.
#[derive(Template)]
#[template(path = "public/presence-response.html")]
pub struct PresenceResponseTemplate {
    pub css: &'static str,
    pub team_name: String,
    pub round_name: String,
    pub round_dates: String,
    pub competition_name: String,
    pub competition_url: String,
    pub deadline: String,
    /// `true` quand la réponse enregistrée est « je viens ».
    pub presente: bool,
    /// L'URL du choix **inverse** — la parade de R4.
    pub url_opposee: String,
    pub libelle_oppose: String,
}

#[derive(Template)]
#[template(path = "public/presence-closed.html")]
pub struct PresenceClosedTemplate {
    pub css: &'static str,
    pub team_name: String,
    pub round_name: String,
    pub competition_name: String,
    /// R27 — trois motifs de clôture, et l'écran dit lequel.
    pub motif: String,
}

/// R26 — **aucun champ hors le CSS**. Ni jeton, ni équipe, ni journée : la page ne
/// doit pas révéler si le lien a existé, et un gabarit qui ne reçoit rien ne peut
/// rien laisser filtrer.
#[derive(Template)]
#[template(path = "public/presence-unknown.html")]
pub struct PresenceUnknownTemplate {
    pub css: &'static str,
}

fn html(rendu: Result<String, askama::Error>, quoi: &str) -> Response {
    match rendu {
        Ok(page) => Html(page).into_response(),
        Err(e) => {
            tracing::error!("{quoi} render: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

impl IntoResponse for PresenceResponseTemplate {
    fn into_response(self) -> Response {
        html(self.render(), "presence response")
    }
}

impl IntoResponse for PresenceClosedTemplate {
    fn into_response(self) -> Response {
        html(self.render(), "presence closed")
    }
}

impl IntoResponse for PresenceUnknownTemplate {
    fn into_response(self) -> Response {
        html(self.render(), "presence unknown")
    }
}

// ── Les deux handlers ────────────────────────────────────────────────────────

pub async fn presence_oui(Path(token): Path<String>, State(state): State<AppState>) -> Response {
    repondre(&token, Venue::Presente, &state).await
}

pub async fn presence_non(Path(token): Path<String>, State(state): State<AppState>) -> Response {
    repondre(&token, Venue::Absente, &state).await
}

/// Convertir, charger, enregistrer, rendre — dans cet ordre, et chacun sa
/// fonction.
async fn repondre(brut: &str, venue: Venue, state: &AppState) -> Response {
    let inconnue = || PresenceUnknownTemplate { css: css() }.into_response();

    // R26 — un jeton illisible et un jeton inconnu rendent la **même** page.
    let Ok(token) = SurveyToken::try_new(brut) else {
        return inconnue();
    };
    let Some(survey) = charger(brut, state).await else {
        return inconnue();
    };
    let Some(reponse) = survey.reponse_par_jeton(&token) else {
        return inconnue();
    };
    let team_id = *reponse.team_id();

    let issue = record_answer_use_case::execute(
        record_answer_use_case::RecordAnswerCommand {
            round_id: *survey.round_id(),
            team_id,
            venue,
            // R28 — ici le **lien** autorise : il n'y a aucun propriétaire
            // d'équipe à confronter, et lui opposer un `CoachId` comparerait la
            // réponse à elle-même.
            par: Repondant::Jeton,
            aujourd_hui: aujourd_hui(),
        },
        state.competitions.presence_survey_repository.as_ref(),
        state.competitions.match_day_repository.as_ref(),
        state.competitions.match_report_status_port.as_ref(),
    )
    .await;

    // Le contexte est hydraté **après** l'écriture : la page doit montrer la
    // réponse qui vient d'être posée, pas celle d'avant.
    let Some(apres) = recharger_le_contexte(brut, &token, state).await else {
        return inconnue();
    };

    match issue {
        Ok(_) => rendre_la_reponse(&apres, venue, brut),
        Err(record_answer_use_case::RecordAnswerError::Refus(motif)) => {
            rendre_la_cloture(&apres, &motif)
        }
        // R19 est inatteignable par ce chemin — le jeton *est* la désignation de
        // la réponse — et les autres cas sont des pannes. On ne montre pas un
        // motif technique à un visiteur anonyme.
        Err(autre) => {
            tracing::error!("presence publique : {autre:?}");
            inconnue()
        }
    }
}

async fn charger(token: &str, state: &AppState) -> Option<PresenceSurvey> {
    state
        .competitions
        .presence_survey_repository
        .find_by_token(token)
        .await
        .ok()
        .flatten()
}

async fn recharger_le_contexte(
    brut: &str,
    token: &SurveyToken,
    state: &AppState,
) -> Option<LandingContext> {
    let survey = charger(brut, state).await?;
    presence_landing_service::hydrater(
        &survey,
        token,
        state.competitions.presence_survey_repository.as_ref(),
        state.competitions.team_info_port.as_ref(),
        &aujourd_hui(),
    )
    .await
}

fn rendre_la_reponse(ctx: &LandingContext, venue: Venue, token: &str) -> Response {
    let presente = matches!(venue, Venue::Presente);
    let routes = AppRoutes::default();
    PresenceResponseTemplate {
        css: css(),
        team_name: ctx.team_name.clone(),
        round_name: ctx.round_name.clone(),
        round_dates: dates_lisibles(ctx),
        competition_name: ctx.competition_name.clone(),
        // L'URL se compose **ici**, dans la couche web : le service d'hydratation
        // rend des identifiants, et lui faire connaître `AppRoutes` l'aurait fait
        // dépendre de la couche web (carte 524).
        competition_url: routes.competitions.competition_detail(
            &ctx.space_id,
            &ctx.competition_id,
            &ctx.season_id,
        ),
        deadline: ctx.deadline.to_string(),
        presente,
        url_opposee: if presente {
            routes.competitions.presence_non(token)
        } else {
            routes.competitions.presence_oui(token)
        },
        libelle_oppose: if presente {
            "Finalement, je ne viens pas".to_string()
        } else {
            "Finalement, je viens".to_string()
        },
    }
    .into_response()
}

fn rendre_la_cloture(ctx: &LandingContext, motif: &DomainError) -> Response {
    PresenceClosedTemplate {
        css: css(),
        team_name: ctx.team_name.clone(),
        round_name: ctx.round_name.clone(),
        competition_name: ctx.competition_name.clone(),
        motif: libelle_du_motif(ctx, motif),
    }
    .into_response()
}

/// R27 — trois motifs, et l'écran dit lequel.
///
/// Le statut du contexte distingue l'échéance de la décision ; `RoundFrozenByReport`
/// est le troisième. Un message unique — « le sondage est clos » — laisserait le
/// coach se demander s'il a raté la date ou si la journée s'est jouée sans lui.
fn libelle_du_motif(ctx: &LandingContext, motif: &DomainError) -> String {
    match motif {
        DomainError::RoundFrozenByReport => {
            "Cette journée a déjà été jouée : un rapport de match y est publié.".to_string()
        }
        _ => match ctx.statut {
            SurveyStatus::Close(Motif::Decision) => {
                "L'organisateur a clos le sondage avant l'échéance.".to_string()
            }
            SurveyStatus::Close(Motif::Echeance) => {
                format!("La date limite de réponse était le {}.", ctx.deadline)
            }
            SurveyStatus::Ouverte => "Le sondage n'accepte plus de réponse.".to_string(),
        },
    }
}

/// Le libellé des dates, composé **ici** : le service rend les bornes brutes, et
/// le mettre en forme relève de la vue (cartes 523 et 524).
fn dates_lisibles(ctx: &LandingContext) -> String {
    match (&ctx.round_date_start, &ctx.round_date_end) {
        (Some(d), Some(f)) if d != f => format!("du {d} au {f}"),
        (Some(d), _) => d.clone(),
        _ => String::new(),
    }
}

fn css() -> &'static str {
    crate::web::css_bundle::chemin_app()
}

fn aujourd_hui() -> DateString {
    let brut = OffsetDateTime::now_utc()
        .date()
        .format(format_description!("[year]-[month]-[day]"))
        .unwrap_or_default();
    DateString::try_new(brut).unwrap_or_default()
}

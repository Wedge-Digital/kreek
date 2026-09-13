//! L'encart du coach connecté : on attend sa réponse, il répond sur place.
//!
//! # Le second chemin de réponse
//!
//! Le premier est le lien reçu par e-mail (unité 2). Celui-ci existe pour quand
//! l'e-mail se perd — filtré, supprimé, jamais arrivé faute d'adresse (R3) — et
//! il se trouve là où le coach va déjà : la page de détail de sa compétition.
//!
//! # Les deux handlers dans un fichier
//!
//! L'onglet d'administration en sépare trois parce qu'il porte deux widgets et
//! dix actions. Ici l'action rend **le même fragment** que le GET, et les séparer
//! ferait deux fichiers dont l'un compterait vingt lignes.
//!
//! # La garde, et ce qu'elle n'est pas
//!
//! `saison_de_la_competition` (carte 530) — **et rien d'autre**. N'importe quel
//! coach voit l'encart ; ce qui le protège est qu'il ne montre que ses propres
//! équipes, et que R28 est vérifiée par le domaine à l'écriture.
//!
//! Poser `require_admin_access` ici aurait réservé l'encart aux organisateurs,
//! c'est-à-dire à ceux qui n'en ont pas besoin.
//!
//! # R28 — l'identité vient de la session
//!
//! Le corps de la requête porte l'équipe, jamais le coach. `Repondant::Coach(id
//! du connecté)` est construit ici, et c'est l'agrégat qui refuse une équipe qui
//! n'appartient pas à ce coach.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::presence_survey::{Presence, Repondant, Venue};
use crate::app::competitions::io::web::admin::admin_scope::saison_de_la_competition;
use crate::app::competitions::use_cases::presences::presence_call_service::{
    self, CampagneOuverte,
};
use crate::app::competitions::use_cases::presences::record_answer_use_case;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::MatchId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use time::macros::format_description;
use time::OffsetDateTime;

// ── Les view models ──────────────────────────────────────────────────────────

/// Ce que le coach a répondu pour une équipe.
///
/// **Un enum et non trois booléens** : le gabarit le `match`, et les
/// combinaisons absurdes — présente *et* absente, présente sans horodatage — sont
/// inexprimables. C'est la raison qui a fait choisir `Presence` à données portées
/// dans le domaine, appliquée à la présentation.
pub enum EtatEquipeVm {
    Attendue,
    Presente { repondu_le: String },
    Absente { repondu_le: String },
}

impl EtatEquipeVm {
    fn from_domain(presence: &Presence) -> Self {
        match presence {
            Presence::SansReponse => Self::Attendue,
            Presence::Declaree {
                venue: Venue::Presente,
                le,
                ..
            } => Self::Presente {
                repondu_le: le.to_string(),
            },
            Presence::Declaree { le, .. } => Self::Absente {
                repondu_le: le.to_string(),
            },
        }
    }
}

pub struct EquipeLigneVm {
    pub team_id: String,
    pub team_name: String,
    pub etat: EtatEquipeVm,
}

pub struct CampagneVm {
    pub round_id: String,
    /// « Journée 3 », « J1 », « Finale » — **le nom que l'organisateur a choisi,
    /// intact**.
    ///
    /// Le premier jet écrivait « Seras-tu là pour la {} ? » avec un
    /// `to_lowercase()`, pour que « Journée 3 » se lise après l'article. Sur la
    /// base de démonstration, dont la journée s'appelle « J1 », ça donnait
    /// « Seras-tu là pour la j1 ? » : une valeur du domaine abîmée par la vue.
    ///
    /// Aucun article ne précède donc le nom, dans aucune des trois phrases : c'est
    /// la seule formulation qui reste juste quel que soit le nom saisi.
    pub round_name: String,
    pub deadline: String,
    /// La question, pour une équipe qui n'a pas répondu.
    pub titre: String,
    pub sous_titre: String,
    pub equipes: Vec<EquipeLigneVm>,
}

impl CampagneVm {
    fn from_domain(c: &CampagneOuverte) -> Self {
        Self {
            round_id: c.round_id.to_string(),
            round_name: c.round_name.clone(),
            deadline: c.deadline.to_string(),
            titre: format!("Seras-tu là pour {} ?", c.round_name),
            sous_titre: sous_titre(c),
            equipes: c
                .mes_equipes
                .iter()
                .map(|e| EquipeLigneVm {
                    team_id: e.team_id.to_string(),
                    team_name: e.team_name.clone(),
                    etat: EtatEquipeVm::from_domain(&e.presence),
                })
                .collect(),
        }
    }

    fn all_from_domain(campagnes: &[CampagneOuverte]) -> Vec<Self> {
        campagnes.iter().map(Self::from_domain).collect()
    }
}

/// « Du 12 au 19 octobre · réponse attendue avant le 10 octobre · 9 équipes ont
/// déjà confirmé ».
///
/// Composé **ici** et non dans le service : le service rend des bornes brutes, et
/// la mise en forme relève de la vue (cartes 523 et 524).
///
/// R29 — le compte, jamais la liste. « 9 équipes ont confirmé » ne devient jamais
/// « Les Rats, Les Crocs et sept autres » : une réponse est donnée à
/// l'organisateur, pas publiée aux autres coachs.
fn sous_titre(c: &CampagneOuverte) -> String {
    let mut morceaux = Vec::new();
    if let Some(dates) = dates_de(c) {
        morceaux.push(dates);
    }
    morceaux.push(format!("réponse attendue avant le {}", c.deadline));
    morceaux.push(match c.confirmes {
        0 => "aucune équipe n'a encore confirmé".to_string(),
        1 => "1 équipe a déjà confirmé".to_string(),
        n => format!("{n} équipes ont déjà confirmé"),
    });
    morceaux.join(" · ")
}

fn dates_de(c: &CampagneOuverte) -> Option<String> {
    match (&c.date_start, &c.date_end) {
        (Some(d), Some(f)) if d != f => Some(format!("du {d} au {f}")),
        (Some(d), _) => Some(d.clone()),
        _ => None,
    }
}

// ── Les gabarits ─────────────────────────────────────────────────────────────

/// `campagnes` vide rend **une page inchangée** : pas d'encart vide, pas de
/// message, pas de bordure. Un `<div>` de zéro hauteur laisserait sa marge, et
/// l'espacement de la page bougerait selon qu'un sondage est ouvert ou non.
#[derive(Template)]
#[template(path = "widgets/presence-call.html")]
pub struct PresenceCallTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub campagnes: Vec<CampagneVm>,
}

/// Ce qu'on rend quand la réponse arrive trop tard.
///
/// **Ce n'est pas une politesse.** Sans elle, un clic arrivé après la clôture
/// réafficherait un encart vide : il disparaîtrait sous le curseur, et le coach
/// lirait ce vide comme un succès.
#[derive(Template)]
#[template(path = "widgets/presence-call-refus.html")]
pub struct PresenceCallRefusTemplate {
    pub motif: String,
}

/// R30 — la réponse est prise, et elle défait une rencontre déjà tirée.
///
/// **Aucun nouvel adversaire n'est annoncé** : la réparation est une proposition
/// que l'organisateur valide (carte 518), et en annoncer un produirait deux coachs
/// qui se croient appariés sur un match qui n'existe pas. **Et pas de silence non
/// plus** : le coach vient de défaire un match que son adversaire avait noté.
#[derive(Template)]
#[template(path = "widgets/presence-call-refait.html")]
pub struct PresenceCallRefaitTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub campagnes: Vec<CampagneVm>,
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

impl IntoResponse for PresenceCallTemplate {
    fn into_response(self) -> Response {
        html(self.render(), "presence call")
    }
}

impl IntoResponse for PresenceCallRefusTemplate {
    fn into_response(self) -> Response {
        html(self.render(), "presence call refus")
    }
}

impl IntoResponse for PresenceCallRefaitTemplate {
    fn into_response(self) -> Response {
        html(self.render(), "presence call refait")
    }
}

// ── Le DTO d'entrée ──────────────────────────────────────────────────────────

/// Identique à l'`AnswerBody` de l'administration, et **ce n'est pas une occasion
/// de factoriser** : les deux portent des routes différentes et se retrouveraient
/// couplés au premier champ que l'un gagnerait sans l'autre. Trois champs
/// dupliqués coûtent moins qu'une dépendance entre deux écrans.
#[derive(Deserialize)]
pub struct PresenceCallAnswerBody {
    pub round_id: String,
    pub team_id: String,
    pub presence: String,
}

// ── Les deux handlers ────────────────────────────────────────────────────────

pub async fn get_presence_call(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Some(coach) = connecte(&auth_session) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if let Err(refus) = saison_de_la_competition(&season_id, &competition_id, &state).await {
        return refus;
    }
    rendre(&space_id, &competition_id, &season_id, &coach, &state).await
}

pub async fn post_presence_call_answer(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(body): Json<PresenceCallAnswerBody>,
) -> Response {
    let Some(coach) = connecte(&auth_session) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if let Err(refus) = saison_de_la_competition(&season_id, &competition_id, &state).await {
        return refus;
    }
    let Some(cmd) = commande(&body, &coach) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let issue = record_answer_use_case::execute(
        cmd,
        state.competitions.presence_survey_repository.as_ref(),
        state.competitions.match_day_repository.as_ref(),
        state.competitions.match_report_status_port.as_ref(),
    )
    .await;

    repondre(
        issue,
        &space_id,
        &competition_id,
        &season_id,
        &coach,
        &state,
    )
    .await
}

/// **L'identité vient de la session, jamais du corps** (R28).
fn connecte(auth_session: &AuthSession) -> Option<CoachId> {
    auth_session.user.as_ref().map(|u| u.id)
}

/// `None` sur un champ mal formé : c'est un défaut de format, pas un refus
/// métier, et les confondre ferait afficher « valeur inconnue » comme une règle du
/// jeu.
fn commande(
    body: &PresenceCallAnswerBody,
    coach: &CoachId,
) -> Option<record_answer_use_case::RecordAnswerCommand> {
    Some(record_answer_use_case::RecordAnswerCommand {
        round_id: MatchId::try_new(&body.round_id).ok()?,
        team_id: TeamId::try_new(&body.team_id).ok()?,
        venue: venue_de(&body.presence)?,
        par: Repondant::Coach(*coach),
        aujourd_hui: aujourd_hui(),
    })
}

fn venue_de(brut: &str) -> Option<Venue> {
    match brut {
        "presente" => Some(Venue::Presente),
        "absente" => Some(Venue::Absente),
        _ => None,
    }
}

/// Trois issues, et deux d'entre elles viennent du même `Refus(DomainError)` —
/// c'est au handler de trancher sur la variante.
async fn repondre(
    issue: Result<record_answer_use_case::AnswerOutcome, record_answer_use_case::RecordAnswerError>,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    coach: &CoachId,
    state: &AppState,
) -> Response {
    match issue {
        Ok(outcome) if outcome.rencontre_a_refaire.is_some() => {
            refait(space_id, competition_id, season_id, coach, state).await
        }
        Ok(_) => rendre(space_id, competition_id, season_id, coach, state).await,
        Err(record_answer_use_case::RecordAnswerError::Refus(motif)) => refus_de(motif),
        Err(autre) => {
            tracing::error!("encart de présence : {autre:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// **Un `403` muet est correct ici**, et c'est le raisonnement de R26 : ces deux
/// refus ne viennent que d'un `team_id` forgé, il n'y a pas d'utilisateur à
/// renseigner, et une carte « cette équipe n'est pas la vôtre » confirmerait à qui
/// essaie que l'équipe existe.
fn refus_de(motif: DomainError) -> Response {
    match motif {
        DomainError::TeamNotOwnedByCoach { .. } | DomainError::TeamNotInSurvey { .. } => {
            StatusCode::FORBIDDEN.into_response()
        }
        autre => PresenceCallRefusTemplate {
            motif: autre.to_string(),
        }
        .into_response(),
    }
}

async fn rendre(
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    coach: &CoachId,
    state: &AppState,
) -> Response {
    PresenceCallTemplate {
        app_routes: AppRoutes::default(),
        space_id: space_id.to_string(),
        competition_id: competition_id.to_string(),
        season_id: season_id.to_string(),
        campagnes: CampagneVm::all_from_domain(&charger(season_id, coach, state).await),
    }
    .into_response()
}

async fn refait(
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    coach: &CoachId,
    state: &AppState,
) -> Response {
    PresenceCallRefaitTemplate {
        app_routes: AppRoutes::default(),
        space_id: space_id.to_string(),
        competition_id: competition_id.to_string(),
        season_id: season_id.to_string(),
        campagnes: CampagneVm::all_from_domain(&charger(season_id, coach, state).await),
    }
    .into_response()
}

async fn charger(season_id: &str, coach: &CoachId, state: &AppState) -> Vec<CampagneOuverte> {
    presence_call_service::hydrater(
        season_id,
        coach,
        state.competitions.presence_survey_repository.as_ref(),
        state.competitions.match_day_repository.as_ref(),
        state.competitions.team_info_port.as_ref(),
        &aujourd_hui(),
    )
    .await
}

fn aujourd_hui() -> DateString {
    let brut = OffsetDateTime::now_utc()
        .date()
        .format(format_description!("[year]-[month]-[day]"))
        .unwrap_or_default();
    DateString::try_new(brut).unwrap_or_default()
}

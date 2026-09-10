//! Les neuf actions de l'onglet Présences, et le protocole qui vaut pour elles.
//!
//! # Ce que rend une action
//!
//! | Cas | Réponse |
//! |---|---|
//! | succès | corps vide + `HX-Trigger: presenceChanged` |
//! | refus métier | le panneau porteur du motif, `HX-Retarget` + `HX-Reswap`, **sans** trigger |
//! | `draw` | l'aperçu, `HX-Retarget` + `HX-Reswap`, sans trigger |
//! | corps mal formé, VO invalide | `400` — refus de **format**, pas de règle |
//! | panne | `500` + `tracing::error!` |
//!
//! Les boutons portent `hx-swap="none"` : le succès ne remplace rien, les deux
//! widgets se rechargent d'eux-mêmes sur `presenceChanged`. **C'est le serveur qui
//! redirige le swap** vers le panneau quand il a quelque chose à y dire —
//! mécanisme htmx standard, sans une ligne de JavaScript.
//!
//! Rendre le fragment *et* déclencher le rechargement peindrait le panneau deux
//! fois ; rendre un corps vide ne laisserait nulle part où poser le refus.
//!
//! **`draw` ne déclenche pas `presenceChanged`**, et c'est ce qui le rend
//! possible : l'aperçu ne persiste rien, un rechargement du panneau le perdrait
//! aussitôt.
//!
//! # Aucune `alert()`, aucun JSON d'erreur
//!
//! Écart assumé avec le Calendrier, qui expose `window.handleScheduleActionResponse`
//! et lève une boîte du navigateur. Un motif de refus est une **explication**, et
//! elle appartient à l'écran qu'elle commente. Une fois les explications dans le
//! panneau, les refus y vont aussi — sinon il faudrait trancher pour chaque
//! nouveau message de quel côté il tombe, et cette frontière dérive.
//!
//! # La garde, sur chaque POST
//!
//! `require_admin_access` puis `journee_de_la_saison`. Le second n'est pas
//! redondant : `round_id` arrive dans le **corps**, et `space_scope` n'a pas de
//! résolveur pour lui — il passerait librement (carte 416).

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::match_day::MatchDay;
use crate::app::competitions::domain::presence_survey::{
    AutoRemind, Repondant, SurveyDeadline, Venue,
};
use crate::app::competitions::io::web::admin::admin_page::require_admin_access;
use crate::app::competitions::io::web::admin::admin_scope::journee_de_la_saison;
use crate::app::competitions::io::web::admin::presences_widgets::{
    charger_le_panneau, rendre_reparation, rendre_tirage, ActionsVm,
};
use crate::app::competitions::use_cases::presences::survey_roster_service;
use crate::app::competitions::use_cases::presences::{
    close_survey_use_case, confirm_draw_use_case, draw_pairings_use_case, launch_survey_use_case,
    propose_repair_use_case, record_answer_use_case, remind_use_case, reopen_survey_use_case,
    repair_pairing_use_case, undo_draw_use_case,
};
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{PairingId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use time::macros::format_description;
use time::OffsetDateTime;

// ── Le protocole ─────────────────────────────────────────────────────────────

/// Corps vide : le succès ne remplace rien, les deux widgets se rechargent sur
/// l'événement.
fn succes() -> Response {
    Response::builder()
        .header("HX-Trigger", "presenceChanged")
        .body(Body::empty())
        .expect("réponse de succès")
}

/// Le panneau, avec son motif, redirigé vers le conteneur du panneau — **sans**
/// trigger, sans quoi le panneau se rechargerait aussitôt par-dessus le motif.
async fn refus(ctx: &Contexte<'_>, motif: String) -> Response {
    match charger_le_panneau(
        ctx.space_id,
        ctx.competition_id,
        ctx.season_id,
        &ctx.round,
        ctx.state,
        motif,
    )
    .await
    {
        Ok(resp) => vers_le_panneau(resp),
        Err(resp) => resp,
    }
}

/// `HX-Retarget` et `HX-Reswap` : **c'est le serveur qui redirige le swap**. Les
/// boutons posent `hx-swap="none"` et n'ont donc pas à connaître la cible.
pub fn vers_le_panneau(mut resp: Response) -> Response {
    let entetes = resp.headers_mut();
    entetes.insert("HX-Retarget", "#presences-panel".parse().expect("en-tête"));
    entetes.insert("HX-Reswap", "innerHTML".parse().expect("en-tête"));
    resp
}

/// Ce que les neuf handlers ont en commun, une fois les gardes franchies.
pub struct Contexte<'a> {
    pub space_id: &'a str,
    pub competition_id: &'a str,
    pub season_id: &'a str,
    /// **Porté, pas emprunté** : il naît dans `contexte()`, et le rendre par
    /// référence obligerait chaque handler à garder une variable locale rien que
    /// pour le tenir en vie.
    pub round: MatchDay,
    pub state: &'a AppState,
    /// L'organisateur connecté — R6 : l'identifiant vient de la session, jamais
    /// du corps de la requête.
    pub organisateur: CoachId,
}

/// Les deux gardes, et le contexte qu'elles produisent.
///
/// Rendre la journée plutôt qu'un booléen évite aux neuf handlers de la relire :
/// un contrôle qui coûte une requête de plus se contourne un jour « pour la
/// performance ». C'est la raison qui a fait rendre l'agrégat à
/// `journee_de_la_saison`, et elle vaut d'un étage plus haut.
async fn contexte<'a>(
    auth_session: &AuthSession,
    space_id: &'a str,
    competition_id: &'a str,
    season_id: &'a str,
    round_id: &str,
    state: &'a AppState,
) -> Result<Contexte<'a>, Response> {
    require_admin_access(auth_session, space_id, competition_id, season_id, state).await?;
    let round = journee_de_la_saison(round_id, season_id, state).await?;
    // `UserId` et `CoachId` sont le même type — `EntityId`. Le prendre tel quel
    // plutôt que par un aller-retour en chaîne, qui pouvait échouer pour rien.
    let organisateur = auth_session
        .user
        .as_ref()
        .map(|u| u.id)
        .ok_or_else(|| StatusCode::UNAUTHORIZED.into_response())?;
    Ok(Contexte {
        space_id,
        competition_id,
        season_id,
        round,
        state,
        organisateur,
    })
}

fn aujourd_hui() -> DateString {
    let brut = OffsetDateTime::now_utc()
        .date()
        .format(format_description!("[year]-[month]-[day]"))
        .unwrap_or_default();
    DateString::try_new(brut).unwrap_or_default()
}

// ── Les DTO d'entrée ─────────────────────────────────────────────────────────

/// La campagne est désignée par `round_id`, **jamais par `survey_id`** : R2
/// garantit une campagne vivante par journée, et le client n'a pas à connaître un
/// identifiant qu'il ne lit nulle part.
#[derive(Deserialize)]
pub struct SurveyIdBody {
    pub round_id: String,
}

#[derive(Deserialize)]
pub struct LaunchBody {
    pub round_id: String,
    pub deadline: String,
    #[serde(default)]
    pub auto_remind: bool,
}

#[derive(Deserialize)]
pub struct AnswerBody {
    pub round_id: String,
    pub team_id: String,
    /// `presente` ou `absente`. Une autre valeur est un `400` : c'est du format.
    pub presence: String,
}

#[derive(Deserialize)]
pub struct ReopenBody {
    pub round_id: String,
    /// R23 — rouvrir exige une nouvelle échéance.
    pub deadline: String,
}

// ── Les cinq actions du cycle ────────────────────────────────────────────────

pub async fn post_launch(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(body): Json<LaunchBody>,
) -> Response {
    let ctx = match contexte(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &body.round_id,
        &state,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let (Ok(deadline), Ok(saison), Ok(espace)) = (
        SurveyDeadline::try_new(body.deadline.clone()),
        SeasonId::try_new(&season_id),
        SpaceId::try_new(&space_id),
    ) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let issue = launch_survey_use_case::execute(
        launch_survey_use_case::LaunchSurveyCommand {
            season_id: saison,
            space_id: espace,
            round_id: ctx.round.id,
            deadline,
            auto_remind: AutoRemind::new(body.auto_remind),
            aujourd_hui: aujourd_hui(),
        },
        launch_survey_use_case::LaunchDeps {
            survey_repo: state.competitions.presence_survey_repository.as_ref(),
            match_day_repo: state.competitions.match_day_repository.as_ref(),
            teams: state.competitions.team_info_port.as_ref(),
            members: state.competitions.space_member_port.as_ref(),
            mailer: state.competitions.survey_mailer.as_ref(),
        },
    )
    .await;

    match issue {
        Ok(_) => succes(),
        Err(launch_survey_use_case::LaunchSurveyError::Refus(e)) => {
            refus(&ctx, e.to_string()).await
        }
        Err(launch_survey_use_case::LaunchSurveyError::SurveyAlreadyExists) => {
            refus(
                &ctx,
                "Une campagne existe déjà sur cette journée : rouvrez-la plutôt que \
                 d'en lancer une seconde, sans quoi ses réponses seraient perdues."
                    .to_string(),
            )
            .await
        }
        Err(autre) => panne("launch", autre),
    }
}

pub async fn post_answer(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(body): Json<AnswerBody>,
) -> Response {
    let ctx = match contexte(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &body.round_id,
        &state,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let (Ok(team_id), Some(venue)) = (TeamId::try_new(&body.team_id), venue_de(&body.presence))
    else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let issue = record_answer_use_case::execute(
        record_answer_use_case::RecordAnswerCommand {
            round_id: ctx.round.id,
            team_id,
            venue,
            // R6 — l'organisateur, et son identifiant vient de la session.
            par: Repondant::Organisateur(ctx.organisateur),
            aujourd_hui: aujourd_hui(),
        },
        state.competitions.presence_survey_repository.as_ref(),
        state.competitions.match_day_repository.as_ref(),
        state.competitions.match_report_status_port.as_ref(),
    )
    .await;

    match issue {
        Ok(_) => succes(),
        Err(record_answer_use_case::RecordAnswerError::Refus(e)) => {
            refus(&ctx, e.to_string()).await
        }
        Err(autre) => panne("answer", autre),
    }
}

/// `presente` ou `absente`, et rien d'autre. Une valeur inconnue est un `400` :
/// c'est un défaut de format, pas un refus métier — et les confondre ferait
/// afficher « valeur inconnue » comme une règle du jeu.
fn venue_de(brut: &str) -> Option<Venue> {
    match brut {
        "presente" => Some(Venue::Presente),
        "absente" => Some(Venue::Absente),
        _ => None,
    }
}

pub async fn post_remind(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(body): Json<SurveyIdBody>,
) -> Response {
    let ctx = match contexte(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &body.round_id,
        &state,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let (Ok(saison), Ok(espace)) = (SeasonId::try_new(&season_id), SpaceId::try_new(&space_id))
    else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let issue = remind_use_case::execute(
        remind_use_case::RemindCommand {
            round_id: ctx.round.id,
            season_id: saison,
            space_id: espace,
            aujourd_hui: aujourd_hui(),
        },
        remind_use_case::RemindDeps {
            survey_repo: state.competitions.presence_survey_repository.as_ref(),
            match_day_repo: state.competitions.match_day_repository.as_ref(),
            teams: state.competitions.team_info_port.as_ref(),
            members: state.competitions.space_member_port.as_ref(),
            mailer: state.competitions.survey_mailer.as_ref(),
        },
    )
    .await;

    match issue {
        Ok(_) => succes(),
        Err(remind_use_case::RemindError::SurveyClosed) => {
            refus(
                &ctx,
                "Le sondage est clos : ses liens ne répondent plus, et une relance \
                 enverrait un e-mail qui se contredit lui-même. Rouvrez-le d'abord."
                    .to_string(),
            )
            .await
        }
        Err(autre) => panne("remind", autre),
    }
}

pub async fn post_close(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(body): Json<SurveyIdBody>,
) -> Response {
    let ctx = match contexte(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &body.round_id,
        &state,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let issue = close_survey_use_case::execute(
        close_survey_use_case::CloseSurveyCommand {
            round_id: ctx.round.id,
            aujourd_hui: aujourd_hui(),
        },
        state.competitions.presence_survey_repository.as_ref(),
    )
    .await;

    match issue {
        Ok(()) => succes(),
        Err(close_survey_use_case::CloseSurveyError::Refus(e)) => refus(&ctx, e.to_string()).await,
        Err(autre) => panne("close", autre),
    }
}

pub async fn post_reopen(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(body): Json<ReopenBody>,
) -> Response {
    let ctx = match contexte(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &body.round_id,
        &state,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let Ok(deadline) = SurveyDeadline::try_new(body.deadline.clone()) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let issue = reopen_survey_use_case::execute(
        reopen_survey_use_case::ReopenSurveyCommand {
            round_id: ctx.round.id,
            deadline,
            aujourd_hui: aujourd_hui(),
        },
        reopen_survey_use_case::ReopenDeps {
            survey_repo: state.competitions.presence_survey_repository.as_ref(),
            match_day_repo: state.competitions.match_day_repository.as_ref(),
            report_status: state.competitions.match_report_status_port.as_ref(),
        },
    )
    .await;

    match issue {
        Ok(()) => succes(),
        Err(reopen_survey_use_case::ReopenSurveyError::Refus(e)) => {
            refus(&ctx, e.to_string()).await
        }
        Err(autre) => panne("reopen", autre),
    }
}

/// Une panne est un `500` **et une ligne de journal**. Le motif technique ne part
/// pas à l'écran : l'organisateur ne peut rien en faire, et il dirait à un membre
/// simple ce que la base contient.
fn panne(action: &str, e: impl std::fmt::Debug) -> Response {
    tracing::error!(action, "présences : {e:?}");
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

// ── Les quatre actions du tirage ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PairBody {
    pub home_team_id: String,
    pub away_team_id: String,
}

/// **`PairBody` ne porte pas `historique`** : le client n'a pas à renvoyer un motif
/// que le serveur recalcule de toute façon (carte 517), et le lui faire porter
/// autoriserait à mentir sur l'historique d'une rencontre — « 1re rencontre » sur
/// une revanche.
#[derive(Deserialize)]
pub struct ConfirmDrawBody {
    pub round_id: String,
    pub rencontres: Vec<PairBody>,
    #[serde(default)]
    pub exemptee: Option<String>,
}

#[derive(Deserialize)]
pub struct RepairBody {
    pub round_id: String,
    /// Les rencontres que la défection rend caduques.
    pub a_defaire: Vec<String>,
    pub rencontres: Vec<PairBody>,
    #[serde(default)]
    pub exemptee: Option<String>,
}

/// Les couples, convertis par smart constructor. Un identifiant illisible est un
/// `400` : c'est du format, pas une règle du jeu.
fn paires_de(brut: &[PairBody]) -> Option<Vec<(TeamId, TeamId)>> {
    brut.iter()
        .map(|p| {
            Some((
                TeamId::try_new(&p.home_team_id).ok()?,
                TeamId::try_new(&p.away_team_id).ok()?,
            ))
        })
        .collect()
}

fn exemptee_de(brut: &Option<String>) -> Result<Option<TeamId>, ()> {
    match brut.as_deref() {
        None | Some("") => Ok(None),
        Some(id) => TeamId::try_new(id).map(Some).map_err(|_| ()),
    }
}

/// **Un POST qui n'écrit rien.** Le verbe reste `POST` parce que le tirage est
/// aléatoire — donc non idempotent — et qu'un `GET` serait rejoué par le navigateur
/// au retour arrière, produisant un tirage différent de celui qu'on regardait.
///
/// Il ne déclenche **pas** `presenceChanged` : l'aperçu ne persiste rien, et un
/// rechargement du panneau le perdrait aussitôt.
pub async fn post_draw(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(body): Json<SurveyIdBody>,
) -> Response {
    let ctx = match contexte(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &body.round_id,
        &state,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let Ok(saison) = SeasonId::try_new(&season_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let issue = draw_pairings_use_case::execute(
        draw_pairings_use_case::DrawCommand {
            round_id: ctx.round.id,
            season_id: saison,
        },
        draw_pairings_use_case::DrawDeps {
            survey_repo: state.competitions.presence_survey_repository.as_ref(),
            match_day_repo: state.competitions.match_day_repository.as_ref(),
            teams: state.competitions.team_info_port.as_ref(),
        },
    )
    .await;

    match issue {
        Ok(prop) => {
            // Le roster nomme les équipes : sans lui, l'aperçu afficherait des
            // identifiants de vingt-six caractères — le défaut de la carte 506.
            let Ok(espace) = SpaceId::try_new(&space_id) else {
                return StatusCode::BAD_REQUEST.into_response();
            };
            let roster = match survey_roster_service::charger(
                &season_id,
                &espace,
                state.competitions.team_info_port.as_ref(),
                state.competitions.space_member_port.as_ref(),
            )
            .await
            {
                Ok(r) => r,
                Err(e) => return panne("draw/roster", e),
            };
            let presents = prop.rencontres.len() * 2 + usize::from(prop.exemptee.is_some());
            vers_le_panneau(rendre_tirage(
                ActionsVm::new(&space_id, &competition_id, &season_id),
                &ctx.round,
                &prop,
                &roster,
                presents,
            ))
        }
        Err(draw_pairings_use_case::DrawError::Refus(e)) => refus(&ctx, e.to_string()).await,
        Err(draw_pairings_use_case::DrawError::PairingsAlreadyExist) => {
            refus(&ctx, MOTIF_DEJA_APPARIEE.to_string()).await
        }
        Err(autre) => panne("draw", autre),
    }
}

/// R11 — l'organisateur vide la journée depuis le Calendrier s'il veut retirer.
/// Le motif le dit, plutôt que de laisser chercher.
const MOTIF_DEJA_APPARIEE: &str = "Cette journée porte déjà des rencontres. \
     Défaites l'appariement avant de retirer au sort.";

pub async fn post_confirm_draw(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(body): Json<ConfirmDrawBody>,
) -> Response {
    let ctx = match contexte(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &body.round_id,
        &state,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let (Some(rencontres), Ok(exemptee), Ok(saison), Ok(espace)) = (
        paires_de(&body.rencontres),
        exemptee_de(&body.exemptee),
        SeasonId::try_new(&season_id),
        SpaceId::try_new(&space_id),
    ) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let issue = confirm_draw_use_case::execute(
        confirm_draw_use_case::ConfirmDrawCommand {
            round_id: ctx.round.id,
            season_id: saison,
            competition_id: competition_id.clone(),
            space_id: espace,
            rencontres,
            exemptee,
        },
        confirm_draw_use_case::ConfirmDeps {
            survey_repo: state.competitions.presence_survey_repository.as_ref(),
            match_day_repo: state.competitions.match_day_repository.as_ref(),
            teams: state.competitions.team_info_port.as_ref(),
            event_bus: &state.competitions.event_bus,
        },
    )
    .await;

    match issue {
        Ok(_) => succes(),
        Err(confirm_draw_use_case::ConfirmDrawError::Refus(e)) => refus(&ctx, e.to_string()).await,
        Err(confirm_draw_use_case::ConfirmDrawError::PairingsAlreadyExist) => {
            refus(&ctx, MOTIF_DEJA_APPARIEE.to_string()).await
        }
        Err(autre) => panne("confirm-draw", autre),
    }
}

pub async fn post_undo_draw(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(body): Json<SurveyIdBody>,
) -> Response {
    let ctx = match contexte(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &body.round_id,
        &state,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let issue = undo_draw_use_case::execute(
        undo_draw_use_case::UndoDrawCommand {
            round_id: ctx.round.id,
        },
        undo_draw_use_case::UndoDeps {
            survey_repo: state.competitions.presence_survey_repository.as_ref(),
            match_day_repo: state.competitions.match_day_repository.as_ref(),
            report_status: state.competitions.match_report_status_port.as_ref(),
            teams: state.competitions.team_info_port.as_ref(),
            event_bus: &state.competitions.event_bus,
        },
    )
    .await;

    match issue {
        Ok(_) => succes(),
        Err(undo_draw_use_case::UndoDrawError::Refus(e)) => refus(&ctx, e.to_string()).await,
        Err(undo_draw_use_case::UndoDrawError::PartiellementDefait { conservees }) => {
            refus(
                &ctx,
                format!(
                    "{conservees} rencontre(s) n'ont pas pu être défaites : un rapport \
                     de match y a été publié entre-temps."
                ),
            )
            .await
        }
        Err(autre) => panne("undo-draw", autre),
    }
}

/// **Un POST qui n'écrit rien**, comme `draw`. La proposition de réparation sort de
/// `tirer` sur le vivier, qui départage au sort (R17) : la recalculer à chaque
/// affichage du panneau montrerait une proposition différente à chaque fois.
///
/// Il ne déclenche pas `presenceChanged`, pour la même raison que `draw`.
pub async fn post_propose_repair(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(body): Json<SurveyIdBody>,
) -> Response {
    let ctx = match contexte(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &body.round_id,
        &state,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let (Ok(saison), Ok(espace)) = (SeasonId::try_new(&season_id), SpaceId::try_new(&space_id))
    else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let issue = propose_repair_use_case::execute(
        propose_repair_use_case::ProposeRepairCommand {
            round_id: ctx.round.id,
            season_id: saison,
        },
        propose_repair_use_case::ProposeDeps {
            survey_repo: state.competitions.presence_survey_repository.as_ref(),
            match_day_repo: state.competitions.match_day_repository.as_ref(),
            report_status: state.competitions.match_report_status_port.as_ref(),
            teams: state.competitions.team_info_port.as_ref(),
        },
    )
    .await;

    match issue {
        Ok(prop) => {
            let roster = match survey_roster_service::charger(
                &season_id,
                &espace,
                state.competitions.team_info_port.as_ref(),
                state.competitions.space_member_port.as_ref(),
            )
            .await
            {
                Ok(r) => r,
                Err(e) => return panne("propose-repair/roster", e),
            };
            vers_le_panneau(rendre_reparation(
                ActionsVm::new(&space_id, &competition_id, &season_id),
                &ctx.round,
                &prop,
                &roster,
            ))
        }
        Err(propose_repair_use_case::ProposeRepairError::AucunDesaccord) => {
            refus(
                &ctx,
                "Les présences et les appariements concordent : il n'y a rien à \
                 réparer."
                    .to_string(),
            )
            .await
        }
        Err(autre) => panne("propose-repair", autre),
    }
}

pub async fn post_repair(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(body): Json<RepairBody>,
) -> Response {
    let ctx = match contexte(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &body.round_id,
        &state,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let a_defaire: Option<Vec<PairingId>> = body
        .a_defaire
        .iter()
        .map(|id| PairingId::try_new(id).ok())
        .collect();
    let (Some(a_defaire), Some(rencontres), Ok(exemptee), Ok(saison), Ok(espace)) = (
        a_defaire,
        paires_de(&body.rencontres),
        exemptee_de(&body.exemptee),
        SeasonId::try_new(&season_id),
        SpaceId::try_new(&space_id),
    ) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let issue = repair_pairing_use_case::execute(
        repair_pairing_use_case::RepairCommand {
            round_id: ctx.round.id,
            season_id: saison,
            competition_id: competition_id.clone(),
            space_id: espace,
            a_defaire,
            rencontres,
            exemptee,
        },
        repair_pairing_use_case::RepairDeps {
            survey_repo: state.competitions.presence_survey_repository.as_ref(),
            match_day_repo: state.competitions.match_day_repository.as_ref(),
            report_status: state.competitions.match_report_status_port.as_ref(),
            teams: state.competitions.team_info_port.as_ref(),
            event_bus: &state.competitions.event_bus,
        },
    )
    .await;

    match issue {
        Ok(_) => succes(),
        Err(repair_pairing_use_case::RepairError::Refus(e)) => refus(&ctx, e.to_string()).await,
        Err(autre) => panne("repair", autre),
    }
}

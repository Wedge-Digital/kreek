//! Le widget « Déplacer sur une autre journée » (carte 557), et ses trois
//! endpoints.
//!
//! **C'est `competitions` qui le sert, et la page du rapport qui le compose.**
//! Les journées, les appariements et la règle « une équipe joue une fois par
//! journée » sont à lui ; la page du rapport ne connaît que l'adresse du
//! widget, `pairing_id` cuit dedans.
//!
//! # Les trois endpoints
//!
//! - `GET …/widget?pairing_id=` — le fragment : bouton, panneau replié,
//!   journée actuelle ;
//! - `GET …/targets?pairing_id=` — les journées cibles en JSON pour le
//!   `kreek-select` : dans l'ordre du calendrier, **sans la journée courante,
//!   sans les journées de repos, sans celles où l'une des deux équipes est déjà
//!   engagée**. Le serveur refuse quand même ces dernières au POST — la liste
//!   évite un refus, elle ne remplace pas la garde ;
//! - `POST …/move-match` — l'action. Un succès rend le widget à jour et
//!   déclenche le toast global ; un refus est un `422` dont le corps porte le
//!   motif, que le script du widget passe au même toast, en erreur.
//!
//! Chacun commence par `require_admin_access`, comme toutes les actions du
//! calendrier.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
use crate::app::competitions::io::web::admin::admin_page::require_admin_access;
use crate::app::competitions::routes::Routes as CompetitionRoutes;
use crate::app::competitions::use_cases::admin::move_pairing_use_case::{
    self, MovePairingCommand, MovePairingError,
};
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId, SeasonId};
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::body::Body;
use axum::extract::{Form, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};

// ── Vue ──────────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "widgets/move-pairing-widget.html")]
pub struct MovePairingWidgetTemplate {
    pub pairing_id: String,
    pub current_round_name: String,
    pub targets_url: String,
    pub post_url: String,
}

impl IntoResponse for MovePairingWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("move_pairing_widget render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[derive(Deserialize)]
pub struct PairingQuery {
    pub pairing_id: String,
}

#[derive(Deserialize)]
pub struct MoveMatchForm {
    pub pairing_id: String,
    pub round_id: String,
}

#[derive(Serialize)]
pub struct TargetRoundJson {
    pub id: String,
    pub name: String,
    pub dates: String,
}

#[derive(Serialize)]
struct ErrorResult {
    error: String,
}

// ── GET widget ───────────────────────────────────────────────────────────────

pub async fn get_move_pairing_widget(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    Query(query): Query<PairingQuery>,
    State(state): State<AppState>,
) -> Response {
    if let Err(refus) = require_admin_access(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &state,
    )
    .await
    {
        return refus;
    }
    let Some((journee, _)) = journee_de(&query.pairing_id, &season_id, &state).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    widget(
        &space_id,
        &competition_id,
        &season_id,
        &query.pairing_id,
        &journee,
    )
    .into_response()
}

fn widget(
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    pairing_id: &str,
    journee: &MatchDay,
) -> MovePairingWidgetTemplate {
    let routes = CompetitionRoutes::default();
    MovePairingWidgetTemplate {
        pairing_id: pairing_id.to_string(),
        current_round_name: journee.name.as_ref().to_string(),
        targets_url: routes.admin_schedule_move_match_targets(
            space_id,
            competition_id,
            season_id,
            pairing_id,
        ),
        post_url: routes.admin_schedule_move_match(space_id, competition_id, season_id),
    }
}

// ── GET targets ──────────────────────────────────────────────────────────────

pub async fn get_move_pairing_targets(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    Query(query): Query<PairingQuery>,
    State(state): State<AppState>,
) -> Response {
    if let Err(refus) = require_admin_access(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &state,
    )
    .await
    {
        return refus;
    }
    let journees = match journees_de_la_saison(&season_id, &state).await {
        Some(j) => j,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let Some((courante, rencontre)) = trouver(&journees, &query.pairing_id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    Json(cibles(&journees, courante, rencontre)).into_response()
}

/// Les journées où la rencontre **peut** aller : jouables, autres que la
/// courante, et libres pour ses deux équipes — la règle de `MatchDay`, lue
/// d'avance pour ne proposer que ce qui sera accepté.
fn cibles(journees: &[MatchDay], courante: &MatchDay, rencontre: &Pairing) -> Vec<TargetRoundJson> {
    journees
        .iter()
        .filter(|j| !j.is_rest() && j.id != courante.id)
        .filter(|j| {
            j.engagement_existant(&rencontre.home_team_id, &rencontre.away_team_id)
                .is_none()
        })
        .map(|j| TargetRoundJson {
            id: j.id.to_string(),
            name: j.name.as_ref().to_string(),
            dates: dates_de(j),
        })
        .collect()
}

fn dates_de(j: &MatchDay) -> String {
    match (&j.date_start, &j.date_end) {
        (Some(s), Some(e)) if s == e => s.as_ref().to_string(),
        (Some(s), Some(e)) => format!("{} \u{2192} {}", s.as_ref(), e.as_ref()),
        (Some(s), None) => s.as_ref().to_string(),
        _ => String::new(),
    }
}

// ── POST move ────────────────────────────────────────────────────────────────

pub async fn post_move_pairing(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Form(form): Form<MoveMatchForm>,
) -> Response {
    if let Err(refus) = require_admin_access(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &state,
    )
    .await
    {
        return refus;
    }
    let Some(cmd) = commande(&form, &season_id, &space_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let resultat = move_pairing_use_case::execute(
        cmd,
        state.competitions.match_day_repository.as_ref(),
        state.competitions.team_info_port.as_ref(),
        &state.competitions.event_bus,
    )
    .await;
    match resultat {
        Ok(fait) => {
            deplace(
                &space_id,
                &competition_id,
                &season_id,
                &form.pairing_id,
                &state,
                &fait.to_round_name,
            )
            .await
        }
        Err(e) => refus(e),
    }
}

fn commande(form: &MoveMatchForm, season_id: &str, space_id: &str) -> Option<MovePairingCommand> {
    Some(MovePairingCommand {
        pairing_id: PairingId::try_new(&form.pairing_id).ok()?,
        to_round_id: MatchId::try_new(&form.round_id).ok()?,
        season_id: SeasonId::try_new(season_id).ok()?,
        space_id: SpaceId::try_new(space_id).ok()?,
    })
}

/// Le widget re-rendu sur sa nouvelle journée, et le toast global qui confirme.
/// Relu depuis le dépôt plutôt que reconstruit : c'est l'état écrit qu'on
/// montre, pas celui qu'on croit avoir écrit.
async fn deplace(
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    pairing_id: &str,
    state: &AppState,
    to_round_name: &str,
) -> Response {
    let Some((journee, _)) = journee_de(pairing_id, season_id, state).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let html = match widget(space_id, competition_id, season_id, pairing_id, &journee).render() {
        Ok(html) => html,
        Err(e) => {
            tracing::error!("move_pairing_widget render: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let toast = serde_json::json!({ "showToast": format!("Match déplacé en {to_round_name}") });
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("HX-Trigger", en_tete_ascii(&toast.to_string()))
        .body(Body::from(html))
        .unwrap()
}

/// Un en-tête HTTP n'est pas de l'UTF-8 : le navigateur le lit en Latin-1, et
/// « déplacé » arrivait « dÃ©placÃ© » dans le toast. Le JSON tolère `\uXXXX`,
/// et c'est la seule forme qui traverse un en-tête intacte.
fn en_tete_ascii(json: &str) -> String {
    let mut sortie = String::with_capacity(json.len());
    for c in json.chars() {
        if c.is_ascii() {
            sortie.push(c);
        } else {
            let mut unites = [0u16; 2];
            for u in c.encode_utf16(&mut unites) {
                sortie.push_str(&format!("\\u{u:04x}"));
            }
        }
    }
    sortie
}

/// Même forme que les refus du calendrier : `422` + `{ "error" }`, que le
/// script du widget rend en toast d'erreur persistant.
fn refus(e: MovePairingError) -> Response {
    let message = match e {
        MovePairingError::TeamAlreadyScheduled { equipe, adversaire } => {
            format!(
                "Match non déplacé : {equipe} affronte déjà {adversaire} lors de cette journée."
            )
        }
        MovePairingError::SameRound => "Match non déplacé : c'est déjà sa journée.".to_string(),
        MovePairingError::RoundOutsideSeason => {
            "Match non déplacé : cette journée appartient à une autre saison.".to_string()
        }
        MovePairingError::RoundNotFound => "Match non déplacé : journée introuvable.".to_string(),
        MovePairingError::PairingNotFound => return StatusCode::NOT_FOUND.into_response(),
        MovePairingError::Repository(err) => {
            tracing::error!("post_move_pairing: {err}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorResult { error: message }),
    )
        .into_response()
}

// ── Lectures ─────────────────────────────────────────────────────────────────

async fn journees_de_la_saison(season_id: &str, state: &AppState) -> Option<Vec<MatchDay>> {
    match state
        .competitions
        .match_day_repository
        .find_by_season(season_id)
        .await
    {
        Ok(j) => Some(j),
        Err(e) => {
            tracing::error!("move_pairing_widget: find_by_season {season_id}: {e}");
            None
        }
    }
}

/// La journée qui porte cette rencontre, et la rencontre. `None` si elle n'est
/// pas de cette saison — c'est aussi le contrôle de la carte 416 : un
/// `pairing_id` d'une autre saison ne passe pas.
async fn journee_de(
    pairing_id: &str,
    season_id: &str,
    state: &AppState,
) -> Option<(MatchDay, Pairing)> {
    let journees = journees_de_la_saison(season_id, state).await?;
    let (journee, rencontre) = trouver(&journees, pairing_id)?;
    Some((journee.clone(), rencontre.clone()))
}

fn trouver<'a>(journees: &'a [MatchDay], pairing_id: &str) -> Option<(&'a MatchDay, &'a Pairing)> {
    journees.iter().find_map(|j| {
        j.pairings
            .iter()
            .find(|p| p.id.to_string() == pairing_id)
            .map(|p| (j, p))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{
        MatchDayName, MatchDayPosition, MatchDayType,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;

    fn journee(position: i32, day_type: MatchDayType, pairings: Vec<Pairing>) -> MatchDay {
        MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new(format!("J{}", position + 1)).unwrap(),
            day_type,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(position).unwrap(),
            pairings,
        }
    }

    fn rencontre(a: &TeamId, b: &TeamId) -> Pairing {
        Pairing {
            id: PairingId::new(),
            home_team_id: a.clone(),
            away_team_id: b.clone(),
        }
    }

    #[test]
    fn l_en_tete_du_toast_ne_porte_que_de_l_ascii() {
        let json = r#"{"showToast":"Match déplacé en J1"}"#;
        let ascii = en_tete_ascii(json);
        assert!(ascii.is_ascii());
        assert_eq!(ascii, r#"{"showToast":"Match d\u00e9plac\u00e9 en J1"}"#);
        let relu: serde_json::Value = serde_json::from_str(&ascii).unwrap();
        assert_eq!(relu["showToast"], "Match déplacé en J1");
    }

    /// Ni la journée courante, ni le repos, ni la journée où l'une des deux
    /// équipes joue déjà : seule J4 reste, et dans l'ordre du calendrier.
    #[test]
    fn les_cibles_excluent_la_courante_le_repos_et_les_journees_prises() {
        let (a, b, c) = (TeamId::new(), TeamId::new(), TeamId::new());
        let deplacee = rencontre(&a, &b);
        let courante = journee(0, MatchDayType::FixedDate, vec![deplacee.clone()]);
        let repos = journee(1, MatchDayType::Rest, vec![]);
        let prise = journee(2, MatchDayType::FixedDate, vec![rencontre(&c, &a)]);
        let libre = journee(3, MatchDayType::FixedDate, vec![]);
        let journees = vec![courante.clone(), repos, prise, libre];

        let cibles = cibles(&journees, &courante, &deplacee);

        let noms: Vec<&str> = cibles.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(noms, vec!["J4"]);
    }
}

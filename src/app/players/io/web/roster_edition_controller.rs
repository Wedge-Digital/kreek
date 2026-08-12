use crate::app::auth::auth_backend::AuthSession;
use crate::app::players::domain::player::{PlayerId, TeamId};
use crate::app::players::domain::value_objects::{DisplayOrder, JerseyVo, PersonalName};
use crate::app::players::io::web::purchase_skill_controller::can_spend_spp;
use crate::app::players::io::web::widgets::player_table_widget::{
    build_player_rows, PlayerRowVm, PlayerTableTemplate,
};
use crate::app::players::ports::RepositoryError;
use crate::app::players::use_cases::commands::{RosterRowCommand, UpdateRosterCommand};
use crate::app::players::use_cases::update_roster_use_case::{self, UpdateRosterError};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

/// Trois tableaux parallèles, un élément par ligne du tableau, dans l'ordre
/// visuel — que le glisser-déposer réordonne dans le DOM.
///
/// Extracteur d'`axum-extra` et non celui d'axum : ce dernier s'appuie sur
/// `serde_urlencoded`, qui refuse les clés répétées (« invalid type: string,
/// expected a sequence ») et ferait échouer toute soumission en 422.
#[derive(Deserialize)]
pub struct RosterUpdateForm {
    #[serde(default)]
    pub player_id: Vec<String>,
    #[serde(default)]
    pub personal_name: Vec<String>,
    #[serde(default)]
    pub jersey: Vec<String>,
}

pub async fn post_update_roster(
    Path((space_id, team_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    axum_extra::extract::Form(form): axum_extra::extract::Form<RosterUpdateForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(space) = SpaceId::try_new(&space_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Some(team) = state.players.roster_port.find_team_info(&team_id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !can_spend_spp(&state, &user, &space, &team).await {
        return StatusCode::FORBIDDEN.into_response();
    }

    let Some(rows) = build_rows(&form) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let cmd = UpdateRosterCommand {
        team_id: TeamId(team_id.clone()),
        rows,
    };

    match update_roster_use_case::execute(cmd, state.players.repository.as_ref(), &state.event_bus)
        .await
    {
        Ok(_) => rendu_apres_succes(&state, &space_id, &team_id).await,
        Err(e) => rendu_apres_echec(&state, &space_id, &team_id, &form, e).await,
    }
}

/// Les trois tableaux doivent avoir la même longueur — ils viennent des mêmes
/// lignes. Une divergence signale un formulaire malformé, pas une saisie
/// invalide : c'est un 400, pas un message au coach.
///
/// `display_order` vient de l'index : aucun champ d'ordre n'est soumis, l'ordre
/// **est** celui des lignes.
fn build_rows(form: &RosterUpdateForm) -> Option<Vec<RosterRowCommand>> {
    let n = form.player_id.len();
    if form.personal_name.len() != n || form.jersey.len() != n {
        return None;
    }
    (0..n)
        .map(|i| {
            Some(RosterRowCommand {
                player_id: PlayerId(form.player_id[i].clone()),
                personal_name: parse_nom(&form.personal_name[i])?,
                jersey: parse_jersey(&form.jersey[i])?,
                display_order: DisplayOrder::new(i as u32),
            })
        })
        .collect()
}

/// Champ vide = pas de nom, et non « nom invalide » : le coach a le droit de
/// laisser la case libre, la lecture retombe alors sur le nom de poste.
/// `Some(None)` = absence voulue, `None` = valeur refusée par le domaine.
fn parse_nom(brut: &str) -> Option<Option<PersonalName>> {
    match brut.trim().is_empty() {
        true => Some(None),
        false => PersonalName::try_new(brut.to_string()).ok().map(Some),
    }
}

fn parse_jersey(brut: &str) -> Option<Option<JerseyVo>> {
    match brut.trim().is_empty() {
        true => Some(None),
        false => brut
            .trim()
            .parse::<u16>()
            .ok()
            .and_then(|n| JerseyVo::try_new(n).ok())
            .map(Some),
    }
}

async fn rendu_apres_succes(state: &AppState, space_id: &str, team_id: &str) -> Response {
    let mut response = rendu_apres_succes_fragment(state, space_id, team_id).await;
    response
        .headers_mut()
        .insert("HX-Trigger", "rosterEditSaved".parse().unwrap());
    response
}

/// Un refus métier répond **200**, pas une erreur HTTP : HTMX doit pouvoir
/// remplacer le fragment pour que le coach retrouve sa saisie et la corrige.
/// Un 4xx ferait échouer le swap et lui ferait tout perdre.
async fn rendu_apres_echec(
    state: &AppState,
    space_id: &str,
    team_id: &str,
    form: &RosterUpdateForm,
    erreur: UpdateRosterError,
) -> Response {
    let message = match erreur {
        UpdateRosterError::UnknownOrInactivePlayer => {
            "un joueur de la liste ne fait plus partie de l'effectif."
        }
        UpdateRosterError::DuplicateJersey => "deux joueurs portent le même numéro.",
        UpdateRosterError::DuplicateDisplayOrder => "deux joueurs occupent le même rang.",
        UpdateRosterError::Repository(RepositoryError::ConcurrentWrite) => {
            "l'effectif a été modifié entre-temps. Vérifiez les valeurs et réessayez."
        }
        UpdateRosterError::Domain(e) => {
            tracing::error!("post_update_roster team={team_id}: domaine: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        UpdateRosterError::Repository(e) => {
            tracing::error!("post_update_roster team={team_id}: repository: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let mut players = build_player_rows(state, &TeamId(team_id.to_string())).await;
    reafficher_saisie(&mut players, form);

    let mut response = PlayerTableTemplate {
        app_routes: AppRoutes::default(),
        space_id: space_id.to_string(),
        team_id: team_id.to_string(),
        players,
        save_error: Some(message.to_string()),
    }
    .into_response();
    response
        .headers_mut()
        .insert("HX-Trigger", "rosterEditSaveFailed".parse().unwrap());
    response
}

/// Réinjecte dans les lignes ce que le coach a tapé, à la place de ce que porte
/// la base : rien n'a été enregistré, et lui reprendre sa saisie l'obligerait à
/// tout retaper pour corriger une seule case.
///
/// Les colonnes en lecture seule (poste, compétences, SPP, valeur) restent
/// celles de la base — elles ne sont pas éditables, la saisie n'en dit rien.
/// L'ordre des lignes suit celui du formulaire, que le glisser-déposer a pu
/// changer.
fn reafficher_saisie(players: &mut Vec<PlayerRowVm>, form: &RosterUpdateForm) {
    let mut reordonnees = Vec::with_capacity(form.player_id.len());
    for (i, player_id) in form.player_id.iter().enumerate() {
        let Some(pos) = players.iter().position(|p| &p.player_id == player_id) else {
            continue; // Joueur inconnu de l'effectif : c'est le motif du refus.
        };
        let mut ligne = players.remove(pos);
        ligne.personal_name = form.personal_name[i].trim().to_string();
        ligne.jersey = form.jersey[i].trim().parse::<i16>().ok();
        reordonnees.push(ligne);
    }
    // Un joueur absent du formulaire reste affiché : il fait partie de
    // l'effectif, le masquer laisserait croire qu'il a disparu.
    reordonnees.append(players);
    *players = reordonnees;
}

async fn rendu_apres_succes_fragment(state: &AppState, space_id: &str, team_id: &str) -> Response {
    let players = build_player_rows(state, &TeamId(team_id.to_string())).await;
    PlayerTableTemplate {
        app_routes: AppRoutes::default(),
        space_id: space_id.to_string(),
        team_id: team_id.to_string(),
        players,
        save_error: None,
    }
    .into_response()
}

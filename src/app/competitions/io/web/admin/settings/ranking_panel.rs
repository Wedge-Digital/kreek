//! Le panneau « Points de classement » — celui qui porte le risque de l'onglet.
//!
//! Modifier un barème en cours de saison **rejoue le classement publié**, dans
//! le même `POST`. Sans ce rejeu, le classement mélangerait deux règles sans que
//! personne ne l'apprenne : les totaux resteraient plausibles.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::competition_rules::RankingRules;
use crate::app::competitions::io::web::admin::admin_page::require_admin_access;
use crate::app::competitions::io::web::admin::settings::builders::build_ranking_vm;
use crate::app::competitions::use_cases::settings::update_ranking_settings_use_case::{
    self, UpdateRankingSettingsCommand, UpdateRankingSettingsError,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

// ── Vue ───────────────────────────────────────────────────────────────────────

pub struct BonusVm {
    pub activated: bool,
    /// Le seuil, dont le **sens dépend du bonus** — TD d'écart, TD encaissés,
    /// sorties. Le libellé qui l'accompagne le porte ; le VM ne le répète pas.
    pub threshold: u32,
    pub points: u32,
}

pub struct TiebreakRowVm {
    pub code: String,
    pub label: String,
    pub activated: bool,
}

pub struct RecomputeVm {
    pub matches_replayed: u32,
    pub teams: u32,
}

pub struct RankingVm {
    pub win_points: u32,
    pub draw_points: u32,
    pub lose_points: u32,
    pub offensive: BonusVm,
    pub defensive: BonusVm,
    pub aggressive: BonusVm,
    /// L'ordre **est** la priorité.
    pub tiebreakers: Vec<TiebreakRowVm>,
    /// Le compte-rendu du dernier rejeu. `None` au `GET` : un décompte affiché
    /// sans qu'un rejeu ait eu lieu serait une promesse, pas un fait.
    pub recompute: Option<RecomputeVm>,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/widgets/settings-ranking.html")]
pub struct SettingsRankingTemplate {
    pub vm: RankingVm,
    pub post_url: String,
}

impl IntoResponse for SettingsRankingTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("settings ranking render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn get_settings_ranking(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
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
    rendre(&state, &space_id, &competition_id, &season_id, None, None).await
}

/// **JSON et non formulaire.** La cible est un agrégat dont chaque champ est un
/// nutype qui valide **à la désérialisation** — vérifié :
/// `from_str::<RankingPoints>("999999")` rend une erreur. `TiebreakConfig`,
/// elle, refuse la liste vide, les doublons et l'absence de critère actif par
/// son `#[serde(try_from)]`.
///
/// Le handler n'a donc aucune validation à écrire : un barème hors bornes est
/// rejeté avant d'atteindre une ligne de ce fichier.
#[derive(Deserialize)]
pub struct RankingSettingsPayload {
    #[serde(flatten)]
    pub ranking_rules: RankingRules,
}

pub async fn post_settings_ranking(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(payload): Json<RankingSettingsPayload>,
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
    let Ok(season) = SeasonId::try_new(&season_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let issue = update_ranking_settings_use_case::execute(
        UpdateRankingSettingsCommand {
            season_id: season,
            ranking_rules: payload.ranking_rules,
        },
        state.competitions.season_repository.as_ref(),
        state.competitions.ranking_recompute_port.as_ref(),
    )
    .await;

    match issue {
        Ok(rapport) => {
            let vu = RecomputeVm {
                matches_replayed: rapport.matches_replayed,
                teams: rapport.teams,
            };
            rendre(
                &state,
                &space_id,
                &competition_id,
                &season_id,
                Some(vu),
                None,
            )
            .await
        }
        // **Le recalcul en échec rend `200`, pas `422`.** L'enregistrement
        // demandé a bien eu lieu ; un `422` rendrait un formulaire déjà
        // sauvegardé, et inviterait à ressaisir ce qui est écrit.
        Err(UpdateRankingSettingsError::RecomputeFailed(motif)) => {
            tracing::error!("settings ranking recompute {season_id}: {motif}");
            let message = "Le barème est enregistré, mais le classement n'a pas pu être \
                           recalculé. Réenregistrez pour relancer le calcul."
                .to_string();
            rendre(
                &state,
                &space_id,
                &competition_id,
                &season_id,
                None,
                Some(message),
            )
            .await
        }
        Err(UpdateRankingSettingsError::SeasonNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(autre) => {
            tracing::error!("settings ranking {season_id}: {autre:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Le widget, relu depuis la base — jamais reconstruit depuis la charge utile.
async fn rendre(
    state: &AppState,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    recompute: Option<RecomputeVm>,
    error: Option<String>,
) -> Response {
    let Ok(season) = SeasonId::try_new(season_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let regles = match state
        .competitions
        .season_repository
        .find_rules(&season)
        .await
    {
        Ok(Some(r)) => r,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("settings ranking find rules {season_id}: {e:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let mut vm = build_ranking_vm(
        &regles.ranking_rules,
        state.competitions.tiebreak_catalog_port.as_ref(),
    );
    vm.recompute = recompute;
    vm.error = error;
    SettingsRankingTemplate {
        vm,
        post_url: AppRoutes::default().competitions.admin_settings_ranking(
            space_id,
            competition_id,
            season_id,
        ),
    }
    .into_response()
}

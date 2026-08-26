use crate::app::players::domain::player::TeamId;
use crate::app::players::io::app_events::team_created_listener::skill_category_css;
use crate::app::players::ports::{AcquiredSkillProjection, ISkillCatalogPort, PlayerProjection};
use crate::app::players::use_cases::player_stats_service::{self, ResolvedPlayerStats};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use std::collections::HashMap;

// ── View models ───────────────────────────────────────────────────────────────

pub struct SkillTagVm {
    pub name: String,
    pub category_css: String,
}

impl std::fmt::Display for SkillTagVm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub struct PlayerRowVm {
    pub player_id: String,
    pub jersey: Option<i16>,
    pub personal_name: String,
    pub position_name: String,
    pub base_skills: Vec<SkillTagVm>,
    pub acquired_skills: Vec<AcquiredSkillProjection>,
    pub spp: i32,
    pub value_kpo: i32,
    /// Caractéristiques résolues — base du poste, moins les malus de séquelles,
    /// plus les augmentations achetées en SPP. `None` si le poste est introuvable
    /// au catalogue : la table affiche alors un tiret plutôt qu'une valeur fausse.
    pub stats: Option<ResolvedPlayerStats>,
    /// « Elfe, Blitzer » — les mots-clefs du poste, déjà joints.
    ///
    /// Vide quand le poste n'en porte pas : le template n'affiche alors rien du
    /// tout, plutôt qu'une ligne vide sous le badge.
    pub keywords: String,
}

/// Les mots-clefs du poste, joints pour l'affichage.
///
/// Aucune requête de plus : `find_position` est déjà appelée pour résoudre les
/// compétences de base, et l'adapter y a joint les libellés (carte 405).
fn mots_clefs_du_poste(roster_line_id: &str, catalog: &dyn ISkillCatalogPort) -> String {
    catalog
        .find_position(roster_line_id)
        .map(|p| p.keywords.join(", "))
        .unwrap_or_default()
}

fn build_base_skills(p: &PlayerProjection, catalog: &dyn ISkillCatalogPort) -> Vec<SkillTagVm> {
    let Some(position) = catalog.find_position(&p.roster_line_id) else {
        return p
            .base_skills
            .iter()
            .map(|n| SkillTagVm {
                name: n.clone(),
                category_css: "type-general".to_string(),
            })
            .collect();
    };
    position
        .base_skills
        .iter()
        .filter_map(|uid| catalog.find_skill(uid))
        .map(|s| SkillTagVm {
            name: s.name.clone(),
            category_css: skill_category_css(&s.category).to_string(),
        })
        .collect()
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "player-table-fragment.html")]
pub struct PlayerTableTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub team_id: String,
    pub players: Vec<PlayerRowVm>,
    /// Motif d'un enregistrement refusé. Sa présence fait rendre le fragment
    /// **déjà en mode édition** : le coach revient à sa saisie pour la corriger,
    /// au lieu de la perdre et de tout recommencer.
    pub save_error: Option<String>,
}

impl IntoResponse for PlayerTableTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("player_table template render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn player_table_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team = TeamId(team_id);
    let players = build_player_rows(&state, &team).await;

    PlayerTableTemplate {
        app_routes: AppRoutes::default(),
        space_id,
        team_id: team.0,
        players,
        save_error: None,
    }
    .into_response()
}

/// Lignes de l'effectif actif, prêtes à rendre. Extrait du handler pour que
/// l'endpoint de sauvegarde (carte 294) rende le même tableau sans dupliquer la
/// résolution des caractéristiques ni celle des compétences.
pub async fn build_player_rows(state: &AppState, team: &TeamId) -> Vec<PlayerRowVm> {
    let projections = state
        .players
        .projection_repository
        .find_by_team_id(team)
        .await
        .unwrap_or_default();

    let catalog = state.players.skill_catalog.as_ref();
    let stats = resolve_team_stats(state, team, catalog).await;

    projections
        .into_iter()
        .map(|p| {
            let base_skills = build_base_skills(&p, catalog);
            let keywords = mots_clefs_du_poste(&p.roster_line_id, catalog);
            let resolved = stats.get(&p.player_id).copied();
            PlayerRowVm {
                player_id: p.player_id,
                jersey: p.jersey,
                personal_name: p.personal_name,
                position_name: p.position_name,
                base_skills,
                acquired_skills: p.acquired_skills,
                spp: p.spp,
                value_kpo: p.value_kpo,
                stats: resolved,
                keywords,
            }
        })
        .collect()
}

/// Caractéristiques résolues de tout l'effectif, indexées par joueur.
///
/// La projection ne porte ni les malus de séquelles ni les augmentations
/// achetées : elle n'enregistre de `PlayerStatIncreased` que son coût en valeur
/// d'équipe. Les caractéristiques sont donc résolues depuis les agrégats — une
/// seule requête pour toute l'équipe, `find_by_team_id` lisant les événements
/// d'un coup et hydratant en mémoire.
async fn resolve_team_stats(
    state: &AppState,
    team: &TeamId,
    catalog: &dyn ISkillCatalogPort,
) -> HashMap<String, ResolvedPlayerStats> {
    state
        .players
        .repository
        .find_by_team_id(team)
        .await
        .unwrap_or_default()
        .iter()
        .filter_map(|player| {
            player_stats_service::resolve_stats(player, catalog)
                .map(|stats| (player.id.0.clone(), stats))
        })
        .collect()
}

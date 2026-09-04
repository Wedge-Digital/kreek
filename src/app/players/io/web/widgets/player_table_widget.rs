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

/// Pourquoi un joueur ne jouera pas le prochain match (carte 489).
///
/// **Un `enum` et non un booléen.** Le gabarit n'a pas à connaître les quatre
/// statuts du domaine ni à décider lesquels comptent comme une absence — c'est
/// une règle métier. Mais un `bool` ne distinguerait pas les deux repères
/// affichés. L'énumération dit à la vue exactement ce qu'elle doit savoir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Absence {
    ProchainMatch,
    Retraite,
}

impl Absence {
    /// Le libellé du repère posé à droite du nom. Un barré nu n'explique rien :
    /// un coach qui ouvre la feuille sans contexte ne sait pas s'il s'agit d'une
    /// blessure, d'une suspension ou d'un défaut d'affichage.
    pub fn libelle(self) -> &'static str {
        match self {
            Self::ProchainMatch => "Manque le prochain match",
            Self::Retraite => "A pris sa retraite",
        }
    }

    pub fn icone(self) -> &'static str {
        match self {
            Self::ProchainMatch => "\u{1FA79}",
            Self::Retraite => "\u{2691}",
        }
    }

    /// Depuis le statut porté par la projection.
    ///
    /// `Dead` n'apparaît pas : le tableau lit `find_alive_by_team_id`, et la
    /// carte 488 a sorti les morts de l'effectif visible. `Retired` n'est posé
    /// par aucun code du domaine aujourd'hui — mais `squad_adapter.rs` le range
    /// déjà parmi les indisponibles, et l'écran ne doit pas dire l'inverse de ce
    /// que `teams` calcule.
    fn depuis_le_statut(statut: &str) -> Option<Self> {
        match statut {
            "MissingNextGame" => Some(Self::ProchainMatch),
            "Retired" => Some(Self::Retraite),
            _ => None,
        }
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
    /// L'absence au prochain match, quand il y en a une (carte 489).
    ///
    /// La donnée existait déjà : `participation_status` vit dans la projection
    /// et le dépôt le lit — il s'arrêtait ici.
    pub absence: Option<Absence>,
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
        .find_alive_by_team_id(team)
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
                absence: Absence::depuis_le_statut(&p.participation_status),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// **Les quatre statuts, et ce que la vue en fait** (carte 489).
    ///
    /// `Dead` rend `None` sans que ce soit une décision d'affichage : le tableau
    /// lit `find_alive_by_team_id`, et un mort n'atteint jamais cette fonction.
    /// La correspondance le dit quand même, pour qu'un futur appelant qui
    /// oublierait le filtre n'obtienne pas un joueur barré « à la retraite ».
    #[test]
    fn chaque_statut_donne_son_absence() {
        assert_eq!(Absence::depuis_le_statut("Available"), None);
        assert_eq!(
            Absence::depuis_le_statut("MissingNextGame"),
            Some(Absence::ProchainMatch)
        );
        assert_eq!(
            Absence::depuis_le_statut("Retired"),
            Some(Absence::Retraite)
        );
        assert_eq!(Absence::depuis_le_statut("Dead"), None);
    }

    /// Un statut inconnu ne barre pas la ligne.
    ///
    /// **Échouer ouvert, ici, est le bon sens** : un statut que la vue ne
    /// connaît pas viendrait d'un domaine qui a évolué sans elle. Barrer par
    /// défaut ferait disparaître visuellement un effectif entier sur une valeur
    /// mal orthographiée ; ne rien barrer laisse la liste lisible et le défaut
    /// se voit au premier joueur blessé qui cesse d'être signalé.
    #[test]
    fn un_statut_inconnu_ne_barre_rien() {
        assert_eq!(Absence::depuis_le_statut(""), None);
        assert_eq!(Absence::depuis_le_statut("missingnextgame"), None);
        assert_eq!(Absence::depuis_le_statut("Suspendu"), None);
    }

    /// Les deux repères disent des choses différentes — c'est la raison d'être
    /// de l'`enum` plutôt que d'un booléen.
    #[test]
    fn les_deux_absences_ne_se_disent_pas_pareil() {
        assert_ne!(
            Absence::ProchainMatch.libelle(),
            Absence::Retraite.libelle()
        );
        assert_ne!(Absence::ProchainMatch.icone(), Absence::Retraite.icone());
        assert!(Absence::ProchainMatch.libelle().contains("prochain match"));
        assert!(Absence::Retraite.libelle().contains("retraite"));
    }
}

use crate::app::auth::auth_backend::AuthSession;
use crate::app::ranking::domain::standings::TiebreakOrder;
use crate::app::ranking::io::web::builders::build_detailed_groups;
use crate::app::ranking::io::web::tiebreak_labels::{tiebreak_label, tiebreak_short_label};
use crate::app::ranking::use_cases::standings_service::tiebreak_order_of;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// En-tête d'une colonne de départage. Partagé par toutes les poules : l'ordre
/// est celui de la compétition.
pub struct TiebreakColumnVm {
    /// Numérotation affichée (1, 2, 3…) — la priorité, pas l'index du catalogue.
    pub position: u32,
    pub short_label: &'static str,
    pub long_label: &'static str,
}

/// État d'une cellule de départage vis-à-vis du groupe d'équipes à égalité de
/// points auquel appartient sa ligne (règles 21 et 22).
#[derive(Debug, PartialEq, Eq)]
pub enum CellState {
    /// Le critère qui a départagé.
    Decisive,
    /// Critère de priorité supérieure au décisif, ou groupe totalement ex æquo.
    Tied,
    /// Aucune égalité à résoudre, ou critère situé après le décisif.
    Neutral,
}

impl CellState {
    /// Seul point de correspondance vers les noms de classes CSS : disséminer
    /// ces noms dans le builder rendrait un renommage invisible au compilateur.
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Decisive => "sd-decisive",
            Self::Tied => "sd-tied",
            Self::Neutral => "",
        }
    }
}

pub struct TiebreakCellVm {
    /// Déjà formatée — « +14 », « −3 », « 24 ».
    pub value: String,
    pub state: CellState,
}

pub struct DetailedRowVm {
    pub rank: u32,
    pub team_name: String,
    pub team_link: String,
    pub played: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    /// Signé, y compris « +0 » : se lit comme une contribution au total.
    pub bonus: String,
    pub total: u32,
    /// Une cellule par colonne de `DetailedStandingsVm::columns`, dans le même ordre.
    pub tiebreaks: Vec<TiebreakCellVm>,
}

pub struct DetailedGroupVm {
    pub title: Option<String>,
    pub has_enrolled_teams: bool,
    pub rows: Vec<DetailedRowVm>,
}

pub struct DetailedStandingsVm {
    pub rules_missing: bool,
    pub columns: Vec<TiebreakColumnVm>,
    pub groups: Vec<DetailedGroupVm>,
}

#[derive(Template)]
#[template(path = "widgets/detailed-standings-widget.html")]
pub struct DetailedStandingsWidgetTemplate {
    pub vm: DetailedStandingsVm,
}

impl IntoResponse for DetailedStandingsWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("detailed_standings_widget render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn detailed_standings_widget(
    auth_session: AuthSession,
    Path((space_id, _competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if auth_session.user.is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let vm = build_vm(&state, &space_id, &season_id).await;
    DetailedStandingsWidgetTemplate { vm }.into_response()
}

async fn build_vm(state: &AppState, space_id: &str, season_id: &str) -> DetailedStandingsVm {
    let (rules, teams, lines, groups) = tokio::join!(
        state.ranking.competition_port.find_ranking_rules(season_id),
        state.ranking.competition_port.find_enrolled_teams(season_id),
        state.ranking.repository.find_latest_lines_for_season(season_id),
        state.ranking.competition_port.find_groups(season_id),
    );

    let rules_missing = rules.is_none();
    let order = tiebreak_order_of(&rules);
    let groups_vm = if rules_missing {
        vec![]
    } else {
        build_detailed_groups(space_id, lines.unwrap_or_default(), &teams, &groups, &order)
    };

    DetailedStandingsVm { rules_missing, columns: build_columns(&order), groups: groups_vm }
}

/// Une colonne par critère **actif**, numérotée dans l'ordre de priorité : la
/// lecture de gauche à droite suit l'algorithme de résolution.
fn build_columns(order: &TiebreakOrder) -> Vec<TiebreakColumnVm> {
    order
        .criteria()
        .iter()
        .enumerate()
        .map(|(idx, criterion)| TiebreakColumnVm {
            position: (idx + 1) as u32,
            short_label: tiebreak_short_label(*criterion),
            long_label: tiebreak_label(*criterion),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ranking::domain::tiebreak::TiebreakCriterion;

    #[test]
    fn columns_follow_the_configured_priority_and_are_numbered_from_one() {
        let order = TiebreakOrder::new(vec![
            TiebreakCriterion::NbCas,
            TiebreakCriterion::DiffTd,
            TiebreakCriterion::NbTd,
        ]);

        let columns = build_columns(&order);

        assert_eq!(columns.iter().map(|c| c.position).collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(
            columns.iter().map(|c| c.short_label).collect::<Vec<_>>(),
            vec!["Bl.", "Δ TD", "TD+"]
        );
    }

    /// Un ordre vide est un état valide : le tableau n'affiche alors aucune
    /// colonne de départage, sans que rien n'échoue.
    #[test]
    fn an_empty_order_yields_no_column() {
        assert!(build_columns(&TiebreakOrder::empty()).is_empty());
    }

    #[test]
    fn css_class_maps_each_state() {
        assert_eq!(CellState::Decisive.css_class(), "sd-decisive");
        assert_eq!(CellState::Tied.css_class(), "sd-tied");
        assert_eq!(CellState::Neutral.css_class(), "");
    }
}

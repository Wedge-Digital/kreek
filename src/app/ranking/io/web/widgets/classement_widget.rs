use crate::app::auth::auth_backend::AuthSession;
use crate::app::ranking::domain::standings::TiebreakOrder;
use crate::app::ranking::io::web::builders::build_classement_groups;
use crate::app::ranking::ports::RankingRulesInfo;
use crate::app::ranking::use_cases::standings_service::to_tiebreak_order;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use askama::Template;

pub struct ClassementRowVm {
    pub rank: u32,
    pub team_name: String,
    pub team_link: String,
    pub played: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub points: u32,
}

/// Un classement à afficher — soit l'unique classement de la saison
/// (`title: None`, saison sans poule ou une seule), soit le classement d'une
/// poule (`title: Some(nom)`) ou des équipes non assignées à une poule.
pub struct ClassementGroupVm {
    pub title: Option<String>,
    pub has_enrolled_teams: bool,
    pub rows: Vec<ClassementRowVm>,
}

pub struct ClassementWidgetVm {
    pub rules_missing: bool,
    pub groups: Vec<ClassementGroupVm>,
}

#[derive(Template)]
#[template(path = "widgets/classement-widget.html")]
pub struct ClassementWidgetTemplate {
    pub vm: ClassementWidgetVm,
}

impl IntoResponse for ClassementWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("classement_widget render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn classement_widget(
    auth_session: AuthSession,
    Path((space_id, _competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if auth_session.user.is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let vm = build_vm(&state, &space_id, &season_id).await;
    ClassementWidgetTemplate { vm }.into_response()
}

async fn build_vm(state: &AppState, space_id: &str, season_id: &str) -> ClassementWidgetVm {
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
        build_classement_groups(space_id, lines.unwrap_or_default(), &teams, &groups, &order)
    };

    ClassementWidgetVm { rules_missing, groups: groups_vm }
}

/// Sans règles configurées, l'ordre est vide — le classement n'est de toute
/// façon pas affiché, mais l'ordre reste un état valide plutôt qu'une absence.
fn tiebreak_order_of(rules: &Option<RankingRulesInfo>) -> TiebreakOrder {
    rules
        .as_ref()
        .map(|r| to_tiebreak_order(&r.tiebreakers))
        .unwrap_or_else(TiebreakOrder::empty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ranking::domain::tiebreak::TiebreakCriterion;
    use crate::app::ranking::ports::{BonusRuleInfo, TiebreakSettingInfo};

    fn rules_with(tiebreakers: Vec<TiebreakSettingInfo>) -> RankingRulesInfo {
        let no_bonus = || BonusRuleInfo { activated: false, threshold: 0, points: 0 };
        RankingRulesInfo {
            win_points: 3,
            draw_points: 1,
            lose_points: 0,
            offensive: no_bonus(),
            defensive: no_bonus(),
            aggressive: no_bonus(),
            tiebreakers,
        }
    }

    fn setting(code: &str, activated: bool) -> TiebreakSettingInfo {
        TiebreakSettingInfo { code: code.into(), activated }
    }

    /// Jonction entre la configuration de la compétition et le chemin de lecture :
    /// si l'ordre n'était pas construit ici, le classement se réduirait
    /// silencieusement aux points et le départage n'aurait aucun effet visible.
    #[test]
    fn tiebreak_order_of_builds_the_configured_order() {
        let rules = Some(rules_with(vec![setting("nb_cas", true), setting("nb_td", true)]));

        let order = tiebreak_order_of(&rules);

        assert_eq!(
            order,
            TiebreakOrder::new(vec![TiebreakCriterion::NbCas, TiebreakCriterion::NbTd])
        );
    }

    #[test]
    fn tiebreak_order_of_drops_deactivated_criteria() {
        let rules = Some(rules_with(vec![setting("nb_cas", false), setting("nb_td", true)]));

        assert_eq!(tiebreak_order_of(&rules), TiebreakOrder::new(vec![TiebreakCriterion::NbTd]));
    }

    /// Sans règles configurées, le classement n'est pas affiché — l'ordre reste
    /// un état valide plutôt qu'une absence à traiter en aval.
    #[test]
    fn tiebreak_order_of_none_is_empty() {
        assert_eq!(tiebreak_order_of(&None), TiebreakOrder::empty());
    }
}

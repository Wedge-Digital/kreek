use crate::app::references::domain::models::SpecialRule;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

// Les UIDs concrets que FAVOURED_OF_CHOOSE_* doit exposer
const FIVE_CHAOS_GODS: [&str; 5] = [
    "FAVOURED_OF_KHORNE",
    "FAVOURED_OF_NURGLE",
    "FAVOURED_OF_SLAANESH",
    "FAVOURED_OF_TZEENTCH",
    "FAVOURED_OF_UNDIVIDED",
];

#[derive(Deserialize)]
pub struct SpecialRuleSelectorParams {
    #[serde(default)]
    pub selected: String,
    pub on_select: String,
    #[serde(default)]
    pub roster_id: String,
}

pub struct SpecialRuleVm {
    pub uid:         String,
    pub label:       String,
    pub is_selected: bool,
}

#[derive(Template)]
#[template(path = "special-rule-selector-fragment.html")]
pub struct SpecialRuleSelectorTemplate {
    pub rules:          Vec<SpecialRuleVm>,
    pub is_choice:      bool,
    pub selected_uid:   String,
    pub selected_label: Option<String>,
    pub on_select:      String,
}

impl IntoResponse for SpecialRuleSelectorTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

/// Règles fixes du roster (pas de choix utilisateur) : affichées telles
/// quelles, dans l'ordre où elles apparaissent sur le roster.
fn resolve_fixed_rules(team_rules: &[String], all_rules: &[SpecialRule]) -> Vec<SpecialRuleVm> {
    team_rules
        .iter()
        .filter_map(|uid| all_rules.iter().find(|r| &r.uid == uid))
        .map(|r| SpecialRuleVm {
            uid:         r.uid.clone(),
            label:       r.label.clone(),
            is_selected: false,
        })
        .collect()
}

/// Les placeholders FAVOURED_OF_CHOOSE_* s'expandent vers les 5 dieux du
/// Chaos. L'ordre est déterminé par FIVE_CHAOS_GODS, pas par le repository.
fn resolve_choice_rules(all_rules: &[SpecialRule], selected: &str) -> Vec<SpecialRuleVm> {
    FIVE_CHAOS_GODS
        .iter()
        .filter_map(|uid| all_rules.iter().find(|r| r.uid == *uid))
        .map(|r| SpecialRuleVm {
            is_selected: r.uid == selected,
            uid:         r.uid.clone(),
            label:       r.label.clone(),
        })
        .collect()
}

pub async fn special_rule_selector(
    Query(params): Query<SpecialRuleSelectorParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let ref_repo = state.references.repository.as_ref();

    let team_rules: Vec<String> = if !params.roster_id.is_empty() {
        ref_repo
            .find_team_by_uid(&params.roster_id)
            .map(|t| t.special_rules.clone())
            .unwrap_or_default()
    } else {
        vec![]
    };

    // Seuls les UIDs FAVOURED_OF_CHOOSE_* déclenchent un choix utilisateur ;
    // les autres rosters affichent leurs règles fixes en lecture seule.
    let has_choose = team_rules
        .iter()
        .any(|r| r.starts_with("FAVOURED_OF_CHOOSE_"));

    let all_rules = ref_repo.list_special_rules();
    let rules = if has_choose {
        resolve_choice_rules(all_rules, &params.selected)
    } else {
        resolve_fixed_rules(&team_rules, all_rules)
    };

    let selected_label = rules.iter().find(|r| r.is_selected).map(|r| r.label.clone());

    SpecialRuleSelectorTemplate {
        rules,
        is_choice: has_choose,
        selected_uid: params.selected,
        selected_label,
        on_select: params.on_select,
    }
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(uid: &str, label: &str) -> SpecialRule {
        SpecialRule { uid: uid.into(), label: label.into() }
    }

    fn all_rules() -> Vec<SpecialRule> {
        vec![
            rule("TEAM_CAPTAIN", "Capitaine d'Équipe"),
            rule("BRAWLIN_BRUTES", "Brutes Bagarreuses"),
            rule("FAVOURED_OF_KHORNE", "Favoris de Khorne"),
            rule("FAVOURED_OF_NURGLE", "Favoris de Nurgle"),
            rule("FAVOURED_OF_SLAANESH", "Favoris de Slaanesh"),
            rule("FAVOURED_OF_TZEENTCH", "Favoris de Tzeentch"),
            rule("FAVOURED_OF_UNDIVIDED", "Favoris des Indivis"),
        ]
    }

    #[test]
    fn resolve_fixed_rules_returns_empty_for_no_rules() {
        let rules = resolve_fixed_rules(&[], &all_rules());
        assert!(rules.is_empty());
    }

    #[test]
    fn resolve_fixed_rules_returns_one_chip_for_single_rule() {
        let team_rules = vec!["TEAM_CAPTAIN".to_string()];
        let rules = resolve_fixed_rules(&team_rules, &all_rules());
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].label, "Capitaine d'Équipe");
        assert!(!rules[0].is_selected);
    }

    #[test]
    fn resolve_fixed_rules_returns_all_chips_in_roster_order() {
        let team_rules = vec!["BRAWLIN_BRUTES".to_string(), "TEAM_CAPTAIN".to_string()];
        let rules = resolve_fixed_rules(&team_rules, &all_rules());
        let labels: Vec<&str> = rules.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, vec!["Brutes Bagarreuses", "Capitaine d'Équipe"]);
    }

    #[test]
    fn resolve_choice_rules_returns_five_chaos_gods_in_fixed_order() {
        let rules = resolve_choice_rules(&all_rules(), "FAVOURED_OF_NURGLE");
        let uids: Vec<&str> = rules.iter().map(|r| r.uid.as_str()).collect();
        assert_eq!(uids, FIVE_CHAOS_GODS.to_vec());
        assert!(rules.iter().find(|r| r.uid == "FAVOURED_OF_NURGLE").unwrap().is_selected);
        assert_eq!(rules.iter().filter(|r| r.is_selected).count(), 1);
    }
}

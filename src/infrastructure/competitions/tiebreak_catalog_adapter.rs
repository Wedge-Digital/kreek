use crate::app::competitions::ports::{ITiebreakCatalogPort, TiebreakCriterionDto};
use crate::app::ranking::domain::tiebreak::TiebreakCriterion;
use crate::app::ranking::io::web::tiebreak_labels::tiebreak_label;

/// Expose au BC `competitions` le catalogue des critères de départage, possédé par
/// le BC `ranking`. Seul fichier autorisé à importer `ranking` pour ce besoin :
/// `app/competitions/` ne le connaît pas.
///
/// Sans état — le catalogue est statique, il n'y a aucune dépendance à injecter.
pub struct TiebreakCatalogAdapter;

impl TiebreakCatalogAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TiebreakCatalogAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ITiebreakCatalogPort for TiebreakCatalogAdapter {
    fn all(&self) -> Vec<TiebreakCriterionDto> {
        TiebreakCriterion::all()
            .into_iter()
            .map(|c| TiebreakCriterionDto {
                code: c.code().to_string(),
                label: tiebreak_label(c).to_string(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_the_seven_criteria_in_canonical_order() {
        let codes: Vec<String> = TiebreakCatalogAdapter::new()
            .all()
            .into_iter()
            .map(|d| d.code)
            .collect();
        assert_eq!(
            codes,
            vec![
                "diff_td",
                "nb_td",
                "nb_td_conceded",
                "nb_cas",
                "nb_wins",
                "nb_fouls",
                "nb_reu"
            ]
        );
    }

    #[test]
    fn every_entry_carries_a_label() {
        for entry in TiebreakCatalogAdapter::new().all() {
            assert!(!entry.label.is_empty(), "libellé manquant pour {}", entry.code);
        }
    }

    #[test]
    fn red_cards_are_not_exposed() {
        assert!(!TiebreakCatalogAdapter::new()
            .all()
            .iter()
            .any(|d| d.code == "nb_red_cards"));
    }
}

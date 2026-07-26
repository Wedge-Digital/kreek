use crate::app::ranking::domain::tiebreak::TiebreakCriterion;

/// Libellé de présentation d'un critère de départage, affiché dans le formulaire
/// de règles de compétition. Le libellé est de la présentation : il ne descend
/// pas dans le domaine.
pub fn tiebreak_label(criterion: TiebreakCriterion) -> &'static str {
    match criterion {
        TiebreakCriterion::DiffTd => "Différence de touchdowns (marqués − encaissés)",
        TiebreakCriterion::NbTd => "Nombre de touchdowns marqués",
        TiebreakCriterion::NbTdConceded => "Nombre de touchdowns encaissés",
        TiebreakCriterion::NbCas => "Nombre de blessures infligées",
        TiebreakCriterion::NbWins => "Nombre de victoires",
        TiebreakCriterion::NbFouls => "Nombre de fautes commises",
        TiebreakCriterion::NbReu => "Nombre de réussites",
    }
}

/// Libellé court pour un en-tête de colonne du classement détaillé, où la place
/// manque : de 1 à 7 colonnes de départage s'ajoutent aux 8 colonnes fixes. Le
/// libellé long reste disponible en infobulle via `tiebreak_label`.
pub fn tiebreak_short_label(criterion: TiebreakCriterion) -> &'static str {
    match criterion {
        TiebreakCriterion::DiffTd => "Δ TD",
        TiebreakCriterion::NbTd => "TD+",
        TiebreakCriterion::NbTdConceded => "TD−",
        TiebreakCriterion::NbCas => "Bl.",
        TiebreakCriterion::NbWins => "V",
        TiebreakCriterion::NbFouls => "Ftes",
        TiebreakCriterion::NbReu => "Réu",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_criterion_has_a_non_empty_label() {
        for criterion in TiebreakCriterion::all() {
            assert!(
                !tiebreak_label(criterion).is_empty(),
                "libellé manquant pour {criterion:?}"
            );
        }
    }

    #[test]
    fn labels_match_the_validated_wording() {
        assert_eq!(
            tiebreak_label(TiebreakCriterion::DiffTd),
            "Différence de touchdowns (marqués − encaissés)"
        );
        assert_eq!(
            tiebreak_label(TiebreakCriterion::NbTdConceded),
            "Nombre de touchdowns encaissés"
        );
    }

    #[test]
    fn every_criterion_has_a_non_empty_short_label() {
        for criterion in TiebreakCriterion::all() {
            assert!(
                !tiebreak_short_label(criterion).is_empty(),
                "libellé court manquant pour {criterion:?}"
            );
        }
    }

    /// Les en-têtes sont côte à côte dans le tableau : deux libellés courts
    /// identiques rendraient deux colonnes indiscernables.
    #[test]
    fn short_labels_are_all_distinct() {
        let all = TiebreakCriterion::all();
        let distinct: HashSet<&str> = all.iter().map(|c| tiebreak_short_label(*c)).collect();
        assert_eq!(distinct.len(), all.len());
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
}

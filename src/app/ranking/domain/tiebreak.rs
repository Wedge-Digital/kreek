/// Catalogue des critères de départage — utilisés pour ordonner les équipes à
/// égalité de points de classement.
///
/// Le catalogue appartient à ce BC : `competitions` ne stocke que le choix du
/// gestionnaire (quels critères sont actifs, dans quel ordre) et consulte ce
/// catalogue via un port. Cf. `docs/specs/ranking/tiebreakers/`.
///
/// Le sens de comparaison et les compteurs cumulés relèvent de l'unité
/// `tiebreak-calc` — cette énumération ne porte que l'identité des critères.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TiebreakCriterion {
    DiffTd,
    NbTd,
    NbTdConceded,
    NbCas,
    NbWins,
    NbFouls,
    NbReu,
}

impl TiebreakCriterion {
    /// Ordre canonique du catalogue : ordre d'affichage par défaut dans le
    /// formulaire de règles, et priorité initiale des départages.
    pub fn all() -> Vec<Self> {
        vec![
            Self::DiffTd,
            Self::NbTd,
            Self::NbTdConceded,
            Self::NbCas,
            Self::NbWins,
            Self::NbFouls,
            Self::NbReu,
        ]
    }

    /// Identifiant stable du critère. Il est référencé par la configuration
    /// persistée côté `competitions` (`competition_seasons.rules`) : ces valeurs
    /// ne doivent plus changer.
    pub fn code(&self) -> &'static str {
        match self {
            Self::DiffTd => "diff_td",
            Self::NbTd => "nb_td",
            Self::NbTdConceded => "nb_td_conceded",
            Self::NbCas => "nb_cas",
            Self::NbWins => "nb_wins",
            Self::NbFouls => "nb_fouls",
            Self::NbReu => "nb_reu",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_returns_the_seven_criteria_in_canonical_order() {
        let codes: Vec<&str> = TiebreakCriterion::all().iter().map(|c| c.code()).collect();
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
    fn codes_are_all_distinct() {
        let all = TiebreakCriterion::all();
        let distinct: HashSet<&str> = all.iter().map(|c| c.code()).collect();
        assert_eq!(distinct.len(), all.len());
    }

    #[test]
    fn red_cards_are_absent_from_the_catalogue() {
        // `MatchActionType` n'expose ni carton rouge ni expulsion : le critère
        // vaudrait 0 pour toutes les équipes. À réintroduire quand les
        // expulsions seront saisies dans le rapport de match.
        assert!(!TiebreakCriterion::all()
            .iter()
            .any(|c| c.code() == "nb_red_cards"));
    }
}

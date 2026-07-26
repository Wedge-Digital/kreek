use crate::app::ranking::domain::ranking_line::CumulativeTotals;

/// Sens de comparaison d'un critère (règle 17).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Le plus grand compteur passe devant.
    Desc,
    /// Le plus petit passe devant.
    Asc,
}

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

    /// Symétrique de `code()`, et seule autorité de résolution du catalogue.
    /// Dérivé de `all()` plutôt que d'un second `match` : deux tables de
    /// correspondance finiraient par diverger sans que rien ne le signale.
    ///
    /// `None` sur un code inconnu — une configuration persistée peut référencer
    /// un critère absent du catalogue (cf. `nb_red_cards`, retiré).
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all().into_iter().find(|criterion| criterion.code() == code)
    }

    /// Règle 17 : décroissant partout, sauf les TD encaissés où le moins est le
    /// mieux. Énuméré sans `_` : l'ajout d'un critère au catalogue casse la
    /// compilation ici plutôt que d'hériter silencieusement d'un sens par défaut.
    pub fn direction(&self) -> Direction {
        match self {
            Self::NbTdConceded => Direction::Asc,
            Self::DiffTd | Self::NbTd | Self::NbCas | Self::NbWins | Self::NbFouls | Self::NbReu => {
                Direction::Desc
            }
        }
    }

    /// Valeur comparable du critère pour une équipe. **`i64` et non `u32`** :
    /// `diff_td` est négatif dès qu'une équipe encaisse plus qu'elle ne marque,
    /// et l'underflow non signé en ferait un nombre gigantesque — la pire
    /// défense passerait en tête.
    pub fn value_of(&self, totals: &CumulativeTotals) -> i64 {
        let td_for = i64::from(totals.td_for.0);
        let td_against = i64::from(totals.td_against.0);
        match self {
            // Règle 13 : dérivé à la comparaison, jamais stocké.
            Self::DiffTd => td_for - td_against,
            Self::NbTd => td_for,
            Self::NbTdConceded => td_against,
            Self::NbCas => i64::from(totals.casualties.0),
            Self::NbWins => i64::from(totals.wins.0),
            Self::NbFouls => i64::from(totals.fouls.0),
            Self::NbReu => i64::from(totals.completions.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ranking::domain::ranking_line::{
        CasualtiesTotal, CompletionsMade, FoulsCommitted, TdAgainst, TdFor, WinCount,
    };
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

    // ── Résolution, sens et valeur (carte 217) ───────────────────────────────

    /// Compteurs tous distincts entre eux : une lecture croisée dans `value_of`
    /// (les victoires à la place de la différence, par exemple) est détectée.
    /// `diff_td` vaut 4, `wins` vaut 8 — sinon les deux se confondraient.
    fn totals() -> CumulativeTotals {
        CumulativeTotals {
            wins: WinCount(8),
            td_for: TdFor(7),
            td_against: TdAgainst(3),
            casualties: CasualtiesTotal(5),
            fouls: FoulsCommitted(2),
            completions: CompletionsMade(6),
            ..CumulativeTotals::ZERO
        }
    }

    #[test]
    fn from_code_resolves_the_seven_codes_and_rejects_the_unknown() {
        for criterion in TiebreakCriterion::all() {
            assert_eq!(TiebreakCriterion::from_code(criterion.code()), Some(criterion));
        }
        assert_eq!(TiebreakCriterion::from_code("nb_red_cards"), None);
        assert_eq!(TiebreakCriterion::from_code(""), None);
    }

    #[test]
    fn from_code_is_the_exact_inverse_of_code() {
        let round_tripped: Vec<TiebreakCriterion> = TiebreakCriterion::all()
            .iter()
            .filter_map(|c| TiebreakCriterion::from_code(c.code()))
            .collect();
        assert_eq!(round_tripped, TiebreakCriterion::all());
    }

    #[test]
    fn conceded_touchdowns_is_the_only_ascending_criterion() {
        let ascending: Vec<&str> = TiebreakCriterion::all()
            .iter()
            .filter(|c| c.direction() == Direction::Asc)
            .map(|c| c.code())
            .collect();
        assert_eq!(ascending, vec!["nb_td_conceded"]);
    }

    #[test]
    fn value_of_reads_the_counter_of_each_criterion() {
        let t = totals();
        assert_eq!(TiebreakCriterion::DiffTd.value_of(&t), 4); // 7 − 3
        assert_eq!(TiebreakCriterion::NbTd.value_of(&t), 7);
        assert_eq!(TiebreakCriterion::NbTdConceded.value_of(&t), 3);
        assert_eq!(TiebreakCriterion::NbCas.value_of(&t), 5);
        assert_eq!(TiebreakCriterion::NbWins.value_of(&t), 8);
        assert_eq!(TiebreakCriterion::NbFouls.value_of(&t), 2);
        assert_eq!(TiebreakCriterion::NbReu.value_of(&t), 6);
    }

    /// Règle 13 sous sa forme la plus dangereuse : en `u32`, ce −4 deviendrait
    /// 4 294 967 292 et placerait cette équipe en tête de la différence de TD.
    #[test]
    fn value_of_diff_td_is_negative_when_a_team_concedes_more_than_it_scores() {
        let leaky = CumulativeTotals { td_for: TdFor(2), td_against: TdAgainst(6), ..CumulativeTotals::ZERO };
        assert_eq!(TiebreakCriterion::DiffTd.value_of(&leaky), -4);
    }
}

use crate::app::competitions::domain::competition_rules::{
    AggressiveBonus, DefensiveBonus, OffensiveBonus, RankingRules,
};

/// Libellé de présentation des bonus de classement pour les récapitulatifs
/// (étape 5 de création et onglet admin). Combine les bonus activés ; retourne
/// `None` si aucun bonus n'est activé.
pub fn format_bonus_label(rr: &RankingRules) -> Option<String> {
    let parts: Vec<String> = [
        format_offensive(&rr.offensive_bonus).map(|s| format!("Offensif ({s})")),
        format_defensive(&rr.defensive_bonus).map(|s| format!("Défensif ({s})")),
        format_aggressive(&rr.aggressive_bonus).map(|s| format!("Agressif ({s})")),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn format_offensive(b: &OffensiveBonus) -> Option<String> {
    b.activated
        .0
        .then(|| format!("+{} si ≥ {} TDs", b.points, b.min_td))
}

fn format_defensive(b: &DefensiveBonus) -> Option<String> {
    b.activated
        .0
        .then(|| format!("+{} si ≤ {} TD encaissés", b.points, b.max_td_conceded))
}

fn format_aggressive(b: &AggressiveBonus) -> Option<String> {
    b.activated
        .0
        .then(|| format!("+{} si > {} sorties", b.points, b.min_casualties))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::competition_rules::{
        Activated, MaxTdConceded, MinCasualties, MinTd, RankingPoints, TiebreakCode, TiebreakConfig,
    };

    fn rules(off: bool, def: bool, agg: bool) -> RankingRules {
        RankingRules {
            win_points: RankingPoints::try_new(3).unwrap(),
            draw_points: RankingPoints::try_new(1).unwrap(),
            lose_points: RankingPoints::try_new(0).unwrap(),
            offensive_bonus: OffensiveBonus {
                activated: Activated(off),
                min_td: MinTd::try_new(3).unwrap(),
                points: RankingPoints::try_new(1).unwrap(),
            },
            defensive_bonus: DefensiveBonus {
                activated: Activated(def),
                points: RankingPoints::try_new(2).unwrap(),
                max_td_conceded: MaxTdConceded::try_new(1).unwrap(),
            },
            aggressive_bonus: AggressiveBonus {
                activated: Activated(agg),
                points: RankingPoints::try_new(1).unwrap(),
                min_casualties: MinCasualties::try_new(2).unwrap(),
            },
            tiebreakers: TiebreakConfig::all_active(vec![
                TiebreakCode::try_new("diff_td").expect("code non vide")
            ])
            .expect("liste non vide"),
        }
    }

    #[test]
    fn all_three_bonuses_combine_in_order() {
        let label = format_bonus_label(&rules(true, true, true));
        assert_eq!(
            label,
            Some(
                "Offensif (+1 si ≥ 3 TDs) · Défensif (+2 si ≤ 1 TD encaissés) · Agressif (+1 si > 2 sorties)"
                    .to_string()
            )
        );
    }

    #[test]
    fn only_offensive_activated() {
        assert_eq!(
            format_bonus_label(&rules(true, false, false)),
            Some("Offensif (+1 si ≥ 3 TDs)".to_string())
        );
    }

    #[test]
    fn only_defensive_activated_uses_dynamic_threshold() {
        assert_eq!(
            format_bonus_label(&rules(false, true, false)),
            Some("Défensif (+2 si ≤ 1 TD encaissés)".to_string())
        );
    }

    #[test]
    fn only_aggressive_activated() {
        assert_eq!(
            format_bonus_label(&rules(false, false, true)),
            Some("Agressif (+1 si > 2 sorties)".to_string())
        );
    }

    #[test]
    fn no_bonus_activated_yields_none() {
        assert_eq!(format_bonus_label(&rules(false, false, false)), None);
    }
}

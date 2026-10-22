//! Le favori et la petite monnaie, en fonctions pures (carte 602).
//!
//! Extraits de `MatchReportPreMatch::topdog_team_id` et `petty_cash` **par
//! copier-coller**, puis adaptés pour prendre deux VE au lieu des champs de
//! l'agrégat. L'agrégat les appelle ; la feuille de match imprimée aussi, avant
//! qu'aucun rapport n'existe. Une seule définition : ils ne peuvent pas
//! diverger.

use crate::app::match_report::domain::value_objects::TeamValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Home,
    Away,
}

/// Le favori : la VE la plus forte ; à égalité, l'équipe à domicile (R10).
pub fn top_dog(home_tv: TeamValue, away_tv: TeamValue) -> Side {
    if away_tv > home_tv {
        Side::Away
    } else {
        Side::Home
    }
}

/// La base de la petite monnaie : l'écart de VE (R11). Le rapport y ajoute ce
/// que le favori dépense ; à l'impression, ce n'est pas encore connu.
pub fn tv_gap(home_tv: TeamValue, away_tv: TeamValue) -> u32 {
    home_tv.into_inner().abs_diff(away_tv.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ve(kpo: u32) -> TeamValue {
        TeamValue::try_new(kpo).unwrap()
    }

    #[test]
    fn le_favori_est_la_ve_la_plus_forte() {
        assert_eq!(top_dog(ve(1200), ve(1100)), Side::Home);
        assert_eq!(top_dog(ve(1100), ve(1200)), Side::Away);
    }

    #[test]
    fn a_egalite_l_equipe_a_domicile_est_favorite() {
        assert_eq!(top_dog(ve(1150), ve(1150)), Side::Home);
    }

    #[test]
    fn l_ecart_de_ve_ne_depend_pas_du_sens() {
        assert_eq!(tv_gap(ve(1440), ve(1200)), 240);
        assert_eq!(tv_gap(ve(1200), ve(1440)), 240);
        assert_eq!(tv_gap(ve(1000), ve(1000)), 0);
    }
}

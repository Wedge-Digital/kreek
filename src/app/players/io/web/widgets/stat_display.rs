//! Présentation des caractéristiques — libellés, clés d'URL et suffixe « + ».
//!
//! Purement visuel, et distinct de `StatKind::improvement_step()` : celui-ci
//! dit dans quel **sens** une caractéristique s'améliore, celle-ci dit comment
//! elle s'**écrit**. AR le montre — elle s'affiche « 8+ » comme AG et PA, mais
//! s'améliore en montant.
//!
//! Le fichier existe pour qu'il n'y ait qu'une table : le panneau de dépense de
//! SPP et celui de customisation affichent les mêmes caractéristiques, et deux
//! tables auraient fini par diverger sur le « + » ou sur un libellé.

use crate::app::players::domain::match_impact::StatKind;

pub struct StatDisplay {
    pub stat: StatKind,
    /// Segment d'URL et clé de formulaire — cf. `parse_stat()` côté contrôleur.
    pub key: &'static str,
    /// Libellé court, pour les en-têtes de carte.
    pub label: &'static str,
    /// Nom complet, pour les phrases (« Amélioration de Force +1 »).
    pub name: &'static str,
    /// Nombre cible à atteindre au dé, donc affiché avec un « + ».
    pub is_target: bool,
}

/// Une table, pas une fonction : l'ordre d'affichage des caractéristiques est
/// une donnée du jeu, et c'est celui-ci que reprennent tous les panneaux.
pub const ALL: [StatDisplay; 5] = [
    StatDisplay {
        stat: StatKind::Ma,
        key: "ma",
        label: "MA",
        name: "Mouvement",
        is_target: false,
    },
    StatDisplay {
        stat: StatKind::St,
        key: "st",
        label: "ST",
        name: "Force",
        is_target: false,
    },
    StatDisplay {
        stat: StatKind::Ag,
        key: "ag",
        label: "AG",
        name: "Agilité",
        is_target: true,
    },
    StatDisplay {
        stat: StatKind::Pa,
        key: "pa",
        label: "PA",
        name: "Passe",
        is_target: true,
    },
    StatDisplay {
        stat: StatKind::Av,
        key: "av",
        label: "AV",
        name: "Armure",
        is_target: true,
    },
];

pub fn format(value: u8, is_target: bool) -> String {
    if is_target {
        format!("{value}+")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_appends_plus_only_for_target_stats() {
        assert_eq!(format(7, false), "7");
        assert_eq!(format(3, true), "3+");
    }

    /// AR est le piège : suffixe « + » comme AG et PA, direction inverse.
    #[test]
    fn armour_is_a_target_but_improves_upward() {
        let av = ALL.iter().find(|d| d.stat == StatKind::Av).unwrap();
        assert!(av.is_target);
        assert_eq!(StatKind::Av.improvement_step(), 1);
    }

    #[test]
    fn every_stat_has_a_distinct_key() {
        let keys: Vec<_> = ALL.iter().map(|d| d.key).collect();
        let mut uniques = keys.clone();
        uniques.sort_unstable();
        uniques.dedup();
        assert_eq!(keys.len(), uniques.len());
    }
}

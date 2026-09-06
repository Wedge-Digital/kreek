//! Le prix d'un coup de pouce pour une équipe donnée.
//!
//! Presque tous les coups de pouce ont un prix unique. Un seul dépend du
//! roster qui l'achète — le maître cuisinier halfling, à 100 kPo pour l'équipe
//! halfling et 300 pour tout le monde.
//!
//! **Une seule fonction pour les deux consommateurs**, et c'est le point de ce
//! module : le prix affiché par le sélecteur et le prix débité par le rapport
//! de match sortaient tous deux de `Inducement::cost`, chacun de son côté.
//! Corriger le seul affichage aurait fait lire 100 au coach en lui prélevant
//! 300 — pire que le défaut d'origine.

use super::models::Inducement;

/// Le coup de pouce dont le prix dépend de l'équipe, et l'équipe qui en
/// bénéficie.
///
/// **En dur, délibérément.** Le corpus porte un `reducedCostFor` qui désigne
/// `HALFLING_THIMBLE_CUP` — une **ligue régionale**, portée par `HALFLING`
/// autant que par `GNOME`. Le brancher donnerait le tarif réduit aux Gnomes,
/// qui n'y ont pas droit ; c'est la conclusion qu'a produite la première
/// analyse de la carte 507, en lisant le corpus comme s'il énonçait la règle.
///
/// Une donnée du corpus n'est pas une règle du jeu. Celle-ci ne parle que de
/// l'équipe halfling, et elle ne changera pas.
const CUISTOT_HALFLING: &str = "HALFLING_MASTER_CHEF";
const ROSTER_HALFLING: &str = "HALFLING";

/// Ce que cette équipe paie ce coup de pouce.
///
/// Le **montant** réduit reste lu dans le corpus, où vivent tous les autres
/// prix. S'il manque, on rend le prix plein : le code ne fabrique aucun
/// chiffre qu'on ne lui a pas donné.
pub fn cout_pour_roster(inducement: &Inducement, roster_id: &str) -> u32 {
    if inducement.uid == CUISTOT_HALFLING && roster_id == ROSTER_HALFLING {
        return inducement.reduced_cost.unwrap_or(inducement.cost);
    }
    inducement.cost
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cuistot(reduced: Option<u32>) -> Inducement {
        Inducement {
            uid: CUISTOT_HALFLING.to_string(),
            name: "Maitre cuisinier halfling".to_string(),
            cost: 300,
            reduced_cost: reduced,
            max_quantity: 1,
            category: "SPECIALIZED".to_string(),
            restricted_to: vec![],
            description: String::new(),
        }
    }

    #[test]
    fn le_cuistot_est_reduit_pour_les_halflings() {
        assert_eq!(cout_pour_roster(&cuistot(Some(100)), ROSTER_HALFLING), 100);
    }

    /// Les Gnomes partagent la ligue `HALFLING_THIMBLE_CUP` avec les halflings.
    /// C'est par eux que la première analyse s'est trompée : ils paient plein
    /// tarif, et ce test est là pour que ça reste vrai.
    #[test]
    fn le_cuistot_reste_plein_tarif_ailleurs() {
        assert_eq!(cout_pour_roster(&cuistot(Some(100)), "GNOME"), 300);
        assert_eq!(cout_pour_roster(&cuistot(Some(100)), "DWARF"), 300);
    }

    /// Sans montant réduit dans le corpus, le prix ne bouge pas — même pour
    /// l'équipe qui y aurait droit. Le code n'invente pas de chiffre.
    #[test]
    fn sans_prix_reduit_le_tarif_ne_bouge_pas() {
        assert_eq!(cout_pour_roster(&cuistot(None), ROSTER_HALFLING), 300);
    }

    /// Un autre coup de pouce portant un `reducedCost` — `BRIBES` en porte un —
    /// n'est pas concerné par cette règle-ci.
    #[test]
    fn un_autre_coup_de_pouce_ne_suit_pas_la_regle_du_cuistot() {
        let mut bribes = cuistot(Some(50));
        bribes.uid = "BRIBES".to_string();
        bribes.cost = 100;
        assert_eq!(cout_pour_roster(&bribes, ROSTER_HALFLING), 100);
    }
}

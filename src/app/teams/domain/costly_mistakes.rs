//! Ce qu'une trésorerie trop grasse coûte à une équipe.
//!
//! Une équipe qui garde plus de 100 kPo après ses recrutements et ses renvois
//! est exposée aux ennuis : un jet de D6 décide de ce qu'il lui en reste. C'est
//! le seul contrepoids du règlement qui sanctionne l'inaction.

use crate::app::teams::domain::value_objects::{IncidentType, Kpo};

/// Au-dessous, aucun jet : l'équipe passe directement en « prête à jouer ».
pub const SEUIL_ERREURS_COUTEUSES: u32 = 100;

/// Une tranche de trésorerie et ce que chaque jet de D6 y produit.
///
/// **Un tableau parcouru, et non un `match` sur des plages** : il se relit à
/// côté du règlement, ligne pour ligne, et c'est cette lecture-là qui a trouvé
/// le trou des 195 (cf. ci-dessous).
struct Tranche {
    /// Borne haute **incluse**. La dernière tranche vaut `u32::MAX`.
    jusqu_a: u32,
    /// Ce que produit chaque jet, de 1 à 6.
    par_jet: [IncidentType; 6],
}

use IncidentType::{Catastrophe, Major, Minor, None as Aucun};

/// **Les tranches sont fermées à la centaine**, là où le règlement écrit
/// `100-195`, `200-295`. Celui-ci suppose des montants en multiples de 5 kPo ;
/// la trésorerie est un `u32`, et une équipe à **197 kPo** ne doit tomber dans
/// aucun trou. Aucun montant régulier ne change de tranche pour autant.
const TABLE: [Tranche; 6] = [
    // 100–199 : 1 → mineur, 2-6 → crise évitée
    Tranche {
        jusqu_a: 199,
        par_jet: [Minor, Aucun, Aucun, Aucun, Aucun, Aucun],
    },
    // 200–299 : 1-2 → mineur
    Tranche {
        jusqu_a: 299,
        par_jet: [Minor, Minor, Aucun, Aucun, Aucun, Aucun],
    },
    // 300–399 : 1 → majeur, 2-3 → mineur
    Tranche {
        jusqu_a: 399,
        par_jet: [Major, Minor, Minor, Aucun, Aucun, Aucun],
    },
    // 400–499 : 1-2 → majeur, 3-4 → mineur
    Tranche {
        jusqu_a: 499,
        par_jet: [Major, Major, Minor, Minor, Aucun, Aucun],
    },
    // 500–599 : 1 → catastrophe, 2-3 → majeur, 4-5 → mineur
    Tranche {
        jusqu_a: 599,
        par_jet: [Catastrophe, Major, Major, Minor, Minor, Aucun],
    },
    // 600 et + : 1-2 → catastrophe, 3-4 → majeur, 5-6 → mineur
    Tranche {
        jusqu_a: u32::MAX,
        par_jet: [Catastrophe, Catastrophe, Major, Major, Minor, Minor],
    },
];

/// L'incident que ce jet produit pour cette trésorerie.
///
/// Sous le seuil, aucun — et sans panique : l'appelant peut interroger la table
/// pour une équipe pauvre sans avoir à s'en garder lui-même.
pub fn incident_for(treasury: Kpo, roll: u8) -> IncidentType {
    if treasury.0 < SEUIL_ERREURS_COUTEUSES || !(1..=6).contains(&roll) {
        return IncidentType::None;
    }
    let tranche = TABLE
        .iter()
        .find(|t| treasury.0 <= t.jusqu_a)
        .expect("la dernière tranche va jusqu'à u32::MAX");
    tranche.par_jet[(roll - 1) as usize]
}

/// Combien de dés de dégâts cet incident réclame.
///
/// Le jet du D6 ne suffit pas : un mineur retire 1D3 × 10, une catastrophe
/// laisse 2D6 × 10. C'est l'appelant qui les lance ; le domaine dit combien.
pub fn dice_needed(incident: IncidentType) -> usize {
    match incident {
        IncidentType::Minor => 1,
        IncidentType::Catastrophe => 2,
        IncidentType::None | IncidentType::Major => 0,
    }
}

/// Ce que l'incident retire, en kPo.
///
/// **L'arrondi porte sur la perte**, pas sur ce qui reste : à 345 kPo, un
/// incident majeur retire 170 et en laisse 175.
///
/// La catastrophe est l'inverse des autres — elle dit ce qui **reste**, et la
/// perte s'en déduit. `saturating_sub` parce que 2D6 peut dépasser la
/// trésorerie d'une équipe à peine au-dessus du seuil.
pub fn loss_for(treasury: Kpo, incident: IncidentType, damage_dice: &[u8]) -> Kpo {
    let somme: u32 = damage_dice.iter().map(|d| *d as u32).sum();
    match incident {
        IncidentType::None => Kpo(0),
        IncidentType::Minor => Kpo(somme * 10),
        IncidentType::Major => Kpo(treasury.0 / 2 / 5 * 5),
        IncidentType::Catastrophe => Kpo(treasury.0.saturating_sub(somme * 10)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les 36 cas du règlement, **six tranches × six jets**, écrits comme la
    /// table s'affiche.
    ///
    /// Ce n'est pas du zèle : c'est la seule règle du projet dont une erreur ne
    /// se voit pas. Un incident majeur là où il fallait un mineur retire de
    /// l'argent sans que personne ne puisse le contester.
    ///
    /// La trésorerie témoin de chaque tranche est prise **au milieu**, et les
    /// bornes sont vérifiées à part.
    #[test]
    fn les_trente_six_cas_de_la_table() {
        //                 tranche      jet 1        jet 2        jet 3   jet 4   jet 5   jet 6
        let attendu: [(u32, [IncidentType; 6]); 6] = [
            (150, [Minor, Aucun, Aucun, Aucun, Aucun, Aucun]),
            (250, [Minor, Minor, Aucun, Aucun, Aucun, Aucun]),
            (350, [Major, Minor, Minor, Aucun, Aucun, Aucun]),
            (450, [Major, Major, Minor, Minor, Aucun, Aucun]),
            (550, [Catastrophe, Major, Major, Minor, Minor, Aucun]),
            (900, [Catastrophe, Catastrophe, Major, Major, Minor, Minor]),
        ];
        for (tresorerie, par_jet) in attendu {
            for (i, incident) in par_jet.iter().enumerate() {
                let jet = (i + 1) as u8;
                assert_eq!(
                    incident_for(Kpo(tresorerie), jet),
                    *incident,
                    "trésorerie {tresorerie} kPo, jet de {jet}"
                );
            }
        }
    }

    /// Ce que la lecture ligne à ligne a trouvé : le règlement écrit `100-195`,
    /// et une trésorerie de 197 kPo n'y est pas prévue.
    ///
    /// **Le défaut ne se voit que sur un jet de 2.** Une borne à 195 ne laisse
    /// pas un trou — 197 glisse dans la tranche suivante — mais y subit un
    /// incident mineur là où l'équipe s'en tirait. Sur un jet de 1 les deux
    /// tranches donnent le même résultat, et un test qui s'en contenterait
    /// serait aveugle : c'est le cas qu'une première version de ce test a
    /// laissé passer.
    #[test]
    fn une_tresorerie_hors_des_multiples_de_cinq_reste_dans_sa_tranche() {
        assert_eq!(
            incident_for(Kpo(197), 2),
            Aucun,
            "197 kPo relève de 100-199 : un jet de 2 y est une crise évitée"
        );
        assert_eq!(
            incident_for(Kpo(199), 2),
            Aucun,
            "la borne haute inclut 199"
        );
        assert_eq!(incident_for(Kpo(196), 2), Aucun);
        assert_eq!(
            incident_for(Kpo(200), 2),
            Minor,
            "200 ouvre bien la tranche suivante"
        );
        assert_eq!(incident_for(Kpo(299), 3), Aucun);
        assert_eq!(incident_for(Kpo(300), 3), Minor, "300 : mineur sur 2-3");
    }

    #[test]
    fn sous_le_seuil_aucun_incident_et_aucune_panique() {
        for jet in 1..=6 {
            assert_eq!(incident_for(Kpo(99), jet), Aucun);
            assert_eq!(incident_for(Kpo(0), jet), Aucun);
        }
    }

    /// Un jet hors bornes ne doit pas indexer la table hors de sa longueur.
    #[test]
    fn un_jet_impossible_ne_panique_pas() {
        assert_eq!(incident_for(Kpo(900), 0), Aucun);
        assert_eq!(incident_for(Kpo(900), 7), Aucun);
    }

    #[test]
    fn un_incident_mineur_retire_dix_fois_le_de() {
        for (de, perte) in [(1, 10), (2, 20), (3, 30)] {
            assert_eq!(loss_for(Kpo(150), Minor, &[de]), Kpo(perte), "1D3 de {de}");
        }
    }

    /// **L'arrondi porte sur la perte**, pas sur ce qui reste : à 345 kPo, un
    /// majeur retire 170 et en laisse 175.
    #[test]
    fn un_incident_majeur_arrondit_la_perte_aux_cinq_kpo_inferieurs() {
        assert_eq!(loss_for(Kpo(345), Major, &[]), Kpo(170));
        assert_eq!(loss_for(Kpo(300), Major, &[]), Kpo(150));
        // Le cas impair : 347/2 = 173 sur des entiers, puis 173/5*5 = 170.
        assert_eq!(loss_for(Kpo(347), Major, &[]), Kpo(170));
    }

    /// La catastrophe dit ce qui **reste**, et la perte s'en déduit.
    #[test]
    fn une_catastrophe_ne_laisse_que_dix_fois_les_deux_des() {
        let perte = loss_for(Kpo(560), Catastrophe, &[3, 4]);
        assert_eq!(perte, Kpo(490));
        assert_eq!(560 - perte.0, 70, "il doit rester 70 kPo");
    }

    /// 2D6 peut dépasser une trésorerie à peine au-dessus du seuil : la perte
    /// est nulle, elle ne devient pas négative.
    #[test]
    fn une_catastrophe_ne_rend_jamais_d_argent() {
        assert_eq!(loss_for(Kpo(100), Catastrophe, &[6, 6]), Kpo(0));
    }

    #[test]
    fn le_nombre_de_des_depend_de_l_incident() {
        assert_eq!(dice_needed(Aucun), 0);
        assert_eq!(dice_needed(Minor), 1);
        assert_eq!(dice_needed(Major), 0);
        assert_eq!(dice_needed(Catastrophe), 2);
    }
}

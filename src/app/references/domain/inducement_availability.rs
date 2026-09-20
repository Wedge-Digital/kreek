//! Quels coups de pouce une équipe a le droit d'acheter.
//!
//! # Ce que `restrictedTo` désigne — trois choses, pas une
//!
//! Le corpus y met, selon l'entrée :
//!
//! | Coup de pouce | `restrictedTo` | Ce que c'est |
//! |---|---|---|
//! | Assistant mortuaire | `MASTERS_OF_UNDEATH` | une règle spéciale |
//! | Médecin de peste | `FAVOURED_OF_NURGLE` | une règle spéciale |
//! | Recrues turbulentes | `LOW_COST_LINEMEN` | une règle spéciale |
//! | Apothicaire itinérant | `APOTHECARY` | un membre du **staff** |
//! | Masseur douteux (démo) | `DEMO_GRANIT` | un **roster** |
//!
//! Le sélecteur ne comparait ces entrées qu'à l'**identifiant du roster**
//! (carte 561). Sur le corpus de production, aucune ne l'est : les quatre
//! coups de pouce restreints n'étaient donc proposés à personne. Sur le jeu de
//! démonstration, la seule entrée est un roster, et le filtre marchait — c'est
//! ce qui a fait passer le défaut inaperçu.
//!
//! Une entrée est satisfaite si elle désigne l'une des trois. Les trois espaces
//! d'identifiants ne se recoupent pas : le corpus de production porte 31
//! rosters, 14 règles spéciales et 6 membres du staff, sans collision. Tester
//! les trois ne peut donc pas ouvrir un droit par homonymie.
//!
//! # Le même filtre des deux côtés
//!
//! Le sélecteur décidait seul, et la liste des coups de pouce autorisés que
//! l'adapter donne au domaine ne filtrait rien : un achat forgé à la main
//! passait. C'est le pendant de ce que la carte 507 a établi pour le prix —
//! ce qu'on montre et ce qu'on accepte sortent du même endroit.

use super::models::Inducement;
use super::profil_roster::ProfilRoster;

/// Cette équipe a-t-elle le droit d'acheter ce coup de pouce ?
///
/// Un `restrictedTo` vide n'restreint personne : c'est le cas de la très
/// grande majorité des coups de pouce.
pub fn est_disponible_pour(inducement: &Inducement, profil: &ProfilRoster<'_>) -> bool {
    if inducement.restricted_to.is_empty() {
        return true;
    }
    inducement
        .restricted_to
        .iter()
        .any(|exigee| satisfait(exigee, profil))
}

fn satisfait(exigee: &str, profil: &ProfilRoster<'_>) -> bool {
    exigee == profil.roster_id
        || profil.regles_speciales.iter().any(|r| r == exigee)
        || profil.staff_autorise.iter().any(|s| s == exigee)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn restreint_a(pour: &[&str]) -> Inducement {
        Inducement {
            uid: "COUP_DE_POUCE".to_string(),
            name: "Coup de pouce".to_string(),
            cost: 100,
            reduced_cost: None,
            reduced_cost_for: vec![],
            max_quantity: 1,
            category: "SPECIALIZED".to_string(),
            restricted_to: pour.iter().map(|s| s.to_string()).collect(),
            description: String::new(),
        }
    }

    fn profil<'a>(
        roster_id: &'a str,
        regles: &'a [String],
        staff: &'a [String],
    ) -> ProfilRoster<'a> {
        ProfilRoster {
            roster_id,
            regles_speciales: regles,
            staff_autorise: staff,
        }
    }

    fn rien() -> Vec<String> {
        vec![]
    }

    #[test]
    fn sans_restriction_tout_le_monde_y_a_droit() {
        let libre = restreint_a(&[]);
        assert!(est_disponible_pour(&libre, &profil("N_IMPORTE", &[], &[])));
    }

    /// Le cas du corpus de production, que le filtre d'avant ne voyait pas :
    /// l'entrée est une **règle spéciale**, jamais un roster.
    #[test]
    fn une_regle_speciale_ouvre_le_droit() {
        let medecin = restreint_a(&["FAVOURED_OF_NURGLE"]);
        let regles = vec!["FAVOURED_OF_NURGLE".to_string()];
        assert!(est_disponible_pour(
            &medecin,
            &profil("NURGLE", &regles, &[])
        ));
        assert!(!est_disponible_pour(
            &medecin,
            &profil("WOOD_ELF", &[], &[])
        ));
    }

    /// L'Apothicaire itinérant n'est offert qu'aux équipes qui **peuvent
    /// engager** un apothicaire. Les quatre qui n'y ont pas droit — Rois des
    /// Tombes, Horreur nécromantique, Nurgle, Morts Ambulants — ne le louent
    /// pas non plus.
    #[test]
    fn un_membre_du_staff_autorise_ouvre_le_droit() {
        let itinerant = restreint_a(&["APOTHECARY"]);
        let staff = vec!["APOTHECARY".to_string(), "CHEERLEADERS".to_string()];
        assert!(est_disponible_pour(
            &itinerant,
            &profil("DWARF", &[], &staff)
        ));

        let sans = vec!["CHEERLEADERS".to_string()];
        assert!(!est_disponible_pour(
            &itinerant,
            &profil("SHAMBLING_UNDEAD", &[], &sans)
        ));
    }

    /// La forme que porte le jeu de démonstration, et la seule que le filtre
    /// d'avant savait lire. Elle doit continuer de marcher.
    #[test]
    fn un_roster_nomme_ouvre_le_droit() {
        let masseur = restreint_a(&["DEMO_GRANIT"]);
        assert!(est_disponible_pour(
            &masseur,
            &profil("DEMO_GRANIT", &[], &[])
        ));
        assert!(!est_disponible_pour(
            &masseur,
            &profil("DEMO_ZEPHYR", &[], &[])
        ));
    }

    /// Un roster absent du catalogue n'a ni règle ni staff : il n'accède à
    /// aucun coup de pouce restreint. Le code n'ouvre pas de droit par défaut.
    #[test]
    fn un_roster_inconnu_du_catalogue_n_a_droit_a_rien_de_restreint() {
        let medecin = restreint_a(&["FAVOURED_OF_NURGLE"]);
        let profil = ProfilRoster::depuis("ROSTER_FANTOME", None);
        assert!(!est_disponible_pour(&medecin, &profil));
    }

    /// Plusieurs entrées : une seule suffit.
    #[test]
    fn une_seule_entree_satisfaite_suffit() {
        let large = restreint_a(&["MASTERS_OF_UNDEATH", "APOTHECARY"]);
        let staff = vec!["APOTHECARY".to_string()];
        assert!(est_disponible_pour(
            &large,
            &profil("DWARF", &rien(), &staff)
        ));
    }
}

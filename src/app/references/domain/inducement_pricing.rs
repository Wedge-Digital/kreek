//! Le prix d'un coup de pouce pour une équipe donnée.
//!
//! Deux raisons font qu'un coup de pouce coûte moins cher à une équipe :
//!
//! - elle porte une **règle spéciale** que le corpus désigne dans
//!   `reducedCostFor` — « Chantage et Corruption » baisse les Pots-de-vin de
//!   100 à 50 et le Représentant véreux de 120 à 80 (carte 560) ;
//! - elle est l'équipe **halfling**, seule à payer son maître cuisinier 100 au
//!   lieu de 300 (carte 507). Ce cas-là est en dur, et la section suivante dit
//!   pourquoi.
//!
//! **Une seule fonction pour les deux consommateurs**, et c'est le point de ce
//! module : le prix affiché par le sélecteur et le prix débité par le rapport
//! de match sortaient tous deux de `Inducement::cost`, chacun de son côté.
//! Corriger le seul affichage aurait fait lire 100 au coach en lui prélevant
//! 300 — pire que le défaut d'origine.

use super::models::Inducement;
use super::profil_roster::ProfilRoster;

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
///
/// **La lecture de `reducedCostFor`, ajoutée par la carte 560, ne le remet pas
/// en cause** : une ligue n'apparaît jamais dans les règles spéciales d'un
/// roster, et aucun identifiant de ligue ne coïncide avec un identifiant de
/// règle spéciale. L'entrée halfling reste donc lettre morte pour le mécanisme
/// général, et ce cas-ci la porte seul.
const CUISTOT_HALFLING: &str = "HALFLING_MASTER_CHEF";
const ROSTER_HALFLING: &str = "HALFLING";

/// Ce que cette équipe paie ce coup de pouce.
///
/// Le **montant** réduit reste lu dans le corpus, où vivent tous les autres
/// prix. S'il manque, on rend le prix plein : le code ne fabrique aucun
/// chiffre qu'on ne lui a pas donné.
pub fn cout_pour_roster(inducement: &Inducement, profil: &ProfilRoster<'_>) -> u32 {
    match a_droit_au_tarif_reduit(inducement, profil) {
        true => inducement.reduced_cost.unwrap_or(inducement.cost),
        false => inducement.cost,
    }
}

fn a_droit_au_tarif_reduit(inducement: &Inducement, profil: &ProfilRoster<'_>) -> bool {
    if inducement.uid == CUISTOT_HALFLING {
        return profil.roster_id == ROSTER_HALFLING;
    }
    inducement
        .reduced_cost_for
        .iter()
        .any(|exigee| profil.regles_speciales.iter().any(|r| r == exigee))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHANTAGE: &str = "BRIBERY_AND_CORRUPTION";

    fn inducement(uid: &str, cost: u32, reduced: Option<u32>, pour: &[&str]) -> Inducement {
        Inducement {
            uid: uid.to_string(),
            name: uid.to_string(),
            cost,
            reduced_cost: reduced,
            reduced_cost_for: pour.iter().map(|s| s.to_string()).collect(),
            max_quantity: 1,
            category: "COMMON".to_string(),
            restricted_to: vec![],
            description: String::new(),
        }
    }

    fn cuistot(reduced: Option<u32>) -> Inducement {
        // Tel que le corpus le porte : son `reducedCostFor` désigne une ligue.
        inducement(CUISTOT_HALFLING, 300, reduced, &["HALFLING_THIMBLE_CUP"])
    }

    fn pots_de_vin() -> Inducement {
        inducement("BRIBES", 100, Some(50), &[CHANTAGE])
    }

    fn representant_vereux() -> Inducement {
        inducement("DODGY_LEAGUE_REP", 120, Some(80), &[CHANTAGE])
    }

    /// Un profil sans passer par le catalogue, pour les tests.
    fn profil<'a>(roster_id: &'a str, regles: &'a [String]) -> ProfilRoster<'a> {
        ProfilRoster {
            roster_id,
            regles_speciales: regles,
            staff_autorise: &[],
        }
    }

    fn avec_chantage() -> Vec<String> {
        vec!["BRAWLIN_BRUTES".to_string(), CHANTAGE.to_string()]
    }

    // ── La règle de la carte 560 ─────────────────────────────────────────────

    #[test]
    fn les_pots_de_vin_sont_a_moitie_prix_avec_chantage_et_corruption() {
        let regles = avec_chantage();
        assert_eq!(
            cout_pour_roster(&pots_de_vin(), &profil("DWARF", &regles)),
            50
        );
    }

    /// Le second coup de pouce de la même règle. Le coach n'a signalé que les
    /// pots-de-vin ; les deux étaient au plein tarif.
    #[test]
    fn le_representant_vereux_suit_la_meme_regle() {
        let regles = avec_chantage();
        assert_eq!(
            cout_pour_roster(&representant_vereux(), &profil("GOBLIN", &regles)),
            80
        );
    }

    #[test]
    fn sans_la_regle_speciale_le_plein_tarif_s_applique() {
        let regles = vec!["MASTERS_OF_UNDEATH".to_string()];
        assert_eq!(
            cout_pour_roster(&pots_de_vin(), &profil("SHAMBLING_UNDEAD", &regles)),
            100
        );
        assert_eq!(
            cout_pour_roster(&representant_vereux(), &profil("SHAMBLING_UNDEAD", &regles)),
            120
        );
    }

    /// Un roster absent du catalogue n'a aucune règle spéciale, donc aucun
    /// rabais. Le code ne fabrique pas d'avantage.
    #[test]
    fn un_roster_inconnu_du_catalogue_paie_plein_tarif() {
        let profil = ProfilRoster::depuis("ROSTER_FANTOME", None);
        assert_eq!(cout_pour_roster(&pots_de_vin(), &profil), 100);
    }

    // ── Le cas halfling de la carte 507, qui ne doit pas bouger ──────────────

    #[test]
    fn le_cuistot_est_reduit_pour_les_halflings() {
        assert_eq!(
            cout_pour_roster(&cuistot(Some(100)), &profil(ROSTER_HALFLING, &[])),
            100
        );
    }

    /// Les Gnomes partagent la ligue `HALFLING_THIMBLE_CUP` avec les halflings.
    /// C'est par eux que la première analyse s'est trompée : ils paient plein
    /// tarif, et ce test est là pour que ça reste vrai.
    ///
    /// **Il garde tout son sens depuis la carte 560** : la ligue que porte le
    /// `reducedCostFor` du cuistot est passée à la moulinette du nouveau
    /// mécanisme, et un roster ne porte jamais de ligue dans ses règles
    /// spéciales. Le test le vérifie en donnant aux Gnomes une règle spéciale.
    #[test]
    fn le_cuistot_reste_plein_tarif_ailleurs() {
        let regles = avec_chantage();
        assert_eq!(
            cout_pour_roster(&cuistot(Some(100)), &profil("GNOME", &regles)),
            300
        );
        assert_eq!(
            cout_pour_roster(&cuistot(Some(100)), &profil("DWARF", &regles)),
            300
        );
    }

    /// Le cas qui ferait tomber une implémentation naïve : une équipe qui
    /// aurait la ligue halfling **dans ses règles spéciales** n'existe pas, mais
    /// si le corpus dérivait, le cuistot resterait à 300 pour tout le monde sauf
    /// l'équipe halfling.
    #[test]
    fn le_cuistot_ignore_les_regles_speciales() {
        let regles = vec!["HALFLING_THIMBLE_CUP".to_string()];
        assert_eq!(
            cout_pour_roster(&cuistot(Some(100)), &profil("GNOME", &regles)),
            300
        );
    }

    /// Sans montant réduit dans le corpus, le prix ne bouge pas — même pour
    /// l'équipe qui y aurait droit. Le code n'invente pas de chiffre.
    #[test]
    fn sans_prix_reduit_le_tarif_ne_bouge_pas() {
        assert_eq!(
            cout_pour_roster(&cuistot(None), &profil(ROSTER_HALFLING, &[])),
            300
        );
        let regles = avec_chantage();
        let mut sans_montant = pots_de_vin();
        sans_montant.reduced_cost = None;
        assert_eq!(
            cout_pour_roster(&sans_montant, &profil("DWARF", &regles)),
            100
        );
    }
}

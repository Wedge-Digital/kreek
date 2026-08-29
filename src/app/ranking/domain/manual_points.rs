//! Les points de classement qui ne viennent d'aucun match.
//!
//! Forfait, sanction, rattrapage : un gestionnaire ajoute ou retire des points à
//! une équipe, et doit dire pourquoi. Ce module ne porte que les deux valeurs —
//! la table arrive avec la carte 450, l'écran avec la 452.

use crate::app::shared_kernel::identity::charset::TEXTE_SAISI;
use nutype::nutype;

/// **Signé** : une pénalité est un point manuel négatif, pas une autre nature de
/// chose. Deux types — un pour le bonus, un pour la pénalité — obligeraient
/// chaque lecteur à additionner deux colonnes de sens contraire.
///
/// **Zéro est refusé** : une ligne qui ne change rien au classement, mais occupe
/// le relevé et réclame un motif, est du bruit.
///
/// **±100** est un garde-fou contre la faute de frappe — un `300` saisi pour un
/// `3` —, pas une règle du jeu. Si une ligue a besoin de davantage, c'est la
/// borne qui bouge, pas le principe.
#[nutype(
    validate(predicate = |n| *n != 0 && n.abs() <= 100),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct ManualPoints(i32);

/// **`TEXTE_SAISI` et non un charset propre.** Onze charsets ont coexisté dans
/// ce projet, dont neuf refusaient l'apostrophe — un « Capitaine d'équipe »
/// s'affichait, s'ajoutait au panier, et n'échouait qu'à la validation, sur une
/// erreur qui accusait le catalogue. On n'en rouvre pas un douzième.
///
/// **200 caractères** parce qu'un motif est une phrase, pas un libellé : « forfait
/// non déclaré au troisième tour, sanction votée par les commissaires » doit
/// tenir.
#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 200, regex = TEXTE_SAISI),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct ManualPointsReason(String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_points_refuse_zero() {
        assert!(ManualPoints::try_new(0).is_err());
    }

    #[test]
    fn manual_points_accepte_un_negatif() {
        // Le signe **porte le sens** : c'est ce qui évite un second type.
        assert_eq!(ManualPoints::try_new(-3).unwrap().into_inner(), -3);
    }

    #[test]
    fn manual_points_refuse_au_dela_de_cent() {
        // Aux **deux** extrémités : une faute de frappe est aussi probable au
        // signe moins.
        assert!(ManualPoints::try_new(101).is_err());
        assert!(ManualPoints::try_new(-101).is_err());
        assert!(ManualPoints::try_new(100).is_ok());
        assert!(ManualPoints::try_new(-100).is_ok());
    }

    #[test]
    fn reason_refuse_le_vide_apres_trim() {
        // `sanitize(trim)` s'applique avant `not_empty` : trois espaces ne sont
        // pas un motif.
        assert!(ManualPointsReason::try_new("   ").is_err());
        assert!(ManualPointsReason::try_new("").is_err());
    }

    #[test]
    fn reason_accepte_une_apostrophe() {
        // Le piège des neuf charsets fautifs, vérifié plutôt que supposé.
        let motif = ManualPointsReason::try_new("forfait de l'équipe adverse").unwrap();
        assert_eq!(motif.as_ref(), "forfait de l'équipe adverse");
    }

    #[test]
    fn reason_refuse_au_dela_de_deux_cents_caracteres() {
        assert!(ManualPointsReason::try_new("a".repeat(200)).is_ok());
        assert!(ManualPointsReason::try_new("a".repeat(201)).is_err());
    }
}

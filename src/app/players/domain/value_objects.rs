use crate::app::shared_kernel::identity::charset::TEXTE_SAISI;
use nutype::nutype;

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = TEXTE_SAISI),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct PositionNameVo(String);

#[nutype(
    validate(not_empty),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        Display,
        AsRef
    )
)]
pub struct RosterLineId(String);

// Bornes alignées sur `team_creation::JerseyNumber` (1..99), d'où viennent
// tous les maillots créés aujourd'hui : le zéro n'a jamais désigné un joueur,
// et au-delà de 99 le numéro ne tient plus sur un maillot.
#[nutype(
    validate(greater_or_equal = 1, less_or_equal = 99),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Serialize,
        Deserialize
    )
)]
pub struct JerseyVo(u16);

// Le nom que le coach donne à son joueur, distinct du nom de poste. L'apostrophe
// est admise — beaucoup de patronymes en portent une — là où `PositionNameVo`
// s'en passe, ses valeurs venant du corpus de règles et non d'une saisie.
#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = TEXTE_SAISI),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct PersonalName(String);

// ── Customisation ─────────────────────────────────────────────────────────────

// Crans d'amélioration demandés par le commissaire. Le sens est celui de la
// **qualité du joueur** : `+1` améliore, `-1` dégrade. La traduction en offset
// brut dépend de la caractéristique et appartient à `StatKind`.
//
// Non nul : un cran de zéro n'est pas une customisation, c'est un clic perdu.
// L'amplitude n'est pas bornée ici — ce sont les bornes de la caractéristique
// qui décident, et elles dépendent de l'état du joueur.
#[nutype(
    validate(predicate = |c| *c != 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct StatCrans(i8);

// Ajustement de prix, en kPo. Signé, non nul. Le plancher à zéro porte sur le
// **résultat**, pas sur le delta : il se juge contre la valeur courante.
#[nutype(
    validate(predicate = |d| *d != 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct KpoDelta(i32);

// SPP ajoutés en une opération. Le plafond de 100 est celui de l'opération, pas
// du total du joueur — il tient donc entièrement dans le value object.
#[nutype(
    validate(greater_or_equal = 1, less_or_equal = 100),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct SppAmount(u8);

// Identifiant d'une ligne du panier. Il meurt avec le panier.
#[nutype(
    validate(not_empty),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        Display,
        AsRef
    )
)]
pub struct BasketLineId(String);

// Identifiant d'une customisation appliquée. Contrairement au précédent, il vit
// dans l'event store — c'est lui que la phase 1 exige unique et permanent.
#[nutype(
    validate(not_empty),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        Display,
        AsRef
    )
)]
pub struct CustomisationId(String);

// Rang libre du joueur dans l'effectif, choisi par glisser-déposer. Aucune
// validation : toute position est licite, c'est l'unicité au sein d'une équipe
// qui compte, et elle relève du use case, pas du value object.
#[nutype(derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize
))]
pub struct DisplayOrder(u32);

#[nutype(
    validate(not_empty),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        Display,
        AsRef
    )
)]
pub struct SkillId(String);

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = TEXTE_SAISI),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct SkillName(String);

#[nutype(
    validate(less_or_equal = 99),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct SppCost(u8);

#[cfg(test)]
mod tests {
    use super::*;

    /// La régression qui a motivé le charset commun : la compétence
    /// « Capitaine d'équipe » s'affichait dans le sélecteur, s'ajoutait au
    /// panier, et n'échouait qu'à la validation — sur un `UnknownSkill` qui
    /// accusait le catalogue alors que seul son nom était en cause.
    #[test]
    fn une_competence_a_apostrophe_est_un_nom_valide() {
        assert!(SkillName::try_new("Capitaine d'équipe".to_string()).is_ok());
        assert!(SkillName::try_new("Capitaine d’équipe".to_string()).is_ok());
    }

    #[test]
    fn les_noms_de_poste_et_de_joueur_admettent_le_francais() {
        assert!(PositionNameVo::try_new("Coureur d'élite".to_string()).is_ok());
        assert!(PersonalName::try_new("Jean-Pierre O’Brien".to_string()).is_ok());
    }

    #[test]
    fn ce_qui_reste_refuse() {
        assert!(SkillName::try_new("Block|Dodge".to_string()).is_err());
        assert!(SkillName::try_new("   ".to_string()).is_err());
        assert!(SkillName::try_new("a".repeat(51)).is_err());
    }
}

use nutype::nutype;

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = r"^[\p{L}0-9 -]+$"),
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
    validate(not_empty, len_char_max = 50, regex = r"^[\p{L}0-9 '-]+$"),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct PersonalName(String);

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
    validate(not_empty, len_char_max = 50, regex = r"^[\p{L}0-9 -]+$"),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct SkillName(String);

#[nutype(
    validate(less_or_equal = 99),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct SppCost(u8);

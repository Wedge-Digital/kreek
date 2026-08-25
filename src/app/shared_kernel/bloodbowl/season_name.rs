use crate::app::shared_kernel::identity::charset::TEXTE_SAISI;
use nutype::nutype;

/// Le nom d'une saison.
///
/// Un `nutype` à part entière, et non un alias : les quatre noms du jeu ont
/// longtemps été **le même type Rust**, si bien que le compilateur acceptait
/// une saison là où une équipe était attendue. Ils partagent leur charset, pas
/// leur identité.
#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = TEXTE_SAISI),
    derive(
        Debug,
        Clone,
        Serialize,
        Deserialize,
        PartialEq,
        Eq,
        Hash,
        Display,
        AsRef
    )
)]
pub struct SeasonName(String);

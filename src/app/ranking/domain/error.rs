//! Les erreurs métier du BC `ranking`.
//!
//! Le BC n'en avait aucune jusqu'ici : `record_match` ne peut pas échouer, il
//! accumule. C'est son **inverse** qui en a besoin — `stats_between` lit des
//! lignes déjà écrites, et deux formes de corruption s'y détectent.
//!
//! `Display` est écrit à la main, comme dans `competitions` : le BC n'utilise
//! pas `thiserror`, et le message sert de corps de réponse.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// Un cumul qui **décroît** d'une ligne à la suivante. Impossible par
    /// construction — `record_match` n'additionne jamais de valeur négative —
    /// donc le signe que les deux lignes ne se suivent pas, ou qu'elles ont été
    /// altérées hors du domaine.
    ///
    /// `checked_sub` plutôt qu'une soustraction nue : sur des `u32`, l'écart
    /// négatif déborderait par le bas et rendrait un nombre colossal, dont le
    /// `u8::try_from` suivant dirait « hors bornes » — le bon refus pour la
    /// mauvaise raison.
    DecreasingTotal {
        field: &'static str,
        previous: u32,
        current: u32,
    },
    /// Un écart qui ne tient pas dans un `MatchScore`, c'est-à-dire un `u8`.
    ///
    /// La conversion passe par `try_from`, jamais par `as` — qui replierait un
    /// écart aberrant en un score parfaitement plausible. Seule une corruption
    /// de lignes le produit, et c'est précisément le cas qu'il ne faut pas
    /// maquiller.
    ScoreOutOfRange { field: &'static str, value: u32 },
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DecreasingTotal {
                field,
                previous,
                current,
            } => write!(
                f,
                "Le cumul « {field} » décroît : {previous} puis {current}. \
                 Les lignes de classement sont incohérentes."
            ),
            Self::ScoreOutOfRange { field, value } => write!(
                f,
                "L'écart de « {field} » vaut {value}, hors des bornes d'un score de match."
            ),
        }
    }
}

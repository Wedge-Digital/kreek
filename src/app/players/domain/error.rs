use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    SkillAlreadyAcquired,
    InsufficientSpp,
    PlayerNotActive,

    // ── Customisation ─────────────────────────────────────────────────────────
    UnknownSkill,
    /// La valeur résolue sortirait des bornes de la caractéristique. `bound`
    /// porte celle qui a été franchie, pour que le message la nomme.
    StatOutOfBounds {
        stat: crate::app::players::domain::match_impact::StatKind,
        bound: u8,
    },
    NegativePlayerValue,
    BasketLineNotFound,
    /// Le retrait viserait des SPP déjà convertis en compétence ou en
    /// caractéristique. `restants` et `offerts` sont dans l'erreur parce que le
    /// message les nomme : « il n'en reste que 3 sur 10 » enseigne la règle, là
    /// où « SPP insuffisants » laisse chercher.
    CustomisationSppSpent {
        restants: u32,
        offerts: u32,
    },
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SkillAlreadyAcquired => write!(f, "compétence déjà possédée"),
            Self::InsufficientSpp => write!(f, "SPP insuffisants"),
            Self::PlayerNotActive => write!(f, "joueur non actif"),
            Self::UnknownSkill => write!(f, "compétence inconnue du catalogue"),
            Self::StatOutOfBounds { bound, .. } => {
                write!(f, "la caractéristique sortirait de ses bornes ({bound})")
            }
            Self::NegativePlayerValue => write!(f, "le prix ne peut pas être négatif"),
            Self::BasketLineNotFound => write!(f, "ligne de panier introuvable"),
            Self::CustomisationSppSpent { restants, offerts } => write!(
                f,
                "ces SPP ont été dépensés — il n'en reste que {restants} sur {offerts}"
            ),
        }
    }
}

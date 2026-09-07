use std::fmt;

/// Erreurs d'invariant du domaine `competitions`.
///
/// `Display` est implémenté à la main, comme dans les autres BCs
/// (`players`, `teams`, `match_report`) : le projet n'utilise pas `thiserror`.
/// Le message est directement exploitable comme corps de réponse 422, le
/// formulaire de règles affichant la réponse telle quelle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    EmptyTiebreakConfig,
    NoActiveTiebreaker,
    DuplicateTiebreakCode {
        code: String,
    },

    // ── Réglages rouverts sur une saison en cours (épic E14) ─────────────────
    DuplicatePoolName {
        name: String,
    },
    DuplicatePoolId {
        id: String,
    },
    /// Le nombre de tiers ne se modifie pas depuis l'onglet Paramètres : seuls
    /// leurs coups de pouce sont rouverts.
    TierCountChanged {
        before: usize,
        after: usize,
    },
    /// `field` est un `&'static str` et non un `String` : il ne peut venir que
    /// du code qui a détecté l'écart, jamais d'une requête.
    ImmutableTierField {
        tier: String,
        field: &'static str,
    },
    RosterInMultipleTiers {
        roster: String,
        tiers: (String, String),
    },
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTiebreakConfig => {
                write!(f, "La configuration de départage est vide.")
            }
            Self::NoActiveTiebreaker => {
                write!(f, "Au moins un critère de départage doit être actif.")
            }
            Self::DuplicateTiebreakCode { code } => {
                write!(
                    f,
                    "Le critère de départage « {code} » est présent plusieurs fois."
                )
            }
            Self::DuplicatePoolName { name } => {
                write!(f, "Deux poules portent le nom « {name} ».")
            }
            Self::DuplicatePoolId { id } => {
                write!(f, "Deux poules portent l'identifiant « {id} ».")
            }
            Self::TierCountChanged { before, after } => {
                write!(
                    f,
                    "Le nombre de tiers ne peut pas changer ici : {before} avant, {after} reçus."
                )
            }
            Self::ImmutableTierField { tier, field } => {
                write!(
                    f,
                    "Le champ « {field} » du tier « {tier} » ne se modifie pas depuis les réglages."
                )
            }
            Self::RosterInMultipleTiers { roster, tiers } => {
                let (a, b) = tiers;
                write!(
                    f,
                    "Le roster « {roster} » figure dans deux tiers : « {a} » et « {b} »."
                )
            }
        }
    }
}

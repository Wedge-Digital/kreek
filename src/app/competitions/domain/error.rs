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
    // ── Sondage de présence (épic E16) ───────────────────────────────────────
    /// R2 — il n'y a rien à apparier une journée de repos, donc rien à sonder.
    SurveyOnRestDay,
    /// R15 — le tirage refuse en dessous de deux présents, et le dit : un aperçu
    /// vide passerait pour une panne.
    NotEnoughPresent {
        presents: usize,
    },
    InvalidSurveyToken,
    InvalidOpenedAt,

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
            Self::SurveyOnRestDay => {
                write!(f, "On ne sonde pas une journée de repos.")
            }
            Self::NotEnoughPresent { presents } => {
                write!(
                    f,
                    "Il faut au moins deux équipes présentes pour tirer au sort ({presents} pour l'instant)."
                )
            }
            Self::InvalidSurveyToken => write!(f, "Ce lien de réponse est illisible."),
            Self::InvalidOpenedAt => write!(f, "Date d'ouverture invalide."),
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

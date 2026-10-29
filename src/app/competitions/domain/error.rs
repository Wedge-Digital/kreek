use crate::app::shared_kernel::bloodbowl::team::TeamId;
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

    // ── Tirage d'une journée (épic E18) ──────────────────────────────────────
    /// R10 — aucune poule n'a deux équipes à apparier.
    NotEnoughPresent,
    /// R9 — une journée de repos ne reçoit pas de rencontres.
    RestDayCannotBeDrawn,
    /// R9, R11 — la journée porte déjà une rencontre.
    RoundAlreadyPaired,
    /// R11 — l'équipe n'est plus inscrite à la compétition.
    ///
    /// Les variants qui suivent portent des `TeamId`, pas des noms : le domaine
    /// ne les connaît pas. Le handler reformule le message avec les noms.
    TeamNotEnrolled {
        team: TeamId,
    },
    /// R11 — une équipe figure dans deux rencontres, ou deux fois dans la même.
    TeamDrawnTwice {
        team: TeamId,
    },
    /// R4 — les deux équipes appartiennent au même coach.
    SameCoachPairing {
        home: TeamId,
        away: TeamId,
    },
    /// R3 — les deux équipes ne sont pas dans la même poule.
    PairingAcrossPools {
        home: TeamId,
        away: TeamId,
    },
    /// R11 — l'exemptée figure aussi dans une rencontre.
    ExemptedTeamAlsoPaired {
        team: TeamId,
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
            Self::NotEnoughPresent => {
                write!(
                    f,
                    "Il faut au moins deux équipes présentes dans une même poule pour tirer une rencontre."
                )
            }
            Self::RestDayCannotBeDrawn => {
                write!(f, "Une journée de repos ne reçoit pas de rencontres.")
            }
            Self::RoundAlreadyPaired => {
                write!(f, "Cette journée a déjà des rencontres.")
            }
            Self::TeamNotEnrolled { .. } => {
                write!(f, "Une équipe tirée n'est plus inscrite à la compétition.")
            }
            Self::TeamDrawnTwice { .. } => {
                write!(f, "Une équipe figure dans deux rencontres du tirage.")
            }
            Self::SameCoachPairing { .. } => {
                write!(
                    f,
                    "Deux équipes d'un même coach ne peuvent pas se rencontrer."
                )
            }
            Self::PairingAcrossPools { .. } => {
                write!(
                    f,
                    "Deux équipes de poules différentes ne peuvent pas se rencontrer."
                )
            }
            Self::ExemptedTeamAlsoPaired { .. } => {
                write!(f, "L'équipe exemptée figure aussi dans une rencontre.")
            }
        }
    }
}

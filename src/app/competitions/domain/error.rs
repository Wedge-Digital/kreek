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
    InvalidReponduLe,
    /// R19 — une réponse ne vaut que pour une équipe de la campagne. Sans cette
    /// garde, un `team_id` forgé poserait une présence pour l'équipe d'une autre
    /// compétition du même espace : `require_admin_access` vérifie que la saison
    /// appartient à la compétition, jamais que l'équipe appartient à la campagne.
    TeamNotInSurvey {
        team: String,
    },
    /// R28 — le coach connecté répond pour ses équipes, et pour elles seules.
    /// R19 vérifie que l'équipe est *dans* la campagne, jamais qu'elle est *à
    /// lui* ; sans cette garde, un `team_id` forgé depuis l'encart poserait une
    /// présence chez le voisin.
    TeamNotOwnedByCoach {
        team: String,
    },
    /// R21 — les deux chemins du coach s'arrêtent à la clôture. L'organisateur,
    /// lui, passe : c'est lui qui rattrape un coup de fil reçu après l'échéance.
    SurveyClosedForCoach,
    /// R13 — une journée dont un rapport est publié ne bouge plus. Changer une
    /// présence y proposerait de refaire une rencontre déjà jouée.
    RoundFrozenByReport,

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
            Self::InvalidReponduLe => write!(f, "Date de réponse invalide."),
            Self::TeamNotInSurvey { team } => {
                write!(
                    f,
                    "L'équipe « {team} » ne fait pas partie de cette campagne de présence."
                )
            }
            Self::TeamNotOwnedByCoach { team } => {
                write!(f, "L'équipe « {team} » n'est pas la vôtre.")
            }
            Self::SurveyClosedForCoach => {
                write!(
                    f,
                    "Le sondage est clos : contactez l'organisateur pour signaler un changement."
                )
            }
            Self::RoundFrozenByReport => {
                write!(
                    f,
                    "Un rapport de match est déjà publié sur cette journée : les présences n'y changent plus."
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

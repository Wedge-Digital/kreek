//! Ce qui identifie un envoi de notification, et rien de plus.
//!
//! # `DeliveryKey` est la clé de l'index, à l'identique
//!
//! Les cinq champs ci-dessous sont **exactement** ceux de
//! `idx_notification_deliveries_key`. Si les deux divergeaient, la protection
//! d'unicité ne porterait plus sur ce que le code croit protéger — et le défaut
//! ne se verrait qu'en production, sous la forme d'un coach recevant deux fois
//! le même e-mail.
//!
//! # `target_date` fait tenir R2 à lui seul
//!
//! C'est la date **visée**, pas la date d'envoi. Une journée décalée change donc
//! la clé, ce qui réarme la notification sans qu'une ligne de code lui soit
//! consacrée.

use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
use crate::app::shared_kernel::identity::ids::CoachId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    RegistrationOpen,
    RoundEve,
    RoundClosing,
    RegistrationDeadline,
}

impl NotificationType {
    /// La valeur stockée. Écrite à la main plutôt que dérivée d'un `Debug` ou
    /// d'un `Serialize` : c'est une donnée persistée, et un renommage de
    /// variante ne doit pas silencieusement réarmer toutes les notifications
    /// déjà envoyées.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RegistrationOpen => "registration_open",
            Self::RoundEve => "round_eve",
            Self::RoundClosing => "round_closing",
            Self::RegistrationDeadline => "registration_deadline",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeliveryKey {
    pub notification_type: NotificationType,
    pub season_id: SeasonId,
    /// `None` pour les deux notifications de saison, qui n'ont pas de journée.
    /// C'est ce cas-là que l'index protège par `COALESCE`, et que deux `NULL`
    /// laisseraient passer.
    pub round_id: Option<MatchId>,
    pub target_date: DateString,
    pub coach_id: CoachId,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les quatre valeurs partent en base. Les figer ici rend visible tout
    /// changement : renommer une variante sans y penser réarmerait chaque
    /// notification déjà envoyée, pour tous les coachs.
    #[test]
    fn les_valeurs_stockees_sont_figees() {
        assert_eq!(
            NotificationType::RegistrationOpen.as_str(),
            "registration_open"
        );
        assert_eq!(NotificationType::RoundEve.as_str(), "round_eve");
        assert_eq!(NotificationType::RoundClosing.as_str(), "round_closing");
        assert_eq!(
            NotificationType::RegistrationDeadline.as_str(),
            "registration_deadline"
        );
    }
}

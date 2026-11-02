//! L'émission des événements appendés, commune aux use cases de saisie
//! (carte 643).

use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

/// Émet sur le bus du BC chaque événement **déjà appendé**, dans l'ordre.
///
/// À n'appeler qu'après un append réussi : un conflit d'écriture ne doit rien
/// faire sortir, puisque rien n'a eu lieu.
pub(crate) fn emit_all(bus: &EventBus, mr_id: &str, events: &[MatchReportDomainEvent]) {
    for event in events {
        emettre(bus, event.to_enveloppe(mr_id));
    }
}

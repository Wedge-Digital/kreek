//! Le premier maillon de la chaîne qui relie une réaction à sa requête.
//!
//! # Ce qui manquait
//!
//! Après les cartes 345, 348 et 350, le journal portait deux morceaux de piste
//! et rien qui les relie :
//!
//! ```text
//! grep rid=01M0AB      → la requête, ses use cases, sa réponse   ⟂ s'arrête ici
//! grep event_id=01M0ZZ → l'émission de l'app event et ses réactions
//! ```
//!
//! **Rien sous le `rid` ne mentionnait d'identifiant d'événement**, donc rien
//! ne disait quoi chercher pour passer du premier bloc au second. La coupure
//! est mécanique : le publisher tourne dans sa propre tâche `tokio`, lancée au
//! démarrage, et ignore tout de la requête qui a mis l'événement sur le bus.
//!
//! Cette fonction pose le chaînon manquant. Appelée depuis la tâche de la
//! requête, sa ligne hérite du span `req` : elle porte donc **le `rid` et
//! l'identifiant de l'événement émis, sur la même ligne**.
//!
//! # Pourquoi pas faire voyager le `rid`
//!
//! C'était le projet initial de la carte 351 : un `tokio::task_local!` posé par
//! la couche web, lu au moment de fabriquer l'enveloppe, pour n'avoir qu'un
//! seul `grep`. Écarté au raffinage, pour son mode de panne — un ambiant oublié
//! à une frontière de tâche donne un `rid` faux ou absent **sans que rien ne le
//! signale**. Trois `grep` immédiats, dont chaque ligne ne dit que ce qu'elle
//! sait réellement, valent mieux qu'un seul `grep` qui peut mentir.
//!
//! # Le niveau
//!
//! `info` et non `debug`. Le filtre de production vaut `info` : une ligne posée
//! en dessous n'existe pas là où on en a besoin. C'est la leçon des cartes 344
//! et 349, rencontrée deux fois sous deux formes différentes — un journal
//! compilé hors du binaire, puis une cible hors du filtre.

use crate::common::event_envelope::EventEnvelope;
use crate::common::services::event_bus::event_bus::EventBus;

/// Publie un domain event sur le bus interne de son BC, et laisse la trace qui
/// permet d'y revenir.
///
/// Symétrique exact de `app_event_publication::publier`, qui fait le même
/// travail un cran plus loin, sur le bus applicatif.
pub fn emettre(bus: &EventBus, enveloppe: EventEnvelope) {
    tracing::info!(
        event = %enveloppe.event_type,
        event_id = %enveloppe.event_id,
        "domain event émis"
    );
    // `Err` signifie « aucun abonné », état normal : tout BC n'écoute pas tout.
    // Il était déjà ignoré aux vingt et un sites d'appel.
    let _ = bus.send(enveloppe);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::services::event_bus::event_bus::new_bus;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::layer::SubscriberExt;

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<String>>>);

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let mut champs = Champs(String::new());
            event.record(&mut champs);
            self.0.lock().unwrap().push(champs.0);
        }
    }

    struct Champs(String);

    impl tracing::field::Visit for Champs {
        fn record_debug(&mut self, champ: &tracing::field::Field, valeur: &dyn std::fmt::Debug) {
            self.0.push_str(&format!("{}={:?} ", champ.name(), valeur));
        }
    }

    fn enveloppe(event_id: &str, event_type: &str) -> EventEnvelope {
        EventEnvelope {
            event_id: event_id.to_string(),
            emitter: "T1".to_string(),
            event_type: event_type.to_string(),
            tags: serde_json::json!([]),
            payload: serde_json::json!({}),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }

    /// Sans cet identifiant sur une ligne portant le `rid`, la piste s'arrête à
    /// la réponse HTTP : c'est toute la raison d'être de la carte 351.
    #[test]
    fn la_ligne_porte_l_identifiant_de_l_evenement_emis() {
        let capture = Capture::default();
        let abonne = tracing_subscriber::registry().with(capture.clone());
        let bus = new_bus();

        tracing::subscriber::with_default(abonne, || {
            emettre(&bus, enveloppe("01M0-EMIS", "TeamSubmitted"));
        });

        let lignes = capture.0.lock().unwrap().clone();
        assert_eq!(lignes.len(), 1, "une ligne et une seule : {lignes:?}");
        assert!(
            lignes[0].contains("01M0-EMIS"),
            "l'identifiant : {lignes:?}"
        );
        assert!(
            lignes[0].contains("TeamSubmitted"),
            "le type du domain event, celui que le publisher verra : {lignes:?}"
        );
    }

    /// La journalisation accompagne la publication, elle ne la remplace pas.
    #[test]
    fn l_enveloppe_est_effectivement_publiee() {
        let bus = new_bus();
        let mut rx = bus.subscribe();

        emettre(&bus, enveloppe("01M0-EMIS", "TeamSubmitted"));

        let recue = rx.try_recv().expect("l'enveloppe doit être sur le bus");
        assert_eq!(recue.event_id, "01M0-EMIS");
    }
}

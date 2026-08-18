//! Le seul chemin par lequel un app event quitte son BC.
//!
//! # Ce que le journal ne disait pas
//!
//! Les 19 listeners ouvrent un span portant `event` et `event_id` depuis la
//! carte 345. Le versant **émission**, lui, était muet — ou plutôt bavard sans
//! contexte, sur le seul publisher qui journalisait quelque chose :
//!
//! ```text
//! INFO …team_creation…app_event_publisher: relaying TeamSubmitted to app bus
//! ```
//!
//! Aucun identifiant. On savait qu'un événement avait été relayé, pas lequel,
//! et rien ne le reliait aux spans des listeners qui allaient y réagir.
//!
//! Le piège est double, et cet exemple le montre en entier : **le nom change en
//! route.** `TeamSubmitted` est le domain event ; l'app event qui en résulte
//! s'appelle `TeamCreated`. C'est la règle de nommage de `CLAUDE.md` — un
//! domain event dit ce qui s'est passé dans son domaine, sans trahir sa
//! destination — mais à la lecture du journal, rien ne disait que ces deux noms
//! désignent le même fait. On cherchait « TeamCreated » et on ne trouvait pas
//! l'émission.
//!
//! # Pourquoi une fonction et pas onze lignes recopiées
//!
//! `to_enveloppe()` **engendre un nouvel identifiant** : l'app event n'a pas
//! celui du domain event dont il est issu. C'est donc l'identifiant de
//! l'enveloppe **produite** qu'il faut journaliser. Une ligne écrite à la main
//! au-dessus du `send` aurait toutes les chances de reprendre celui de
//! l'enveloppe reçue — et de produire une trace qui a l'air correcte et ne
//! corrèle rien.
//!
//! Ici la question ne se pose pas : la fonction ne voit que l'enveloppe
//! produite. Le piège est fermé par construction plutôt que par discipline.
//!
//! # Le nom du domain event d'origine
//!
//! Il vient du span ouvert par le publisher sur l'enveloppe reçue, dont le
//! champ `event_type` porte déjà exactement ce nom. Le faire descendre en
//! paramètre aurait demandé de traverser cinq signatures dans `match_report`,
//! dont les app events partent de fonctions appelées à trois niveaux de
//! profondeur — c'est-à-dire décorer l'appelant, ce que l'épic a écarté pour
//! les use cases.

use crate::common::event_envelope::EventEnvelope;
use crate::common::services::event_bus::event_bus::EventBus;

/// Publie sur le bus applicatif, et laisse la trace qui rend la suite lisible.
///
/// Le format de champs est celui des listeners (`event=`, `event_id=`), pour
/// qu'un même `grep event_id=…` ramène l'émission **et** toutes les réactions.
pub fn publier(bus: &EventBus, enveloppe: EventEnvelope) {
    tracing::info!(
        event = %enveloppe.event_type,
        event_id = %enveloppe.event_id,
        "app event émis"
    );
    // Le `Err` de `send` signifie « aucun abonné », ce qui est un état normal :
    // tout BC n'écoute pas tout. Il était déjà ignoré aux onze sites d'appel.
    let _ = bus.send(enveloppe);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::identity::ids::EventId;
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

    fn lignes_produites(corps: impl FnOnce()) -> Vec<String> {
        let capture = Capture::default();
        let abonne = tracing_subscriber::registry().with(capture.clone());
        tracing::subscriber::with_default(abonne, corps);
        let lignes = capture.0.lock().unwrap().clone();
        lignes
    }

    /// Le point central de la carte 350 : c'est l'identifiant de l'enveloppe
    /// **produite** qui doit être journalisé. Journaliser celui reçu sur le bus
    /// interne donnerait une trace qui a l'air correcte et ne corrèle rien —
    /// `to_enveloppe()` en engendre un nouveau.
    #[test]
    fn la_ligne_porte_l_identifiant_de_l_enveloppe_publiee() {
        let bus = new_bus();
        let publiee = enveloppe("01M0-PRODUIT", "teams.player_recruited");

        let lignes = lignes_produites(|| publier(&bus, publiee));

        assert_eq!(lignes.len(), 1, "une ligne et une seule : {lignes:?}");
        assert!(
            lignes[0].contains("01M0-PRODUIT"),
            "l'identifiant publié : {lignes:?}"
        );
        assert!(
            lignes[0].contains("teams.player_recruited"),
            "le type d'app event, celui que les listeners verront : {lignes:?}"
        );
    }

    /// L'enveloppe part bien sur le bus : la journalisation ne remplace pas la
    /// publication, elle l'accompagne.
    #[test]
    fn l_enveloppe_est_effectivement_publiee() {
        let bus = new_bus();
        let mut rx = bus.subscribe();

        publier(
            &bus,
            enveloppe(&EventId::new().to_string(), "teams.player_dismissed"),
        );

        let recue = rx.try_recv().expect("l'enveloppe doit être sur le bus");
        assert_eq!(recue.event_type, "teams.player_dismissed");
    }
}

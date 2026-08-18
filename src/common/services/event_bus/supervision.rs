//! Un listener qui meurt ne doit pas mourir en silence.
//!
//! Chaque listener est une tâche `tokio::spawn` tenant une boucle
//! `loop { rx.recv().await }`. Un panic à l'intérieur fait sortir la tâche de
//! sa boucle, et **personne n'attend son `JoinHandle`** : le `JoinError` part à
//! la poubelle. Le BC cesse alors de réagir aux app events définitivement,
//! sans qu'aucune ligne ne le signale — les projections cessent de se mettre à
//! jour, et le seul symptôme visible est une donnée qui ne bouge plus.
//!
//! C'est le pendant, côté bus, de ce que `CatchPanicLayer` fait pour les
//! requêtes HTTP (carte 349).
//!
//! # Ce que ça fait, et ce que ça ne fait pas
//!
//! **Ça rend la mort bruyante.** Une ligne `ERROR` nomme le listener disparu.
//! Le message du panic lui-même n'est pas perdu pour autant : le gestionnaire
//! de panique par défaut l'écrit sur la sortie d'erreur, que Docker capture au
//! même titre que le reste.
//!
//! **Ça ne ressuscite personne.** Reprendre la souscription supposerait de
//! reconstruire la boucle, donc de recloner ses dépendances et de se
//! réabonner — le `rx` en cours étant consommé. C'est un cran de plus, qui ne
//! se justifiera que si des panics se produisent réellement : aujourd'hui, le
//! manque est qu'on ne le saurait même pas.

use futures_util::FutureExt;
use std::future::Future;
use std::panic::AssertUnwindSafe;

/// Remplace `tokio::spawn` dans les `init()` de listeners.
///
/// `nom` vaut `module_path!()` sur tous les sites d'appel : le chemin du module
/// désigne le listener sans qu'on ait à le nommer une seconde fois, et il ne
/// peut pas diverger du code.
pub fn spawn_listener<F>(nom: &'static str, souscription: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        if AssertUnwindSafe(souscription).catch_unwind().await.is_err() {
            tracing::error!(
                listener = nom,
                "panic — le listener est mort, plus aucun événement ne lui parviendra \
                 jusqu'au redémarrage"
            );
            return;
        }
        // Sortie normale : le bus est fermé, ce qui n'arrive qu'à l'extinction.
        tracing::debug!(listener = nom, "souscription terminée");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
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
            if *event.metadata().level() != tracing::Level::ERROR {
                return;
            }
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

    /// Ce que la supervision doit garantir : la mort du listener produit une
    /// ligne qui le nomme. Sans elle, la tâche disparaît et le seul symptôme
    /// est une projection qui cesse de bouger, des heures plus tard.
    #[tokio::test]
    async fn un_listener_qui_panique_laisse_une_ligne_qui_le_nomme() {
        let capture = Capture::default();
        let abonne = tracing_subscriber::registry().with(capture.clone());
        let _garde = tracing::subscriber::set_default(abonne);

        spawn_listener("bc::le_listener_de_test", async {
            panic!("boum — panic volontaire de test");
        });
        tokio::task::yield_now().await;

        let lignes = capture.0.lock().unwrap().clone();
        assert_eq!(lignes.len(), 1, "une ligne d'erreur attendue : {lignes:?}");
        assert!(
            lignes[0].contains("bc::le_listener_de_test"),
            "la ligne doit nommer le listener disparu : {lignes:?}"
        );
    }

    /// Le pendant : une souscription qui se termine normalement — le bus fermé
    /// à l'extinction — ne doit pas crier au panic.
    #[tokio::test]
    async fn une_souscription_terminee_normalement_ne_produit_aucune_erreur() {
        let capture = Capture::default();
        let abonne = tracing_subscriber::registry().with(capture.clone());
        let _garde = tracing::subscriber::set_default(abonne);

        spawn_listener("bc::le_listener_de_test", async {});
        tokio::task::yield_now().await;

        assert!(capture.0.lock().unwrap().is_empty());
    }
}

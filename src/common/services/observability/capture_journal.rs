//! Capturer les lignes de journal **sous le filtre réellement construit au
//! démarrage**, pour les tests.
//!
//! Le CLAUDE.md le dit en tête de sa section observabilité : une ligne n'existe
//! en production que si sa cible relève de `kreek::` et que son niveau franchit
//! le filtre. Un test qui capte tout ce qui est émis ne prouve donc rien — il
//! passerait sur une ligne qui, en production, n'est écrite nulle part.
//!
//! D'où `filtre_depuis_config` plutôt qu'une chaîne recopiée : un test qui
//! recopie le filtre continue de passer quand le vrai change.
//!
//! Le motif vient de `web/middleware/panic_response.rs` (carte 349), où il
//! servait déjà à prouver qu'une ligne émise sur `tower_http::catch_panic` se
//! perd. Il est ici pour que la troisième copie n'ait pas lieu.

use std::sync::{Arc, Mutex};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;

/// Recueille cible et champs de chaque événement qui **franchit** le filtre.
/// Ce qui est filtré n'arrive jamais ici : c'est exactement ce qu'on mesure.
#[derive(Clone, Default)]
pub struct Capture(Arc<Mutex<Vec<String>>>);

impl Capture {
    pub fn lignes(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl<S: tracing::Subscriber> Layer<S> for Capture {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut champs = Champs(String::new());
        event.record(&mut champs);
        self.0
            .lock()
            .unwrap()
            .push(format!("{} {}", event.metadata().target(), champs.0));
    }
}

struct Champs(String);

impl tracing::field::Visit for Champs {
    fn record_debug(&mut self, champ: &tracing::field::Field, valeur: &dyn std::fmt::Debug) {
        self.0.push_str(&format!("{}={:?} ", champ.name(), valeur));
    }
}

/// Pose l'abonné de production sur le thread courant et rend la capture.
///
/// Tant que le garde vit, les lignes émises **sur ce thread** y atterrissent.
/// C'est la forme utilisable depuis un `#[tokio::test]`, dont le runtime est
/// mono-thread par défaut : `with_default` prend une fermeture synchrone et ne
/// peut pas envelopper un `.await`.
pub fn capture_sous_le_filtre_de_production() -> (Capture, tracing::subscriber::DefaultGuard) {
    let capture = Capture::default();
    let (filtre, _) = crate::filtre_depuis_config("info");
    let abonne = tracing_subscriber::registry().with(capture.clone().with_filter(filtre));
    (capture.clone(), tracing::subscriber::set_default(abonne))
}

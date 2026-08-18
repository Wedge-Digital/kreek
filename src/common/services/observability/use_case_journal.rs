//! La ligne qui dit ce qu'on a demandé au système.
//!
//! # Pourquoi une couche et pas seulement `#[instrument]`
//!
//! `#[instrument]` crée un **span**, il n'émet pas d'événement. Un span ne
//! devient une ligne que si l'abonné est configuré avec `FmtSpan`, ce que
//! `init_journal` ne fait pas. Posé seul sur des use cases muets — et 62 des
//! 63 fichiers de `use_cases/` le sont — l'attribut n'aurait donc produit
//! **aucune ligne** : il aurait enrichi les `error!` existants du contexte de
//! la commande, ce qui est utile, mais le chemin nominal serait resté
//! inexistant. C'est exactement le manque que la carte 348 devait combler.
//!
//! `FmtSpan::CLOSE` global aurait fait exister la ligne, au prix d'une paire
//! de lignes par span — donc quatre par requête — ce que la carte 345 avait
//! déjà pesé et écarté. Cette couche fait le même travail sur le seul dossier
//! qui nous intéresse : elle ne parle que des spans dont la cible contient
//! `::use_cases::`, et laisse les spans de requête et d'app event exactement
//! comme la 345 les a laissés.
//!
//! # À la fermeture, pas à l'ouverture
//!
//! Une ligne au lieu de deux, et la durée y tient. Un panic ne fait pas de
//! trou : le span est détruit pendant le déroulement de pile, donc la ligne
//! sort quand même.
//!
//! **Le seul angle mort est le use case qui ne rend jamais la main** — boucle
//! infinie, verrou de base non relâché. Celui-là ne laisse rien. C'est le prix
//! de la ligne unique ; si le cas se présente, on ajoutera une ligne
//! d'ouverture plutôt que de renoncer à celle-ci.
//!
//! # Le nom du use case
//!
//! `tracing` imprime la cible de l'événement, qui est ici celle de **cette
//! couche** et non celle du use case. Le chemin de module part donc dans un
//! champ `use_case=…`, reconstruit depuis les métadonnées du span. Un
//! `grep validate_customisation` fonctionne pareil.

use std::fmt;
use std::time::Instant;
use tracing::span::{Attributes, Id};
use tracing::Subscriber;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

/// Ce qui distingue un span de use case de tous les autres. La convention de
/// nommage des dossiers est le seul marqueur disponible — et depuis la carte
/// 347 elle est enfin uniforme, `spaces` ayant longtemps écrit `uses_cases`.
const MARQUEUR: &str = "::use_cases::";

/// Cible de la ligne émise, et non le chemin de ce module — qui occuperait
/// cinquante caractères sur chaque ligne du journal sans rien apprendre.
///
/// Le préfixe `kreek::` n'est pas décoratif : le filtre est
/// `kreek=<niveau>,sqlx=warn`, et une cible qui n'en relève pas n'est activée
/// par aucune directive. La carte 349 a perdu une demi-journée sur exactement
/// ce point, avec `tower_http::catch_panic`.
const CIBLE: &str = "kreek::use_case";

pub struct UseCaseJournal;

/// Les champs du span, rendus au moment de sa création, et l'instant du
/// départ. Rangés dans les extensions du span : c'est le seul endroit qui
/// survit entre l'ouverture et la fermeture sans état global.
struct Trace {
    champs: String,
    debut: Instant,
}

/// Les champs de `tracing` sont statiques ; on ne peut pas en réémettre un
/// nombre variable. Ils sont donc aplatis en une chaîne, qui devient le
/// message de la ligne.
#[derive(Default)]
struct Champs(String);

impl tracing::field::Visit for Champs {
    fn record_debug(&mut self, champ: &tracing::field::Field, valeur: &dyn fmt::Debug) {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        self.0.push_str(&format!("{}={:?}", champ.name(), valeur));
    }
}

impl<S> Layer<S> for UseCaseJournal
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        if !attrs.metadata().target().contains(MARQUEUR) {
            return;
        }
        let mut champs = Champs::default();
        attrs.record(&mut champs);
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(Trace {
                champs: champs.0,
                debut: Instant::now(),
            });
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(&id) else { return };
        let extensions = span.extensions();
        // Absent dès que la cible n'est pas celle d'un use case : c'est le
        // filtre, et il tient en une ligne parce que `on_new_span` a déjà
        // tranché.
        let Some(trace) = extensions.get::<Trace>() else {
            return;
        };
        tracing::info!(
            target: CIBLE,
            use_case = span.metadata().target(),
            duree_ms = trace.debut.elapsed().as_millis(),
            "{}",
            trace.champs
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::layer::SubscriberExt;

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<String>>>);

    impl<S: Subscriber> Layer<S> for Capture {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            let mut champs = Champs::default();
            event.record(&mut champs);
            self.0.lock().unwrap().push(champs.0);
        }
    }

    fn lignes_produites(corps: impl FnOnce()) -> Vec<String> {
        let capture = Capture::default();
        let abonne = tracing_subscriber::registry()
            .with(UseCaseJournal)
            .with(capture.clone());
        tracing::subscriber::with_default(abonne, corps);
        let lignes = capture.0.lock().unwrap().clone();
        lignes
    }

    /// La cible d'un span est celle du module qui l'ouvre. On la force ici,
    /// parce qu'un test ne vit pas dans `use_cases/` — et que c'est
    /// précisément la cible, et rien d'autre, qui décide.
    #[test]
    fn un_span_de_use_case_produit_sa_ligne_avec_ses_champs() {
        let lignes = lignes_produites(|| {
            let span = tracing::info_span!(
                target: "kreek::app::players::use_cases::validate_customisation_use_case",
                "execute",
                cmd = "ValidateCustomisationCommand { player_id: P123 }"
            );
            let _entree = span.enter();
        });

        assert_eq!(lignes.len(), 1, "une ligne et une seule : {lignes:?}");
        assert!(
            lignes[0].contains("P123"),
            "les champs du span : {lignes:?}"
        );
        assert!(
            lignes[0].contains("validate_customisation"),
            "le nom du use case doit rester greppable : {lignes:?}"
        );
        assert!(lignes[0].contains("duree_ms"), "la durée : {lignes:?}");
    }

    /// Le point qui justifie la couche plutôt qu'un `FmtSpan` global : les
    /// spans de requête et d'app event restent exactement comme la carte 345
    /// les a laissés.
    #[test]
    fn un_span_hors_use_cases_ne_produit_aucune_ligne() {
        let lignes = lignes_produites(|| {
            let span = tracing::info_span!(
                target: "kreek::web::middleware::request_log",
                "req",
                rid = "01M0"
            );
            let _entree = span.enter();
        });

        assert!(lignes.is_empty(), "aucune ligne attendue : {lignes:?}");
    }
}

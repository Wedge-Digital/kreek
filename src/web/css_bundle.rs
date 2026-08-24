//! Les feuilles de style, réunies en un fichier unique servi depuis la mémoire.
//!
//! # Le clignotement, et pourquoi le cache n'y peut rien
//!
//! Chaque page et chaque widget transportait sa feuille dans son propre
//! fragment. Un `<link>` rencontré pendant l'analyse du HTML bloque le rendu de
//! ce qui suit ; un `<link>` **inséré dans un DOM déjà vivant** — ce que fait
//! chaque swap HTMX — ne bloque rien. Le markup est peint immédiatement, sans
//! ses styles, puis re-stylé quand la feuille arrive : 50 à 200 ms de contenu
//! nu, soit 3 à 12 images à 60 Hz.
//!
//! Aucun en-tête de cache n'accélère un fichier qui n'a **jamais** été
//! téléchargé. Le cas froid n'est réductible que par l'absence de requête.
//!
//! # Construction au démarrage, pas au build
//!
//! Le projet n'a ni `build.rs`, ni `package.json`, ni node. Le bundle se
//! construit au lancement du serveur : lecture, concaténation dans l'ordre
//! imposé, minification, empreinte du contenu. Trois conséquences —
//! **aucune cible de build à brancher**, donc rien qui puisse être oublié dans
//! `make dev` ou dans la CI ; la boucle de développement fonctionne telle
//! quelle, `cargo watch` surveillant déjà `assets/static/css` ; et le
//! cache-busting est gratuit, l'empreinte changeant exactement quand le contenu
//! change.
//!
//! # L'ordre n'est pas libre
//!
//! Dans un fichier unique, c'est l'ordre de concaténation qui arbitre les
//! égalités de spécificité. Il est donc **écrit en dur ici**, jamais tiré d'un
//! parcours de dossier dont le tri dépendrait du système de fichiers.
//!
//! La carte 341 a beaucoup réduit l'enjeu : toutes les feuilles de page et de
//! widget sont désormais scopées, donc à `(0,2,0)`, et battent les globales à
//! `(0,1,0)` quel que soit l'ordre. Il ne compte plus qu'**entre globales** —
//! `common.css`, les layouts, `components/`, et les deux feuilles de `pages/`
//! déclarées `css:global`. Le cas réel restant est `.ts-team-*`, que
//! `pages/match-report-shared.css` gagne aujourd'hui sur
//! `components/team-selection.css` : l'ordre ci-dessous le reproduit.
//!
//! # Un seul bundle, et pourquoi `auth` en est exclu
//!
//! `auth` est un **BC extractible** : l'axe 9 de `check-arch` lui interdit de
//! référencer `crate::web::`, et lui câbler le filtre du bundle créerait
//! exactement l'adhérence que ce statut proscrit.
//!
//! Il n'y perd rien. Ses pages passent par leur propre layout, en chargement
//! complet sans swap HTMX — donc sans clignotement, le phénomène que cette
//! carte supprime. Leurs trois feuilles restent chargées comme aujourd'hui,
//! bloquant le rendu comme elles le doivent.

use std::collections::HashMap;
use std::sync::OnceLock;

use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, StyleSheet};

const RACINE: &str = "assets/static/css";

/// Ordre de cascade : global, puis composants, puis pages, puis widgets.
///
/// `pages/new-competition.css` en est **absente** : sa seule règle cible
/// `.new-competition`, une classe qu'aucun template ne porte. Être liée ne
/// suffit pas à être vivante, et du CSS mort dans un fichier bloquant au rendu
/// est exactement ce que cette carte cherche à éviter.
const FEUILLES_APP: &[&str] = &[
    // Les `@font-face` d'abord : une police déclarée après son usage fonctionne,
    // mais la déclaration ouvre le fichier là où l'`@import` qu'elle remplace
    // l'ouvrait déjà.
    "fonts.css",
    "common.css",
    "layout-app.css",
    "components/competition-card.css",
    "components/competition-widget.css",
    "components/create-card.css",
    "components/kreek-select.css",
    "components/league-selector.css",
    "components/team-card.css",
    "components/team-selection.css",
    "components/tom-select.css",
    "components/upload.css",
    "pages/admin-container.css",
    "pages/allspace-home-grid.css",
    "pages/app-home.css",
    "pages/app-new-team.css",
    "pages/app-news-feed.css",
    "pages/article-container.css",
    "pages/competition-admin-dashboard.css",
    "pages/competition-admin-enrollments.css",
    "pages/competition-admin-groups.css",
    "pages/competition-admin-schedule.css",
    "pages/competition-container.css",
    "pages/competition-detail.css",
    "pages/editor-container.css",
    "pages/finalize-page.css",
    "pages/match-report-actions.css",
    "pages/match-report-inducements.css",
    "pages/match-report-pre-match.css",
    "pages/match-report-shared.css",
    "pages/match-report-step1.css",
    "pages/match-report-step5.css",
    "pages/ms-page.css",
    "pages/my-teams-container.css",
    "pages/new-competition-phase-2.css",
    "pages/new-competition-phase-3.css",
    "pages/new-competition-phase-4.css",
    "pages/new-competition-phase-5.css",
    "pages/new-space.css",
    "pages/player-debug.css",
    "pages/player-page.css",
    "pages/space-admin.css",
    "pages/team-build.css",
    "pages/team-page.css",
    "pages/widget-tester-layout.css",
    "widgets/coach-creation.css",
    "widgets/coach-search.css",
    "widgets/space-admin-candidates.css",
    "widgets/space-admin-members.css",
    "widgets/dis-page.css",
    "widgets/inducement-grid.css",
    "widgets/inducement-selector.css",
    "widgets/merco-selector.css",
    "widgets/my-teams-widget.css",
    "widgets/notification-settings.css",
    "widgets/pd-right.css",
    "widgets/players-widget.css",
    "widgets/ranking-classement-widget.css",
    "widgets/ranking-detailed-standings-widget.css",
    "widgets/rec-page.css",
    "widgets/roster-picker-widget.css",
    "widgets/roster-picker.css",
    "widgets/skill-picker.css",
    // En **dernier**, et c'est délibéré : la feuille amont de Tom Select était
    // un `<link>` posé après celui du bundle, donc elle gagnait les égalités de
    // spécificité contre tout ce qu'il contient — `components/tom-select.css`
    // compris, dont `.ts-dropdown` est à (0,1,0) comme le sien. La placer avant
    // nos surcharges serait plus logique et **changerait le rendu**, ce que la
    // carte 341 interdit : on déplace la portée, jamais le résultat. Le jour où
    // l'on veut que nos surcharges gagnent, c'est une carte qui s'assume comme
    // un changement visuel.
    "vendor/tom-select.min.css",
];

pub struct Bundle {
    pub chemin: String,
    pub contenu: String,
}

static BUNDLES: OnceLock<HashMap<&'static str, Bundle>> = OnceLock::new();

/// Lit, concatène, minifie et empreinte les deux bundles.
///
/// Appelée une fois au démarrage. Un échec de lecture ou de minification est
/// **fatal** : servir une application sans styles n'a pas de sens, et un
/// démarrage qui réussit à moitié se diagnostique plus mal qu'un démarrage qui
/// échoue.
pub fn construire() {
    let _ = tous();
}

/// Construit à la première demande.
///
/// `construire()` reste appelée explicitement au démarrage — pour échouer tôt
/// et pour mesurer — mais la paresse évite que tout ce qui rend un template
/// hors serveur ait à y penser. Le harnais de test au niveau handler (carte
/// 311) monte le vrai routeur : il est tombé sur un `panic` avant que cette
/// construction ne soit paresseuse.
fn tous() -> &'static HashMap<&'static str, Bundle> {
    BUNDLES.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("app", batir("app", FEUILLES_APP));
        m
    })
}

fn batir(nom: &str, feuilles: &[&str]) -> Bundle {
    let mut brut = String::new();
    for f in feuilles {
        let chemin = format!("{RACINE}/{f}");
        let contenu = std::fs::read_to_string(&chemin)
            .unwrap_or_else(|e| panic!("feuille de style illisible — {chemin} : {e}"));
        brut.push_str(&format!("\n/* ── {f} ── */\n"));
        brut.push_str(&contenu);
    }

    let feuille = StyleSheet::parse(&brut, ParserOptions::default())
        .unwrap_or_else(|e| panic!("bundle {nom} : CSS invalide — {e}"));
    let mut feuille = feuille;
    feuille
        .minify(MinifyOptions::default())
        .unwrap_or_else(|e| panic!("bundle {nom} : minification impossible — {e}"));
    let rendu = feuille
        .to_css(PrinterOptions {
            minify: true,
            ..Default::default()
        })
        .unwrap_or_else(|e| panic!("bundle {nom} : rendu impossible — {e}"));

    Bundle {
        chemin: format!("/css/kreek-{nom}.{}.css", empreinte(&rendu.code)),
        contenu: rendu.code,
    }
}

/// Empreinte courte du contenu — c'est elle qui autorise `immutable` sur le
/// cache : l'URL change exactement quand le contenu change.
fn empreinte(contenu: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    contenu.hash(&mut h);
    format!("{:016x}", h.finish())
}

pub fn bundle(nom: &str) -> &'static Bundle {
    tous()
        .get(nom)
        .unwrap_or_else(|| panic!("bundle inconnu : {nom}"))
}

/// Rend le bundle dont l'URL est demandée, si l'empreinte correspond.
pub fn par_chemin(chemin: &str) -> Option<&'static Bundle> {
    tous().values().find(|b| b.chemin == chemin)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construit réellement le bundle. Le test vaut recette du parsage :
    /// `lightningcss` refuse le CSS invalide, et un `panic` ici vaut mieux qu'un
    /// serveur qui démarre sans styles.
    #[test]
    fn le_bundle_se_construit() {
        let app = batir("app", FEUILLES_APP);

        assert!(
            app.contenu.len() > 100_000,
            "bundle app suspect : {} octets",
            app.contenu.len()
        );
        assert!(app.chemin.starts_with("/css/kreek-app."));

        // La minification doit gagner quelque chose, sinon elle ne sert à rien.
        let brut: usize = FEUILLES_APP
            .iter()
            .map(|f| {
                std::fs::read_to_string(format!("{RACINE}/{f}"))
                    .unwrap()
                    .len()
            })
            .sum();
        assert!(
            app.contenu.len() < brut * 9 / 10,
            "minification inopérante : {} → {} octets",
            brut,
            app.contenu.len()
        );
        println!("  app : {} → {} octets", brut, app.contenu.len());
    }

    /// Les deux positions imposées de la liste, qui ne se déduisent d'aucun tri
    /// et qu'un ajout inséré « au bon endroit alphabétique » casserait en
    /// silence : le rendu changerait sans qu'aucune compilation ne bronche.
    ///
    /// `vendor/tom-select.min.css` **en dernier** reproduit la cascade d'avant
    /// la carte 17, où elle était un `<link>` posé après celui du bundle et
    /// gagnait donc les égalités de spécificité contre `components/`.
    /// `fonts.css` **en premier** met les `@font-face` là où se trouvait
    /// l'`@import` qu'ils remplacent.
    #[test]
    fn l_ordre_impose_de_la_liste_est_tenu() {
        assert_eq!(
            FEUILLES_APP.first(),
            Some(&"fonts.css"),
            "les @font-face ouvrent le bundle"
        );
        assert_eq!(
            FEUILLES_APP.last(),
            Some(&"vendor/tom-select.min.css"),
            "la feuille amont de Tom Select ferme le bundle"
        );
        let vendor = FEUILLES_APP.iter().position(|f| f.starts_with("vendor/"));
        let surcouche = FEUILLES_APP
            .iter()
            .position(|f| *f == "components/tom-select.css");
        assert!(
            vendor > surcouche,
            "nos surcharges Tom Select doivent précéder la feuille amont"
        );
    }

    /// L'empreinte doit suivre le contenu, sinon `immutable` sur le cache est un
    /// piège : une feuille modifiée continuerait d'être servie depuis le cache
    /// des navigateurs.
    #[test]
    fn l_empreinte_suit_le_contenu() {
        assert_eq!(empreinte("a"), empreinte("a"));
        assert_ne!(empreinte("a"), empreinte("b"));
    }
}

/// Sert un bundle depuis la mémoire.
///
/// `immutable` est posé sans réserve : l'URL porte l'empreinte du contenu, donc
/// une modification produit une autre URL. C'est ce que la construction au
/// démarrage achète — sans empreinte, ce cache-là serait un piège.
pub async fn servir(
    axum::extract::Path(fichier): axum::extract::Path<String>,
) -> axum::response::Response {
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;

    match par_chemin(&format!("/css/{fichier}")) {
        Some(b) => (
            [
                (header::CONTENT_TYPE, "text/css; charset=utf-8"),
                (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
            ],
            b.contenu.as_str(),
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Le chemin du bundle applicatif, pour les templates.
///
/// Appelée directement depuis Askama par son chemin complet —
/// `{{ crate::web::css_bundle::chemin_app() }}` — plutôt que par un filtre.
///
/// Un filtre aurait paru plus élégant, mais Askama le compile **dans le module
/// de la struct**, et le `<link>` vit dans `app-layout.html`, étendu par une
/// soixantaine de templates : il aurait fallu équiper autant de modules d'un
/// `use crate::filters;`. Le chemin complet ne demande rien.
pub fn chemin_app() -> &'static str {
    &bundle("app").chemin
}

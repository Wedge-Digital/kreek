//! L'aperçu d'impression (carte 596) — et ce qui sort de l'imprimante.
//!
//! La page « Imprimer » est sa propre vue d'impression : pas d'iframe. Ce
//! fragment rend les feuilles A4 ; chaque feuille pose des emplacements que les
//! fragments des BCs remplissent (`hx-trigger="load"`). À l'écran le script de
//! la page réduit les feuilles ; à l'impression le CSS les remet à taille
//! réelle et masque tout le reste.
//!
//! **Les options sont des types fermés.** Une valeur inconnue fait échouer
//! l'extracteur `Query` — 400 — au lieu de se replier en silence sur un défaut.
//! Chaque BC déclare les siens : pas de type partagé pour un paramètre d'URL.

use crate::app::routes::AppRoutes;
use crate::app::teams::domain::team::Team;
use crate::app::teams::use_cases::team_print_service::{self, TeamPrintError};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

// ── Options ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    #[default]
    Normal,
    Compact,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Ink {
    #[default]
    Color,
    Mono,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SheetStyle {
    #[default]
    Plain,
    Official,
    Pitch,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UnavailableMode {
    Show,
    #[default]
    Strike,
    Hide,
}

impl Orientation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Portrait => "portrait",
            Self::Landscape => "landscape",
        }
    }
}

impl Density {
    fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Compact => "compact",
        }
    }
}

impl Ink {
    fn as_str(self) -> &'static str {
        match self {
            Self::Color => "color",
            Self::Mono => "mono",
        }
    }
}

impl SheetStyle {
    fn as_str(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Official => "official",
            Self::Pitch => "pitch",
        }
    }
}

impl UnavailableMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Strike => "strike",
            Self::Hide => "hide",
        }
    }
}

/// Les options du formulaire de la page. Une case décochée est absente de la
/// requête : `false` par défaut.
#[derive(Debug, Default, Deserialize)]
pub struct PrintOptionsQuery {
    #[serde(default)]
    pub orientation: Orientation,
    #[serde(default)]
    pub density: Density,
    #[serde(default)]
    pub ink: Ink,
    #[serde(default)]
    pub style: SheetStyle,
    #[serde(default)]
    pub unavailable: UnavailableMode,
    #[serde(default)]
    pub blank_rows: bool,
    #[serde(default)]
    pub skills_annex: bool,
    #[serde(default)]
    pub inducements: bool,
    #[serde(default)]
    pub star_players: bool,
    #[serde(default)]
    pub match_sheet: bool,
    #[serde(default)]
    pub two_copies: bool,
    /// Qui reçoit (R10) : l'option « prochain adversaire » le sait, une autre
    /// équipe non — mon équipe reçoit alors.
    #[serde(default)]
    pub home: HomeSide,
    /// La journée du prochain match, pour l'en-tête de la feuille de match.
    #[serde(default)]
    pub round_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HomeSide {
    #[default]
    Own,
    Other,
}

impl HomeSide {
    fn as_str(self) -> &'static str {
        match self {
            Self::Own => "own",
            Self::Other => "other",
        }
    }
}

impl PrintOptionsQuery {
    /// Les classes de la racine : le CSS en tire format, densité, encre, style.
    pub fn root_class(&self) -> String {
        format!(
            "orientation-{} density-{} ink-{} style-{}",
            self.orientation.as_str(),
            self.density.as_str(),
            self.ink.as_str(),
            self.style.as_str()
        )
    }

    /// Ce que le tableau des joueurs reçoit — le sous-ensemble qui le concerne.
    fn players_query(&self) -> String {
        format!(
            "unavailable={}&blank_rows={}",
            self.unavailable.as_str(),
            self.blank_rows
        )
    }

    fn layout_query(&self) -> String {
        format!(
            "orientation={}&density={}",
            self.orientation.as_str(),
            self.density.as_str()
        )
    }

    fn inducements_query(&self) -> String {
        format!(
            "inducements={}&star_players={}&{}",
            self.inducements,
            self.star_players,
            self.layout_query()
        )
    }

    /// La journée ne passe que si c'est un identifiant : une valeur forgée ne
    /// doit rien injecter dans l'URL du fragment. `match_report` la vérifie
    /// encore contre la saison de l'équipe.
    fn match_sheet_query(&self, copy: Option<u8>) -> String {
        let mut query = format!("home={}", self.home.as_str());
        if let Some(round) = self
            .round_id
            .as_deref()
            .filter(|r| r.chars().all(|c| c.is_ascii_alphanumeric()))
        {
            query.push_str(&format!("&round_id={round}"));
        }
        if let Some(n) = copy {
            query.push_str(&format!("&copy={n}"));
        }
        query
    }

    /// Le rôle de la fiche — « Mon équipe » ou « Adversaire ». La trésorerie,
    /// déjà publique sur la fiche d'équipe, s'imprime sur toutes (R8).
    fn head_query(&self, role: SheetRole) -> String {
        format!("role={}", role.as_str())
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SheetRole {
    #[default]
    Own,
    Other,
}

impl SheetRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Own => "own",
            Self::Other => "other",
        }
    }
}

// ── View models ───────────────────────────────────────────────────────────────

/// Une feuille de l'aperçu : son libellé, et les fragments qui la remplissent,
/// dans l'ordre.
/// Un emplacement de l'aperçu.
///
/// La fiche d'une équipe est une feuille que **l'hôte** ouvre, et que deux
/// fragments remplissent. Les autres fragments — annexes, feuille de match —
/// rendent **eux-mêmes** leurs feuilles, une ou plusieurs : seul leur BC sait
/// combien il en faut. L'hôte ne pose alors que l'emplacement.
pub struct PrintSlotVm {
    pub label: String,
    pub fragment_urls: Vec<String>,
    pub wraps_sheet: bool,
}

pub struct PrintPreviewVm {
    pub root_class: String,
    pub slots: Vec<PrintSlotVm>,
}

impl PrintPreviewVm {
    pub fn build(
        space_id: &str,
        own: &Team,
        other: Option<&Team>,
        opts: &PrintOptionsQuery,
    ) -> Self {
        let mut slots = vec![team_sheet_slot(space_id, own, SheetRole::Own, opts)];
        if let Some(other) = other {
            slots.push(team_sheet_slot(space_id, other, SheetRole::Other, opts));
        }
        let ids = TeamPair {
            space_id,
            own: own.id.to_string(),
            other: other.map(|t| t.id.to_string()),
        };
        slots.extend(extra_slots(&ids, opts));
        Self {
            root_class: opts.root_class(),
            slots,
        }
    }
}

/// La fiche d'une équipe : l'en-tête (teams), puis les joueurs (players).
fn team_sheet_slot(
    space_id: &str,
    team: &Team,
    role: SheetRole,
    opts: &PrintOptionsQuery,
) -> PrintSlotVm {
    let routes = AppRoutes::default();
    let team_id = team.id.to_string();
    let head = routes.teams.team_print_sheet_head(space_id, &team_id);
    let players = routes.players.players_by_team_print(space_id, &team_id);
    PrintSlotVm {
        label: format!("Fiche · {}", team.name),
        fragment_urls: vec![
            format!("{head}?{}", opts.head_query(role)),
            format!("{players}?{}", opts.players_query()),
        ],
        wraps_sheet: true,
    }
}

/// Les équipes d'une impression, telles que les URLs des fragments les portent.
struct TeamPair<'a> {
    space_id: &'a str,
    own: String,
    other: Option<String>,
}

/// Les pages en plus, dans l'ordre de la maquette : compétences, coups de
/// pouce et star players, feuille de match.
fn extra_slots(ids: &TeamPair<'_>, opts: &PrintOptionsQuery) -> Vec<PrintSlotVm> {
    let routes = AppRoutes::default();
    let other = ids.other.as_deref();
    let mut slots = Vec::new();
    if opts.skills_annex {
        let base = match other {
            Some(o) => routes.players.print_skills_with(ids.space_id, &ids.own, o),
            None => routes.players.print_skills(ids.space_id, &ids.own),
        };
        slots.push(fragment_slot(
            "Compétences",
            format!("{base}?{}", opts.layout_query()),
        ));
    }
    if opts.inducements || opts.star_players {
        let base = routes
            .match_report
            .print_inducements(ids.space_id, &ids.own, other);
        slots.push(fragment_slot(
            "Coups de pouce",
            format!("{base}?{}", opts.inducements_query()),
        ));
    }
    if opts.match_sheet {
        slots.extend(match_sheet_slots(ids, opts));
    }
    slots
}

fn match_sheet_slots(ids: &TeamPair<'_>, opts: &PrintOptionsQuery) -> Vec<PrintSlotVm> {
    let base = AppRoutes::default().match_report.print_match_sheet(
        ids.space_id,
        &ids.own,
        ids.other.as_deref(),
    );
    let copies: Vec<Option<u8>> = if opts.two_copies {
        vec![Some(1), Some(2)]
    } else {
        vec![None]
    };
    copies
        .into_iter()
        .map(|copy| {
            fragment_slot(
                "Feuille de match",
                format!("{base}?{}", opts.match_sheet_query(copy)),
            )
        })
        .collect()
}

fn fragment_slot(label: &str, url: String) -> PrintSlotVm {
    PrintSlotVm {
        label: label.to_string(),
        fragment_urls: vec![url],
        wraps_sheet: false,
    }
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "widgets/teams-print-preview.html")]
pub struct PrintPreviewTemplate {
    pub vm: PrintPreviewVm,
}

impl IntoResponse for PrintPreviewTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("teams print preview render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// L'aperçu de mon équipe seule.
pub async fn get_print_preview(
    Path((space_id, team_id)): Path<(String, String)>,
    Query(opts): Query<PrintOptionsQuery>,
    State(state): State<AppState>,
) -> Response {
    let repo = state.teams.team_repository.as_ref();
    match team_print_service::load_printable(repo, &team_id).await {
        Ok(own) => PrintPreviewTemplate {
            vm: PrintPreviewVm::build(&space_id, &own, None, &opts),
        }
        .into_response(),
        Err(e) => refus(e),
    }
}

/// L'aperçu à deux équipes. `{other_team_id}` est contrôlé par `space_scope`
/// (carte 594) : une équipe d'un autre espace n'arrive jamais ici.
///
/// La seconde équipe n'a pas à être prête à jouer : un adversaire en
/// recrutement s'imprime tel qu'il est.
pub async fn get_print_preview_with(
    Path((space_id, team_id, other_team_id)): Path<(String, String, String)>,
    Query(opts): Query<PrintOptionsQuery>,
    State(state): State<AppState>,
) -> Response {
    let repo = state.teams.team_repository.as_ref();
    let own = match team_print_service::load_printable(repo, &team_id).await {
        Ok(t) => t,
        Err(e) => return refus(e),
    };
    match team_print_service::load(repo, &other_team_id).await {
        Ok(other) => PrintPreviewTemplate {
            vm: PrintPreviewVm::build(&space_id, &own, Some(&other), &opts),
        }
        .into_response(),
        Err(e) => refus(e),
    }
}

fn refus(e: TeamPrintError) -> Response {
    match e {
        TeamPrintError::TeamNotFound | TeamPrintError::NotPrintable { .. } => {
            StatusCode::NOT_FOUND.into_response()
        }
        TeamPrintError::Repository(cause) => {
            tracing::error!("teams print preview: {cause}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Par l'extracteur même du handler : c'est lui qui répond 400.
    fn options(query: &str) -> Result<PrintOptionsQuery, ()> {
        let uri: axum::http::Uri = format!("/print/preview?{query}").parse().unwrap();
        Query::<PrintOptionsQuery>::try_from_uri(&uri)
            .map(|Query(o)| o)
            .map_err(|_| ())
    }

    #[test]
    fn les_options_absentes_prennent_leur_defaut() {
        let opts = options("").unwrap();
        assert_eq!(
            opts.root_class(),
            "orientation-portrait density-normal ink-color style-plain"
        );
        assert_eq!(opts.unavailable, UnavailableMode::Strike);
        assert!(!opts.blank_rows);
    }

    #[test]
    fn une_valeur_inconnue_est_refusee_et_non_repliee() {
        assert!(options("orientation=diagonale").is_err());
        assert!(options("style=flashy").is_err());
    }

    #[test]
    fn les_options_choisies_composent_la_classe_racine() {
        let opts =
            options("orientation=landscape&density=compact&ink=mono&style=official").unwrap();
        assert_eq!(
            opts.root_class(),
            "orientation-landscape density-compact ink-mono style-official"
        );
    }

    #[test]
    fn la_fiche_ne_recoit_que_son_role() {
        let opts = options("").unwrap();
        assert_eq!(opts.head_query(SheetRole::Own), "role=own");
        assert_eq!(opts.head_query(SheetRole::Other), "role=other");
    }

    #[test]
    fn la_feuille_de_match_porte_le_camp_qui_recoit_et_la_journee() {
        let opts = options("home=other&round_id=01JABCDEF").unwrap();
        assert_eq!(
            opts.match_sheet_query(Some(2)),
            "home=other&round_id=01JABCDEF&copy=2"
        );
    }

    /// Une journée forgée n'injecte rien dans l'URL du fragment.
    #[test]
    fn une_journee_qui_n_est_pas_un_identifiant_est_ecartee() {
        let opts = options("round_id=x%26home%3Dother").unwrap();
        assert_eq!(opts.match_sheet_query(None), "home=own");
    }

    #[test]
    fn les_coups_de_pouce_recoivent_leurs_options_et_la_mise_en_page() {
        let opts = options("star_players=true&orientation=landscape").unwrap();
        assert_eq!(
            opts.inducements_query(),
            "inducements=false&star_players=true&orientation=landscape&density=normal"
        );
    }

    #[test]
    fn le_tableau_des_joueurs_recoit_ses_seules_options() {
        let opts = options("unavailable=hide&blank_rows=true&density=compact&ink=mono").unwrap();
        assert_eq!(opts.players_query(), "unavailable=hide&blank_rows=true");
    }
}

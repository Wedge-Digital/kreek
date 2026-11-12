//! La mise en forme des notifications de match (carte 645, épic E23).
//!
//! Le BC dit ce qui s'est passé, l'hôte dit comment l'annoncer : le titre, le
//! sous-titre et le lien sont composés ici, depuis les seuls faits de l'app
//! event. Rien n'est relu ailleurs.
//!
//! # Ce que la décision 7 fait toaster
//!
//! Le démarrage, une action (MVP compris), la sélection des coups de pouce
//! validée — avec sa liste —, leur annulation. Le reste ne fait qu'un signal
//! (646), ou rien. L'aiguillage de `notice_of` est **sans joker** : un variant
//! ajouté à `MatchReportAppEvent` casse la compilation ici, et son auteur
//! tranche.
//!
//! # Un nom absent n'est pas remplacé
//!
//! Les `*_name` de la 643 sont des `Option` : l'identité ou `teams` ont pu ne
//! pas connaître le nom. La clause qui le porterait est alors **retirée** —
//! « Saisie du match démarrée », sans « par » —, jamais remplie d'un « un
//! coach » qui ne se distinguerait pas d'un vrai nom.
//!
//! # Pourquoi « Touchdown : Griff #9 (London Rats) — par Morgane »
//!
//! Le joueur arrive tel que `match_report` l'affiche (`Griff #9`) ; en tirer le
//! seul numéro serait recomposer une valeur. La forme à deux-points évite aussi
//! l'élision (« de Orc ») et l'accord du participe selon l'action.

use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::app_events::match_report_app_events::{
    ActionTypePayload, InducementItemPayload, InducementKindPayload, MatchReportAppEvent,
};
use crate::web::realtime::message::ToastPayload;

/// Le sous-titre de tout toast de match : c'est lui qui porte le lien.
pub const SEE_MATCH: &str = "Voir le match";

/// Les seuls `event_type` qui font un toast : le routeur n'appelle son port
/// pour rien d'autre, et ne journalise l'échec de lecture que de ceux-là.
pub const TOAST_EVENT_TYPES: [&str; 4] = [
    MatchReportAppEvent::MATCH_REPORT_CONFIRMED,
    MatchReportAppEvent::MATCH_ACTION_RECORDED,
    MatchReportAppEvent::MATCH_INDUCEMENTS_RECORDED,
    MatchReportAppEvent::MATCH_INDUCEMENTS_CANCELLED,
];

/// Un toast prêt à router : à qui il s'adresse (le match), qui l'a causé
/// (exclu des destinataires), et ce qu'il dit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchNotice {
    pub match_report_id: String,
    pub space_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
    pub author: String,
    pub toast: ToastPayload,
}

/// Le match d'un fait, tel que les quatre variants le portent.
struct MatchScope<'a> {
    match_report_id: &'a str,
    space_id: &'a str,
    home_team_id: &'a str,
    away_team_id: &'a str,
}

impl MatchScope<'_> {
    fn notice(&self, author: &str, title: String) -> MatchNotice {
        MatchNotice {
            match_report_id: self.match_report_id.to_string(),
            space_id: self.space_id.to_string(),
            home_team_id: self.home_team_id.to_string(),
            away_team_id: self.away_team_id.to_string(),
            author: author.to_string(),
            toast: ToastPayload {
                title,
                subtitle: Some(SEE_MATCH.to_string()),
                link: Some(match_link(self.space_id, self.match_report_id)),
            },
        }
    }
}

/// Le toast d'un fait, ou `None` s'il n'en fait pas (décision 7).
pub fn notice_of(event: &MatchReportAppEvent) -> Option<MatchNotice> {
    use MatchReportAppEvent as E;
    match event {
        E::MatchReportConfirmed { .. } => started_notice(event),
        E::MatchActionRecorded { .. } => action_notice(event),
        E::MatchInducementsRecorded { .. } => inducements_notice(event),
        E::MatchInducementsCancelled { .. } => cancelled_notice(event),
        // Le retrait d'une action ne fait qu'un signal (décision 7) ; les
        // autres faits n'en font même pas un ici.
        E::MatchActionDeleted { .. }
        | E::MatchReportCancelled { .. }
        | E::JourneymenFielded { .. }
        | E::JourneymenWithdrawn { .. }
        | E::MatchReportPublished(_)
        | E::MatchReportUnpublished(_)
        | E::MatchReportRoundReassigned { .. }
        // Le fan factor d'un camp : un signal, pas de toast (§ 12.7 de E22).
        | E::MatchFanFactorRecorded { .. } => None,
    }
}

/// Le lien de tout toast de match : la page du direct.
pub fn match_link(space_id: &str, match_report_id: &str) -> String {
    AppRoutes::default()
        .match_report
        .live(space_id, match_report_id)
}

fn started_notice(event: &MatchReportAppEvent) -> Option<MatchNotice> {
    let MatchReportAppEvent::MatchReportConfirmed {
        match_report_id,
        space_id,
        home_team_id,
        away_team_id,
        confirmed_by,
        confirmed_by_name,
        ..
    } = event
    else {
        return None;
    };
    let scope = MatchScope {
        match_report_id,
        space_id,
        home_team_id,
        away_team_id,
    };
    let title = format!(
        "Saisie du match démarrée{}",
        by(confirmed_by_name.as_deref())
    );
    Some(scope.notice(confirmed_by, title))
}

fn action_notice(event: &MatchReportAppEvent) -> Option<MatchNotice> {
    let MatchReportAppEvent::MatchActionRecorded {
        match_report_id,
        space_id,
        home_team_id,
        away_team_id,
        team_name,
        action,
        player_display_name,
        recorded_by,
        recorded_by_name,
        ..
    } = event
    else {
        return None;
    };
    let scope = MatchScope {
        match_report_id,
        space_id,
        home_team_id,
        away_team_id,
    };
    let title = action_title(
        action,
        player_display_name,
        team_name.as_deref(),
        recorded_by_name.as_deref(),
    );
    Some(scope.notice(recorded_by, title))
}

fn inducements_notice(event: &MatchReportAppEvent) -> Option<MatchNotice> {
    let MatchReportAppEvent::MatchInducementsRecorded {
        match_report_id,
        space_id,
        home_team_id,
        away_team_id,
        team_name,
        recorded_by,
        recorded_by_name,
        items,
        ..
    } = event
    else {
        return None;
    };
    let scope = MatchScope {
        match_report_id,
        space_id,
        home_team_id,
        away_team_id,
    };
    let title = inducements_title(team_name.as_deref(), items, recorded_by_name.as_deref());
    Some(scope.notice(recorded_by, title))
}

fn cancelled_notice(event: &MatchReportAppEvent) -> Option<MatchNotice> {
    let MatchReportAppEvent::MatchInducementsCancelled {
        match_report_id,
        space_id,
        home_team_id,
        away_team_id,
        cancelled_by,
        cancelled_by_name,
        ..
    } = event
    else {
        return None;
    };
    let scope = MatchScope {
        match_report_id,
        space_id,
        home_team_id,
        away_team_id,
    };
    let title = format!("Coups de pouce annulés{}", by(cancelled_by_name.as_deref()));
    Some(scope.notice(cancelled_by, title))
}

/// « Touchdown : Griff #9 (London Rats) — par Morgane ».
fn action_title(
    action: &ActionTypePayload,
    player: &str,
    team_name: Option<&str>,
    author: Option<&str>,
) -> String {
    let team = team_name.map(|t| format!(" ({t})")).unwrap_or_default();
    let author = author.map(|a| format!(" — par {a}")).unwrap_or_default();
    format!("{} : {player}{team}{author}", action_label(action))
}

/// « London Rats : Relance d'équipe ×2, Pot-de-vin — validé par Morgane ».
fn inducements_title(
    team_name: Option<&str>,
    items: &[InducementItemPayload],
    author: Option<&str>,
) -> String {
    let subject = team_name.unwrap_or("Coups de pouce");
    let list = if items.is_empty() {
        "aucun coup de pouce".to_string()
    } else {
        items.iter().map(item_label).collect::<Vec<_>>().join(", ")
    };
    format!("{subject} : {list} — validé{}", by(author))
}

/// Les libellés du journal de saisie (`action_log_widget.rs`).
fn action_label(action: &ActionTypePayload) -> &'static str {
    match action {
        ActionTypePayload::Touchdown => "Touchdown",
        ActionTypePayload::Passe => "Passe",
        ActionTypePayload::Interception => "Interception",
        ActionTypePayload::Agression => "Agression",
        ActionTypePayload::Lancer => "Lancer",
        ActionTypePayload::Sortie => "Sortie",
        ActionTypePayload::Mvp => "MVP",
        ActionTypePayload::Blesse { .. } => "Blessure",
    }
}

/// Le libellé résolu par le publisher, la nature d'un mercenaire telle que le
/// récapitulatif la nomme (`view_models.rs`), la quantité seulement au-delà
/// d'un — sans pluriel fabriqué.
fn item_label(item: &InducementItemPayload) -> String {
    let kind = match item.kind {
        InducementKindPayload::Mercenary { skilled: true } => " (Mercenaire Niv. 1)",
        InducementKindPayload::Mercenary { skilled: false } => " (Mercenaire)",
        InducementKindPayload::Inducement | InducementKindPayload::StarPlayer => "",
    };
    let qty = if item.qty > 1 {
        format!(" ×{}", item.qty)
    } else {
        String::new()
    };
    format!("{}{kind}{qty}", item.label)
}

/// « par Morgane », ou rien si le nom est inconnu.
fn by(author: Option<&str>) -> String {
    author.map(|a| format!(" par {a}")).unwrap_or_default()
}

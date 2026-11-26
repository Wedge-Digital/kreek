//! « Mes matchs » demande à `match_report` le bouton de chaque prochain match
//! (carte 677).
//!
//! **Seul fichier de la chaîne à importer le BC source.** `competitions` ne
//! connaît que `IMatchEntryPort`, `MatchEntryLinkDto` et `MatchEntryKind`.
//!
//! # La règle est celle du BC, appelée et non réécrite
//!
//! L'action vient de `entry_actions_for_pairings` (carte 676), qui s'appuie
//! sur `can_write_report`. Cet adapter n'y ajoute que l'URL, construite par
//! `AppRoutes` — `match_report` ne construit pas d'URL pour un autre BC, et
//! `competitions` ne connaît pas les routes de saisie :
//!
//! - « Commencer » et « Continuer » → `from_pairing`, qui mène à « Qui
//!   saisit ? » ou à l'étape en cours ;
//! - « Suivre » → `live`, la lecture seule du rapport.

use crate::app::auth::domain::user::User;
use crate::app::competitions::domain::match_entry::MatchEntryKind;
use crate::app::competitions::ports::{IMatchEntryPort, MatchEntryLinkDto};
use crate::app::match_report::domain::entry_action::EntryAction;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::ports::ITeamDataPort;
use crate::app::match_report::use_cases::match_entry_action_service::{
    entry_actions_for_pairings, PairingEntryAction,
};
use crate::app::match_report::use_cases::match_report_access_service::AccesRapportDeps;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::admin_access::IAdminAccessPort;
use async_trait::async_trait;
use std::sync::Arc;

pub struct MatchEntryAdapter {
    repo: Arc<dyn IMatchReportRepository>,
    admin_access: Arc<dyn IAdminAccessPort>,
    team_data: Arc<dyn ITeamDataPort>,
}

impl MatchEntryAdapter {
    pub fn new(
        repo: Arc<dyn IMatchReportRepository>,
        admin_access: Arc<dyn IAdminAccessPort>,
        team_data: Arc<dyn ITeamDataPort>,
    ) -> Self {
        Self {
            repo,
            admin_access,
            team_data,
        }
    }

    fn deps(&self) -> AccesRapportDeps<'_> {
        AccesRapportDeps {
            admin_access: self.admin_access.as_ref(),
            team_data: self.team_data.as_ref(),
        }
    }
}

/// Le bouton d'un appariement et son URL ; `None` quand `match_report` n'en
/// propose aucun — rapport clos, ou coach étranger au match.
pub fn to_link(found: PairingEntryAction, space_id: &str) -> Option<MatchEntryLinkDto> {
    let action = found.action?;
    let url = entry_url(action, space_id, &found.pairing_id, &found.match_report_id);
    Some(MatchEntryLinkDto {
        pairing_id: found.pairing_id,
        kind: entry_kind(action),
        url,
    })
}

/// La traduction d'une action de `match_report` dans la langue de ce BC.
pub fn entry_kind(action: EntryAction) -> MatchEntryKind {
    match action {
        EntryAction::Start => MatchEntryKind::Start,
        EntryAction::Continue => MatchEntryKind::Continue,
        EntryAction::Follow => MatchEntryKind::Follow,
    }
}

/// Où mène le bouton : la saisie par l'appariement, ou le suivi en direct.
pub fn entry_url(
    action: EntryAction,
    space_id: &str,
    pairing_id: &str,
    match_report_id: &str,
) -> String {
    let routes = AppRoutes::default().match_report;
    match action {
        EntryAction::Start | EntryAction::Continue => routes.from_pairing(space_id, pairing_id),
        EntryAction::Follow => routes.live(space_id, match_report_id),
    }
}

#[async_trait]
impl IMatchEntryPort for MatchEntryAdapter {
    async fn find_entry_links(
        &self,
        user: &User,
        space_id: &str,
        pairing_ids: &[String],
    ) -> Result<Vec<MatchEntryLinkDto>, String> {
        let found = entry_actions_for_pairings(
            self.repo.as_ref(),
            &self.deps(),
            user,
            space_id,
            pairing_ids,
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok(found
            .into_iter()
            .filter_map(|f| to_link(f, space_id))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::match_report_repository_port::PairingEntryRow;
    use crate::app::match_report::domain::value_objects::EntryMode;
    use crate::app::match_report::use_cases::test_doubles::{NamedTeams, PairingEntryRepository};
    use crate::app::shared_kernel::bloodbowl::admin_access::FakeAdminAccess;
    use crate::app::shared_kernel::identity::coach_name::CoachName;
    use crate::app::shared_kernel::identity::email::Email;
    use crate::app::shared_kernel::identity::ids::UserId;

    const SPACE: &str = "00000000000000000000000071";
    const COMPETITION: &str = "00000000000000000000000072";
    const HOME_COACH: &str = "00000000000000000000000073";
    const AWAY_COACH: &str = "00000000000000000000000074";
    const HOME: &str = "home-team";
    const AWAY: &str = "away-team";

    fn user(id: &str) -> User {
        User::new(
            UserId::try_new(id).unwrap(),
            CoachName::try_new("Testeur".to_string()).unwrap(),
            None,
            Email::try_new("testeur@example.com".to_string()).unwrap(),
            String::new(),
        )
    }

    fn row(pairing: &str, phase: &str, starter: Option<&str>) -> PairingEntryRow {
        PairingEntryRow {
            pairing_id: pairing.to_string(),
            match_report_id: format!("report-{pairing}"),
            competition_id: COMPETITION.to_string(),
            home_team_id: HOME.to_string(),
            away_team_id: AWAY.to_string(),
            phase: phase.to_string(),
            entry_mode: EntryMode::SoloEntry.as_str().to_string(),
            started_by: starter.map(str::to_string),
        }
    }

    fn adapter(rows: Vec<PairingEntryRow>) -> MatchEntryAdapter {
        let teams = NamedTeams::default()
            .coached(HOME, HOME_COACH)
            .coached(AWAY, AWAY_COACH);
        MatchEntryAdapter::new(
            Arc::new(PairingEntryRepository::holding(rows)),
            Arc::new(FakeAdminAccess::new()),
            Arc::new(teams),
        )
    }

    fn action(pairing: &str, action: Option<EntryAction>) -> PairingEntryAction {
        PairingEntryAction {
            pairing_id: pairing.into(),
            match_report_id: format!("report-{pairing}"),
            action,
        }
    }

    #[test]
    fn start_and_continue_lead_to_the_entry_by_pairing() {
        let routes = AppRoutes::default().match_report;
        let from_pairing = routes.from_pairing(SPACE, "p1");
        assert_eq!(
            entry_url(EntryAction::Start, SPACE, "p1", "r1"),
            from_pairing
        );
        assert_eq!(
            entry_url(EntryAction::Continue, SPACE, "p1", "r1"),
            from_pairing
        );
    }

    #[test]
    fn follow_leads_to_the_live_report() {
        let live = AppRoutes::default().match_report.live(SPACE, "r1");
        assert_eq!(entry_url(EntryAction::Follow, SPACE, "p1", "r1"), live);
    }

    #[test]
    fn each_action_keeps_its_kind() {
        assert_eq!(entry_kind(EntryAction::Start), MatchEntryKind::Start);
        assert_eq!(entry_kind(EntryAction::Continue), MatchEntryKind::Continue);
        assert_eq!(entry_kind(EntryAction::Follow), MatchEntryKind::Follow);
    }

    #[test]
    fn no_action_gives_no_link() {
        assert_eq!(to_link(action("p1", None), SPACE), None);
    }

    #[test]
    fn a_follow_link_points_to_its_report() {
        let link = to_link(action("p1", Some(EntryAction::Follow)), SPACE).unwrap();
        assert_eq!(link.pairing_id, "p1");
        assert_eq!(link.kind, MatchEntryKind::Follow);
        assert!(link.url.contains("report-p1"), "{}", link.url);
    }

    /// Un brouillon, un match commencé par l'adversaire en « Je saisis tout »,
    /// un rapport publié — vus par le coach de l'équipe extérieure.
    async fn links_of_the_away_coach() -> Vec<MatchEntryLinkDto> {
        let rows = vec![
            row("p-draft", "Draft", None),
            row("p-other", "PreMatch", Some(HOME_COACH)),
            row("p-done", "Published", Some(HOME_COACH)),
        ];
        let ids: Vec<String> = rows.iter().map(|r| r.pairing_id.clone()).collect();
        adapter(rows)
            .find_entry_links(&user(AWAY_COACH), SPACE, &ids)
            .await
            .unwrap()
    }

    /// Par le vrai service de 676, sur une doublure du dépôt : chaque état
    /// donne son bouton, un rapport publié n'en donne pas.
    #[tokio::test]
    async fn the_adapter_asks_match_report_and_builds_the_urls() {
        let links = links_of_the_away_coach().await;
        let kinds: Vec<_> = links
            .iter()
            .map(|l| (l.pairing_id.as_str(), l.kind))
            .collect();
        assert_eq!(
            kinds,
            vec![
                ("p-draft", MatchEntryKind::Start),
                ("p-other", MatchEntryKind::Follow)
            ]
        );
        let follow = AppRoutes::default()
            .match_report
            .live(SPACE, "report-p-other");
        assert_eq!(links[1].url, follow);
    }

    #[tokio::test]
    async fn a_coach_foreign_to_the_match_gets_no_link() {
        let links = adapter(vec![row("p1", "Draft", None)])
            .find_entry_links(&user("00000000000000000000000079"), SPACE, &["p1".into()])
            .await
            .unwrap();
        assert!(links.is_empty());
    }
}

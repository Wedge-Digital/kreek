//! L'action de saisie de chaque match d'un coach (carte 676, I7-E1-KR-T2).
//!
//! **Service de lecture public du BC**, appelé par l'adapter qu'une autre carte
//! posera dans `src/infrastructure/` pour la page « Mes matchs » (carte 677).
//! Le BC ne connaît pas son consommateur et ne construit aucune URL : il rend,
//! pour chaque appariement, l'identifiant du rapport et l'action.
//!
//! La règle d'écriture n'est pas réécrite : elle reste `can_write_report`. Ce
//! service n'ajoute qu'une chose, que la règle d'écriture ne dit pas : un
//! coach qui n'entraîne aucune des deux équipes n'a **aucune** action, même
//! admin — « Mes matchs » ne montre que ses équipes (I7-B1).

use crate::app::auth::domain::user::User;
use crate::app::match_report::domain::entry_action::{CoachStanding, EntryAction, EntryProgress};
use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, PairingEntryRow, RepositoryError,
};
use crate::app::match_report::use_cases::match_report_access_service::{
    camp_du_coach, can_write_report, AccesRapportDeps, CampDuCoach, EntryScope, PorteeRapport,
};
use futures_util::future::join_all;

/// L'action d'un coach sur le rapport d'un appariement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingEntryAction {
    pub pairing_id: String,
    pub match_report_id: String,
    /// `None` : publié, annulé, ou coach étranger au match — pas de bouton.
    pub action: Option<EntryAction>,
}

/// Pour chaque appariement de cet espace qui a un rapport non annulé, son
/// identifiant et l'action de ce coach.
///
/// - un appariement **sans rapport** n'a pas de ligne dans la réponse ;
/// - l'ordre de la réponse n'est pas celui de `pairing_ids` : l'appelant
///   apparie par `pairing_id` ;
/// - une erreur de lecture de la projection est rendue telle quelle ; une
///   erreur de port d'équipe ou d'admin, elle, refuse comme partout dans
///   `match_report_access_service` (un contrôle d'accès échoue fermé).
// arch:no-instrument — service de lecture : une vue pour « Mes matchs », sans intention métier
pub async fn entry_actions_for_pairings(
    repo: &dyn IMatchReportRepository,
    deps: &AccesRapportDeps<'_>,
    user: &User,
    space_id: &str,
    pairing_ids: &[String],
) -> Result<Vec<PairingEntryAction>, RepositoryError> {
    if pairing_ids.is_empty() {
        return Ok(vec![]);
    }
    let rows = repo
        .find_entry_states_by_pairings(space_id, pairing_ids)
        .await?;
    let actions = rows
        .iter()
        .map(|row| entry_action_for(deps, user, space_id, row));
    Ok(join_all(actions).await)
}

/// L'action de ce coach sur ce rapport. Un rapport clos ne consulte aucun
/// port : il n'offre rien, à personne.
async fn entry_action_for(
    deps: &AccesRapportDeps<'_>,
    user: &User,
    space_id: &str,
    row: &PairingEntryRow,
) -> PairingEntryAction {
    let progress = EntryProgress::from_projection(&row.phase, row.started_by.as_deref());
    let action = match progress {
        EntryProgress::Closed => None,
        _ => EntryAction::decide(progress, coach_standing(deps, user, space_id, row).await),
    };
    PairingEntryAction {
        pairing_id: row.pairing_id.clone(),
        match_report_id: row.match_report_id.clone(),
        action,
    }
}

/// La place du coach dans ce match : étranger s'il n'entraîne aucune des deux
/// équipes — admin compris —, sinon le verdict de `can_write_report`.
async fn coach_standing(
    deps: &AccesRapportDeps<'_>,
    user: &User,
    space_id: &str,
    row: &PairingEntryRow,
) -> CoachStanding {
    let scope = access_scope_of(row);
    if camp_du_coach(deps.team_data, &scope, &user.id.to_string()).await == CampDuCoach::Aucun {
        return CoachStanding::Outsider;
    }
    let entry = entry_scope_of(row);
    match can_write_report(deps, user, space_id, &scope, &entry).await {
        true => CoachStanding::Writer,
        false => CoachStanding::Reader,
    }
}

/// La portée d'accès du rapport, telle que `can_write_report` la lit.
fn access_scope_of(row: &PairingEntryRow) -> PorteeRapport {
    PorteeRapport {
        competition_id: row.competition_id.clone(),
        home_team_id: row.home_team_id.clone(),
        away_team_id: row.away_team_id.clone(),
    }
}

/// Le mode de saisie et le démarreur, tels que `can_write_report` les lit.
fn entry_scope_of(row: &PairingEntryRow) -> EntryScope {
    EntryScope {
        entry_mode: row.entry_mode.clone(),
        started_by: row.started_by.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::EntryMode;
    use crate::app::match_report::use_cases::test_doubles::{NamedTeams, PairingEntryRepository};
    use crate::app::shared_kernel::bloodbowl::admin_access::FakeAdminAccess;
    use crate::app::shared_kernel::identity::coach_name::CoachName;
    use crate::app::shared_kernel::identity::email::Email;
    use crate::app::shared_kernel::identity::ids::{EntityId, UserId};

    const SPACE: &str = "00000000000000000000000051";
    const COMPETITION: &str = "00000000000000000000000052";
    const HOME_COACH: &str = "00000000000000000000000053";
    const AWAY_COACH: &str = "00000000000000000000000054";
    /// Admin de l'espace, sans équipe dans le match.
    const ADMIN: &str = "00000000000000000000000055";
    const HOME: &str = "home-team";
    const AWAY: &str = "away-team";
    const PAIRING: &str = "pairing";
    const REPORT: &str = "report";

    fn user(id: &str) -> User {
        User::new(
            UserId::try_new(id).unwrap(),
            CoachName::try_new("Testeur".to_string()).unwrap(),
            None,
            Email::try_new("testeur@example.com".to_string()).unwrap(),
            String::new(),
        )
    }

    fn row(phase: &str, mode: EntryMode, starter: Option<&str>) -> PairingEntryRow {
        PairingEntryRow {
            pairing_id: PAIRING.to_string(),
            match_report_id: REPORT.to_string(),
            competition_id: COMPETITION.to_string(),
            home_team_id: HOME.to_string(),
            away_team_id: AWAY.to_string(),
            phase: phase.to_string(),
            entry_mode: mode.as_str().to_string(),
            started_by: starter.map(str::to_string),
        }
    }

    fn teams() -> NamedTeams {
        NamedTeams::default()
            .coached(HOME, HOME_COACH)
            .coached(AWAY, AWAY_COACH)
    }

    fn admins() -> FakeAdminAccess {
        FakeAdminAccess::new().space_admin(
            &EntityId::try_new(ADMIN).unwrap(),
            &EntityId::try_new(SPACE).unwrap(),
        )
    }

    async fn action_with(
        who: &str,
        row: PairingEntryRow,
        teams: NamedTeams,
    ) -> Option<EntryAction> {
        let (admin_access, repo) = (admins(), PairingEntryRepository::holding(vec![row]));
        let deps = AccesRapportDeps {
            admin_access: &admin_access,
            team_data: &teams,
        };
        let pairings = vec![PAIRING.to_string()];
        let found = entry_actions_for_pairings(&repo, &deps, &user(who), SPACE, &pairings)
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].match_report_id, REPORT);
        found[0].action
    }

    async fn action(who: &str, row: PairingEntryRow) -> Option<EntryAction> {
        action_with(who, row, teams()).await
    }

    #[tokio::test]
    async fn either_coach_starts_an_unstarted_draft() {
        let draft = || row("Draft", EntryMode::SoloEntry, None);
        assert_eq!(action(HOME_COACH, draft()).await, Some(EntryAction::Start));
        assert_eq!(action(AWAY_COACH, draft()).await, Some(EntryAction::Start));
    }

    /// « Je saisis tout » : le démarreur continue.
    #[tokio::test]
    async fn the_solo_starter_continues() {
        let started = row("PreMatch", EntryMode::SoloEntry, Some(HOME_COACH));
        assert_eq!(
            action(HOME_COACH, started).await,
            Some(EntryAction::Continue)
        );
    }

    /// « Je saisis tout » commencé par l'adversaire : le coach suit.
    #[tokio::test]
    async fn the_other_coach_follows_a_solo_report() {
        let started = row("PreMatch", EntryMode::SoloEntry, Some(HOME_COACH));
        assert_eq!(action(AWAY_COACH, started).await, Some(EntryAction::Follow));
    }

    /// « Chacun son équipe » commencé par l'adversaire : le coach continue.
    #[tokio::test]
    async fn the_other_coach_continues_a_team_entry_report() {
        let started = row("PreMatch", EntryMode::TeamEntry, Some(HOME_COACH));
        assert_eq!(
            action(AWAY_COACH, started).await,
            Some(EntryAction::Continue)
        );
    }

    /// Un rapport prêt à publier — ou dépublié — se continue encore.
    #[tokio::test]
    async fn a_report_ready_to_publish_is_still_continued() {
        let ready = row("ReadyToPublish", EntryMode::SoloEntry, Some(HOME_COACH));
        assert_eq!(action(HOME_COACH, ready).await, Some(EntryAction::Continue));
    }

    /// « Je saisis tout » démarré par un admin sans équipe : les deux coachs
    /// écrivent, d'après `can_write_report` — donc continuent.
    #[tokio::test]
    async fn both_coaches_continue_when_a_non_coach_admin_started() {
        let started = || row("PreMatch", EntryMode::SoloEntry, Some(ADMIN));
        assert_eq!(
            action(HOME_COACH, started()).await,
            Some(EntryAction::Continue)
        );
        assert_eq!(
            action(AWAY_COACH, started()).await,
            Some(EntryAction::Continue)
        );
    }

    #[tokio::test]
    async fn a_published_report_offers_nothing() {
        let published = row("Published", EntryMode::SoloEntry, Some(HOME_COACH));
        assert_eq!(action(HOME_COACH, published).await, None);
    }

    /// Le dépôt ne rend pas un rapport annulé ; s'il le faisait, rien.
    #[tokio::test]
    async fn a_cancelled_report_offers_nothing() {
        let cancelled = row("Cancelled", EntryMode::SoloEntry, None);
        assert_eq!(action(HOME_COACH, cancelled).await, None);
    }

    /// I7-B1 : un admin qui n'entraîne aucune des deux équipes n'a aucune
    /// action, bien que `can_write_report` l'autorise.
    #[tokio::test]
    async fn an_admin_without_a_team_in_the_match_gets_nothing() {
        assert_eq!(
            action(ADMIN, row("Draft", EntryMode::SoloEntry, None)).await,
            None
        );
        let started = row("PreMatch", EntryMode::SoloEntry, Some(HOME_COACH));
        assert_eq!(action(ADMIN, started).await, None);
    }

    /// Un admin qui entraîne l'une des équipes est un coach du match : la règle
    /// d'écriture l'autorise toujours, il continue même ce que l'autre a démarré.
    #[tokio::test]
    async fn an_admin_who_coaches_a_team_continues() {
        let teams = NamedTeams::default()
            .coached(HOME, ADMIN)
            .coached(AWAY, AWAY_COACH);
        let started = row("PreMatch", EntryMode::SoloEntry, Some(AWAY_COACH));
        assert_eq!(
            action_with(ADMIN, started, teams).await,
            Some(EntryAction::Continue)
        );
    }

    /// Un coach étranger au match n'a rien, même sur un brouillon.
    #[tokio::test]
    async fn a_coach_outside_the_match_gets_nothing() {
        let stranger = "00000000000000000000000056";
        assert_eq!(
            action(stranger, row("Draft", EntryMode::SoloEntry, None)).await,
            None
        );
    }

    /// Pas d'appariement, pas de lecture : la doublure, qui rend tout ce
    /// qu'elle tient quels que soient les appariements, n'est pas consultée.
    #[tokio::test]
    async fn no_pairing_reads_nothing() {
        let (admin_access, teams) = (admins(), teams());
        let repo = PairingEntryRepository::holding(vec![row("Draft", EntryMode::SoloEntry, None)]);
        let deps = AccesRapportDeps {
            admin_access: &admin_access,
            team_data: &teams,
        };
        let found = entry_actions_for_pairings(&repo, &deps, &user(HOME_COACH), SPACE, &[])
            .await
            .unwrap();
        assert!(found.is_empty());
    }

    /// Un appariement sans rapport n'a pas de ligne.
    #[tokio::test]
    async fn a_pairing_without_a_report_has_no_line() {
        let (admin_access, teams) = (admins(), teams());
        let repo = PairingEntryRepository::holding(vec![]);
        let deps = AccesRapportDeps {
            admin_access: &admin_access,
            team_data: &teams,
        };
        let pairings = vec![PAIRING.to_string()];
        let found = entry_actions_for_pairings(&repo, &deps, &user(HOME_COACH), SPACE, &pairings)
            .await
            .unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn the_row_gives_the_access_scope_and_the_entry_scope() {
        let started = row("PreMatch", EntryMode::TeamEntry, Some(HOME_COACH));
        let scope = access_scope_of(&started);
        assert_eq!(
            (
                scope.competition_id.as_str(),
                scope.home_team_id.as_str(),
                scope.away_team_id.as_str()
            ),
            (COMPETITION, HOME, AWAY)
        );
        assert_eq!(
            entry_scope_of(&started),
            EntryScope {
                entry_mode: "TeamEntry".to_string(),
                started_by: Some(HOME_COACH.to_string()),
            }
        );
    }
}

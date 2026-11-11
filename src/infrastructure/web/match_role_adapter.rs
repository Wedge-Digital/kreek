//! Le canal temps réel demande à `match_report` ce qu'un utilisateur peut faire
//! sur un match (carte 644).
//!
//! **Seul fichier de la chaîne à importer le BC source.** `src/web/realtime/`
//! ne connaît que `IMatchRolePort` et `MatchRole`.
//!
//! # Les règles sont celles du BC, appelées et non réécrites
//!
//! `is_authorized` décide du suivi, `can_write_report` de l'écriture — les
//! mêmes fonctions que les gardes `garde_suivi` et `garde_saisie`. Un rôle
//! affiché qui divergerait de la garde montrerait « saisit » à quelqu'un que
//! la garde refuse.
//!
//! # L'espace est vérifié ici
//!
//! Le `POST` de présence ne porte pas `{space_id}` dans son chemin :
//! `space_scope` ne le garde pas. Sans la comparaison de `find_space_id`, un
//! admin de l'espace A se déclarerait « saisit » sur un match de l'espace B.

use crate::app::auth::domain::user::User;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::ports::ITeamDataPort;
use crate::app::match_report::use_cases::match_report_access_service::{
    can_write_report, is_authorized, load_entry_scope, AccesRapportDeps, EntryScope, PorteeRapport,
};
use crate::app::shared_kernel::bloodbowl::admin_access::IAdminAccessPort;
use crate::app::shared_kernel::identity::ids::EntityId;
use crate::web::ports::{IMatchRolePort, MatchRole};
use async_trait::async_trait;
use std::sync::Arc;

pub struct MatchRoleAdapter {
    repo: Arc<dyn IMatchReportRepository>,
    admin_access: Arc<dyn IAdminAccessPort>,
    team_data: Arc<dyn ITeamDataPort>,
}

/// Ce que le dépôt dit du match, et dont le rôle se déduit.
struct MatchFacts {
    space_id: Option<String>,
    scope: Option<PorteeRapport>,
    entry: Option<EntryScope>,
}

impl MatchRoleAdapter {
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

    /// Espace, portée et mode de saisie, lus sur la projection. Une erreur de
    /// lecture est rendue telle quelle : l'appelant la change en `None`.
    async fn load_facts(&self, match_report_id: &str) -> Result<MatchFacts, String> {
        let space_id = self.repo.find_space_id(match_report_id).await;
        let scope = self.repo.find_access_scope(match_report_id).await;
        let entry = load_entry_scope(self.repo.as_ref(), match_report_id).await;
        Ok(MatchFacts {
            space_id: space_id.map_err(|e| e.to_string())?,
            scope: scope.map_err(|e| e.to_string())?.map(to_scope),
            entry: entry.map_err(|e| e.to_string())?,
        })
    }
}

fn to_scope(
    (competition_id, home_team_id, away_team_id): (String, String, String),
) -> PorteeRapport {
    PorteeRapport {
        competition_id,
        home_team_id,
        away_team_id,
    }
}

/// Le match est-il bien de cet espace ? Un match inconnu ne l'est pas.
pub fn belongs_to_space(found: Option<&str>, space_id: &str) -> bool {
    found == Some(space_id)
}

/// Le rôle, d'après les deux règles du BC : sans suivi, rien ; avec
/// l'écriture, `Write` ; sinon `Follow`.
pub async fn decide_role(
    deps: &AccesRapportDeps<'_>,
    user: &User,
    space_id: &str,
    scope: &PorteeRapport,
    entry: &EntryScope,
) -> MatchRole {
    if !is_authorized(deps, user, space_id, scope).await {
        return MatchRole::None;
    }
    if can_write_report(deps, user, space_id, scope, entry).await {
        return MatchRole::Write;
    }
    MatchRole::Follow
}

#[async_trait]
impl IMatchRolePort for MatchRoleAdapter {
    async fn match_role(
        &self,
        user: &User,
        space_id: &EntityId,
        match_report_id: &EntityId,
    ) -> MatchRole {
        let (space_id, match_report_id) = (space_id.to_string(), match_report_id.to_string());
        let facts = match self.load_facts(&match_report_id).await {
            Ok(facts) => facts,
            Err(e) => {
                tracing::warn!(%match_report_id, error = %e, "rôle sur un match illisible");
                return MatchRole::None;
            }
        };
        if !belongs_to_space(facts.space_id.as_deref(), &space_id) {
            return MatchRole::None;
        }
        let (Some(scope), Some(entry)) = (facts.scope, facts.entry) else {
            return MatchRole::None;
        };
        decide_role(&self.deps(), user, &space_id, &scope, &entry).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::EntryMode;
    use crate::app::match_report::ports::{JourneymanPositionDto, RosterPositionDto, TeamInfoDto};
    use crate::app::shared_kernel::bloodbowl::admin_access::FakeAdminAccess;
    use crate::app::shared_kernel::identity::coach_name::CoachName;
    use crate::app::shared_kernel::identity::email::Email;
    use crate::app::shared_kernel::identity::ids::UserId;

    const HOME: &str = "home-team";
    const AWAY: &str = "away-team";
    const SPACE: &str = "00000000000000000000000051";
    const COMPETITION: &str = "00000000000000000000000052";
    const HOME_COACH: &str = "00000000000000000000000053";
    const AWAY_COACH: &str = "00000000000000000000000054";
    const THIRD_PARTY: &str = "00000000000000000000000055";
    const ADMIN: &str = "00000000000000000000000056";

    /// Chaque coach tient son équipe.
    struct FakeTeamData;

    #[async_trait]
    impl ITeamDataPort for FakeTeamData {
        async fn is_coach_of_team(&self, team_id: &str, user_id: &str) -> Result<bool, String> {
            Ok((team_id == HOME && user_id == HOME_COACH)
                || (team_id == AWAY && user_id == AWAY_COACH))
        }
        async fn find_team_coach(&self, _: &str) -> Result<Option<String>, String> {
            Ok(None)
        }
        async fn is_team_ready_to_play(&self, _: &str) -> Result<bool, String> {
            Ok(true)
        }
        async fn is_team_in_player_improvement(&self, _: &str) -> Result<bool, String> {
            Ok(false)
        }
        async fn find_team_info(&self, _: &str) -> Option<TeamInfoDto> {
            None
        }
        async fn find_team_value(&self, _: &str) -> Option<u32> {
            None
        }
        async fn find_team_treasury(&self, _: &str) -> Option<u32> {
            None
        }
        async fn find_journeyman_position(&self, _: &str) -> Option<JourneymanPositionDto> {
            None
        }
        async fn find_roster_positions(&self, _: &str) -> Vec<RosterPositionDto> {
            vec![]
        }
        async fn find_team_season(&self, _: &str) -> Option<String> {
            None
        }
    }

    fn user(id: &str) -> User {
        User::new(
            UserId::try_new(id).unwrap(),
            CoachName::try_new("Testeur".to_string()).unwrap(),
            None,
            Email::try_new("testeur@example.com".to_string()).unwrap(),
            String::new(),
        )
    }

    /// Un match démarré en « Je saisis tout » par le coach à domicile.
    async fn role_of(who: &str) -> MatchRole {
        let admin_access = FakeAdminAccess::new().space_admin(
            &EntityId::try_new(ADMIN).unwrap(),
            &EntityId::try_new(SPACE).unwrap(),
        );
        let deps = AccesRapportDeps {
            admin_access: &admin_access,
            team_data: &FakeTeamData,
        };
        let scope = to_scope((COMPETITION.into(), HOME.into(), AWAY.into()));
        let entry = EntryScope {
            entry_mode: EntryMode::SoloEntry.as_str().to_string(),
            started_by: Some(HOME_COACH.to_string()),
        };
        decide_role(&deps, &user(who), SPACE, &scope, &entry).await
    }

    #[tokio::test]
    async fn the_starter_writes() {
        assert_eq!(role_of(HOME_COACH).await, MatchRole::Write);
    }

    #[tokio::test]
    async fn an_admin_writes() {
        assert_eq!(role_of(ADMIN).await, MatchRole::Write);
    }

    #[tokio::test]
    async fn the_other_coach_follows() {
        assert_eq!(role_of(AWAY_COACH).await, MatchRole::Follow);
    }

    #[tokio::test]
    async fn a_third_party_has_no_role() {
        assert_eq!(role_of(THIRD_PARTY).await, MatchRole::None);
    }

    #[test]
    fn a_match_of_another_space_or_unknown_does_not_belong() {
        assert!(belongs_to_space(Some(SPACE), SPACE));
        assert!(!belongs_to_space(Some("autre-espace"), SPACE));
        assert!(!belongs_to_space(None, SPACE));
    }
}

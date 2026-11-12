//! Qui a le droit d'agir sur un rapport de match.
//!
//! **Une seule définition**, appelée par le récapitulatif et par l'annulation
//! (carte 433). Trois prédicats identiques divergeraient, et c'est ce qui donne
//! un bouton visible sur une action refusée — la carte 389 vient d'en corriger
//! un de cette famille.
//!
//! Le prédicat vivait dans `recap_controller` ; il en a été **déplacé**, pas
//! réécrit.

use crate::app::auth::domain::user::User;
use crate::app::match_report::domain::match_report_repository_port::{
    EntryModeRow, IMatchReportRepository, RepositoryError,
};
use crate::app::match_report::domain::value_objects::{EntryMode, TeamSide};
use crate::app::match_report::ports::ITeamDataPort;
use crate::app::shared_kernel::bloodbowl::admin_access::{is_admin, IAdminAccessPort};
use crate::app::shared_kernel::identity::ids::EntityId;
use crate::state::AppState;

pub struct AccesRapportDeps<'a> {
    /// Qui est admin — le service commun à tout kreek (carte 570).
    pub admin_access: &'a dyn IAdminAccessPort,
    pub team_data: &'a dyn ITeamDataPort,
}

impl<'a> AccesRapportDeps<'a> {
    pub fn from_state(state: &'a AppState) -> Self {
        Self {
            admin_access: state.match_report.admin_access.as_ref(),
            team_data: state.match_report.team_data.as_ref(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PorteeRapport {
    pub competition_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
}

/// Autorisé si l'utilisateur est admin d'espace, admin de la compétition du
/// rapport, ou coach de l'une des deux équipes concernées.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn is_authorized(
    deps: &AccesRapportDeps<'_>,
    user: &User,
    space_id: &str,
    scope: &PorteeRapport,
) -> bool {
    if est_administrateur(deps, user, space_id, &scope.competition_id).await {
        return true;
    }
    is_coach_of_either_team(deps, scope, &user.id.to_string()).await
}

/// Peut retirer une action du journal — ce qui décide du ✕ (carte 635).
///
/// La règle de l'écriture, `can_write_report` (carte 634) : un ✕ visible doit
/// mener à une suppression acceptée, et la suppression passe par la garde de
/// saisie. C'est ici, et nulle part dans le gabarit, que la règle se décide.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn can_delete_actions(
    deps: &AccesRapportDeps<'_>,
    user: &User,
    space_id: &str,
    scope: &PorteeRapport,
    entry: &EntryScope,
) -> bool {
    can_write_report(deps, user, space_id, scope, entry).await
}

/// Le mode de saisie d'un rapport et son démarreur (carte 631), lus sur la
/// projection comme `PorteeRapport` : la garde les demande à chaque requête.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryScope {
    /// La forme écrite de `EntryMode` ; une forme inconnue n'ouvre rien.
    pub entry_mode: String,
    /// Vide tant que le rapport n'est qu'un brouillon : personne ne l'a démarré.
    pub started_by: Option<String>,
}

impl From<EntryModeRow> for EntryScope {
    fn from(row: EntryModeRow) -> Self {
        Self {
            entry_mode: row.entry_mode,
            started_by: row.started_by,
        }
    }
}

/// Le mode de saisie et le démarreur de ce rapport, `None` s'il n'existe pas.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn load_entry_scope(
    repo: &dyn IMatchReportRepository,
    match_report_id: &str,
) -> Result<Option<EntryScope>, RepositoryError> {
    Ok(repo
        .find_entry_mode(match_report_id)
        .await?
        .map(EntryScope::from))
}

/// Peut écrire dans ce compte rendu — saisir, publier, dépublier, corriger
/// (carte 634, règle R5 de E21).
///
/// - un admin, toujours ;
/// - un tiers, jamais : la règle d'`is_authorized` reste en dessous ;
/// - un brouillon n'a pas de démarreur : tout coach du match peut le démarrer ;
/// - en « Je saisis tout », le démarreur seul — **sauf s'il ne coache aucune
///   des deux équipes** (un admin qui a démarré pour eux) : il ne compte alors
///   pas, et les deux coachs écrivent (décision du 2026-11-01).
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn can_write_report(
    deps: &AccesRapportDeps<'_>,
    user: &User,
    space_id: &str,
    scope: &PorteeRapport,
    entry: &EntryScope,
) -> bool {
    if est_administrateur(deps, user, space_id, &scope.competition_id).await {
        return true;
    }
    if !is_coach_of_either_team(deps, scope, &user.id.to_string()).await {
        return false;
    }
    coach_may_write(deps.team_data, scope, entry, &user.id.to_string()).await
}

/// La part de la règle propre à un coach du match, admin exclu.
async fn coach_may_write(
    team_data: &dyn ITeamDataPort,
    scope: &PorteeRapport,
    entry: &EntryScope,
    user_id: &str,
) -> bool {
    let Some(starter) = entry.started_by.as_deref() else {
        return true;
    };
    match EntryMode::from_written(&entry.entry_mode) {
        Some(EntryMode::SoloEntry) => solo_entry_writer(team_data, scope, starter, user_id).await,
        // « Chacun son équipe » : les deux coachs écrivent dans le rapport ;
        // c'est `can_write_side` qui coupe ensuite par camp (carte 652).
        Some(EntryMode::TeamEntry) => true,
        // Un mode que ce code ne connaît pas : un contrôle d'accès échoue fermé.
        None => false,
    }
}

/// En « Je saisis tout », le démarreur — ou les deux coachs, si le démarreur
/// ne coache aucune des deux équipes.
async fn solo_entry_writer(
    team_data: &dyn ITeamDataPort,
    scope: &PorteeRapport,
    starter: &str,
    user_id: &str,
) -> bool {
    if starter == user_id {
        return true;
    }
    !starter_coaches_a_team(team_data, scope, starter).await
}

/// Peut écrire dans **ce camp** du compte rendu (carte 652, R1 à R3 de E22).
///
/// - hors « Chacun son équipe », la règle de `can_write_report`, inchangée —
///   et un mode inconnu y reste fermé ;
/// - en « Chacun son équipe », l'admin écrit les deux camps (§ 12.6), un coach
///   le sien seul, d'après `camp_du_coach`.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn can_write_side(
    deps: &AccesRapportDeps<'_>,
    user: &User,
    space_id: &str,
    scope: &PorteeRapport,
    entry: &EntryScope,
    side: TeamSide,
) -> bool {
    if EntryMode::from_written(&entry.entry_mode) != Some(EntryMode::TeamEntry) {
        return can_write_report(deps, user, space_id, scope, entry).await;
    }
    if est_administrateur(deps, user, space_id, &scope.competition_id).await {
        return true;
    }
    camp_du_coach(deps.team_data, scope, &user.id.to_string())
        .await
        .holds(side)
}

/// Ce mode peut-il être choisi pour ce match ? (carte 652, § 12.9 de E22)
///
/// « Je saisis tout », toujours. « Chacun son équipe » exige **deux coachs
/// distincts**, quel que soit le démarreur, admin compris : le mode B garde
/// toujours deux interlocuteurs. Une équipe introuvable ou un port en erreur
/// refusent — un contrôle d'accès échoue fermé.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn may_choose_entry_mode(
    team_data: &dyn ITeamDataPort,
    home_team_id: &str,
    away_team_id: &str,
    mode: EntryMode,
) -> bool {
    if mode == EntryMode::SoloEntry {
        return true;
    }
    let (home, away) = tokio::join!(
        team_data.find_team_coach(home_team_id),
        team_data.find_team_coach(away_team_id),
    );
    match (home, away) {
        (Ok(Some(home)), Ok(Some(away))) => home != away,
        (Err(e), _) | (_, Err(e)) => {
            tracing::warn!("may_choose_entry_mode : coach illisible, mode B refusé : {e}");
            false
        }
        _ => false,
    }
}

/// Le démarreur coache-t-il l'une des deux équipes ?
///
/// **Une erreur de port vaut « oui »**, à l'inverse de `camp_du_coach` : ici,
/// « non » ouvre l'écriture à l'autre coach. Le contrôle échoue fermé dans les
/// deux cas, ce qui demande deux sens.
async fn starter_coaches_a_team(
    team_data: &dyn ITeamDataPort,
    scope: &PorteeRapport,
    starter: &str,
) -> bool {
    let (home, away) = tokio::join!(
        team_data.is_coach_of_team(&scope.home_team_id, starter),
        team_data.is_coach_of_team(&scope.away_team_id, starter),
    );
    home.unwrap_or(true) || away.unwrap_or(true)
}

/// Admin d'espace ou de compétition — **sans** les coachs des deux équipes.
/// La règle est celle de tout kreek, `is_admin` (carte 570) ; un identifiant
/// illisible refuse.
///
/// `is_authorized` répond « a le droit d'agir sur ce rapport », ce qui inclut
/// les deux coachs : c'est leur match. Celui-ci répond « a le droit de changer
/// la sélection », ce qui les exclut quand la compétition interdit les matchs
/// hors calendrier (carte 550).
///
/// Deux prédicats et non un paramètre : mêler les deux questions dans une
/// fonction unique cacherait **laquelle** a répondu, et c'est le reproche que le
/// port de `ranking` fait déjà à `require_admin_access`.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn est_administrateur(
    deps: &AccesRapportDeps<'_>,
    user: &User,
    space_id: &str,
    competition_id: &str,
) -> bool {
    let (Ok(space), Ok(competition)) = (
        EntityId::try_new(space_id),
        EntityId::try_new(competition_id),
    ) else {
        return false;
    };
    is_admin(deps.admin_access, &user.id, &space, Some(&competition)).await
}

/// Coach de l'une des deux équipes, ou des deux — un camp, quel qu'il soit.
async fn is_coach_of_either_team(
    deps: &AccesRapportDeps<'_>,
    scope: &PorteeRapport,
    user_id: &str,
) -> bool {
    camp_du_coach(deps.team_data, scope, user_id).await != CampDuCoach::Aucun
}

/// Le camp d'un coach dans ce match. Un coach peut tenir les deux équipes.
///
/// L'admin n'a pas de camp : il agit sur le match, pas pour une équipe —
/// `is_authorized` le traite à part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampDuCoach {
    Domicile,
    Visiteur,
    LesDeux,
    Aucun,
}

impl CampDuCoach {
    /// Ce camp est-il le sien ? Un coach des deux équipes tient les deux.
    pub fn holds(&self, side: TeamSide) -> bool {
        matches!(
            (self, side),
            (Self::LesDeux, _)
                | (Self::Domicile, TeamSide::Home)
                | (Self::Visiteur, TeamSide::Away)
        )
    }
}

/// Qui est ce coach dans ce match. Une erreur de port vaut « pas coach » de
/// cette équipe : un contrôle d'accès échoue fermé.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn camp_du_coach(
    team_data: &dyn ITeamDataPort,
    scope: &PorteeRapport,
    user_id: &str,
) -> CampDuCoach {
    let (home, away) = tokio::join!(
        team_data.is_coach_of_team(&scope.home_team_id, user_id),
        team_data.is_coach_of_team(&scope.away_team_id, user_id),
    );
    match (home.unwrap_or(false), away.unwrap_or(false)) {
        (true, true) => CampDuCoach::LesDeux,
        (true, false) => CampDuCoach::Domicile,
        (false, true) => CampDuCoach::Visiteur,
        (false, false) => CampDuCoach::Aucun,
    }
}

/// Le camp où arrive un utilisateur sur l'avant-match de « Chacun son
/// équipe » (carte 654, § 12.6 de E22) : le visiteur sur le sien, tout autre
/// — coach du domicile, admin sans équipe dans le match — au domicile. L'admin
/// choisit ensuite l'autre camp s'il le veut ; la garde de camp reste juge.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn arrival_side(
    team_data: &dyn ITeamDataPort,
    scope: &PorteeRapport,
    user_id: &str,
) -> TeamSide {
    match camp_du_coach(team_data, scope, user_id).await {
        CampDuCoach::Visiteur => TeamSide::Away,
        CampDuCoach::Domicile | CampDuCoach::LesDeux | CampDuCoach::Aucun => TeamSide::Home,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::ports::{JourneymanPositionDto, RosterPositionDto, TeamInfoDto};

    const DOMICILE: &str = "domicile";
    const VISITEUR: &str = "visiteur";

    /// Les équipes que l'utilisateur coache, et celle dont le port échoue.
    struct FakeTeamData {
        coached: Vec<&'static str>,
        failing: Option<&'static str>,
    }

    #[async_trait::async_trait]
    impl ITeamDataPort for FakeTeamData {
        async fn is_coach_of_team(&self, team_id: &str, _: &str) -> Result<bool, String> {
            if self.failing == Some(team_id) {
                return Err("port indisponible".to_string());
            }
            Ok(self.coached.contains(&team_id))
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

    fn scope() -> PorteeRapport {
        PorteeRapport {
            competition_id: "competition".to_string(),
            home_team_id: DOMICILE.to_string(),
            away_team_id: VISITEUR.to_string(),
        }
    }

    async fn camp(coached: Vec<&'static str>, failing: Option<&'static str>) -> CampDuCoach {
        camp_du_coach(&FakeTeamData { coached, failing }, &scope(), "coach").await
    }

    #[tokio::test]
    async fn the_home_coach_is_on_the_home_side() {
        assert_eq!(camp(vec![DOMICILE], None).await, CampDuCoach::Domicile);
    }

    #[tokio::test]
    async fn the_away_coach_is_on_the_away_side() {
        assert_eq!(camp(vec![VISITEUR], None).await, CampDuCoach::Visiteur);
    }

    #[tokio::test]
    async fn a_coach_of_both_teams_holds_both_sides() {
        assert_eq!(
            camp(vec![DOMICILE, VISITEUR], None).await,
            CampDuCoach::LesDeux
        );
    }

    #[tokio::test]
    async fn a_third_party_has_no_side() {
        assert_eq!(camp(vec![], None).await, CampDuCoach::Aucun);
    }

    /// Le port en erreur sur une équipe ne donne pas ce camp — même au coach
    /// qui le tient : un contrôle d'accès échoue fermé.
    #[tokio::test]
    async fn a_failing_port_denies_that_side_only() {
        assert_eq!(
            camp(vec![DOMICILE, VISITEUR], Some(DOMICILE)).await,
            CampDuCoach::Visiteur
        );
    }

    async fn arrival(coached: Vec<&'static str>) -> TeamSide {
        arrival_side(
            &FakeTeamData {
                coached,
                failing: None,
            },
            &scope(),
            "coach",
        )
        .await
    }

    /// Carte 654 : chaque coach arrive sur son camp.
    #[tokio::test]
    async fn each_coach_arrives_on_his_own_side() {
        assert_eq!(arrival(vec![DOMICILE]).await, TeamSide::Home);
        assert_eq!(arrival(vec![VISITEUR]).await, TeamSide::Away);
    }

    /// L'admin sans équipe dans le match arrive au domicile ; il choisira.
    #[tokio::test]
    async fn an_admin_without_a_team_arrives_home() {
        assert_eq!(arrival(vec![]).await, TeamSide::Home);
    }
}

#[cfg(test)]
mod write_tests {
    use super::*;
    use crate::app::match_report::ports::{JourneymanPositionDto, RosterPositionDto, TeamInfoDto};
    use crate::app::shared_kernel::bloodbowl::admin_access::FakeAdminAccess;
    use crate::app::shared_kernel::identity::coach_name::CoachName;
    use crate::app::shared_kernel::identity::email::Email;
    use crate::app::shared_kernel::identity::ids::UserId;

    const HOME: &str = "home-team";
    const AWAY: &str = "away-team";
    /// Une seconde équipe du coach à domicile.
    const SECOND_HOME: &str = "second-home-team";
    const SPACE: &str = "00000000000000000000000041";
    const COMPETITION: &str = "00000000000000000000000042";
    /// Coach de l'équipe à domicile.
    const HOME_COACH: &str = "00000000000000000000000043";
    /// Coach de l'équipe visiteuse.
    const AWAY_COACH: &str = "00000000000000000000000044";
    /// Ni admin ni coach du match.
    const THIRD_PARTY: &str = "00000000000000000000000045";
    /// Admin de l'espace, sans équipe dans le match.
    const ADMIN: &str = "00000000000000000000000046";

    /// Chaque coach tient son équipe ; `failing` est l'équipe dont le port échoue.
    struct FakeTeamData {
        failing: Option<&'static str>,
    }

    impl FakeTeamData {
        fn coach_of(team_id: &str) -> Option<String> {
            match team_id {
                HOME | SECOND_HOME => Some(HOME_COACH.to_string()),
                AWAY => Some(AWAY_COACH.to_string()),
                _ => None,
            }
        }
    }

    #[async_trait::async_trait]
    impl ITeamDataPort for FakeTeamData {
        async fn is_coach_of_team(&self, team_id: &str, user_id: &str) -> Result<bool, String> {
            if self.failing == Some(team_id) {
                return Err("port indisponible".to_string());
            }
            Ok(Self::coach_of(team_id).as_deref() == Some(user_id))
        }
        async fn find_team_coach(&self, team_id: &str) -> Result<Option<String>, String> {
            if self.failing == Some(team_id) {
                return Err("port indisponible".to_string());
            }
            Ok(Self::coach_of(team_id))
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

    fn scope() -> PorteeRapport {
        PorteeRapport {
            competition_id: COMPETITION.to_string(),
            home_team_id: HOME.to_string(),
            away_team_id: AWAY.to_string(),
        }
    }

    fn started_by(starter: Option<&str>) -> EntryScope {
        EntryScope {
            entry_mode: EntryMode::SoloEntry.as_str().to_string(),
            started_by: starter.map(str::to_string),
        }
    }

    async fn may_write_with(who: &str, entry: EntryScope, failing: Option<&'static str>) -> bool {
        let admin_access = FakeAdminAccess::new().space_admin(
            &EntityId::try_new(ADMIN).unwrap(),
            &EntityId::try_new(SPACE).unwrap(),
        );
        let team_data = FakeTeamData { failing };
        let deps = AccesRapportDeps {
            admin_access: &admin_access,
            team_data: &team_data,
        };
        can_write_report(&deps, &user(who), SPACE, &scope(), &entry).await
    }

    async fn may_write(who: &str, starter: Option<&str>) -> bool {
        may_write_with(who, started_by(starter), None).await
    }

    #[tokio::test]
    async fn the_starter_writes() {
        assert!(may_write(HOME_COACH, Some(HOME_COACH)).await);
    }

    #[tokio::test]
    async fn the_other_coach_only_follows() {
        assert!(!may_write(AWAY_COACH, Some(HOME_COACH)).await);
    }

    #[tokio::test]
    async fn an_admin_who_did_not_start_writes() {
        assert!(may_write(ADMIN, Some(HOME_COACH)).await);
    }

    #[tokio::test]
    async fn a_third_party_never_writes() {
        assert!(!may_write(THIRD_PARTY, Some(THIRD_PARTY)).await);
        assert!(!may_write(THIRD_PARTY, None).await);
    }

    /// Un brouillon n'est pas démarré : l'un ou l'autre coach peut le démarrer.
    #[tokio::test]
    async fn either_coach_writes_a_draft() {
        assert!(may_write(HOME_COACH, None).await);
        assert!(may_write(AWAY_COACH, None).await);
    }

    /// Décision du 2026-11-01 : un admin qui ne coache aucune des deux équipes
    /// a démarré pour eux — il ne compte pas comme démarreur.
    #[tokio::test]
    async fn both_coaches_write_when_a_non_coach_admin_started() {
        assert!(may_write(HOME_COACH, Some(ADMIN)).await);
        assert!(may_write(AWAY_COACH, Some(ADMIN)).await);
    }

    /// Le port en erreur sur l'équipe du démarreur ne le fait pas passer pour
    /// « non coach » : l'autre coach n'en gagne pas l'écriture.
    #[tokio::test]
    async fn a_failing_port_does_not_open_writing_to_the_other_coach() {
        let entry = started_by(Some(HOME_COACH));
        assert!(!may_write_with(AWAY_COACH, entry, Some(HOME)).await);
    }

    /// Un mode que ce code ne connaît pas n'ouvre rien à un coach ; l'admin,
    /// lui, écrit quel que soit le mode.
    #[tokio::test]
    async fn an_unknown_entry_mode_opens_nothing_to_a_coach() {
        let entry = EntryScope {
            entry_mode: "UnknownEntry".to_string(),
            started_by: Some(ADMIN.to_string()),
        };
        assert!(!may_write_with(HOME_COACH, entry.clone(), None).await);
        assert!(may_write_with(ADMIN, entry, None).await);
    }

    // ── « Chacun son équipe » (carte 652) ─────────────────────────────────────

    fn team_entry() -> EntryScope {
        EntryScope {
            entry_mode: EntryMode::TeamEntry.as_str().to_string(),
            started_by: Some(HOME_COACH.to_string()),
        }
    }

    async fn may_write_side_with(
        who: &str,
        entry: EntryScope,
        side: TeamSide,
        failing: Option<&'static str>,
    ) -> bool {
        let admin_access = FakeAdminAccess::new().space_admin(
            &EntityId::try_new(ADMIN).unwrap(),
            &EntityId::try_new(SPACE).unwrap(),
        );
        let team_data = FakeTeamData { failing };
        let deps = AccesRapportDeps {
            admin_access: &admin_access,
            team_data: &team_data,
        };
        can_write_side(&deps, &user(who), SPACE, &scope(), &entry, side).await
    }

    async fn may_write_side(who: &str, side: TeamSide) -> bool {
        may_write_side_with(who, team_entry(), side, None).await
    }

    /// Le démarreur ne compte plus : les deux coachs écrivent dans le rapport.
    #[tokio::test]
    async fn both_coaches_write_a_team_entry_report() {
        assert!(may_write_with(HOME_COACH, team_entry(), None).await);
        assert!(may_write_with(AWAY_COACH, team_entry(), None).await);
        assert!(!may_write_with(THIRD_PARTY, team_entry(), None).await);
    }

    #[tokio::test]
    async fn a_team_entry_coach_writes_his_own_side() {
        assert!(may_write_side(HOME_COACH, TeamSide::Home).await);
        assert!(may_write_side(AWAY_COACH, TeamSide::Away).await);
    }

    #[tokio::test]
    async fn a_team_entry_coach_does_not_write_the_other_side() {
        assert!(!may_write_side(HOME_COACH, TeamSide::Away).await);
        assert!(!may_write_side(AWAY_COACH, TeamSide::Home).await);
    }

    /// § 12.6 : l'admin écrit les deux camps, sans en coacher aucun.
    #[tokio::test]
    async fn an_admin_writes_both_sides() {
        assert!(may_write_side(ADMIN, TeamSide::Home).await);
        assert!(may_write_side(ADMIN, TeamSide::Away).await);
    }

    #[tokio::test]
    async fn a_third_party_writes_no_side() {
        assert!(!may_write_side(THIRD_PARTY, TeamSide::Home).await);
        assert!(!may_write_side(THIRD_PARTY, TeamSide::Away).await);
    }

    /// En « Je saisis tout », le camp ne change rien : la règle du rapport.
    #[tokio::test]
    async fn solo_entry_sides_follow_the_report_rule() {
        let solo = || started_by(Some(HOME_COACH));
        for side in [TeamSide::Home, TeamSide::Away] {
            assert!(may_write_side_with(HOME_COACH, solo(), side, None).await);
            assert!(!may_write_side_with(AWAY_COACH, solo(), side, None).await);
        }
    }

    /// Un mode inconnu reste fermé aux coachs, camp ou pas.
    #[tokio::test]
    async fn an_unknown_entry_mode_opens_no_side() {
        let entry = EntryScope {
            entry_mode: "UnknownEntry".to_string(),
            started_by: Some(ADMIN.to_string()),
        };
        assert!(!may_write_side_with(HOME_COACH, entry, TeamSide::Home, None).await);
    }

    #[tokio::test]
    async fn a_failing_port_denies_the_side() {
        let entry = team_entry();
        assert!(!may_write_side_with(HOME_COACH, entry, TeamSide::Home, Some(HOME)).await);
    }

    async fn may_choose(home: &str, away: &str, failing: Option<&'static str>) -> bool {
        let team_data = FakeTeamData { failing };
        may_choose_entry_mode(&team_data, home, away, EntryMode::TeamEntry).await
    }

    #[tokio::test]
    async fn two_distinct_coaches_may_start_in_team_entry() {
        assert!(may_choose(HOME, AWAY, None).await);
    }

    /// § 12.9 élargi : deux équipes d'un même coach, quel que soit le démarreur.
    #[tokio::test]
    async fn two_teams_of_one_coach_may_not_start_in_team_entry() {
        assert!(!may_choose(HOME, SECOND_HOME, None).await);
    }

    #[tokio::test]
    async fn a_failing_port_or_an_unknown_team_refuses_team_entry() {
        assert!(!may_choose(HOME, AWAY, Some(AWAY)).await);
        assert!(!may_choose(HOME, "equipe-inconnue", None).await);
    }

    #[tokio::test]
    async fn solo_entry_may_always_be_chosen() {
        let team_data = FakeTeamData {
            failing: Some(HOME),
        };
        assert!(may_choose_entry_mode(&team_data, HOME, HOME, EntryMode::SoloEntry).await);
    }
}

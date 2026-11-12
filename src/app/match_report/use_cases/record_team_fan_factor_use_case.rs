//! Le fan factor d'un camp, en « Chacun son équipe » (carte 654, épic E22).
//!
//! Chaque coach lance son jet. Le premier des deux fige les valeurs d'équipe
//! des deux camps (R4) : elles ne sont lues que si l'agrégat les attend, et le
//! second jet ne les relit pas. La règle est celle du domaine
//! (`record_team_fan_factor`) ; ce use case ne fait que charger, lire les
//! ports qu'elle demande, appender et émettre.
//!
//! **Le réessai sur conflit** (carte 653) : un second jet du même camp est
//! refusé par le domaine, donc rejouer la commande sur l'agrégat relu est sûr.
//! Chaque essai relit tout — les valeurs d'équipe comprises : si l'autre camp
//! vient de les figer, le réessai ne les relit plus.
//!
//! Le camp vient de la route, que `garde_camp` a vérifiée : jamais du
//! formulaire.

use crate::app::match_report::domain::match_report_pre_match::{
    MatchReportPreMatch, TeamFanFactor,
};
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::value_objects::{D3Roll, DedicatedFans, TeamSide, TeamValue};
use crate::app::match_report::ports::{ICompetitionDataPort, ITeamDataPort};
use crate::app::match_report::use_cases::conflict_retry::{lost_the_race, settle};
use crate::app::match_report::use_cases::event_emission::emit_all;
use crate::app::match_report::use_cases::record_fan_factor_use_case::{
    fetch_team_values, load_pre_match, next_step, RecordFanFactorError, RecordFanFactorOutcome,
};
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct RecordTeamFanFactorCommand {
    pub match_report_id: MatchReportId,
    pub team_side: TeamSide,
    pub fan_roll: D3Roll,
    pub recorded_by: CoachId,
}

/// Les ports que lit un essai.
pub struct TeamFanFactorDeps<'a> {
    pub repo: &'a dyn IMatchReportRepository,
    pub team_data: &'a dyn ITeamDataPort,
    pub competition_data: &'a dyn ICompetitionDataPort,
    pub bus: &'a EventBus,
}

const COMMAND: &str = "record_team_fan_factor";

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RecordTeamFanFactorCommand,
    deps: &TeamFanFactorDeps<'_>,
) -> Result<RecordFanFactorOutcome, RecordFanFactorError> {
    let first = record_once(&cmd, deps).await;
    if !lost_the_race(COMMAND, &first) {
        return first;
    }
    settle(COMMAND, record_once(&cmd, deps).await)
}

/// Un essai : relire l'agrégat, le laisser décider, appender à la version lue,
/// émettre ce qui a été appendé.
async fn record_once(
    cmd: &RecordTeamFanFactorCommand,
    deps: &TeamFanFactorDeps<'_>,
) -> Result<RecordFanFactorOutcome, RecordFanFactorError> {
    let mr_id = cmd.match_report_id.to_string();
    let pm = load_pre_match(deps.repo, &mr_id).await?;
    let fan_factor = TeamFanFactor {
        side: cmd.team_side,
        roll: cmd.fan_roll,
        dedicated_fans: dedicated_fans_of(&pm, cmd.team_side, deps.team_data).await,
    };
    let team_values = team_values_if_awaited(&pm, deps.team_data).await?;
    let (updated, events) = pm
        .record_team_fan_factor(fan_factor, cmd.recorded_by, team_values)
        .map_err(RecordFanFactorError::Domain)?;
    deps.repo
        .append_many(&mr_id, events.clone(), pm.version)
        .await
        .map_err(RecordFanFactorError::from_repository)?;
    emit_all(deps.bus, &mr_id, &events);
    let team_id = updated.team_id_of(cmd.team_side).to_string();
    Ok(next_step(&updated, team_id, deps.team_data, deps.competition_data).await)
}

/// Les fans dévoués du camp, lus au moment du jet comme en « Je saisis tout ».
async fn dedicated_fans_of(
    pm: &MatchReportPreMatch,
    side: TeamSide,
    team_data: &dyn ITeamDataPort,
) -> DedicatedFans {
    let team_id = pm.team_id_of(side).to_string();
    let fans = team_data
        .find_team_info(&team_id)
        .await
        .map(|i| i.dedicated_fans)
        .unwrap_or(0);
    DedicatedFans::try_new(fans).unwrap_or_default()
}

/// Les valeurs d'équipe, lues **seulement** si l'agrégat les attend : le
/// second jet ne les relit pas (R4).
async fn team_values_if_awaited(
    pm: &MatchReportPreMatch,
    team_data: &dyn ITeamDataPort,
) -> Result<Option<(TeamValue, TeamValue)>, RecordFanFactorError> {
    if !pm.awaits_team_values() {
        return Ok(None);
    }
    fetch_team_values(pm, team_data).await.map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::error::DomainError;
    use crate::app::match_report::domain::events::MatchReportDomainEvent;
    use crate::app::match_report::domain::match_report_state::MatchReportState;
    use crate::app::match_report::domain::value_objects::EntryMode;
    use crate::app::match_report::ports::{JourneymanPositionDto, RosterPositionDto, TeamInfoDto};
    use crate::app::match_report::use_cases::test_doubles::{
        pre_match, PlainCompetition, ScriptedRepository,
    };
    use crate::common::services::event_bus::event_bus::new_bus;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Des équipes valorisées, qui comptent les lectures de valeur d'équipe.
    #[derive(Default)]
    struct CountingTeams {
        values_read: AtomicUsize,
    }

    #[async_trait]
    impl ITeamDataPort for CountingTeams {
        async fn is_team_ready_to_play(&self, _: &str) -> Result<bool, String> {
            Ok(true)
        }
        async fn is_team_in_player_improvement(&self, _: &str) -> Result<bool, String> {
            Ok(false)
        }
        async fn is_coach_of_team(&self, _: &str, _: &str) -> Result<bool, String> {
            Ok(true)
        }
        async fn find_team_coach(&self, _: &str) -> Result<Option<String>, String> {
            Ok(None)
        }
        async fn find_team_info(&self, _: &str) -> Option<TeamInfoDto> {
            Some(TeamInfoDto {
                dedicated_fans: 2,
                ..Default::default()
            })
        }
        async fn find_team_value(&self, _: &str) -> Option<u32> {
            self.values_read.fetch_add(1, Ordering::SeqCst);
            Some(1000)
        }
        async fn find_team_treasury(&self, _: &str) -> Option<u32> {
            Some(0)
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

    /// Un avant-match en « Chacun son équipe », aucun jet lancé ni valeur
    /// figée.
    fn team_entry() -> MatchReportPreMatch {
        let mut pm = pre_match();
        pm.entry_mode = EntryMode::TeamEntry;
        pm.home_team_value = None;
        pm.away_team_value = None;
        pm
    }

    /// Le même, le domicile ayant lancé — valeurs figées.
    fn home_rolled() -> MatchReportPreMatch {
        let tv = TeamValue::try_new(1000).unwrap();
        let fan_factor = TeamFanFactor {
            side: TeamSide::Home,
            roll: D3Roll::try_new(1).unwrap(),
            dedicated_fans: DedicatedFans::default(),
        };
        team_entry()
            .record_team_fan_factor(fan_factor, CoachId::new(), Some((tv, tv)))
            .unwrap()
            .0
    }

    fn command(side: TeamSide) -> RecordTeamFanFactorCommand {
        RecordTeamFanFactorCommand {
            match_report_id: MatchReportId::new(),
            team_side: side,
            fan_roll: D3Roll::try_new(2).unwrap(),
            recorded_by: CoachId::new(),
        }
    }

    async fn run(
        cmd: RecordTeamFanFactorCommand,
        repo: &ScriptedRepository,
        teams: &CountingTeams,
        bus: &EventBus,
    ) -> Result<RecordFanFactorOutcome, RecordFanFactorError> {
        let deps = TeamFanFactorDeps {
            repo,
            team_data: teams,
            competition_data: &PlainCompetition,
            bus,
        };
        execute(cmd, &deps).await
    }

    fn types(events: &[MatchReportDomainEvent]) -> Vec<&'static str> {
        events.iter().map(|e| e.type_name()).collect()
    }

    /// Le premier jet fige les valeurs, et les deux faits sont émis.
    #[tokio::test]
    async fn the_first_roll_freezes_the_team_values_and_emits() {
        let repo = ScriptedRepository::reading(vec![MatchReportState::PreMatch(team_entry())]);
        let (teams, bus) = (CountingTeams::default(), new_bus());
        let mut subscriber = bus.subscribe();
        run(command(TeamSide::Away), &repo, &teams, &bus)
            .await
            .unwrap();
        assert_eq!(
            types(&repo.appended()),
            ["TeamFanFactorRecorded", "TeamValuesRecorded"]
        );
        assert_eq!(teams.values_read.load(Ordering::SeqCst), 2);
        let emitted = subscriber.try_recv().unwrap();
        assert_eq!(emitted.event_type, "TeamFanFactorRecorded");
    }

    /// R4 : le second jet ne relit pas les valeurs d'équipe.
    #[tokio::test]
    async fn the_second_roll_does_not_read_team_values() {
        let repo = ScriptedRepository::reading(vec![MatchReportState::PreMatch(home_rolled())]);
        let (teams, bus) = (CountingTeams::default(), new_bus());
        run(command(TeamSide::Away), &repo, &teams, &bus)
            .await
            .unwrap();
        assert_eq!(types(&repo.appended()), ["TeamFanFactorRecorded"]);
        assert_eq!(teams.values_read.load(Ordering::SeqCst), 0);
    }

    /// Les fans dévoués sont ceux de l'équipe du camp, lus au jet.
    #[tokio::test]
    async fn the_roll_carries_the_dedicated_fans_of_its_team() {
        let repo = ScriptedRepository::reading(vec![MatchReportState::PreMatch(home_rolled())]);
        run(
            command(TeamSide::Away),
            &repo,
            &CountingTeams::default(),
            &new_bus(),
        )
        .await
        .unwrap();
        let appended = repo.appended();
        let Some(MatchReportDomainEvent::TeamFanFactorRecorded { dedicated_fans, .. }) =
            appended.first()
        else {
            panic!("TeamFanFactorRecorded attendu");
        };
        assert_eq!(dedicated_fans.into_inner(), 2);
    }

    /// Carte 653 : perdu contre l'autre camp, le jet relit l'agrégat — où
    /// l'autre vient de figer les valeurs — et passe sans les relire.
    #[tokio::test]
    async fn a_roll_that_lost_the_race_is_replayed_on_the_fresh_aggregate() {
        let states = vec![
            MatchReportState::PreMatch(team_entry()),
            MatchReportState::PreMatch(home_rolled()),
        ];
        let repo = ScriptedRepository::reading(states).refusing(1);
        let (teams, bus) = (CountingTeams::default(), new_bus());
        run(command(TeamSide::Away), &repo, &teams, &bus)
            .await
            .unwrap();
        assert_eq!(repo.append_attempts(), 2);
        assert_eq!(types(&repo.appended()), ["TeamFanFactorRecorded"]);
        assert_eq!(
            teams.values_read.load(Ordering::SeqCst),
            2,
            "le seul premier essai"
        );
    }

    /// Deux conflits de suite : le 409 revient au client.
    #[tokio::test]
    async fn two_conflicts_return_the_conflict() {
        let states = vec![
            MatchReportState::PreMatch(home_rolled()),
            MatchReportState::PreMatch(home_rolled()),
        ];
        let repo = ScriptedRepository::reading(states).refusing(2);
        let result = run(
            command(TeamSide::Away),
            &repo,
            &CountingTeams::default(),
            &new_bus(),
        )
        .await;
        assert!(matches!(result, Err(RecordFanFactorError::Conflict)));
        assert!(repo.appended().is_empty());
    }

    /// Le même camp a lancé entre-temps : le réessai est refusé par le
    /// domaine, sans écriture — le « dernier qui écrit gagne » n'existe pas.
    #[tokio::test]
    async fn a_retry_on_a_side_already_rolled_is_refused() {
        let states = vec![
            MatchReportState::PreMatch(team_entry()),
            MatchReportState::PreMatch(home_rolled()),
        ];
        let repo = ScriptedRepository::reading(states).refusing(1);
        let result = run(
            command(TeamSide::Home),
            &repo,
            &CountingTeams::default(),
            &new_bus(),
        )
        .await;
        assert!(matches!(
            result,
            Err(RecordFanFactorError::Domain(
                DomainError::FanFactorAlreadyRecorded(TeamSide::Home)
            ))
        ));
        assert!(repo.appended().is_empty());
    }
}

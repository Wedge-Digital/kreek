//! Doublures partagées par les tests des use cases d'écriture (carte 623).
//!
//! `ConflictingRepository` rend l'état qu'on lui confie, puis refuse toute
//! écriture par `ConcurrentWrite` — ce que fait l'event store quand une autre
//! écriture est passée entre le chargement et l'append. `ScriptedRepository`
//! en refuse un nombre choisi, puis accepte : c'est la doublure du réessai
//! (carte 653).

use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, MatchActionRow, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{
    DedicatedFans, MatchReportOrigin, TeamValue,
};
use crate::app::match_report::ports::{
    CreationAppariementError, ICompetitionDataPort, IPlayerDataPort, ITeamDataPort,
    JourneymanPositionDto, PositionCountDto, RosterPositionDto, RoundContextDto, TeamInfoDto,
    TierRulesDto,
};
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Mutex;

/// Rend l'état confié au premier `find_by_id`, et refuse tout append.
///
/// `MatchReportState` n'est pas `Clone` : l'état est rendu une seule fois, ce
/// qui suffit aux use cases, qui ne chargent l'agrégat qu'une fois.
pub struct ConflictingRepository {
    state: Mutex<Option<MatchReportState>>,
}

impl ConflictingRepository {
    pub fn holding(state: MatchReportState) -> Self {
        Self {
            state: Mutex::new(Some(state)),
        }
    }
}

#[async_trait]
impl IMatchReportRepository for ConflictingRepository {
    async fn append(
        &self,
        _: &str,
        _: &MatchReportDomainEvent,
        _: u64,
    ) -> Result<u64, RepositoryError> {
        Err(RepositoryError::ConcurrentWrite)
    }
    async fn find_space_id(&self, _: &str) -> Result<Option<String>, RepositoryError> {
        Ok(None)
    }
    async fn find_by_id(&self, _: &str) -> Result<Option<MatchReportState>, RepositoryError> {
        Ok(self.state.lock().unwrap().take())
    }
    async fn find_team_ids(&self, _: &str) -> Result<Option<(String, String)>, RepositoryError> {
        Ok(None)
    }
    async fn find_entry_mode(
        &self,
        _: &str,
    ) -> Result<
        Option<crate::app::match_report::domain::match_report_repository_port::EntryModeRow>,
        RepositoryError,
    > {
        Ok(None)
    }
    async fn find_access_scope(
        &self,
        _: &str,
    ) -> Result<Option<(String, String, String)>, RepositoryError> {
        Ok(None)
    }
    async fn append_many(
        &self,
        _: &str,
        _: Vec<MatchReportDomainEvent>,
        _: u64,
    ) -> Result<u64, RepositoryError> {
        Err(RepositoryError::ConcurrentWrite)
    }
    async fn find_id_by_pairing(&self, _: &str) -> Result<Option<String>, RepositoryError> {
        Ok(None)
    }
    async fn find_phases_by_pairings(
        &self,
        _: &[String],
    ) -> Result<Vec<(String, String)>, RepositoryError> {
        Ok(vec![])
    }
    async fn find_id_by_round_and_teams(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<Option<String>, RepositoryError> {
        Ok(None)
    }
    async fn find_actions_by_match(&self, _: &str) -> Result<Vec<MatchActionRow>, RepositoryError> {
        Ok(vec![])
    }
}

/// Rend l'état confié au premier `find_by_id`, accepte tout append et garde les
/// événements reçus, pour qu'un test lise ce que le use case a écrit (carte 626).
pub struct RecordingRepository {
    state: Mutex<Option<MatchReportState>>,
    appended: Mutex<Vec<MatchReportDomainEvent>>,
}

impl RecordingRepository {
    pub fn holding(state: MatchReportState) -> Self {
        Self {
            state: Mutex::new(Some(state)),
            appended: Mutex::new(vec![]),
        }
    }

    /// Les événements appendus, dans l'ordre.
    pub fn appended(&self) -> Vec<MatchReportDomainEvent> {
        self.appended.lock().unwrap().clone()
    }
}

#[async_trait]
impl IMatchReportRepository for RecordingRepository {
    async fn append(
        &self,
        _: &str,
        event: &MatchReportDomainEvent,
        version: u64,
    ) -> Result<u64, RepositoryError> {
        self.appended.lock().unwrap().push(event.clone());
        Ok(version + 1)
    }
    async fn find_space_id(&self, _: &str) -> Result<Option<String>, RepositoryError> {
        Ok(None)
    }
    async fn find_by_id(&self, _: &str) -> Result<Option<MatchReportState>, RepositoryError> {
        Ok(self.state.lock().unwrap().take())
    }
    async fn find_team_ids(&self, _: &str) -> Result<Option<(String, String)>, RepositoryError> {
        Ok(None)
    }
    async fn find_entry_mode(
        &self,
        _: &str,
    ) -> Result<
        Option<crate::app::match_report::domain::match_report_repository_port::EntryModeRow>,
        RepositoryError,
    > {
        Ok(None)
    }
    async fn find_access_scope(
        &self,
        _: &str,
    ) -> Result<Option<(String, String, String)>, RepositoryError> {
        Ok(None)
    }
    async fn append_many(
        &self,
        _: &str,
        events: Vec<MatchReportDomainEvent>,
        version: u64,
    ) -> Result<u64, RepositoryError> {
        let count = events.len() as u64;
        self.appended.lock().unwrap().extend(events);
        Ok(version + count)
    }
    async fn find_id_by_pairing(&self, _: &str) -> Result<Option<String>, RepositoryError> {
        Ok(None)
    }
    async fn find_phases_by_pairings(
        &self,
        _: &[String],
    ) -> Result<Vec<(String, String)>, RepositoryError> {
        Ok(vec![])
    }
    async fn find_id_by_round_and_teams(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<Option<String>, RepositoryError> {
        Ok(None)
    }
    async fn find_actions_by_match(&self, _: &str) -> Result<Vec<MatchActionRow>, RepositoryError> {
        Ok(vec![])
    }
}

/// Rend un état par lecture, dans l'ordre confié, refuse les `n` premiers
/// appends par `ConcurrentWrite`, puis enregistre les suivants (carte 653).
///
/// La file d'états joue l'agrégat **avant** et **après** l'écriture
/// concurrente : le réessai d'un use case relit le second, et un test peut y
/// mettre ce que l'autre camp vient d'écrire — un MVP, des coups de pouce.
pub struct ScriptedRepository {
    states: Mutex<VecDeque<MatchReportState>>,
    conflicts_left: Mutex<u32>,
    append_attempts: Mutex<u32>,
    appended: Mutex<Vec<MatchReportDomainEvent>>,
}

impl ScriptedRepository {
    /// Un état par lecture attendue ; aucun conflit tant que `refusing` n'en
    /// demande pas.
    pub fn reading(states: Vec<MatchReportState>) -> Self {
        Self {
            states: Mutex::new(states.into()),
            conflicts_left: Mutex::new(0),
            append_attempts: Mutex::new(0),
            appended: Mutex::new(vec![]),
        }
    }

    /// Les `conflicts` premiers appends perdent la course.
    pub fn refusing(self, conflicts: u32) -> Self {
        *self.conflicts_left.lock().unwrap() = conflicts;
        self
    }

    /// Les événements effectivement écrits, dans l'ordre.
    pub fn appended(&self) -> Vec<MatchReportDomainEvent> {
        self.appended.lock().unwrap().clone()
    }

    /// Les appends tentés, perdus compris.
    pub fn append_attempts(&self) -> u32 {
        *self.append_attempts.lock().unwrap()
    }

    fn write(
        &self,
        events: Vec<MatchReportDomainEvent>,
        version: u64,
    ) -> Result<u64, RepositoryError> {
        *self.append_attempts.lock().unwrap() += 1;
        let mut left = self.conflicts_left.lock().unwrap();
        if *left > 0 {
            *left -= 1;
            return Err(RepositoryError::ConcurrentWrite);
        }
        let count = events.len() as u64;
        self.appended.lock().unwrap().extend(events);
        Ok(version + count)
    }
}

#[async_trait]
impl IMatchReportRepository for ScriptedRepository {
    async fn append(
        &self,
        _: &str,
        event: &MatchReportDomainEvent,
        version: u64,
    ) -> Result<u64, RepositoryError> {
        self.write(vec![event.clone()], version)
    }
    async fn find_space_id(&self, _: &str) -> Result<Option<String>, RepositoryError> {
        Ok(None)
    }
    async fn find_by_id(&self, _: &str) -> Result<Option<MatchReportState>, RepositoryError> {
        Ok(self.states.lock().unwrap().pop_front())
    }
    async fn find_team_ids(&self, _: &str) -> Result<Option<(String, String)>, RepositoryError> {
        Ok(None)
    }
    async fn find_entry_mode(
        &self,
        _: &str,
    ) -> Result<
        Option<crate::app::match_report::domain::match_report_repository_port::EntryModeRow>,
        RepositoryError,
    > {
        Ok(None)
    }
    async fn find_access_scope(
        &self,
        _: &str,
    ) -> Result<Option<(String, String, String)>, RepositoryError> {
        Ok(None)
    }
    async fn append_many(
        &self,
        _: &str,
        events: Vec<MatchReportDomainEvent>,
        version: u64,
    ) -> Result<u64, RepositoryError> {
        self.write(events, version)
    }
    async fn find_id_by_pairing(&self, _: &str) -> Result<Option<String>, RepositoryError> {
        Ok(None)
    }
    async fn find_phases_by_pairings(
        &self,
        _: &[String],
    ) -> Result<Vec<(String, String)>, RepositoryError> {
        Ok(vec![])
    }
    async fn find_id_by_round_and_teams(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<Option<String>, RepositoryError> {
        Ok(None)
    }
    async fn find_actions_by_match(&self, _: &str) -> Result<Vec<MatchActionRow>, RepositoryError> {
        Ok(vec![])
    }
}

/// La portée d'un rapport telle que la projection la rend — espace, puis
/// compétition et équipes —, pour le publisher, qui ne relit pas l'agrégat
/// d'un fait de saisie (carte 643). `None` : un rapport inconnu.
pub struct ScopedRepository {
    scope: Option<(String, String, String, String)>,
}

impl ScopedRepository {
    pub fn of(space_id: &str, home_team_id: &str, away_team_id: &str) -> Self {
        Self {
            scope: Some((
                space_id.into(),
                "COMPETITION".into(),
                home_team_id.into(),
                away_team_id.into(),
            )),
        }
    }

    pub fn unknown() -> Self {
        Self { scope: None }
    }
}

#[async_trait]
impl IMatchReportRepository for ScopedRepository {
    async fn append(
        &self,
        _: &str,
        _: &MatchReportDomainEvent,
        _: u64,
    ) -> Result<u64, RepositoryError> {
        panic!("le publisher n'écrit pas")
    }
    async fn find_space_id(&self, _: &str) -> Result<Option<String>, RepositoryError> {
        Ok(self.scope.as_ref().map(|s| s.0.clone()))
    }
    async fn find_by_id(&self, _: &str) -> Result<Option<MatchReportState>, RepositoryError> {
        panic!("un fait de saisie ne relit pas l'agrégat")
    }
    async fn find_team_ids(&self, _: &str) -> Result<Option<(String, String)>, RepositoryError> {
        Ok(None)
    }
    async fn find_entry_mode(
        &self,
        _: &str,
    ) -> Result<
        Option<crate::app::match_report::domain::match_report_repository_port::EntryModeRow>,
        RepositoryError,
    > {
        Ok(None)
    }
    async fn find_access_scope(
        &self,
        _: &str,
    ) -> Result<Option<(String, String, String)>, RepositoryError> {
        Ok(self
            .scope
            .as_ref()
            .map(|s| (s.1.clone(), s.2.clone(), s.3.clone())))
    }
    async fn append_many(
        &self,
        _: &str,
        _: Vec<MatchReportDomainEvent>,
        _: u64,
    ) -> Result<u64, RepositoryError> {
        panic!("le publisher n'écrit pas")
    }
    async fn find_id_by_pairing(&self, _: &str) -> Result<Option<String>, RepositoryError> {
        Ok(None)
    }
    async fn find_phases_by_pairings(
        &self,
        _: &[String],
    ) -> Result<Vec<(String, String)>, RepositoryError> {
        Ok(vec![])
    }
    async fn find_id_by_round_and_teams(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<Option<String>, RepositoryError> {
        Ok(None)
    }
    async fn find_actions_by_match(&self, _: &str) -> Result<Vec<MatchActionRow>, RepositoryError> {
        Ok(vec![])
    }
}

/// Un avant-match dont les valeurs d'équipe sont déjà saisies.
pub fn pre_match() -> MatchReportPreMatch {
    MatchReportPreMatch {
        id: MatchReportId::new(),
        space_id: SpaceId::new(),
        competition_id: CompetitionId::new(),
        season_id: SeasonId::new(),
        round_id: RoundId::new(),
        home_team_id: TeamId::new(),
        away_team_id: TeamId::new(),
        created_by: CoachId::new(),
        entry_mode: crate::app::match_report::domain::value_objects::EntryMode::SoloEntry,
        started_by: CoachId::new(),
        origin: MatchReportOrigin::Manual,
        pairing_id: None,
        home_fan_roll: None,
        away_fan_roll: None,
        home_dedicated_fans: DedicatedFans::default(),
        away_dedicated_fans: DedicatedFans::default(),
        home_team_value: Some(TeamValue::try_new(1000).unwrap()),
        away_team_value: Some(TeamValue::try_new(1000).unwrap()),
        home_inducements: None,
        away_inducements: None,
        home_inducements_recorded_by: None,
        away_inducements_recorded_by: None,
        star_engagements: vec![],
        home_temp_players: vec![],
        away_temp_players: vec![],
        home_actions: vec![],
        away_actions: vec![],
        drawn_mvps: vec![],
        version: 1,
    }
}

/// Le même avant-match, chaque camp ayant son MVP au tour 16 : l'après-match
/// ne se clôt pas sans (carte 638).
pub fn pre_match_with_mvps() -> MatchReportPreMatch {
    with_mvps(pre_match())
}

/// Ajoute un MVP à chaque camp, comme l'étape MVP l'aurait inscrit.
pub fn with_mvps(mut pm: MatchReportPreMatch) -> MatchReportPreMatch {
    use crate::app::match_report::domain::value_objects::{
        ActionId, ActionPlayer, MatchAction, MatchActionType, TurnNumber,
    };
    use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
    let mvp = |id: &str| MatchAction {
        id: ActionId(id.into()),
        turn: TurnNumber::LAST,
        player: ActionPlayer::Regular(PlayerId::new()),
        action: MatchActionType::Mvp,
        player_display_name: format!("MVP {id}"),
        player_position: String::new(),
    };
    pm.home_actions.push(mvp("mvp-home"));
    pm.away_actions.push(mvp("mvp-away"));
    pm
}

/// Des équipes prêtes à jouer, valorisées et solvables : aucun refus ne vient
/// de ce port, seul l'append peut échouer.
pub struct ReadyTeams;

#[async_trait]
impl ITeamDataPort for ReadyTeams {
    async fn is_team_ready_to_play(&self, _: &str) -> Result<bool, String> {
        Ok(true)
    }
    async fn is_team_in_player_improvement(&self, _: &str) -> Result<bool, String> {
        Ok(true)
    }
    async fn is_coach_of_team(&self, _: &str, _: &str) -> Result<bool, String> {
        Ok(true)
    }
    async fn find_team_coach(&self, _: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
    async fn find_team_info(&self, _: &str) -> Option<TeamInfoDto> {
        None
    }
    async fn find_team_value(&self, _: &str) -> Option<u32> {
        Some(1000)
    }
    async fn find_team_treasury(&self, _: &str) -> Option<u32> {
        Some(1_000_000)
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

/// Des joueurs connus, assez nombreux pour qu'aucun journalier ne soit requis.
pub struct KnownPlayers;

#[async_trait]
impl IPlayerDataPort for KnownPlayers {
    async fn count_available_players(&self, _: &str) -> Result<usize, String> {
        Ok(11)
    }
    async fn find_player_display(&self, _: &str) -> Option<String> {
        Some("Joueur".to_string())
    }
    async fn find_player_position(&self, _: &str) -> Option<String> {
        None
    }
    async fn find_player_counts_by_position(&self, _: &str) -> Vec<PositionCountDto> {
        vec![]
    }
    async fn has_spent_spp_since_match(&self, _: &str, _: &str) -> Result<bool, String> {
        Ok(false)
    }
    async fn list_alignable_players(
        &self,
        _: &str,
    ) -> Vec<crate::app::match_report::ports::AlignablePlayerDto> {
        Vec::new()
    }
}

/// Une compétition qui connaît le tier, sans y autoriser aucun coup de pouce.
pub struct PlainCompetition;

#[async_trait]
impl ICompetitionDataPort for PlainCompetition {
    async fn find_tier_rules_for_roster(&self, _: &str, _: &str) -> Option<TierRulesDto> {
        Some(TierRulesDto::default())
    }
    async fn find_round_context(&self, _: &str, _: &str) -> Option<RoundContextDto> {
        None
    }
    async fn autorise_hors_calendrier(&self, _: &str) -> bool {
        true
    }
    async fn creer_appariement(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<String, CreationAppariementError> {
        Err(CreationAppariementError::Indisponible("doublure".into()))
    }
}

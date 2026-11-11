//! Deux camps qui écrivent à la même version, sur la vraie base (carte 653).
//!
//! La course est **provoquée**, pas espérée : `InterleavingRepository` écrit
//! l'événement de l'autre camp juste avant de déléguer l'append du use case,
//! à la même version. C'est la contrainte `match_report_es_version` de
//! Postgres qui refuse alors l'append — le vrai `ConcurrentWrite`, pas une
//! doublure — et le use case doit relire l'agrégat et rejouer sa commande.
//! Aucune attente, aucune chance : le conflit a lieu à chaque exécution.

use super::match_report_repository::MatchReportRepository;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_repository_port::{
    EntryModeRow, IMatchReportRepository, MatchActionRow, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{
    ActionId, ActionPlayer, MatchActionType, MatchReportOrigin, TeamSide, TurnNumber,
};
use crate::app::match_report::ports::{IKeywordCatalogPort, KeywordDto};
use crate::app::match_report::use_cases::record_action_use_case::{
    self, RecordActionCommand, RecordActionError,
};
use crate::app::match_report::use_cases::test_doubles::KnownPlayers;
use crate::app::shared_kernel::bloodbowl::ids::{
    CompetitionId, MatchReportId, PlayerId, RoundId, SeasonId,
};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::common::services::event_bus::event_bus::new_bus;
use async_trait::async_trait;
use sqlx::PgPool;
use std::collections::VecDeque;
use std::sync::Mutex;

/// Le vrai dépôt, devancé à chaque append par une écriture de l'autre camp
/// tant qu'il lui en reste une à faire.
struct InterleavingRepository {
    inner: MatchReportRepository,
    other_side: Mutex<VecDeque<MatchReportDomainEvent>>,
}

impl InterleavingRepository {
    fn new(pool: PgPool, other_side: Vec<MatchReportDomainEvent>) -> Self {
        Self {
            inner: MatchReportRepository::new(pool),
            other_side: Mutex::new(other_side.into()),
        }
    }

    /// L'autre camp écrit à la version que le use case vient de lire : il
    /// gagne la course, et l'append qui suit la perd.
    async fn let_the_other_side_win(&self, mr_id: &str, version: u64) {
        let next = self.other_side.lock().unwrap().pop_front();
        if let Some(event) = next {
            self.inner
                .append(mr_id, &event, version)
                .await
                .expect("l'autre camp écrit le premier");
        }
    }
}

#[async_trait]
impl IMatchReportRepository for InterleavingRepository {
    async fn append(
        &self,
        mr_id: &str,
        event: &MatchReportDomainEvent,
        version: u64,
    ) -> Result<u64, RepositoryError> {
        self.let_the_other_side_win(mr_id, version).await;
        self.inner.append(mr_id, event, version).await
    }
    async fn find_space_id(&self, id: &str) -> Result<Option<String>, RepositoryError> {
        self.inner.find_space_id(id).await
    }
    async fn find_by_id(&self, id: &str) -> Result<Option<MatchReportState>, RepositoryError> {
        self.inner.find_by_id(id).await
    }
    async fn find_access_scope(
        &self,
        id: &str,
    ) -> Result<Option<(String, String, String)>, RepositoryError> {
        self.inner.find_access_scope(id).await
    }
    async fn find_entry_mode(&self, id: &str) -> Result<Option<EntryModeRow>, RepositoryError> {
        self.inner.find_entry_mode(id).await
    }
    async fn find_team_ids(&self, id: &str) -> Result<Option<(String, String)>, RepositoryError> {
        self.inner.find_team_ids(id).await
    }
    async fn append_many(
        &self,
        mr_id: &str,
        events: Vec<MatchReportDomainEvent>,
        version: u64,
    ) -> Result<u64, RepositoryError> {
        self.let_the_other_side_win(mr_id, version).await;
        self.inner.append_many(mr_id, events, version).await
    }
    async fn find_id_by_pairing(&self, id: &str) -> Result<Option<String>, RepositoryError> {
        self.inner.find_id_by_pairing(id).await
    }
    async fn find_phases_by_pairings(
        &self,
        ids: &[String],
    ) -> Result<Vec<(String, String)>, RepositoryError> {
        self.inner.find_phases_by_pairings(ids).await
    }
    async fn find_id_by_round_and_teams(
        &self,
        round_id: &str,
        team_a: &str,
        team_b: &str,
    ) -> Result<Option<String>, RepositoryError> {
        self.inner
            .find_id_by_round_and_teams(round_id, team_a, team_b)
            .await
    }
    async fn find_actions_by_match(
        &self,
        id: &str,
    ) -> Result<Vec<MatchActionRow>, RepositoryError> {
        self.inner.find_actions_by_match(id).await
    }
}

struct NoKeywords;

impl IKeywordCatalogPort for NoKeywords {
    fn list_hateable(&self) -> Vec<KeywordDto> {
        vec![]
    }
    fn find_hateable(&self, _: &str) -> Option<KeywordDto> {
        None
    }
}

/// Un compte rendu en avant-match, écrit par le vrai dépôt : version 2.
async fn pre_match_report(pool: &PgPool) -> String {
    let repo = MatchReportRepository::new(pool.clone());
    let mr_id = MatchReportId::new().to_string();
    let created = MatchReportDomainEvent::MatchReportCreated {
        match_report_id: MatchReportId::try_new(&mr_id).unwrap(),
        space_id: SpaceId::new(),
        competition_id: CompetitionId::new(),
        season_id: SeasonId::new(),
        round_id: RoundId::new(),
        home_team_id: TeamId::new(),
        away_team_id: TeamId::new(),
        created_by: CoachId::new(),
        origin: MatchReportOrigin::Manual,
        pairing_id: None,
    };
    let confirmed = MatchReportDomainEvent::SelectionConfirmed {
        confirmed_by: CoachId::new(),
    };
    repo.append_many(&mr_id, vec![created, confirmed], 0)
        .await
        .expect("avant-match écrit");
    mr_id
}

/// Un touchdown que l'autre camp, le visiteur, inscrit au même instant.
fn away_touchdown(id: &str) -> MatchReportDomainEvent {
    MatchReportDomainEvent::ActionRecorded {
        action_id: ActionId(id.into()),
        team_side: TeamSide::Away,
        turn: TurnNumber::try_new(3).unwrap(),
        player: ActionPlayer::Regular(PlayerId::new()),
        action: MatchActionType::Touchdown,
        player_display_name: "Visiteur".into(),
        player_position: String::new(),
        recorded_by: CoachId::new(),
    }
}

fn home_touchdown(mr_id: &str) -> RecordActionCommand {
    RecordActionCommand {
        match_report_id: MatchReportId::try_new(mr_id).unwrap(),
        team_side: TeamSide::Home,
        turn: TurnNumber::try_new(3).unwrap(),
        player: ActionPlayer::Regular(PlayerId::new()),
        action: MatchActionType::Touchdown,
        hatred: None,
        recorded_by: CoachId::new(),
    }
}

/// Les événements du flux, dans l'ordre de leurs versions.
async fn stream(pool: &PgPool, mr_id: &str) -> Vec<(i64, String)> {
    sqlx::query_as(
        "SELECT version, event_type FROM match_report_event_store
         WHERE match_report_id = $1 ORDER BY version",
    )
    .bind(mr_id)
    .fetch_all(pool)
    .await
    .expect("flux lu")
}

async fn actions_by_side(repo: &dyn IMatchReportRepository, mr_id: &str) -> (usize, usize) {
    let Some(MatchReportState::PreMatch(pm)) = repo.find_by_id(mr_id).await.unwrap() else {
        panic!("attendu un avant-match");
    };
    (pm.home_actions.len(), pm.away_actions.len())
}

/// Le domicile perd la course contre le visiteur : relu, il réécrit à la
/// version suivante. Les deux actions sont au flux — rien n'est perdu.
#[sqlx::test]
async fn two_sides_writing_at_the_same_version_both_land(pool: PgPool) {
    let mr_id = pre_match_report(&pool).await;
    let repo = InterleavingRepository::new(pool.clone(), vec![away_touchdown("AWAY-1")]);

    record_action_use_case::execute(
        home_touchdown(&mr_id),
        &repo,
        &KnownPlayers,
        &NoKeywords,
        &new_bus(),
    )
    .await
    .expect("le réessai passe");

    let events: Vec<_> = stream(&pool, &mr_id).await;
    let types: Vec<&str> = events.iter().map(|(_, t)| t.as_str()).collect();
    assert_eq!(
        types,
        [
            "MatchReportCreated",
            "SelectionConfirmed",
            "ActionRecorded",
            "ActionRecorded"
        ]
    );
    assert_eq!(events.last().map(|(v, _)| *v), Some(4));
    assert_eq!(actions_by_side(&repo, &mr_id).await, (1, 1));
}

/// Deux courses perdues de suite : l'état change sous la main, le use case
/// rend le conflit — le client recevra son 409. Seules les deux actions de
/// l'autre camp sont écrites.
#[sqlx::test]
async fn two_lost_races_in_a_row_return_the_conflict(pool: PgPool) {
    let mr_id = pre_match_report(&pool).await;
    let other_side = vec![away_touchdown("AWAY-1"), away_touchdown("AWAY-2")];
    let repo = InterleavingRepository::new(pool.clone(), other_side);

    let result = record_action_use_case::execute(
        home_touchdown(&mr_id),
        &repo,
        &KnownPlayers,
        &NoKeywords,
        &new_bus(),
    )
    .await;

    assert!(
        matches!(result, Err(RecordActionError::Conflict)),
        "{:?}",
        result.err()
    );
    assert_eq!(actions_by_side(&repo, &mr_id).await, (0, 2));
}

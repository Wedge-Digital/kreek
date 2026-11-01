//! Écrire le tirage au calendrier. Le navigateur ne rend que des couples et des
//! exemptées : **tout est revérifié ici** (R11) — c'est la journée qui accepte
//! ou refuse, et les étiquettes de revanche ne transitent jamais.
//!
//! Écrites, les rencontres sont ordinaires (R13) : rien ne les distingue d'un
//! match ajouté à la main.

use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::match_day::Pairing;
use crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError;
use crate::app::competitions::use_cases::admin::draw::draw_input::{
    draw_rules, load_draw_context, write_round, DrawContext, DrawDeps, DrawLoadError,
};
use crate::app::competitions::use_cases::admin::pairing_created::PairingLocation;
use crate::app::competitions::use_cases::admin::team_enrollment::filter_enrolled_team_ids;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchId, PairingId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::common::services::event_bus::event_bus::EventBus;
use std::collections::HashSet;

#[derive(Debug)]
pub struct ConfirmDrawCommand {
    pub round_id: MatchId,
    pub season_id: SeasonId,
    pub competition_id: CompetitionId,
    pub space_id: SpaceId,
    pub pairings: Vec<(TeamId, TeamId)>,
    /// Une par poule impaire.
    pub exempted: Vec<TeamId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfirmOutcome {
    pub written: usize,
    pub exempted_names: Vec<String>,
    pub absent_names: Vec<String>,
}

#[derive(Debug)]
pub enum ConfirmDrawError {
    RoundNotFound,
    IsRestDay,
    /// Déjà pourvue — à la lecture, ou remplie entre la lecture et l'écriture.
    PairingsAlreadyExist,
    /// Équipe non inscrite, même coach, équipe en double, poules mêlées.
    Refused(DomainError),
    Repository(String),
}

impl From<DrawLoadError> for ConfirmDrawError {
    fn from(e: DrawLoadError) -> Self {
        match e {
            DrawLoadError::RoundNotFound => Self::RoundNotFound,
            DrawLoadError::Repository(msg) => Self::Repository(msg),
        }
    }
}

impl From<DomainError> for ConfirmDrawError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::RestDayCannotBeDrawn => Self::IsRestDay,
            DomainError::RoundAlreadyPaired => Self::PairingsAlreadyExist,
            other => Self::Refused(other),
        }
    }
}

impl From<MatchDayRepositoryError> for ConfirmDrawError {
    fn from(e: MatchDayRepositoryError) -> Self {
        match e {
            MatchDayRepositoryError::PairingsAlreadyExist => Self::PairingsAlreadyExist,
            MatchDayRepositoryError::Database(msg) => Self::Repository(msg),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ConfirmDrawCommand,
    deps: &DrawDeps<'_>,
    event_bus: &EventBus,
) -> Result<ConfirmOutcome, ConfirmDrawError> {
    let (round_id, competition_id, space_id) = (
        cmd.round_id.to_string(),
        cmd.competition_id.to_string(),
        cmd.space_id.to_string(),
    );
    let mut ctx = load_draw_context(&round_id, &cmd.season_id.to_string(), deps).await?;
    let pairings = accept(&mut ctx, &cmd)?;
    let location = PairingLocation {
        competition_id: &competition_id,
        space_id: &space_id,
        round_id: &round_id,
    };
    write_round(&ctx, &pairings, location, deps.match_day_repo, event_bus).await?;
    Ok(outcome(&ctx, &pairings, &cmd.exempted))
}

/// La journée accepte les rencontres, chacune sous un identifiant neuf, ou
/// refuse sans rien changer. Un tirage sans rencontre n'a rien à écrire.
fn accept(
    ctx: &mut DrawContext,
    cmd: &ConfirmDrawCommand,
) -> Result<Vec<Pairing>, ConfirmDrawError> {
    if cmd.pairings.is_empty() {
        return Err(ConfirmDrawError::Refused(DomainError::NotEnoughPresent));
    }
    let pairings: Vec<Pairing> = cmd
        .pairings
        .iter()
        .map(|(home, away)| Pairing {
            id: PairingId::new(),
            home_team_id: *home,
            away_team_id: *away,
        })
        .collect();
    let rules = draw_rules(ctx);
    ctx.match_day
        .accept_draw(pairings.clone(), &cmd.exempted, &rules)?;
    Ok(pairings)
}

/// Les absentes se déduisent : les inscrites des poules, ni appariées ni
/// exemptées.
fn outcome(ctx: &DrawContext, pairings: &[Pairing], exempted: &[TeamId]) -> ConfirmOutcome {
    let placed: HashSet<String> = pairings
        .iter()
        .flat_map(|p| [p.home_team_id, p.away_team_id])
        .chain(exempted.iter().copied())
        .map(|team| team.to_string())
        .collect();
    let mut absent_names: Vec<String> = ctx
        .groups
        .iter()
        .flat_map(|group| filter_enrolled_team_ids(&group.team_ids, &ctx.teams).0)
        .filter(|id| !placed.contains(id))
        .filter_map(|id| ctx.teams.get(&id).map(|t| t.team_name.clone()))
        .collect();
    absent_names.sort();
    ConfirmOutcome {
        written: pairings.len(),
        exempted_names: exempted
            .iter()
            .filter_map(|team| {
                ctx.teams
                    .get(&team.to_string())
                    .map(|t| t.team_name.clone())
            })
            .collect(),
        absent_names,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::group_repository_port::{
        GroupRepositoryError, GroupWithTeams, IGroupRepository,
    };
    use crate::app::competitions::domain::match_day::{
        MatchDay, MatchDayName, MatchDayPosition, MatchDayType,
    };
    use crate::app::competitions::domain::match_day_repository_port::{
        IMatchDayRepository, NewPairingProjection, PairingDisplayDto,
    };
    use crate::app::competitions::ports::{ITeamInfoPort, TeamInfoDto};
    use async_trait::async_trait;
    use std::sync::Mutex;

    // ── Doublures — reprises de `generate_pairings` ──────────────────────────

    /// La journée tirée, ce que `save_pairings` a reçu, et l'échec qu'il doit
    /// rendre, s'il en a un.
    struct FakeMatchDayRepo {
        day: MatchDay,
        saved: Mutex<Vec<Pairing>>,
        save_fails_with: Option<fn() -> MatchDayRepositoryError>,
    }

    impl FakeMatchDayRepo {
        fn of(day: MatchDay) -> Self {
            Self {
                day,
                saved: Mutex::new(vec![]),
                save_fails_with: None,
            }
        }

        fn saved(&self) -> Vec<Pairing> {
            self.saved.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl IMatchDayRepository for FakeMatchDayRepo {
        async fn find_by_season(&self, _: &str) -> Result<Vec<MatchDay>, MatchDayRepositoryError> {
            Ok(vec![self.day.clone()])
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<MatchDay>, MatchDayRepositoryError> {
            Ok(Some(self.day.clone()))
        }
        async fn save_match_day(&self, _: &MatchDay) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn delete_match_day(&self, _: &str) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn save_pairings(
            &self,
            _: &str,
            pairings: &[(Pairing, NewPairingProjection)],
        ) -> Result<(), MatchDayRepositoryError> {
            if let Some(failure) = self.save_fails_with {
                return Err(failure());
            }
            let mut saved = self.saved.lock().unwrap();
            saved.extend(pairings.iter().map(|(pairing, _)| pairing.clone()));
            Ok(())
        }
        async fn save_pairing(
            &self,
            _: &str,
            _: &Pairing,
            _: &NewPairingProjection,
        ) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn find_pairing_id(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<Option<String>, MatchDayRepositoryError> {
            Ok(None)
        }
        async fn delete_pairing(&self, _: &str) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn move_pairing(&self, _: &str, _: &MatchDay) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn ensure_match_days_from_structure(
            &self,
            _: &str,
            _: &[(String, String, String, Option<String>, Option<String>)],
        ) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn list_resultats(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_calendrier(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_team_matches(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_latest_completed_results(
            &self,
            _: &str,
            _: i64,
        ) -> Result<
            Vec<crate::app::competitions::domain::match_day_repository_port::LatestResultDto>,
            MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
    }

    struct FakeGroupRepo(Vec<GroupWithTeams>);
    #[async_trait]
    impl IGroupRepository for FakeGroupRepo {
        async fn find_groups(&self, _: &str) -> Result<Vec<GroupWithTeams>, GroupRepositoryError> {
            Ok(self.0.clone())
        }
        async fn save_assignments(
            &self,
            _: &[(String, String)],
        ) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn reset_assignments(&self, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn assign_team(&self, _: &str, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn unassign_team(&self, _: &str) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
        async fn ensure_groups_from_structure(
            &self,
            _: &str,
            _: &[(String, String)],
        ) -> Result<(), GroupRepositoryError> {
            Ok(())
        }
    }

    struct FakeTeamInfoPortWithEnrolled(Vec<TeamInfoDto>);
    #[async_trait]
    impl ITeamInfoPort for FakeTeamInfoPortWithEnrolled {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(self.0.clone())
        }
        async fn find_team_names(&self, _: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_enrollment(
            &self,
            _: &str,
        ) -> Result<Option<crate::app::competitions::ports::TeamEnrollmentDto>, String> {
            Ok(None)
        }
    }

    // ── Le décor : une poule de cinq inscrites, coachs distincts ────────────

    struct Setup {
        teams: Vec<TeamId>,
        repo: FakeMatchDayRepo,
        groups: FakeGroupRepo,
        port: FakeTeamInfoPortWithEnrolled,
    }

    fn empty_day() -> MatchDay {
        MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("J4".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(3).unwrap(),
            pairings: vec![],
        }
    }

    fn info(id: &TeamId, rank: usize, coach: &str) -> TeamInfoDto {
        TeamInfoDto {
            team_id: id.to_string(),
            team_name: format!("Équipe {rank}"),
            coach_id: coach.to_string(),
            coach_name: coach.to_string(),
            roster_name: "Nains".to_string(),
            logo_url: None,
        }
    }

    fn setup() -> Setup {
        let teams: Vec<TeamId> = (0..5).map(|_| TeamId::new()).collect();
        let infos = teams
            .iter()
            .enumerate()
            .map(|(rank, id)| info(id, rank, &format!("coach-{rank}")))
            .collect();
        Setup {
            repo: FakeMatchDayRepo::of(empty_day()),
            groups: FakeGroupRepo(vec![GroupWithTeams {
                group_id: "A".to_string(),
                group_name: "Poule A".to_string(),
                position: 0,
                team_ids: teams.iter().map(|t| t.to_string()).collect(),
            }]),
            port: FakeTeamInfoPortWithEnrolled(infos),
            teams,
        }
    }

    /// Équipes 0-1 et 2-3, la 4 exemptée.
    fn command(t: &[TeamId]) -> ConfirmDrawCommand {
        ConfirmDrawCommand {
            round_id: MatchId::new(),
            season_id: SeasonId::new(),
            competition_id: CompetitionId::new(),
            space_id: SpaceId::new(),
            pairings: vec![(t[0], t[1]), (t[2], t[3])],
            exempted: vec![t[4]],
        }
    }

    async fn run(
        s: &Setup,
        cmd: ConfirmDrawCommand,
        bus: &EventBus,
    ) -> Result<ConfirmOutcome, ConfirmDrawError> {
        let deps = DrawDeps {
            match_day_repo: &s.repo,
            group_repo: &s.groups,
            team_port: &s.port,
        };
        execute(cmd, &deps, bus).await
    }

    fn announced(
        rx: &mut tokio::sync::broadcast::Receiver<crate::common::event_envelope::EventEnvelope>,
    ) -> usize {
        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        count
    }

    #[tokio::test]
    async fn a_valid_draw_is_written_and_announced() {
        let s = setup();
        let bus = crate::common::services::event_bus::event_bus::new_bus();
        let mut rx = bus.subscribe();

        let outcome = run(&s, command(&s.teams), &bus).await.unwrap();

        assert_eq!(s.repo.saved().len(), 2);
        assert_eq!(announced(&mut rx), 2, "un PairingCreated par rencontre");
        assert_eq!(outcome.written, 2);
        assert_eq!(outcome.exempted_names, vec!["Équipe 4"]);
        assert!(outcome.absent_names.is_empty());
    }

    /// R4 — la journée refuse deux équipes d'un même coach ; rien n'est écrit,
    /// rien n'est annoncé.
    #[tokio::test]
    async fn a_refusal_of_the_round_writes_and_announces_nothing() {
        let mut s = setup();
        s.port.0[1].coach_id = s.port.0[0].coach_id.clone();
        let bus = crate::common::services::event_bus::event_bus::new_bus();
        let mut rx = bus.subscribe();

        let refus = run(&s, command(&s.teams), &bus).await;

        assert!(matches!(
            refus,
            Err(ConfirmDrawError::Refused(
                DomainError::SameCoachPairing { .. }
            ))
        ));
        assert!(s.repo.saved().is_empty());
        assert_eq!(announced(&mut rx), 0);
    }

    /// R11 — une équipe tirée deux fois : refus de la journée, relayé.
    #[tokio::test]
    async fn a_team_drawn_twice_is_refused() {
        let s = setup();
        let bus = crate::common::services::event_bus::event_bus::new_bus();
        let mut cmd = command(&s.teams);
        cmd.pairings.push((s.teams[0], s.teams[4]));
        cmd.exempted.clear();

        let refus = run(&s, cmd, &bus).await;

        assert!(matches!(refus, Err(ConfirmDrawError::Refused(_))));
        assert!(s.repo.saved().is_empty());
    }

    /// La journée remplie entre l'aperçu et l'écriture : le dépôt refuse sous
    /// verrou, et rien n'est annoncé.
    #[tokio::test]
    async fn a_round_filled_in_between_is_refused_without_announcement() {
        let mut s = setup();
        s.repo.save_fails_with = Some(|| MatchDayRepositoryError::PairingsAlreadyExist);
        let bus = crate::common::services::event_bus::event_bus::new_bus();
        let mut rx = bus.subscribe();

        let refus = run(&s, command(&s.teams), &bus).await;

        assert!(matches!(refus, Err(ConfirmDrawError::PairingsAlreadyExist)));
        assert_eq!(announced(&mut rx), 0);
    }

    /// Jamais d'annonce pour une rencontre que la transaction n'a pas écrite.
    #[tokio::test]
    async fn a_failed_write_announces_nothing() {
        let mut s = setup();
        s.repo.save_fails_with = Some(|| MatchDayRepositoryError::Database("panne".to_string()));
        let bus = crate::common::services::event_bus::event_bus::new_bus();
        let mut rx = bus.subscribe();

        let refus = run(&s, command(&s.teams), &bus).await;

        assert!(matches!(refus, Err(ConfirmDrawError::Repository(_))));
        assert_eq!(announced(&mut rx), 0);
    }

    #[tokio::test]
    async fn the_absent_teams_are_deduced() {
        let s = setup();
        let bus = crate::common::services::event_bus::event_bus::new_bus();
        let mut cmd = command(&s.teams);
        cmd.pairings.pop();
        cmd.exempted.clear();
        cmd.pairings.push((s.teams[2], s.teams[4]));

        let outcome = run(&s, cmd, &bus).await.unwrap();

        assert_eq!(outcome.absent_names, vec!["Équipe 3"]);
        assert!(outcome.exempted_names.is_empty());
    }
}

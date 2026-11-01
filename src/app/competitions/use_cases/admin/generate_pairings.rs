use crate::app::competitions::domain::draw::{draw_pools, DrawProposal};
use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::group_repository_port::IGroupRepository;
use crate::app::competitions::domain::match_day::Pairing;
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError,
};
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::competitions::use_cases::admin::draw::draw_input::{
    draw_rules, load_draw_context, parse_team_ids, pool_input, write_round, DrawContext, DrawDeps,
    DrawLoadError,
};
use crate::app::competitions::use_cases::admin::pairing_created::PairingLocation;
use crate::app::competitions::use_cases::admin::team_enrollment::{
    filter_enrolled_team_ids, resolve_team_names,
};
use crate::app::shared_kernel::bloodbowl::ids::PairingId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::common::services::event_bus::event_bus::EventBus;
use rand::SeedableRng;

#[derive(Debug)]
pub enum GenerateError {
    MatchDayNotFound,
    IsRestDay,
    PairingsAlreadyExist,
    NoGroups,
    /// La journée a refusé le tirage. Ne devrait pas arriver : le tirage
    /// respecte déjà R3 et R4 — un refus signale un défaut, pas une saisie.
    Refused(DomainError),
    Repository(String),
}

impl From<DrawLoadError> for GenerateError {
    fn from(e: DrawLoadError) -> Self {
        match e {
            DrawLoadError::RoundNotFound => Self::MatchDayNotFound,
            DrawLoadError::Repository(msg) => Self::Repository(msg),
        }
    }
}

impl From<DomainError> for GenerateError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::RestDayCannotBeDrawn => Self::IsRestDay,
            DomainError::RoundAlreadyPaired => Self::PairingsAlreadyExist,
            other => Self::Refused(other),
        }
    }
}

/// La journée remplie entre la lecture et l'écriture se refuse comme une
/// journée trouvée pourvue (R11).
impl From<MatchDayRepositoryError> for GenerateError {
    fn from(e: MatchDayRepositoryError) -> Self {
        match e {
            MatchDayRepositoryError::PairingsAlreadyExist => Self::PairingsAlreadyExist,
            MatchDayRepositoryError::Database(msg) => Self::Repository(msg),
        }
    }
}

/// `skipped_team_names` : équipes présentes dans un groupe/poule mais non
/// `Enrolled` pour la saison — jamais appariées (BR : pas de pairing pour une
/// équipe non enrôlée, donc pas d'event `PairingCreated` la concernant).
/// `skipped_group_names` : poules avec moins de 2 équipes enrôlées assignées
/// — aucun appariement possible, sans quoi la génération réussirait
/// silencieusement à 0 rencontre pour cette poule (BR : signaler explicitement
/// plutôt que de laisser l'admin croire que la génération a échoué).
#[derive(Debug, Default)]
pub struct GenerateOutcome {
    pub skipped_team_names: Vec<String>,
    pub skipped_group_names: Vec<String>,
}

/// Ce que la génération écarte, et doit signaler.
#[derive(Default)]
struct Skipped {
    team_ids: Vec<String>,
    group_names: Vec<String>,
}

/// « Générer une journée complète » (R7) : tire parmi **toutes** les inscrites,
/// poule par poule, et écrit la journée d'un bloc (R14).
#[tracing::instrument(skip_all, fields(match_day_id = ?match_day_id))]
pub async fn execute(
    match_day_id: &str,
    season_id: &str,
    competition_id: &str,
    space_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    group_repo: &dyn IGroupRepository,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
) -> Result<GenerateOutcome, GenerateError> {
    let deps = DrawDeps {
        match_day_repo,
        group_repo,
        team_port,
    };
    let mut ctx = load_draw_context(match_day_id, season_id, &deps).await?;
    check_round(&ctx)?;
    let (pools, skipped) = drawable_pools(&ctx);
    let pairings = draw_round(&mut ctx, pools)?;
    let location = PairingLocation {
        competition_id,
        space_id,
        round_id: match_day_id,
    };
    write_round(&ctx, &pairings, location, match_day_repo, event_bus).await?;
    Ok(outcome(skipped, team_port).await)
}

/// R9 — une journée de match vide ; et une saison qui a des équipes à tirer.
/// Dans cet ordre : une journée pourvue se refuse comme telle, même sans poule.
fn check_round(ctx: &DrawContext) -> Result<(), GenerateError> {
    ctx.match_day.can_receive_draw()?;
    if ctx.groups.is_empty() {
        return Err(GenerateError::NoGroups);
    }
    Ok(())
}

async fn outcome(skipped: Skipped, team_port: &dyn ITeamInfoPort) -> GenerateOutcome {
    GenerateOutcome {
        skipped_team_names: resolve_team_names(skipped.team_ids, team_port).await,
        skipped_group_names: skipped.group_names,
    }
}

/// Les poules à deux inscrites ou plus. Les équipes non inscrites et les
/// poules trop petites sont écartées, et signalées.
fn drawable_pools(ctx: &DrawContext) -> (Vec<Vec<TeamId>>, Skipped) {
    let mut pools = Vec::new();
    let mut skipped = Skipped::default();
    for group in &ctx.groups {
        let (kept, dropped) = filter_enrolled_team_ids(&group.team_ids, &ctx.teams);
        skipped.team_ids.extend(dropped);
        if kept.len() < 2 {
            skipped.group_names.push(group.group_name.clone());
            continue;
        }
        pools.push(parse_team_ids(kept.iter()));
    }
    (pools, skipped)
}

/// Tire chaque poule, puis fait accepter le tout par la journée. Aucune poule
/// tirable : rien à tirer, et ce n'est pas une erreur — les poules sont
/// signalées.
fn draw_round(
    ctx: &mut DrawContext,
    pools: Vec<Vec<TeamId>>,
) -> Result<Vec<Pairing>, GenerateError> {
    if pools.is_empty() {
        return Ok(vec![]);
    }
    let inputs: Vec<_> = pools
        .into_iter()
        .map(|teams| pool_input(teams, ctx))
        .collect();
    let mut rng = rand::rngs::StdRng::from_os_rng();
    let proposals = draw_pools(&inputs, &mut rng)?;
    let (pairings, exempted) = to_pairings(&proposals);
    let rules = draw_rules(ctx);
    ctx.match_day
        .accept_draw(pairings.clone(), &exempted, &rules)?;
    Ok(pairings)
}

/// Les rencontres proposées, chacune sous un identifiant neuf, et les
/// exemptées — une par poule impaire.
fn to_pairings(proposals: &[DrawProposal]) -> (Vec<Pairing>, Vec<TeamId>) {
    let pairings = proposals
        .iter()
        .flat_map(|proposal| &proposal.pairings)
        .map(|p| Pairing {
            id: PairingId::new(),
            home_team_id: p.home,
            away_team_id: p.away,
        })
        .collect();
    let exempted = proposals
        .iter()
        .filter_map(|proposal| proposal.exempted)
        .collect();
    (pairings, exempted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::group_repository_port::{
        GroupRepositoryError, GroupWithTeams,
    };
    use crate::app::competitions::domain::match_day::{
        MatchDay, MatchDayName, MatchDayPosition, MatchDayType,
    };
    use crate::app::competitions::domain::match_day_repository_port::{
        NewPairingProjection, PairingDisplayDto,
    };
    use crate::app::competitions::ports::TeamInfoDto;
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
    use async_trait::async_trait;
    use std::collections::HashSet;
    use std::sync::Mutex;

    /// La journée tirée, les journées passées de la saison, et ce que
    /// `save_pairings` a reçu.
    struct FakeMatchDayRepo {
        day: MatchDay,
        past: Vec<MatchDay>,
        saved: Mutex<Vec<Pairing>>,
    }

    impl FakeMatchDayRepo {
        fn of(day: MatchDay) -> Self {
            Self::with_past(day, vec![])
        }

        fn with_past(day: MatchDay, past: Vec<MatchDay>) -> Self {
            Self {
                day,
                past,
                saved: Mutex::new(vec![]),
            }
        }

        fn saved(&self) -> Vec<Pairing> {
            self.saved.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl IMatchDayRepository for FakeMatchDayRepo {
        async fn find_by_season(&self, _: &str) -> Result<Vec<MatchDay>, MatchDayRepositoryError> {
            let mut days = self.past.clone();
            days.push(self.day.clone());
            Ok(days)
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
            let mut saved = self.saved.lock().unwrap();
            saved.extend(pairings.iter().map(|(pairing, _)| pairing.clone()));
            Ok(())
        }
        async fn save_pairing(
            &self,
            _: &str,
            _: &Pairing,
            _: &crate::app::competitions::domain::match_day_repository_port::NewPairingProjection,
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

    struct FakeTeamInfoPort;
    #[async_trait]
    impl ITeamInfoPort for FakeTeamInfoPort {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
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

    struct FakeGroupRepoWithEmptyGroup(&'static str);
    #[async_trait]
    impl IGroupRepository for FakeGroupRepoWithEmptyGroup {
        async fn find_groups(&self, _: &str) -> Result<Vec<GroupWithTeams>, GroupRepositoryError> {
            Ok(vec![GroupWithTeams {
                group_id: "g1".to_string(),
                group_name: self.0.to_string(),
                position: 0,
                team_ids: vec![],
            }])
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

    fn match_day_with_pairings(pairings: Vec<Pairing>) -> MatchDay {
        MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 1".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(0).unwrap(),
            pairings,
        }
    }

    #[tokio::test]
    async fn refuses_when_match_day_already_has_pairings() {
        let existing = Pairing {
            id: PairingId::new(),
            home_team_id: TeamId::try_new("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap(),
            away_team_id: TeamId::try_new("01ARZ3NDEKTSV4RRFFQ69G5FAW").unwrap(),
        };
        let match_day_repo = FakeMatchDayRepo::of(match_day_with_pairings(vec![existing]));
        let group_repo = FakeGroupRepo(vec![]);
        let team_port = FakeTeamInfoPort;
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();

        let result = execute(
            "d1",
            "s1",
            "c1",
            "sp1",
            &match_day_repo,
            &group_repo,
            &team_port,
            &event_bus,
        )
        .await;

        assert!(matches!(result, Err(GenerateError::PairingsAlreadyExist)));
    }

    #[tokio::test]
    async fn proceeds_when_match_day_has_no_pairings() {
        let match_day_repo = FakeMatchDayRepo::of(match_day_with_pairings(vec![]));
        let group_repo = FakeGroupRepo(vec![]);
        let team_port = FakeTeamInfoPort;
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();

        let result = execute(
            "d1",
            "s1",
            "c1",
            "sp1",
            &match_day_repo,
            &group_repo,
            &team_port,
            &event_bus,
        )
        .await;

        // pas de groupes ni d'équipes enrôlées -> NoGroups, mais surtout PAS PairingsAlreadyExist
        assert!(matches!(result, Err(GenerateError::NoGroups)));
    }

    #[tokio::test]
    async fn reports_group_with_fewer_than_two_teams_as_skipped_instead_of_silent_success() {
        let match_day_repo = FakeMatchDayRepo::of(match_day_with_pairings(vec![]));
        let group_repo = FakeGroupRepoWithEmptyGroup("Poule 1");
        let team_port = FakeTeamInfoPortWithEnrolled(vec![TeamInfoDto {
            team_id: "t1".into(),
            team_name: "Team 1".into(),
            coach_id: String::new(),
            coach_name: String::new(),
            roster_name: String::new(),
            logo_url: None,
        }]);
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();

        let outcome = execute(
            "d1",
            "s1",
            "c1",
            "sp1",
            &match_day_repo,
            &group_repo,
            &team_port,
            &event_bus,
        )
        .await
        .expect("ne doit pas échouer, juste signaler la poule ignorée");

        assert_eq!(outcome.skipped_group_names, vec!["Poule 1".to_string()]);
    }

    // ── Le nouveau tirage (épic E18, carte 615) ──────────────────────────────

    fn team(coach: &str) -> TeamInfoDto {
        let id = TeamId::new().to_string();
        TeamInfoDto {
            team_id: id.clone(),
            team_name: format!("Équipe {id}"),
            coach_id: coach.to_string(),
            coach_name: coach.to_string(),
            roster_name: "Humains".to_string(),
            logo_url: None,
        }
    }

    fn id_of(dto: &TeamInfoDto) -> TeamId {
        TeamId::try_new(&dto.team_id).unwrap()
    }

    fn played(home: &TeamInfoDto, away: &TeamInfoDto, position: i32) -> MatchDay {
        let mut day = match_day_with_pairings(vec![Pairing {
            id: PairingId::new(),
            home_team_id: id_of(home),
            away_team_id: id_of(away),
        }]);
        day.position = MatchDayPosition::try_new(position).unwrap();
        day
    }

    fn group(name: &str, teams: &[TeamInfoDto]) -> GroupWithTeams {
        GroupWithTeams {
            group_id: name.to_string(),
            group_name: name.to_string(),
            position: 0,
            team_ids: teams.iter().map(|t| t.team_id.clone()).collect(),
        }
    }

    /// Lance la génération sur la journée du dépôt, et rend ce qui a été écrit.
    async fn generate(
        repo: &FakeMatchDayRepo,
        groups: Vec<GroupWithTeams>,
        teams: Vec<TeamInfoDto>,
    ) -> Vec<Pairing> {
        let round_id = repo.day.id.to_string();
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();
        let (groups, teams) = (FakeGroupRepo(groups), FakeTeamInfoPortWithEnrolled(teams));
        execute(
            &round_id, "s1", "c1", "sp1", repo, &groups, &teams, &event_bus,
        )
        .await
        .expect("la génération réussit");
        repo.saved()
    }

    fn teams_of(pairings: &[Pairing]) -> HashSet<TeamId> {
        pairings
            .iter()
            .flat_map(|p| [p.home_team_id, p.away_team_id])
            .collect()
    }

    /// Le défaut de l'ancien tirage : A-B, A-C et B-C déjà jouées, il
    /// n'appariait que A-D et renvoyait B et C chez elles. Une revanche vaut
    /// mieux qu'une équipe sans match (R5).
    #[tokio::test]
    async fn four_teams_who_met_three_times_all_play() {
        let t: Vec<TeamInfoDto> = ["a", "b", "c", "d"].map(team).to_vec();
        let past = vec![
            played(&t[0], &t[1], 0),
            played(&t[0], &t[2], 1),
            played(&t[1], &t[2], 2),
        ];
        let mut day = match_day_with_pairings(vec![]);
        day.position = MatchDayPosition::try_new(3).unwrap();
        let repo = FakeMatchDayRepo::with_past(day, past);

        let saved = generate(&repo, vec![], t.clone()).await;

        assert_eq!(saved.len(), 2, "deux rencontres, personne sur le banc");
        assert_eq!(teams_of(&saved), t.iter().map(id_of).collect());
    }

    /// R4 — sur trente tirages, les deux équipes du coach « a » ne se
    /// rencontrent jamais.
    #[tokio::test]
    async fn two_teams_of_the_same_coach_never_meet() {
        let t: Vec<TeamInfoDto> = ["a", "a", "b", "c"].map(team).to_vec();
        let forbidden: HashSet<TeamId> = [id_of(&t[0]), id_of(&t[1])].into();
        for _ in 0..30 {
            let repo = FakeMatchDayRepo::of(match_day_with_pairings(vec![]));
            let saved = generate(&repo, vec![], t.clone()).await;
            assert_eq!(saved.len(), 2);
            for p in &saved {
                let pair: HashSet<TeamId> = [p.home_team_id, p.away_team_id].into();
                assert_ne!(pair, forbidden, "deux équipes d'un même coach opposées");
            }
        }
    }

    /// Deux poules de trois : une rencontre et une exemptée chacune, et la
    /// journée accepte les deux exemptées ensemble (R3).
    #[tokio::test]
    async fn each_odd_pool_gets_one_pairing_and_one_exempted_team() {
        let t: Vec<TeamInfoDto> = ["a", "b", "c", "d", "e", "f"].map(team).to_vec();
        let groups = vec![group("Poule 1", &t[..3]), group("Poule 2", &t[3..])];
        let repo = FakeMatchDayRepo::of(match_day_with_pairings(vec![]));

        let saved = generate(&repo, groups, t.clone()).await;

        assert_eq!(saved.len(), 2);
        let first: HashSet<TeamId> = t[..3].iter().map(id_of).collect();
        for p in &saved {
            let same_pool = first.contains(&p.home_team_id) == first.contains(&p.away_team_id);
            assert!(same_pool, "une rencontre entre deux poules");
        }
    }
}

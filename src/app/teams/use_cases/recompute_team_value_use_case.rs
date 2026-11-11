use crate::app::teams::domain::team::TeamDomainEvent;
use crate::app::teams::ports::{
    IJourneymanTypePort, IRosterCatalogPort, ISquadPort, ITeamRepository, RepositoryError,
};
use crate::app::teams::use_cases::team_value_service::resolve_team_value;

/// Combien de fois le recalcul est tenté avant d'abandonner (carte 664).
///
/// Une tentative ne perd que si une autre écriture sur la même équipe s'est
/// glissée entre sa lecture et son ajout. Les déclencheurs connus vont **par
/// deux** — un événement domaine de `teams` et l'annonce de `players` qui le
/// suit — : la deuxième tentative, qui relit l'agrégat après l'écriture
/// concurrente, passe donc d'ordinaire. La troisième laisse place à un
/// troisième écrivain simultané (un geste du coach au même instant). Au-delà,
/// ce n'est plus une course mais un défaut, et il doit se voir dans le journal
/// plutôt que de tourner en boucle.
const MAX_ATTEMPTS: u32 = 3;

#[derive(Debug)]
pub enum RecomputeTeamValueError {
    TeamNotFound,
    Repository(String),
}

/// Recalcule la valeur d'une équipe et l'appende en valeur absolue.
///
/// L'événement est appendu **même si la valeur n'a pas changé** : la suite des
/// `TeamValueRecomputed` est l'historique de progression de la TV sur la saison,
/// information qu'aucune autre trace ne porte.
///
/// Sur une écriture concurrente, l'agrégat est relu et la valeur recalculée,
/// au plus `MAX_ATTEMPTS` fois : c'est la concurrence optimiste ordinaire. Le
/// recalcul est idempotent et part toujours de l'état relu — rien n'est perdu
/// à le refaire, tout l'est à l'abandonner.
#[tracing::instrument(skip_all, fields(team_id = ?team_id))]
pub async fn execute(
    team_id: &str,
    repo: &dyn ITeamRepository,
    squad_port: &dyn ISquadPort,
    roster_catalog_port: &dyn IRosterCatalogPort,
    journeyman_type_port: &dyn IJourneymanTypePort,
) -> Result<(), RecomputeTeamValueError> {
    let ports = Ports {
        repo,
        squad_port,
        roster_catalog_port,
        journeyman_type_port,
    };
    let mut attempt = 1;
    loop {
        match recompute_once(team_id, &ports).await {
            Err(Attempt::Conflict) if attempt < MAX_ATTEMPTS => {
                announce_retry(attempt);
                attempt += 1;
            }
            Err(Attempt::Conflict) => return Err(exhausted()),
            Err(Attempt::Failed(e)) => return Err(e),
            Ok(()) => return Ok(()),
        }
    }
}

/// Les quatre ports, portés ensemble d'une tentative à l'autre.
struct Ports<'a> {
    repo: &'a dyn ITeamRepository,
    squad_port: &'a dyn ISquadPort,
    roster_catalog_port: &'a dyn IRosterCatalogPort,
    journeyman_type_port: &'a dyn IJourneymanTypePort,
}

/// L'issue d'une tentative : un conflit se retente, le reste sort tel quel.
enum Attempt {
    Conflict,
    Failed(RecomputeTeamValueError),
}

/// Une tentative : relire l'agrégat, recalculer, appender à la version lue.
async fn recompute_once(team_id: &str, ports: &Ports<'_>) -> Result<(), Attempt> {
    let team = ports
        .repo
        .find_by_id(team_id)
        .await
        .map_err(|e| Attempt::Failed(RecomputeTeamValueError::Repository(e.to_string())))?
        .ok_or(Attempt::Failed(RecomputeTeamValueError::TeamNotFound))?;

    let value = resolve_team_value(
        &team,
        ports.squad_port,
        ports.roster_catalog_port,
        ports.journeyman_type_port,
    )
    .await;

    let event = TeamDomainEvent::TeamValueRecomputed { value };
    match ports.repo.append(team_id, &event, team.version).await {
        Ok(_) => Ok(()),
        Err(RepositoryError::ConcurrentWrite) => Err(Attempt::Conflict),
        Err(e) => Err(Attempt::Failed(RecomputeTeamValueError::Repository(
            e.to_string(),
        ))),
    }
}

/// Émise sous `kreek::app::teams::use_cases::…`, dans le span du use case qui
/// porte déjà `team_id`. Le libellé évite à dessein « écriture concurrente
/// détectée » : cette chaîne-là reste réservée à l'échec définitif, celui
/// qu'on recherche dans `docker logs`.
fn announce_retry(attempt: u32) {
    tracing::warn!(
        attempt,
        max_attempts = MAX_ATTEMPTS,
        "recalcul de la valeur d'équipe : conflit de version, relecture et nouvelle tentative"
    );
}

/// L'épuisement garde le libellé de `RepositoryError::ConcurrentWrite` : la
/// recherche de production (carte 664) le retrouve avant comme après le
/// correctif. Le listener le journalise en `error`.
fn exhausted() -> RecomputeTeamValueError {
    RecomputeTeamValueError::Repository(format!(
        "{} ({MAX_ATTEMPTS} tentatives épuisées)",
        RepositoryError::ConcurrentWrite
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RosterId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::staff_counts::{
        ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
    use crate::app::teams::domain::value_objects::{DedicatedFans, Kpo, RosterName, TeamName};
    use crate::app::teams::ports::JourneymanTypeDto;
    use crate::app::teams::use_cases::test_doubles::{
        FakeRosterCatalogPort, FakeSquadPort, FakeTeamRepository, PIETAILLE,
    };

    const TEAM: &str = "00000000000000000000000001";

    struct FakeJourneyman;

    impl IJourneymanTypePort for FakeJourneyman {
        fn journeyman_type_for_roster(&self, _: &str) -> JourneymanTypeDto {
            JourneymanTypeDto {
                position_name: "Piétaille des Carrières".into(),
                price_kpo: 50,
            }
        }
    }

    fn equipe() -> Vec<TeamDomainEvent> {
        vec![TeamDomainEvent::TeamCreated {
            team_id: TeamId::try_new(TEAM).unwrap(),
            space_id: SpaceId::try_new("00000000000000000000000002").unwrap(),
            competition_id: CompetitionId::try_new("00000000000000000000000003").unwrap(),
            competition_name: "Ligue de Condate".into(),
            season_id: SeasonId::try_new("00000000000000000000000004").unwrap(),
            season_name: "Saison 2025".into(),
            name: TeamName::try_new("Les Korrigans FC".to_string()).unwrap(),
            logo_url: None,
            roster_id: RosterId::try_new("00000000000000000000000005").unwrap(),
            roster_name: RosterName::try_new("Nains du Granit".to_string()).unwrap(),
            coach_id: CoachId::try_new("00000000000000000000000006").unwrap(),
            coach_name: "Colonel Castor".into(),
            treasury: Kpo(0),
            dedicated_fans: DedicatedFans::try_new(2).unwrap(),
            rerolls: RerollCount(0),
            apothecaries: ApothecaryCount(0),
            assistants: AssistantCount(0),
            cheerleaders: CheerleaderCount(0),
        }]
    }

    async fn recalculer(repo: &FakeTeamRepository) -> Result<(), RecomputeTeamValueError> {
        let squad = FakeSquadPort(vec![PIETAILLE, PIETAILLE]);
        execute(TEAM, repo, &squad, &FakeRosterCatalogPort, &FakeJourneyman).await
    }

    fn valeurs_ecrites(repo: &FakeTeamRepository) -> Vec<Kpo> {
        repo.appended()
            .into_iter()
            .filter_map(|e| match e {
                TeamDomainEvent::TeamValueRecomputed { value } => Some(value),
                _ => None,
            })
            .collect()
    }

    /// La référence : sans conflit, une valeur est écrite du premier coup.
    #[tokio::test]
    async fn team_value_is_written_without_conflict() {
        let repo = FakeTeamRepository::with_events(equipe());
        recalculer(&repo).await.expect("aucun conflit");
        assert_eq!(valeurs_ecrites(&repo).len(), 1);
    }

    /// Le cas de la CI du 2026-11-01 : le premier ajout perd la course, la
    /// relecture le rattrape, et la valeur est bien écrite — la même que sans
    /// conflit.
    #[tokio::test]
    async fn team_value_is_written_after_a_concurrent_write() {
        let sans_conflit = FakeTeamRepository::with_events(equipe());
        recalculer(&sans_conflit).await.unwrap();

        let repo = FakeTeamRepository::en_conflit(equipe(), 1);
        recalculer(&repo)
            .await
            .expect("la deuxième tentative passe");

        assert_eq!(valeurs_ecrites(&repo), valeurs_ecrites(&sans_conflit));
    }

    /// La dernière tentative permise passe encore.
    #[tokio::test]
    async fn team_value_is_written_on_the_last_allowed_attempt() {
        let repo = FakeTeamRepository::en_conflit(equipe(), (MAX_ATTEMPTS - 1) as u8);
        recalculer(&repo)
            .await
            .expect("la dernière tentative passe");
        assert_eq!(valeurs_ecrites(&repo).len(), 1);
    }

    /// Un dépôt qui refuse toujours : l'erreur sort après `MAX_ATTEMPTS`
    /// essais, pas un de plus — le compteur restant le prouve — et rien n'est
    /// écrit. Son libellé garde « écriture concurrente détectée », que la
    /// recherche de production retrouve.
    #[tokio::test]
    async fn the_error_comes_out_after_max_attempts() {
        let repo = FakeTeamRepository::en_conflit(equipe(), u8::MAX);

        let e = recalculer(&repo)
            .await
            .expect_err("le dépôt refuse toujours");

        let RecomputeTeamValueError::Repository(message) = e else {
            panic!("attendu Repository, reçu {e:?}");
        };
        assert!(
            message.contains("écriture concurrente détectée"),
            "{message}"
        );
        assert_eq!(
            *repo.conflits_a_simuler.lock().unwrap(),
            u8::MAX - MAX_ATTEMPTS as u8,
            "exactement {MAX_ATTEMPTS} ajouts tentés"
        );
        assert!(valeurs_ecrites(&repo).is_empty());
    }

    /// Une équipe absente n'est pas un conflit : elle sort au premier essai.
    #[tokio::test]
    async fn a_missing_team_is_not_retried() {
        let repo = FakeTeamRepository::en_conflit(Vec::new(), u8::MAX);
        let e = recalculer(&repo).await.expect_err("aucune équipe");
        assert!(matches!(e, RecomputeTeamValueError::TeamNotFound));
        assert_eq!(*repo.conflits_a_simuler.lock().unwrap(), u8::MAX);
    }
}

//! Équipes de test pour la couche web : une équipe hydratée pour les tests
//! d'affichage (événement copié de `team_detail.rs::created_event`), et une
//! équipe semée en base par le vrai dépôt pour les tests de routeur.

use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RosterId, SeasonId};
use crate::app::shared_kernel::bloodbowl::staff_counts::{
    ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::shared_kernel::identity::sulid::SUlid;
use crate::app::teams::domain::team::{Team, TeamDomainEvent};
use crate::app::teams::domain::value_objects::{DedicatedFans, Kpo, RosterName, TeamName};
use crate::app::teams::ports::ITeamRepository;

pub fn equipe_de_test() -> Team {
    Team::hydrate(&[TeamDomainEvent::TeamCreated {
        team_id: TeamId::try_new("00000000000000000000000001").unwrap(),
        space_id: SpaceId::try_new("00000000000000000000000002").unwrap(),
        competition_id: CompetitionId::try_new("00000000000000000000000003").unwrap(),
        competition_name: "Ligue de Condate".to_string(),
        season_id: SeasonId::try_new("00000000000000000000000004").unwrap(),
        season_name: "Saison 2025".to_string(),
        name: TeamName::try_new("Les Korrigans FC".to_string()).unwrap(),
        logo_url: None,
        roster_id: RosterId::try_new("00000000000000000000000005").unwrap(),
        roster_name: RosterName::try_new("Elfes Sylvestres".to_string()).unwrap(),
        coach_id: CoachId::try_new("00000000000000000000000006").unwrap(),
        coach_name: "Colonel Castor".to_string(),
        treasury: Kpo(1000),
        dedicated_fans: DedicatedFans::try_new(2).unwrap(),
        rerolls: RerollCount(3),
        apothecaries: ApothecaryCount(1),
        assistants: AssistantCount(2),
        cheerleaders: CheerleaderCount(3),
    }])
    .unwrap()
}

/// Une équipe à 1000 kPo dans l'espace E2E, semée **par le dépôt** : une équipe
/// posée en projection seule rendrait `404`, et les refus se liraient alors
/// pour une raison étrangère.
pub async fn equipe_semee(pool: &sqlx::PgPool) -> (String, String) {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");

    let (space_id,): (String,) =
        sqlx::query_as("SELECT id FROM spaces WHERE space_name = 'Espace E2E'")
            .fetch_one(pool)
            .await
            .expect("espace E2E semé");
    let (proprietaire,): (String,) =
        sqlx::query_as("SELECT id FROM auth__users WHERE coach_name = 'E2E Coach 02'")
            .fetch_one(pool)
            .await
            .expect("coach propriétaire semé");

    let team_id = SUlid::new().to_string();
    let repo = crate::app::teams::io::repository::team_repository::TeamRepository::new(
        pool.clone(),
        crate::common::services::event_bus::event_bus::new_bus(),
    );
    repo.append(
        &team_id,
        &TeamDomainEvent::TeamCreated {
            team_id: TeamId::try_new(&team_id).unwrap(),
            space_id: SpaceId::try_new(&space_id).unwrap(),
            competition_id: CompetitionId::try_new(&SUlid::new().to_string()).unwrap(),
            competition_name: "Ligue de Condate".to_string(),
            season_id: SeasonId::try_new(&SUlid::new().to_string()).unwrap(),
            season_name: "Saison 2025".to_string(),
            name: TeamName::try_new("Les Korrigans FC".to_string()).unwrap(),
            logo_url: None,
            roster_id: RosterId::try_new(&SUlid::new().to_string()).unwrap(),
            roster_name: RosterName::try_new("Elfes Sylvestres".to_string()).unwrap(),
            coach_id: CoachId::try_new(&proprietaire).unwrap(),
            coach_name: "E2E Coach 02".to_string(),
            treasury: Kpo(1000),
            dedicated_fans: DedicatedFans::try_new(2).unwrap(),
            rerolls: RerollCount(3),
            apothecaries: ApothecaryCount(1),
            assistants: AssistantCount(2),
            cheerleaders: CheerleaderCount(3),
        },
        0,
    )
    .await
    .expect("équipe semée par le dépôt");

    (space_id, team_id)
}

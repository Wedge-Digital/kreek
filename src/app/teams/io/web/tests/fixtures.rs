//! Une équipe hydratée, pour les tests de la couche web qui n'ont besoin que
//! de ses champs d'affichage. Événement copié de `team_detail.rs::created_event`.

use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RosterId, SeasonId};
use crate::app::shared_kernel::bloodbowl::staff_counts::{
    ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::teams::domain::team::{Team, TeamDomainEvent};
use crate::app::teams::domain::value_objects::{DedicatedFans, Kpo, RosterName, TeamName};

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

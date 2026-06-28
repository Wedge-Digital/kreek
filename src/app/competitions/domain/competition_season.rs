use crate::app::shared_kernel::common_types::{CompetitionId, SeasonId};
use crate::app::shared_kernel::season_name::SeasonName;

pub struct CompetitionSeason {
    pub id: SeasonId,
    pub competition_id: CompetitionId,
    pub name: SeasonName,
}

impl CompetitionSeason {
    pub fn new(competition_id: CompetitionId, name: SeasonName) -> Self {
        Self {
            id: SeasonId::new(),
            competition_id,
            name,
        }
    }
}

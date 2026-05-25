use crate::app::shared_kernel::common_types::{CompetitionId, SeasonId};

pub struct CompetitionSeason {
    pub id:             SeasonId,
    pub competition_id: CompetitionId,
    pub name:           String,
}

impl CompetitionSeason {
    pub fn new(competition_id: CompetitionId, name: String) -> Self {
        Self { id: SeasonId::new(), competition_id, name }
    }
}
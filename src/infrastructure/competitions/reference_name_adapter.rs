use crate::app::competitions::ports::ICompetitionReferencePort;
use crate::app::references::domain::port::IReferenceRepository;
use std::sync::Arc;

pub struct ReferenceNameAdapter {
    reference_repo: Arc<dyn IReferenceRepository>,
}

impl ReferenceNameAdapter {
    pub fn new(reference_repo: Arc<dyn IReferenceRepository>) -> Self {
        Self { reference_repo }
    }
}

impl ICompetitionReferencePort for ReferenceNameAdapter {
    fn find_inducement_name(&self, uid: &str) -> Option<String> {
        self.reference_repo
            .find_inducement_by_uid(uid)
            .map(|i| i.name.clone())
    }

    fn find_star_player_name(&self, uid: &str) -> Option<String> {
        self.reference_repo
            .find_star_player_by_uid(uid)
            .map(|s| s.name.clone())
    }

    /// Un roster est une `Team` au corpus : le mot diffère, la chose est la
    /// même — le modèle d'équipe dont les coachs héritent leur effectif.
    fn find_roster_name(&self, uid: &str) -> Option<String> {
        self.reference_repo
            .find_team_by_uid(uid)
            .map(|t| t.name.clone())
    }
}

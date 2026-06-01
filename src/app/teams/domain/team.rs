/// Agrégat Team — event sourcé.
/// Implémentation complète en carte 28.
#[derive(Debug, Clone)]
pub struct Team {
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParticipationStatus {
    PendingEnrollment,
    Enrolled,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GamePhase {
    ReadyToPlay,
    PlayerImprovement,
    Recruitment,
    Dismissals,
    TemporaryRetirement,
    OffSeason,
}

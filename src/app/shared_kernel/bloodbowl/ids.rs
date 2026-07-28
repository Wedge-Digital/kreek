//! Identifiants propres au métier Blood Bowl. Ils restent des `EntityId` du
//! noyau d'identité : le métier dépend du noyau, jamais l'inverse.
use crate::app::shared_kernel::identity::ids::EntityId;
use std::fmt::Display;

pub type CompetitionId = EntityId;

pub type SeasonId = EntityId;

pub type ArticleId = EntityId;

pub type CommentId = EntityId;

pub type PlayerId = EntityId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RosterId(pub String);

impl RosterId {
    pub fn try_new(s: &str) -> Result<Self, ()> {
        Ok(RosterId(s.to_string()))
    }
}

impl Display for RosterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub type PositionId = EntityId;

pub type MatchReportId = EntityId;

pub type RoundId = EntityId;

pub type MatchId = EntityId;

pub type PairingId = EntityId;

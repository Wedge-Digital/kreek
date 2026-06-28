use crate::app::shared_kernel::sulid::SUlid;
use nutype::nutype;
use std::fmt::Display;

pub type CoachId = EntityId;

pub type UserId = EntityId;

pub type SpaceId = EntityId;

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

pub type EntityId = SUlid;

pub type EventId = SUlid;

pub trait Entity: PartialEq<Self> {
    fn get_id(&self) -> EntityId;

    fn get_created_by(&self) -> EntityId;

    fn eq(&self, other: &Self) -> bool {
        self.get_id() == other.get_id()
    }
}

#[nutype(
    sanitize(trim),
    validate(regex = r"^https://res\.cloudinary\.com/[a-zA-Z0-9_-]+/.+$"),
    derive(Debug, Clone, Serialize, Deserialize, PartialEq, AsRef)
)]
pub struct CloudinaryImage(String);

impl Display for CloudinaryImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl CloudinaryImage {
    pub fn thumbnail(&self, w: u32, h: u32) -> String {
        crate::app::shared_kernel::cloudinary::transform(
            self.as_ref(),
            &format!("c_fill,w_{w},h_{h},q_auto,f_auto"),
        )
    }
}

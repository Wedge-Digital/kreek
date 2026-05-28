use std::fmt::Display;
use crate::app::shared_kernel::sulid::SUlid;
use nutype::nutype;

pub type CoachId = EntityId;

pub type UserId = EntityId;

pub type SpaceId = EntityId;

pub type CompetitionId = EntityId;

pub type SeasonId = EntityId;

pub type ArticleId = EntityId;

pub type CommentId = EntityId;

pub type EntityId = SUlid;

pub type EventId = SUlid;

pub trait Entity:PartialEq<Self> {
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

impl Display for SUlid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::charset::TEXTE_SAISI;
use nutype::nutype;

/// Le nom d'une journée — son propre type, cf. [`SeasonName`].
#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = TEXTE_SAISI),
    derive(
        Debug,
        Clone,
        Serialize,
        Deserialize,
        PartialEq,
        Eq,
        Hash,
        Display,
        AsRef
    )
)]
pub struct MatchDayName(String);

#[nutype(
    validate(greater_or_equal = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, Display)
)]
pub struct MatchDayPosition(i32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchDayType {
    FixedDate,
    TimeFrame,
    Rest,
}

impl MatchDayType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FixedDate => "fixed_date",
            Self::TimeFrame => "time_frame",
            Self::Rest => "rest",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "fixed_date" => Self::FixedDate,
            "rest" => Self::Rest,
            _ => Self::TimeFrame,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pairing {
    pub id: PairingId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
}

#[derive(Debug, Clone)]
pub struct MatchDay {
    pub id: MatchId,
    pub season_id: SeasonId,
    pub name: MatchDayName,
    pub day_type: MatchDayType,
    pub date_start: Option<DateString>,
    pub date_end: Option<DateString>,
    pub position: MatchDayPosition,
    pub pairings: Vec<Pairing>,
}

impl MatchDay {
    pub fn is_rest(&self) -> bool {
        self.day_type == MatchDayType::Rest
    }
}

use crate::app::competitions::domain::match_day::MatchDayName;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ranking_group_id::RankingGroupId;
use crate::app::shared_kernel::identity::charset::TEXTE_SAISI;
use nutype::nutype;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UseRankingGroups(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UsePlayoffsPhase(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FinalPhaseMatchForThirdPlace(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UseSchedule(pub bool);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionStructure {
    pub ranking_group: RankingGroupConfig,
    pub play_offs_phase: PlayOffsPhase,
    pub schedule: ScheduleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DispatchType {
    Automatic,
    Manual,
    #[serde(other)]
    Unknown,
}

impl Default for DispatchType {
    fn default() -> Self {
        Self::Automatic
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingGroupConfig {
    pub use_ranking_groups: UseRankingGroups,
    #[serde(default)]
    pub dispatch_type: DispatchType,
    pub ranking_groups: Vec<RankingGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingGroup {
    pub id: RankingGroupId,
    pub name: RankingGroupName,
}

/// Le nom d'un groupe de classement.
///
/// Il portait un `NameVo` nu — le type générique que quatre autres noms
/// partageaient. Lui donner le sien est ce que demande la règle « pas de type
/// primitif nu » du `CLAUDE.md`, un cran au-dessus : un `String` validé qui
/// désigne n'importe quel nom n'est guère mieux qu'un `String`.
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
pub struct RankingGroupName(String);

#[nutype(
    validate(less_or_equal = 100),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)
)]
pub struct QualifiedTeamPerPool(u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayOffsPhase {
    pub use_playoffs_phase: UsePlayoffsPhase,
    pub qualified_team_per_pool: QualifiedTeamPerPool,
    pub final_phase_match_for_third_place: FinalPhaseMatchForThirdPlace,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleType {
    #[default]
    Unknown,
    FixedDate,
    TimeFrame,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub use_schedule: UseSchedule,
    #[serde(default)]
    pub schedule_type: ScheduleType,
    #[serde(default)]
    pub schedule_start_date: DateString,
    #[serde(default)]
    pub play_off_start_date: DateString,
    #[serde(default)]
    pub play_off_end_date: DateString,
    #[serde(default)]
    pub schedule_end_date: DateString,
    pub scheduled_dates: Vec<ScheduledDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScheduledDate {
    FixedDate {
        name: MatchDayName,
        multiplexe_date: DateString,
    },
    TimeFrame {
        name: MatchDayName,
        start_date: DateString,
        end_date: DateString,
    },
}

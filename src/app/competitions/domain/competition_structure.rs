use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionStructure {
    pub ranking_group:   RankingGroupConfig,
    pub play_offs_phase: PlayOffsPhase,
    pub schedule:        ScheduleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingGroupConfig {
    pub use_ranking_groups: bool,
    pub dispatch_type:      String,
    pub ranking_groups:     Vec<RankingGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingGroup {
    pub id:   String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayOffsPhase {
    pub use_playoffs_phase:               bool,
    pub qualified_team_per_pool:          u32,
    pub final_phase_match_for_third_place: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub use_schedule:          bool,
    pub schedule_type:         String,
    pub schedule_start_date:   String,
    pub play_off_start_date:   String,
    pub play_off_end_date:     String,
    pub schedule_end_date:     String,
    pub schedule_timezone:     String,
    pub use_mail_notification: bool,
    pub scheduled_dates:       Vec<ScheduledDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScheduledDate {
    FixedDate {
        name:            String,
        multiplexe_date: String,
    },
    TimeFrame {
        name:       String,
        start_date: String,
        end_date:   String,
    },
}
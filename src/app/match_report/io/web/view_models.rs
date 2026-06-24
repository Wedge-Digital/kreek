pub struct CompetitionOptionVm {
    pub id: String,
    pub name: String,
    pub selected: bool,
}

pub struct SeasonOptionVm {
    pub id: String,
    pub name: String,
    pub selected: bool,
}

pub struct RoundOptionVm {
    pub id: String,
    pub name: String,
    pub dates: String,
    pub selected: bool,
}

pub struct TeamOptionVm {
    pub id: String,
    pub name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub tv: String,
    pub is_own_team: bool,
}

pub struct SelectedMatchVm {
    pub match_report_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
}

#[derive(Clone, Copy)]
pub enum UserRoleVm {
    Admin,
    Coach,
}

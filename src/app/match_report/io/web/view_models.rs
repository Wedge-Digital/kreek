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

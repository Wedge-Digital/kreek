use crate::app::team_creation::common_types::{BaseTeamInfo, CloudinaryImage, CoachId, TeamName, UserId};
use crate::app::team_creation::team_draft::DraftTeam;
use crate::services::id_service::IdService;

pub fn create_draft_team<T: IdService>(
    id_service: &T,
    creator_id: UserId,
    team_name: TeamName,
    coach_id: CoachId,
    logo_url: Option<CloudinaryImage>) -> DraftTeam {
    let base_infos: BaseTeamInfo = BaseTeamInfo::new(team_name, coach_id, logo_url);
    let new_draft_team = DraftTeam::new(id_service, creator_id, base_infos);
    return new_draft_team;
}
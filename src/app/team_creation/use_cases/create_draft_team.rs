use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, UserId};
use crate::app::shared_kernel::id_service::IdService;
use crate::app::shared_kernel::team::{BaseTeamInfo, TeamName};
use crate::app::team_creation::domain::team_draft::DraftTeam;

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
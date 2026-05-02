// #[derive(Debug)]
// pub enum RegisterNewTeamError {
//     RepositoryError(RepositoryError),
// }
// 
// pub struct RegisterNewTeamUseCase {
//     id_service: Arc<dyn IdService + Send + Sync>,
//     team_repository: Arc<dyn TeamRepository>,
// }
// 
// impl RegisterNewTeamUseCase {
//     pub fn new(
//         id_service: Arc<dyn IdService + Send + Sync>,
//         team_repository: Arc<dyn TeamRepository>,
//     ) -> Self {
//         RegisterNewTeamUseCase { id_service, team_repository }
//     }
// 
//     pub async fn execute(&self, command: RegisterNewTeamCommand) -> Result<(), RegisterNewTeamError> {
//         let team = create_draft_team(
//             self.id_service.as_ref(),
//             command.created_by,
//             command.team_name,
//             command.coach_id,
//             command.logo_url,
//         );
// 
//         self.team_repository
//             .save_draft(&team)
//             .await
//             .map_err(RegisterNewTeamError::RepositoryError)
//     }
// }
// #[derive(Debug)]
// pub enum SelectRulesetError {
//     TeamNotFound,
//     DomainError(DomainError),
//     RepositoryError(RepositoryError),
// }
//
// pub struct SelectRulesetUseCase {
//     team_repository: Arc<dyn TeamRepository>,
// }
//
// impl SelectRulesetUseCase {
//     pub fn new(team_repository: Arc<dyn TeamRepository>) -> Self {
//         SelectRulesetUseCase { team_repository }
//     }
//
//     pub async fn execute(&self, command: SelectRulesetCommand) -> Result<(), SelectRulesetError> {
//         // let draft_team = self.team_repository
//         //     .find_draft(&command.team_id)
//         //     .await
//         //     .map_err(SelectRulesetError::RepositoryError)?
//         //     .ok_or(SelectRulesetError::TeamNotFound)?;
//         //
//         // let ruleset_team = draft_team
//         //     .select_ruleset(command.ruleset)
//         //     .map_err(SelectRulesetError::DomainError)?;
//         //
//         // self.team_repository
//         //     .save_ruleset_selected(&ruleset_team)
//         //     .await
//         //     .map_err(SelectRulesetError::RepositoryError)
//     }
// }
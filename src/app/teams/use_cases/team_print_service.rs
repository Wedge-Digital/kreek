//! Les lectures de l'impression d'une équipe (épic E16, cartes 595 à 597).
//!
//! **Aucune mutation.** Ce sont des services de lecture : ils chargent l'agrégat
//! et ce que les ports savent de lui, et laissent la couche web composer ses
//! vues. Le seul jugement qu'ils portent — l'équipe s'imprime-t-elle ? — est
//! demandé à l'agrégat (`Team::is_printable`, R1).

use crate::app::teams::domain::team::Team;
use crate::app::teams::ports::{
    IRosterCatalogPort, ISquadPort, ITeamRepository, RosterCatalogDto, SquadMemberDto,
};

#[derive(Debug, PartialEq, Eq)]
pub enum TeamPrintError {
    TeamNotFound,
    /// L'équipe existe mais n'est pas prête à jouer : la page l'explique.
    NotPrintable {
        team_name: String,
    },
    Repository(String),
}

/// Ce qu'il faut pour rendre l'en-tête d'une fiche imprimée.
pub struct SheetHeadData {
    pub team: Team,
    /// `None` si le corpus ne connaît pas le roster : les prix valent alors
    /// zéro, plutôt qu'une fiche qui refuse de s'afficher.
    pub catalog: Option<RosterCatalogDto>,
    pub squad: Vec<SquadMemberDto>,
}

// arch:no-instrument — service de lecture : charge une équipe pour l'impression, sans intention métier
pub async fn load(repo: &dyn ITeamRepository, team_id: &str) -> Result<Team, TeamPrintError> {
    match repo.find_by_id(team_id).await {
        Ok(Some(team)) => Ok(team),
        Ok(None) => Err(TeamPrintError::TeamNotFound),
        Err(e) => Err(TeamPrintError::Repository(e.to_string())),
    }
}

/// L'équipe, si elle s'imprime (R1).
// arch:no-instrument — service de lecture : vérifie qu'une équipe s'imprime, sans intention métier
pub async fn load_printable(
    repo: &dyn ITeamRepository,
    team_id: &str,
) -> Result<Team, TeamPrintError> {
    let team = load(repo, team_id).await?;
    if team.is_printable() {
        Ok(team)
    } else {
        Err(TeamPrintError::NotPrintable {
            team_name: team.name.to_string(),
        })
    }
}

// arch:no-instrument — service de lecture : assemble l'en-tête d'une fiche imprimée
pub async fn load_sheet_head(
    repo: &dyn ITeamRepository,
    catalog_port: &dyn IRosterCatalogPort,
    squad_port: &dyn ISquadPort,
    team_id: &str,
) -> Result<SheetHeadData, TeamPrintError> {
    let team = load(repo, team_id).await?;
    let catalog = catalog_port.find_catalog(&team.roster_id.to_string());
    let squad = squad_port.find_squad(team_id).await;
    Ok(SheetHeadData {
        team,
        catalog,
        squad,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::teams::use_cases::test_doubles::FakeTeamRepository;

    #[tokio::test]
    async fn une_equipe_inconnue_rend_team_not_found() {
        let repo = FakeTeamRepository::default();
        assert_eq!(
            load_printable(&repo, "01JZZZZZZZZZZZZZZZZZZZZZZZ")
                .await
                .err(),
            Some(TeamPrintError::TeamNotFound)
        );
    }
}

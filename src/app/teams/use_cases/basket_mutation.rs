//! Les trois mutations du panier — ajouter un joueur, ajouter du staff, retirer
//! une ligne.
//!
//! Elles ont la même forme et **ne décident de rien** : charger l'équipe,
//! hydrater le panier, appeler la méthode domaine, persister avec garde de
//! version. Quotas, limites croisées, plafond d'effectif et trésorerie sont
//! évalués par l'agrégat, jamais ici.

use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::recruitment_basket::{
    BasketLineId, RecruitmentBasket, RosterLineId,
};
use crate::app::teams::domain::team::GamePhase;
use crate::app::teams::ports::{
    IPhaseBasketRepository, IRosterCatalogPort, ISquadPort, ITeamRepository, PhaseBasketState,
    RepositoryError,
};
use crate::app::teams::use_cases::basket_hydration_service::{
    hydrate_recruitment_basket, HydrationError,
};
use crate::app::teams::use_cases::commands::{
    AddBasketPlayerCommand, AddBasketStaffCommand, RemoveBasketLineCommand,
};

#[derive(Debug)]
pub enum BasketMutationError {
    TeamNotFound,
    WrongPhase(Option<GamePhase>),
    /// Un autre onglet a modifié le panier entre son affichage et cette action.
    ConcurrentWrite,
    Domain(DomainError),
    Hydration(HydrationError),
    Repository(RepositoryError),
}

impl From<RepositoryError> for BasketMutationError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::ConcurrentWrite => Self::ConcurrentWrite,
            autre => Self::Repository(autre),
        }
    }
}

/// Charge l'équipe, vérifie qu'elle est bien dans la phase attendue, et hydrate
/// son panier de recrutement.
async fn ouvrir_panier(
    team_id: &str,
    team_repo: &dyn ITeamRepository,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog: &dyn IRosterCatalogPort,
    squad: &dyn ISquadPort,
) -> Result<RecruitmentBasket, BasketMutationError> {
    let team = team_repo
        .find_by_id(team_id)
        .await?
        .ok_or(BasketMutationError::TeamNotFound)?;

    if team.game_phase != Some(GamePhase::Recruitment) {
        return Err(BasketMutationError::WrongPhase(team.game_phase.clone()));
    }

    hydrate_recruitment_basket(&team, basket_repo, catalog, squad)
        .await
        .map_err(BasketMutationError::Hydration)
}

async fn persister(
    basket: &RecruitmentBasket,
    space_id: &str,
    phase: GamePhase,
    basket_repo: &dyn IPhaseBasketRepository,
    expected_version: u32,
) -> Result<u32, BasketMutationError> {
    let state = serde_json::to_value(basket.lines())
        .map_err(|e| BasketMutationError::Repository(RepositoryError::Serialization(e)))?;

    let etat = PhaseBasketState {
        team_id: basket.team_id().to_string(),
        space_id: space_id.to_string(),
        phase,
        state,
        version: expected_version,
    };
    Ok(basket_repo.save(&etat, expected_version).await?)
}

pub async fn add_player(
    cmd: AddBasketPlayerCommand,
    space_id: &str,
    team_repo: &dyn ITeamRepository,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog: &dyn IRosterCatalogPort,
    squad: &dyn ISquadPort,
) -> Result<RecruitmentBasket, BasketMutationError> {
    let team_id = cmd.team_id.to_string();
    let mut basket = ouvrir_panier(&team_id, team_repo, basket_repo, catalog, squad).await?;

    basket
        .add_player(RosterLineId(cmd.roster_line_id))
        .map_err(BasketMutationError::Domain)?;

    persister(
        &basket,
        space_id,
        GamePhase::Recruitment,
        basket_repo,
        cmd.expected_version,
    )
    .await?;
    Ok(basket)
}

pub async fn add_staff(
    cmd: AddBasketStaffCommand,
    space_id: &str,
    team_repo: &dyn ITeamRepository,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog: &dyn IRosterCatalogPort,
    squad: &dyn ISquadPort,
) -> Result<RecruitmentBasket, BasketMutationError> {
    let team_id = cmd.team_id.to_string();
    let mut basket = ouvrir_panier(&team_id, team_repo, basket_repo, catalog, squad).await?;

    basket
        .add_staff(cmd.staff_type)
        .map_err(BasketMutationError::Domain)?;

    persister(
        &basket,
        space_id,
        GamePhase::Recruitment,
        basket_repo,
        cmd.expected_version,
    )
    .await?;
    Ok(basket)
}

/// Retrait d'une ligne — **partagé avec les renvois**.
///
/// La phase ne sert qu'à savoir quel panier ouvrir : l'opération elle-même est
/// la même. Le retrait passe par l'agrégat plutôt que par un filtrage du JSON
/// persisté, pour qu'il n'existe qu'une seule expression de cette règle et de
/// son erreur.
pub async fn remove_line(
    cmd: RemoveBasketLineCommand,
    space_id: &str,
    team_repo: &dyn ITeamRepository,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog: &dyn IRosterCatalogPort,
    squad: &dyn ISquadPort,
) -> Result<RecruitmentBasket, BasketMutationError> {
    match cmd.phase {
        GamePhase::Recruitment => {}
        // `DismissalsBasket` naît avec la carte 267 ; c'est elle qui ajoutera
        // ce bras. L'erreur est explicite plutôt que silencieuse : mieux vaut
        // un appel qui échoue bruyamment qu'un retrait qui ne retire rien.
        autre => return Err(BasketMutationError::WrongPhase(Some(autre))),
    }

    let team_id = cmd.team_id.to_string();
    let mut basket = ouvrir_panier(&team_id, team_repo, basket_repo, catalog, squad).await?;

    basket
        .remove_line(&BasketLineId(cmd.line_id))
        .map_err(BasketMutationError::Domain)?;

    persister(
        &basket,
        space_id,
        cmd.phase,
        basket_repo,
        cmd.expected_version,
    )
    .await?;
    Ok(basket)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RosterId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::staff_counts::{
        ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
    use crate::app::teams::domain::team::TeamDomainEvent;
    use crate::app::teams::domain::value_objects::{
        DedicatedFans, Kpo, RosterName, StaffType, TeamName,
    };
    use crate::app::teams::use_cases::test_doubles::{
        FakeBasketRepository, FakeRosterCatalogPort, FakeSquadPort, FakeTeamRepository, PIETAILLE,
    };

    const TEAM: &str = "00000000000000000000000001";
    const SPACE: &str = "00000000000000000000000002";

    fn team_id() -> TeamId {
        TeamId::try_new(TEAM).unwrap()
    }

    fn events(phase: GamePhase) -> Vec<TeamDomainEvent> {
        let admin = CoachId::try_new("00000000000000000000000006").unwrap();
        vec![
            TeamDomainEvent::TeamCreated {
                team_id: team_id(),
                space_id: SpaceId::try_new(SPACE).unwrap(),
                competition_id: CompetitionId::try_new("00000000000000000000000003").unwrap(),
                competition_name: "Ligue de Condate".into(),
                season_id: SeasonId::try_new("00000000000000000000000004").unwrap(),
                season_name: "Saison 2025".into(),
                name: TeamName::try_new("Les Korrigans FC".to_string()).unwrap(),
                logo_url: None,
                roster_id: RosterId::try_new("00000000000000000000000005").unwrap(),
                roster_name: RosterName::try_new("Nains du Granit".to_string()).unwrap(),
                coach_id: admin.clone(),
                coach_name: "Colonel Castor".into(),
                treasury: Kpo(1000),
                dedicated_fans: DedicatedFans::try_new(2).unwrap(),
                rerolls: RerollCount(0),
                apothecaries: ApothecaryCount(0),
                assistants: AssistantCount(0),
                cheerleaders: CheerleaderCount(0),
            },
            TeamDomainEvent::GamePhaseOverridden {
                admin_id: admin,
                from_phase: Some(GamePhase::ReadyToPlay),
                to_phase: phase,
                reason: None,
            },
        ]
    }

    /// Un panier neuf part de la version zéro — celle que `save` attend pour
    /// créer la ligne.
    #[tokio::test]
    async fn ajouter_un_joueur_persiste_la_ligne_et_incremente_la_version() {
        let teams = FakeTeamRepository::with_events(events(GamePhase::Recruitment));
        let baskets = FakeBasketRepository::default();
        let cmd = AddBasketPlayerCommand {
            team_id: team_id(),
            roster_line_id: PIETAILLE.to_string(),
            expected_version: 0,
        };

        let panier = add_player(
            cmd,
            SPACE,
            &teams,
            &baskets,
            &FakeRosterCatalogPort,
            &FakeSquadPort::empty(),
        )
        .await
        .unwrap();

        assert_eq!(panier.lines().len(), 1);
        let persiste = baskets.state.lock().unwrap().clone().unwrap();
        assert_eq!(persiste.version, 1);
        assert_eq!(persiste.phase, GamePhase::Recruitment);
    }

    /// Le use case ne connaît pas la liste des staffs autorisés : c'est le
    /// panier qui refuse, sur son catalogue.
    #[tokio::test]
    async fn un_staff_hors_catalogue_est_refuse_par_le_domaine() {
        let teams = FakeTeamRepository::with_events(events(GamePhase::Recruitment));
        let baskets = FakeBasketRepository::default();
        let cmd = AddBasketStaffCommand {
            team_id: team_id(),
            staff_type: StaffType::Assistant, // absent d'`allowed_staff`
            expected_version: 0,
        };

        let erreur = add_staff(
            cmd,
            SPACE,
            &teams,
            &baskets,
            &FakeRosterCatalogPort,
            &FakeSquadPort::empty(),
        )
        .await
        .expect_err("le roster n'y a pas droit");

        assert!(
            matches!(erreur, BasketMutationError::Domain(_)),
            "{erreur:?}"
        );
        assert!(
            baskets.state.lock().unwrap().is_none(),
            "un refus n'écrit rien"
        );
    }

    /// Hors de la phase, le panier n'existe pas — il ne suffit pas de refuser à
    /// l'écran, le use case garde la porte.
    #[tokio::test]
    async fn muter_hors_phase_de_recrutement_est_refuse() {
        let teams = FakeTeamRepository::with_events(events(GamePhase::Dismissals));
        let baskets = FakeBasketRepository::default();
        let cmd = RemoveBasketLineCommand {
            team_id: team_id(),
            phase: GamePhase::Recruitment,
            line_id: "line-1".into(),
            expected_version: 0,
        };

        let erreur = remove_line(
            cmd,
            SPACE,
            &teams,
            &baskets,
            &FakeRosterCatalogPort,
            &FakeSquadPort::empty(),
        )
        .await
        .expect_err("l'équipe est en renvois");

        assert!(matches!(
            erreur,
            BasketMutationError::WrongPhase(Some(GamePhase::Dismissals))
        ));
    }
}

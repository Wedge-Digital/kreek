//! Les mutations de panier des deux phases — ajouter, marquer, retirer.
//!
//! Elles ont la même forme et **ne décident de rien** : charger l'équipe,
//! hydrater le panier, appeler la méthode domaine, persister avec garde de
//! version. Quotas, limites croisées, plafond d'effectif, trésorerie et plancher
//! des éligibles sont évalués par l'agrégat, jamais ici.
//!
//! Aucune ne rend l'agrégat muté. Ce serait un cadeau empoisonné : `save` rend
//! la nouvelle version sans la reposer sur l'agrégat, dont le champ `version`
//! reste celui d'avant écriture. Un appelant qui le cuirait dans les `hx-vals`
//! du prochain geste ferait échouer chaque second clic en écriture concurrente
//! — le piège de la carte 264, qui ne se voit qu'en navigateur. Les handlers
//! relisent, et c'est la seule façon correcte.

use crate::app::teams::domain::basket::{BasketLineId, RosterLineId};
use crate::app::teams::domain::dismissals_basket::DismissalsBasket;
use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::recruitment_basket::RecruitmentBasket;
use crate::app::teams::domain::team::{GamePhase, Team};
use crate::app::teams::ports::{
    IPhaseBasketRepository, IRosterCatalogPort, ISquadPort, ITeamRepository, PhaseBasketState,
    RepositoryError,
};
use crate::app::teams::use_cases::basket_hydration_service::{
    hydrate_dismissals_basket, hydrate_recruitment_basket, HydrationError,
};
use crate::app::teams::use_cases::commands::{
    AddBasketPlayerCommand, AddBasketStaffCommand, MarkPlayerForDismissalCommand,
    MarkStaffForDismissalCommand, RemoveBasketLineCommand,
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

/// Charge l'équipe et vérifie qu'elle est bien dans la phase attendue.
///
/// Hors de sa phase, un panier n'existe pas : il ne suffit pas de désactiver le
/// bouton, le use case garde la porte.
async fn charger_en_phase(
    team_id: &str,
    phase: GamePhase,
    team_repo: &dyn ITeamRepository,
) -> Result<Team, BasketMutationError> {
    let team = team_repo
        .find_by_id(team_id)
        .await?
        .ok_or(BasketMutationError::TeamNotFound)?;

    if team.game_phase != Some(phase) {
        return Err(BasketMutationError::WrongPhase(team.game_phase.clone()));
    }
    Ok(team)
}

async fn ouvrir_panier_recrutement(
    team_id: &str,
    team_repo: &dyn ITeamRepository,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog: &dyn IRosterCatalogPort,
    squad: &dyn ISquadPort,
) -> Result<RecruitmentBasket, BasketMutationError> {
    let team = charger_en_phase(team_id, GamePhase::Recruitment, team_repo).await?;
    hydrate_recruitment_basket(&team, basket_repo, catalog, squad)
        .await
        .map_err(BasketMutationError::Hydration)
}

async fn ouvrir_panier_renvois(
    team_id: &str,
    team_repo: &dyn ITeamRepository,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog: &dyn IRosterCatalogPort,
    squad: &dyn ISquadPort,
) -> Result<DismissalsBasket, BasketMutationError> {
    let team = charger_en_phase(team_id, GamePhase::Dismissals, team_repo).await?;
    hydrate_dismissals_basket(&team, basket_repo, catalog, squad)
        .await
        .map_err(BasketMutationError::Hydration)
}

/// Persiste les lignes d'un panier, quelles qu'elles soient.
///
/// Seules les lignes sont écrites — catalogue, effectif et trésorerie sont
/// rechargés à chaque hydratation, pour qu'un vieux panier soit évalué contre
/// l'état du jour.
async fn persister<L: serde::Serialize>(
    team_id: &str,
    lines: &[L],
    space_id: &str,
    phase: GamePhase,
    basket_repo: &dyn IPhaseBasketRepository,
    expected_version: u32,
) -> Result<(), BasketMutationError> {
    let state = serde_json::to_value(lines)
        .map_err(|e| BasketMutationError::Repository(RepositoryError::Serialization(e)))?;

    let etat = PhaseBasketState {
        team_id: team_id.to_string(),
        space_id: space_id.to_string(),
        phase,
        state,
        version: expected_version,
    };
    basket_repo.save(&etat, expected_version).await?;
    Ok(())
}

pub async fn add_player(
    cmd: AddBasketPlayerCommand,
    space_id: &str,
    team_repo: &dyn ITeamRepository,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog: &dyn IRosterCatalogPort,
    squad: &dyn ISquadPort,
) -> Result<(), BasketMutationError> {
    let team_id = cmd.team_id.to_string();
    let mut basket =
        ouvrir_panier_recrutement(&team_id, team_repo, basket_repo, catalog, squad).await?;

    basket
        .add_player(RosterLineId(cmd.roster_line_id))
        .map_err(BasketMutationError::Domain)?;

    persister(
        &team_id,
        basket.lines(),
        space_id,
        GamePhase::Recruitment,
        basket_repo,
        cmd.expected_version,
    )
    .await
}

pub async fn add_staff(
    cmd: AddBasketStaffCommand,
    space_id: &str,
    team_repo: &dyn ITeamRepository,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog: &dyn IRosterCatalogPort,
    squad: &dyn ISquadPort,
) -> Result<(), BasketMutationError> {
    let team_id = cmd.team_id.to_string();
    let mut basket =
        ouvrir_panier_recrutement(&team_id, team_repo, basket_repo, catalog, squad).await?;

    basket
        .add_staff(cmd.staff_type)
        .map_err(BasketMutationError::Domain)?;

    persister(
        &team_id,
        basket.lines(),
        space_id,
        GamePhase::Recruitment,
        basket_repo,
        cmd.expected_version,
    )
    .await
}

// ── Renvois ───────────────────────────────────────────────────────────────────

/// Marquer, et non retirer : le joueur reste dans l'effectif et compte encore
/// dans le plancher des éligibles jusqu'à la validation du lot. C'est ce qui
/// rend l'annulation gratuite.
pub async fn mark_player(
    cmd: MarkPlayerForDismissalCommand,
    space_id: &str,
    team_repo: &dyn ITeamRepository,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog: &dyn IRosterCatalogPort,
    squad: &dyn ISquadPort,
) -> Result<(), BasketMutationError> {
    let team_id = cmd.team_id.to_string();
    let mut basket =
        ouvrir_panier_renvois(&team_id, team_repo, basket_repo, catalog, squad).await?;

    basket
        .mark_player(cmd.player_id)
        .map_err(BasketMutationError::Domain)?;

    persister(
        &team_id,
        basket.lines(),
        space_id,
        GamePhase::Dismissals,
        basket_repo,
        cmd.expected_version,
    )
    .await
}

pub async fn mark_staff(
    cmd: MarkStaffForDismissalCommand,
    space_id: &str,
    team_repo: &dyn ITeamRepository,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog: &dyn IRosterCatalogPort,
    squad: &dyn ISquadPort,
) -> Result<(), BasketMutationError> {
    let team_id = cmd.team_id.to_string();
    let mut basket =
        ouvrir_panier_renvois(&team_id, team_repo, basket_repo, catalog, squad).await?;

    basket
        .mark_staff(cmd.staff_type)
        .map_err(BasketMutationError::Domain)?;

    persister(
        &team_id,
        basket.lines(),
        space_id,
        GamePhase::Dismissals,
        basket_repo,
        cmd.expected_version,
    )
    .await
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
) -> Result<(), BasketMutationError> {
    let team_id = cmd.team_id.to_string();
    let ligne = BasketLineId(cmd.line_id);

    // Les deux paniers sont des types distincts : le partage est celui de
    // l'opération et de son erreur, pas d'un agrégat commun. Chaque bras est
    // court parce que tout ce qui l'entoure est mutualisé.
    match cmd.phase {
        GamePhase::Recruitment => {
            let mut basket =
                ouvrir_panier_recrutement(&team_id, team_repo, basket_repo, catalog, squad).await?;
            basket
                .remove_line(&ligne)
                .map_err(BasketMutationError::Domain)?;
            persister(
                &team_id,
                basket.lines(),
                space_id,
                GamePhase::Recruitment,
                basket_repo,
                cmd.expected_version,
            )
            .await
        }
        GamePhase::Dismissals => {
            let mut basket =
                ouvrir_panier_renvois(&team_id, team_repo, basket_repo, catalog, squad).await?;
            basket
                .remove_line(&ligne)
                .map_err(BasketMutationError::Domain)?;
            persister(
                &team_id,
                basket.lines(),
                space_id,
                GamePhase::Dismissals,
                basket_repo,
                cmd.expected_version,
            )
            .await
        }
        // Les autres phases n'ont pas de panier. L'erreur est explicite plutôt
        // que silencieuse : mieux vaut un appel qui échoue bruyamment qu'un
        // retrait qui ne retire rien.
        autre => Err(BasketMutationError::WrongPhase(Some(autre))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, PlayerId, RosterId, SeasonId};
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

        add_player(
            cmd,
            SPACE,
            &teams,
            &baskets,
            &FakeRosterCatalogPort,
            &FakeSquadPort::empty(),
        )
        .await
        .unwrap();

        let persiste = baskets.state.lock().unwrap().clone().unwrap();
        assert_eq!(persiste.state.as_array().unwrap().len(), 1);
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

    // ── Renvois ───────────────────────────────────────────────────────────

    fn id_de(n: u8) -> PlayerId {
        PlayerId::try_new(&format!("{n:0>26}")).unwrap()
    }

    /// Un effectif de `n` joueurs disponibles.
    fn effectif(n: usize) -> FakeSquadPort {
        FakeSquadPort(vec![PIETAILLE; n])
    }

    #[tokio::test]
    async fn marquer_un_joueur_persiste_la_ligne() {
        let teams = FakeTeamRepository::with_events(events(GamePhase::Dismissals));
        let baskets = FakeBasketRepository::default();
        let cmd = MarkPlayerForDismissalCommand {
            team_id: team_id(),
            player_id: id_de(0),
            expected_version: 0,
        };

        mark_player(
            cmd,
            SPACE,
            &teams,
            &baskets,
            &FakeRosterCatalogPort,
            &effectif(13),
        )
        .await
        .unwrap();

        let persiste = baskets.state.lock().unwrap().clone().unwrap();
        assert_eq!(persiste.phase, GamePhase::Dismissals);
        assert_eq!(persiste.version, 1);
        assert_eq!(persiste.state.as_array().unwrap().len(), 1);
    }

    /// Le use case ne connaît pas le plancher des onze : c'est le panier qui
    /// refuse, sur l'effectif qu'il porte.
    #[tokio::test]
    async fn le_plancher_est_refuse_par_le_domaine_et_rien_n_est_ecrit() {
        let teams = FakeTeamRepository::with_events(events(GamePhase::Dismissals));
        let baskets = FakeBasketRepository::default();
        let cmd = MarkPlayerForDismissalCommand {
            team_id: team_id(),
            player_id: id_de(0),
            expected_version: 0,
        };

        let erreur = mark_player(
            cmd,
            SPACE,
            &teams,
            &baskets,
            &FakeRosterCatalogPort,
            &effectif(11),
        )
        .await
        .expect_err("onze éligibles, le plancher mord");

        assert!(matches!(
            erreur,
            BasketMutationError::Domain(DomainError::EligibleFloorReached)
        ));
        assert!(
            baskets.state.lock().unwrap().is_none(),
            "un refus n'écrit rien"
        );
    }

    /// L'équipe ne possède aucun assistant : marquer le renvoi d'un staff qu'on
    /// n'a pas est refusé par le panier.
    #[tokio::test]
    async fn marquer_un_staff_non_possede_est_refuse() {
        let teams = FakeTeamRepository::with_events(events(GamePhase::Dismissals));
        let baskets = FakeBasketRepository::default();
        let cmd = MarkStaffForDismissalCommand {
            team_id: team_id(),
            staff_type: StaffType::Assistant,
            expected_version: 0,
        };

        let erreur = mark_staff(
            cmd,
            SPACE,
            &teams,
            &baskets,
            &FakeRosterCatalogPort,
            &effectif(13),
        )
        .await
        .expect_err("aucun assistant au vestiaire");

        assert!(matches!(
            erreur,
            BasketMutationError::Domain(DomainError::InsufficientStaff)
        ));
    }

    #[tokio::test]
    async fn marquer_hors_phase_de_renvois_est_refuse() {
        let teams = FakeTeamRepository::with_events(events(GamePhase::Recruitment));
        let baskets = FakeBasketRepository::default();
        let cmd = MarkPlayerForDismissalCommand {
            team_id: team_id(),
            player_id: id_de(0),
            expected_version: 0,
        };

        let erreur = mark_player(
            cmd,
            SPACE,
            &teams,
            &baskets,
            &FakeRosterCatalogPort,
            &effectif(13),
        )
        .await
        .expect_err("l'équipe est en recrutement");

        assert!(matches!(
            erreur,
            BasketMutationError::WrongPhase(Some(GamePhase::Recruitment))
        ));
    }

    /// Le retrait est partagé : le même use case démarque, sur le panier que la
    /// phase désigne. C'est ce que la carte 263 annonçait et que la 267 a rendu
    /// possible.
    #[tokio::test]
    async fn demarquer_passe_par_le_meme_use_case_que_le_retrait_de_recrutement() {
        let teams = FakeTeamRepository::with_events(events(GamePhase::Dismissals));
        let baskets = FakeBasketRepository::default();

        mark_player(
            MarkPlayerForDismissalCommand {
                team_id: team_id(),
                player_id: id_de(0),
                expected_version: 0,
            },
            SPACE,
            &teams,
            &baskets,
            &FakeRosterCatalogPort,
            &effectif(13),
        )
        .await
        .unwrap();

        let ligne = baskets.state.lock().unwrap().clone().unwrap().state[0]["Player"]["id"]
            .as_str()
            .unwrap()
            .to_string();

        remove_line(
            RemoveBasketLineCommand {
                team_id: team_id(),
                phase: GamePhase::Dismissals,
                line_id: ligne,
                expected_version: 1,
            },
            SPACE,
            &teams,
            &baskets,
            &FakeRosterCatalogPort,
            &effectif(13),
        )
        .await
        .unwrap();

        let persiste = baskets.state.lock().unwrap().clone().unwrap();
        assert_eq!(persiste.state.as_array().unwrap().len(), 0);
        assert_eq!(persiste.version, 2);
    }
}

//! Applique le panier de renvois et clôt la phase.
//!
//! Deux étapes, et elles sont nommées : `appliquer_les_renvois` produit les
//! événements de sortie, `cloturer_la_phase` ajoute la transition. Les séparer
//! rend visible la couture que le jour où l'ordre des phases deviendra
//! configurable il faudra ouvrir — étant entendu que cet ordre est aujourd'hui
//! écrit dans `Team::apply`, pas ici.
//!
//! **Mais une seule écriture.** Les deux étapes alimentent le même
//! `append_batch`, et ce n'est pas une commodité : si les renvois étaient
//! appendus puis la transition séparément, un échec entre les deux laisserait
//! l'équipe en phase `Dismissals` avec des joueurs déjà sortis. Or
//! `dismiss_player` ne vérifie que la phase — une revalidation les renverrait
//! une seconde fois. C'est la transition-en-dernier-du-même-lot qui ferme ce
//! trou.

use crate::app::teams::domain::basket::RejectedLine;
use crate::app::teams::domain::dismissals_basket::{DismissalAppliedLine, DismissalsBasket};
use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::team::{GamePhase, Team, TeamDomainEvent};
use crate::app::teams::domain::value_objects::StaffQuantity;
use crate::app::teams::ports::{
    IPhaseBasketRepository, IRosterCatalogPort, ISquadPort, ITeamRepository, RepositoryError,
};
use crate::app::teams::use_cases::basket_hydration_service::{
    hydrate_dismissals_basket, HydrationError,
};
use crate::app::teams::use_cases::commands::ValidateDismissalsPhaseCommand;

#[derive(Debug)]
pub enum ValidateDismissalsPhaseError {
    TeamNotFound,
    /// Le panier ne passe plus contre l'effectif du jour. Les lignes fautives
    /// sont nommées avec leur cause **structurée** : c'est la couche web qui
    /// formule, pas ce use case.
    BasketNoLongerValid(Vec<RejectedLine>),
    Domain(DomainError),
    Hydration(HydrationError),
    Repository(RepositoryError),
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ValidateDismissalsPhaseCommand,
    team_repo: &dyn ITeamRepository,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog: &dyn IRosterCatalogPort,
    squad: &dyn ISquadPort,
) -> Result<ValidateDismissalsOutcome, ValidateDismissalsPhaseError> {
    let team_id = cmd.team_id.to_string();
    let team = team_repo
        .find_by_id(&team_id)
        .await
        .map_err(ValidateDismissalsPhaseError::Repository)?
        .ok_or(ValidateDismissalsPhaseError::TeamNotFound)?;

    let basket = hydrate_dismissals_basket(&team, basket_repo, catalog, squad)
        .await
        .map_err(ValidateDismissalsPhaseError::Hydration)?;

    let lot = appliquer_les_renvois(&team, &basket)?;
    let lot = cloturer_la_phase(&team, lot)?;
    // L'issue se lit sur le dernier événement du lot : c'est `cloturer_la_phase`
    // qui a tranché, en interrogeant le domaine.
    let issue = ValidateDismissalsOutcome::depuis_le_lot(&lot);

    team_repo
        .append_batch(&team_id, &lot, team.version)
        .await
        .map_err(ValidateDismissalsPhaseError::Repository)?;

    // Hors transaction, et c'est sûr pour la même raison qu'au recrutement : le
    // dernier événement du lot fait passer l'équipe en `ReadyToPlay`, donc une
    // revalidation échoue sur `expect_phase(Dismissals)`. Un panier résiduel
    // serait de toute façon purgé à cette même entrée en `ReadyToPlay`.
    basket_repo
        .delete(&team_id, &GamePhase::Dismissals)
        .await
        .map_err(ValidateDismissalsPhaseError::Repository)?;

    Ok(issue)
}

/// Où l'équipe se retrouve après la validation.
///
/// Le contrôleur en a besoin : une équipe au-dessus du seuil doit être menée à
/// l'écran du jet (carte 410), les autres retournent à leur fiche.
#[derive(Debug, PartialEq, Eq)]
pub enum ValidateDismissalsOutcome {
    /// Trésorerie sous le seuil : l'équipe est prête à jouer.
    PreteAJouer,
    /// Un jet lui reste à faire.
    ErreursCouteuses,
}

impl ValidateDismissalsOutcome {
    fn depuis_le_lot(lot: &[TeamDomainEvent]) -> Self {
        match lot.last() {
            Some(TeamDomainEvent::CostlyMistakesPhaseStarted) => Self::ErreursCouteuses,
            _ => Self::PreteAJouer,
        }
    }
}

/// Étape 1 — un événement **par ligne**, jamais un événement de lot : l'event
/// store reste lisible, « ce joueur a été renvoyé tel jour ».
///
/// Aucune vérification de trésorerie : rien n'entre, rien ne sort. `validate_all`
/// n'a contrôlé que le plancher des éligibles et la possession du staff.
///
/// L'ordre est libre, et le lot se construit depuis un `Team` qui n'avance pas :
/// la possession du staff ayant été vérifiée en cumul par le panier, aucune
/// ligne ne peut échouer en cours de lot faute d'unités.
fn appliquer_les_renvois(
    team: &Team,
    basket: &DismissalsBasket,
) -> Result<Vec<TeamDomainEvent>, ValidateDismissalsPhaseError> {
    let applied = basket
        .validate_all()
        .map_err(ValidateDismissalsPhaseError::BasketNoLongerValid)?;

    let mut events = Vec::with_capacity(applied.len() + 1);
    for ligne in applied {
        let event = match ligne {
            DismissalAppliedLine::Player {
                player_id,
                value_at_dismissal,
            } => team.dismiss_player(player_id, value_at_dismissal),
            DismissalAppliedLine::Staff { staff_type } => team.dismiss_staff(
                staff_type,
                StaffQuantity::try_new(1).expect("une ligne de panier vaut une unité"),
            ),
        };
        events.push(event.map_err(ValidateDismissalsPhaseError::Domain)?);
    }
    Ok(events)
}

/// Étape 2 — la transition, **en dernier**.
///
/// Sa position n'est pas cosmétique : c'est elle qui rend la double application
/// impossible. Un test la vérifie explicitement.
fn cloturer_la_phase(
    team: &Team,
    mut lot: Vec<TeamDomainEvent>,
) -> Result<Vec<TeamDomainEvent>, ValidateDismissalsPhaseError> {
    lot.push(
        team.validate_dismissals_phase()
            .map_err(ValidateDismissalsPhaseError::Domain)?,
    );
    Ok(lot)
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
    use crate::app::teams::domain::basket::BasketLineId;
    use crate::app::teams::domain::dismissals_basket::DismissalBasketLine;
    use crate::app::teams::domain::treasury::MovementReason;
    use crate::app::teams::domain::value_objects::{
        DedicatedFans, Kpo, RosterName, StaffType, TeamName,
    };
    use crate::app::teams::ports::PhaseBasketState;
    use crate::app::teams::use_cases::test_doubles::{
        FakeBasketRepository, FakeRosterCatalogPort, FakeSquadPort, FakeTeamRepository, PIETAILLE,
    };

    const TEAM: &str = "00000000000000000000000001";
    const SPACE: &str = "00000000000000000000000002";

    fn team_id() -> TeamId {
        TeamId::try_new(TEAM).unwrap()
    }

    fn id_de(n: u8) -> PlayerId {
        PlayerId::try_new(&format!("{n:0>26}")).unwrap()
    }

    /// Une équipe en phase de renvois, avec un apothicaire au vestiaire.
    fn events() -> Vec<TeamDomainEvent> {
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
                apothecaries: ApothecaryCount(1),
                assistants: AssistantCount(0),
                cheerleaders: CheerleaderCount(0),
            },
            TeamDomainEvent::GamePhaseOverridden {
                admin_id: admin,
                from_phase: Some(GamePhase::ReadyToPlay),
                to_phase: GamePhase::Dismissals,
                reason: None,
            },
        ]
    }

    /// Un effectif de `n` joueurs disponibles, tous à la même ligne de roster.
    fn effectif(n: usize) -> FakeSquadPort {
        FakeSquadPort(vec![PIETAILLE; n])
    }

    fn panier_persiste(lignes: Vec<DismissalBasketLine>) -> PhaseBasketState {
        PhaseBasketState {
            team_id: TEAM.to_string(),
            space_id: SPACE.to_string(),
            phase: GamePhase::Dismissals,
            state: serde_json::to_value(lignes).unwrap(),
            version: 1,
        }
    }

    fn ligne_joueur(n: u8) -> DismissalBasketLine {
        DismissalBasketLine::Player {
            id: BasketLineId(format!("ligne-{n}")),
            player_id: id_de(n),
        }
    }

    async fn valider(
        teams: &FakeTeamRepository,
        baskets: &FakeBasketRepository,
        squad: &FakeSquadPort,
    ) -> Result<ValidateDismissalsOutcome, ValidateDismissalsPhaseError> {
        execute(
            ValidateDismissalsPhaseCommand { team_id: team_id() },
            teams,
            baskets,
            &FakeRosterCatalogPort,
            squad,
        )
        .await
    }

    /// Le cas que la bannière déclenche aujourd'hui : le coach clôt la phase
    /// sans avoir rien marqué.
    #[tokio::test]
    async fn panier_vide_appende_la_seule_transition() {
        let teams = FakeTeamRepository::with_events(events());
        let baskets = FakeBasketRepository::default();

        valider(&teams, &baskets, &effectif(12)).await.unwrap();

        let appendus = teams.appended();
        assert_eq!(appendus.len(), 1);
        // L'équipe de test a 1000 kPo : depuis la carte 408, la transition est
        // celle des erreurs coûteuses. Ce test porte sur la **forme du lot** —
        // une seule transition, aucun renvoi — pas sur laquelle des deux.
        assert!(matches!(
            appendus[0],
            TeamDomainEvent::CostlyMistakesPhaseStarted
        ));
    }

    /// L'issue est ce dont le contrôleur a besoin pour savoir où mener le
    /// coach — l'écran du jet arrive avec la carte 410.
    ///
    /// Les trois tests voisins ont échoué à l'introduction de la nouvelle
    /// transition : ils affirmaient une issue qui était devenue conditionnelle
    /// sans que rien ne l'exprime. Celui-ci la nomme.
    #[tokio::test]
    async fn l_issue_dit_ou_l_equipe_se_retrouve() {
        let teams = FakeTeamRepository::with_events(events());
        let baskets = FakeBasketRepository::default();

        let issue = valider(&teams, &baskets, &effectif(12)).await.unwrap();

        assert_eq!(
            issue,
            ValidateDismissalsOutcome::ErreursCouteuses,
            "l'équipe de test a 1000 kPo en caisse"
        );
    }

    #[tokio::test]
    async fn un_renvoi_donne_un_evenement_par_ligne_et_la_transition_en_dernier() {
        let teams = FakeTeamRepository::with_events(events());
        let baskets =
            FakeBasketRepository::with(panier_persiste(vec![ligne_joueur(0), ligne_joueur(1)]));

        valider(&teams, &baskets, &effectif(14)).await.unwrap();

        let appendus = teams.appended();
        assert_eq!(appendus.len(), 3, "deux renvois plus la transition");
        assert!(matches!(
            appendus[0],
            TeamDomainEvent::PlayerDismissed { .. }
        ));
        assert!(matches!(
            appendus[1],
            TeamDomainEvent::PlayerDismissed { .. }
        ));
        assert!(
            matches!(appendus[2], TeamDomainEvent::CostlyMistakesPhaseStarted),
            "la transition doit clore le lot — c'est elle qui interdit la double application"
        );
    }

    /// Le lot n'est écrit qu'une fois : c'est cette atomicité qui rend sûre la
    /// suppression du panier hors transaction.
    #[tokio::test]
    async fn le_lot_est_ecrit_en_une_seule_fois() {
        let teams = FakeTeamRepository::with_events(events());
        let baskets = FakeBasketRepository::with(panier_persiste(vec![ligne_joueur(0)]));

        valider(&teams, &baskets, &effectif(13)).await.unwrap();

        assert_eq!(teams.batch_count(), 1);
        assert_eq!(baskets.deleted(), vec![GamePhase::Dismissals]);
    }

    /// La garde anti-double-application, vérifiée plutôt que supposée : la
    /// première validation pose `ReadyToPlay`, la seconde échoue sur la phase.
    #[tokio::test]
    async fn revalider_apres_succes_echoue_sur_la_phase() {
        let mut historique = events();
        historique.push(TeamDomainEvent::DismissalsPhaseValidated);
        let teams = FakeTeamRepository::with_events(historique);
        let baskets = FakeBasketRepository::default();

        let erreur = valider(&teams, &baskets, &effectif(12))
            .await
            .expect_err("la phase est close");

        assert!(
            matches!(
                erreur,
                ValidateDismissalsPhaseError::Domain(DomainError::WrongGamePhase(_))
            ),
            "{erreur:?}"
        );
        assert!(teams.appended().is_empty(), "un refus n'écrit rien");
    }

    /// Refus en bloc : sous le plancher, aucune ligne n'est appliquée.
    #[tokio::test]
    async fn un_panier_sous_le_plancher_est_refuse_en_bloc() {
        let teams = FakeTeamRepository::with_events(events());
        let baskets =
            FakeBasketRepository::with(panier_persiste(vec![ligne_joueur(0), ligne_joueur(1)]));

        // Douze éligibles : le premier renvoi passe, le second tombe sous onze.
        let erreur = valider(&teams, &baskets, &effectif(12))
            .await
            .expect_err("le plancher mord");

        match erreur {
            ValidateDismissalsPhaseError::BasketNoLongerValid(lignes) => {
                assert_eq!(lignes.len(), 1);
                assert_eq!(lignes[0].cause, DomainError::EligibleFloorReached);
            }
            autre => panic!("attendu BasketNoLongerValid, obtenu {autre:?}"),
        }
        assert!(teams.appended().is_empty(), "rien n'est appliqué");
        assert!(baskets.deleted().is_empty(), "le panier survit au refus");
    }

    /// « Un renvoi ne rembourse rien » — vérifié sur les deux événements.
    ///
    /// Le contraste avec un achat est dans le test : sans lui, un
    /// `treasury_movement` qui retournerait `None` pour *tout* passerait pour
    /// une confirmation de la règle.
    #[test]
    fn aucun_renvoi_ne_produit_de_mouvement_de_tresorerie() {
        let team = Team::hydrate(&events()).unwrap();

        let renvoi_joueur = team.dismiss_player(id_de(0), Kpo(50)).unwrap();
        let renvoi_staff = team
            .dismiss_staff(StaffType::Apothecary, StaffQuantity::try_new(1).unwrap())
            .unwrap();

        assert!(team.treasury_movement(&renvoi_joueur).is_none());
        assert!(team.treasury_movement(&renvoi_staff).is_none());

        let achat = TeamDomainEvent::StaffBought {
            staff_type: StaffType::Apothecary,
            quantity: StaffQuantity::try_new(1).unwrap(),
            cost_kpo: Kpo(50),
        };
        let mouvement = team
            .treasury_movement(&achat)
            .expect("un achat, lui, débite bien");
        assert_eq!(mouvement.reason, MovementReason::StaffPurchase);

        // Le compteur baisse malgré tout : c'est le staff qui part, pas l'argent
        // qui revient.
        assert_eq!(team.apply(&renvoi_staff).apothecaries.0, 0);
    }
}

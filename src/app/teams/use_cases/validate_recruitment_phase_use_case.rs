use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
use crate::app::teams::domain::basket::RejectedLine;
use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::recruitment_basket::AppliedLine;
use crate::app::teams::domain::team::{GamePhase, TeamDomainEvent};
use crate::app::teams::domain::value_objects::StaffQuantity;
use crate::app::teams::ports::{
    IPhaseBasketRepository, IRosterCatalogPort, ISquadPort, ITeamRepository, RepositoryError,
};
use crate::app::teams::use_cases::basket_hydration_service::{
    hydrate_recruitment_basket, HydrationError,
};
use crate::app::teams::use_cases::commands::ValidateRecruitmentPhaseCommand;

#[derive(Debug)]
pub enum ValidateRecruitmentPhaseError {
    TeamNotFound,
    /// Le panier ne passe plus contre l'état du jour. Les lignes fautives sont
    /// nommées avec leur cause **structurée** : c'est la couche web qui
    /// formule, pas ce use case.
    BasketNoLongerValid(Vec<RejectedLine>),
    Domain(DomainError),
    Hydration(HydrationError),
    Repository(RepositoryError),
}

/// Applique le panier et clôt la phase de recrutement.
///
/// Le panier est réévalué **contre l'état du jour** — prix, effectif et
/// trésorerie rechargés — et refusé **en bloc** : une seule ligne devenue
/// invalide et rien n'est appliqué, pas même les lignes saines.
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ValidateRecruitmentPhaseCommand,
    team_repo: &dyn ITeamRepository,
    basket_repo: &dyn IPhaseBasketRepository,
    catalog: &dyn IRosterCatalogPort,
    squad: &dyn ISquadPort,
) -> Result<(), ValidateRecruitmentPhaseError> {
    let team_id = cmd.team_id.to_string();
    let team = team_repo
        .find_by_id(&team_id)
        .await
        .map_err(ValidateRecruitmentPhaseError::Repository)?
        .ok_or(ValidateRecruitmentPhaseError::TeamNotFound)?;

    let basket = hydrate_recruitment_basket(&team, basket_repo, catalog, squad)
        .await
        .map_err(ValidateRecruitmentPhaseError::Hydration)?;

    let applied = basket
        .validate_all()
        .map_err(ValidateRecruitmentPhaseError::BasketNoLongerValid)?;

    let events = build_events(&team, applied)?;

    team_repo
        .append_batch(&team_id, &events, team.version)
        .await
        .map_err(ValidateRecruitmentPhaseError::Repository)?;

    // Hors transaction, et c'est sûr : le dernier événement du lot fait passer
    // l'équipe en `Dismissals`, donc une revalidation échoue sur
    // `expect_phase(Recruitment)`. La double application est impossible, et un
    // panier résiduel sera purgé à l'entrée suivante en `ReadyToPlay`.
    basket_repo
        .delete(&team_id, &GamePhase::Recruitment)
        .await
        .map_err(ValidateRecruitmentPhaseError::Repository)?;

    Ok(())
}

/// Un événement **par ligne**, jamais un événement de lot : l'event store reste
/// lisible — « ce joueur a été recruté tel jour » — et le grand livre de
/// trésorerie en découle directement.
///
/// L'ordre est libre : la trésorerie ayant été vérifiée en total par
/// `validate_all`, aucune ligne ne peut échouer en cours de lot faute d'argent.
fn build_events(
    team: &crate::app::teams::domain::team::Team,
    applied: Vec<AppliedLine>,
) -> Result<Vec<TeamDomainEvent>, ValidateRecruitmentPhaseError> {
    let mut events = Vec::with_capacity(applied.len() + 1);

    for ligne in applied {
        let event = match ligne {
            // L'identifiant est frappé ici, pas dans l'agrégat : le domaine
            // reste déterministe, et l'identité devient un fait persisté dès
            // que l'événement est appendu.
            AppliedLine::Player {
                roster_line,
                base_value,
                cost,
            } => team.recruit_player(PlayerId::new(), roster_line, base_value, cost),
            AppliedLine::Staff { staff_type, cost } => team.buy_staff(
                staff_type,
                StaffQuantity::try_new(1).expect("une ligne de panier vaut une unité"),
                cost,
            ),
        };
        events.push(event.map_err(ValidateRecruitmentPhaseError::Domain)?);
    }

    events.push(
        team.validate_recruitment_phase()
            .map_err(ValidateRecruitmentPhaseError::Domain)?,
    );
    Ok(events)
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
    use crate::app::teams::domain::basket::{BasketLineId, RosterLineId};
    use crate::app::teams::domain::recruitment_basket::BasketLine;
    use crate::app::teams::domain::value_objects::{DedicatedFans, Kpo, RosterName, TeamName};
    use crate::app::teams::ports::PhaseBasketState;
    use crate::app::teams::use_cases::test_doubles::{
        FakeBasketRepository, FakeRosterCatalogPort, FakeSquadPort, FakeTeamRepository, PERCUTEUR,
        PIETAILLE,
    };

    const TEAM: &str = "00000000000000000000000001";

    fn team_id() -> TeamId {
        TeamId::try_new(TEAM).unwrap()
    }

    /// Une équipe en phase de recrutement, avec la trésorerie demandée.
    fn events_en_recrutement(treasury: u32) -> Vec<TeamDomainEvent> {
        vec![
            TeamDomainEvent::TeamCreated {
                team_id: team_id(),
                space_id: SpaceId::try_new("00000000000000000000000002").unwrap(),
                competition_id: CompetitionId::try_new("00000000000000000000000003").unwrap(),
                competition_name: "Ligue de Condate".into(),
                season_id: SeasonId::try_new("00000000000000000000000004").unwrap(),
                season_name: "Saison 2025".into(),
                name: TeamName::try_new("Les Korrigans FC".to_string()).unwrap(),
                logo_url: None,
                roster_id: RosterId::try_new("00000000000000000000000005").unwrap(),
                roster_name: RosterName::try_new("Nains du Granit".to_string()).unwrap(),
                coach_id: CoachId::try_new("00000000000000000000000006").unwrap(),
                coach_name: "Colonel Castor".into(),
                treasury: Kpo(treasury),
                dedicated_fans: DedicatedFans::try_new(2).unwrap(),
                rerolls: RerollCount(0),
                apothecaries: ApothecaryCount(0),
                assistants: AssistantCount(0),
                cheerleaders: CheerleaderCount(0),
            },
            TeamDomainEvent::GamePhaseOverridden {
                admin_id: CoachId::try_new("00000000000000000000000006").unwrap(),
                from_phase: Some(GamePhase::ReadyToPlay),
                to_phase: GamePhase::Recruitment,
                reason: None,
            },
        ]
    }

    fn panier(lignes: Vec<BasketLine>, version: u32) -> PhaseBasketState {
        PhaseBasketState {
            team_id: TEAM.into(),
            space_id: "00000000000000000000000002".into(),
            phase: GamePhase::Recruitment,
            state: serde_json::to_value(&lignes).unwrap(),
            version,
        }
    }

    fn ligne_joueur(uid: &str, price: u32) -> BasketLine {
        BasketLine::Player {
            id: BasketLineId(format!("line-{uid}-{price}")),
            roster_line: RosterLineId(uid.to_string()),
            price: Kpo(price),
        }
    }

    fn cmd() -> ValidateRecruitmentPhaseCommand {
        ValidateRecruitmentPhaseCommand { team_id: team_id() }
    }

    async fn valider(
        teams: &FakeTeamRepository,
        baskets: &FakeBasketRepository,
        squad: FakeSquadPort,
    ) -> Result<(), ValidateRecruitmentPhaseError> {
        execute(cmd(), teams, baskets, &FakeRosterCatalogPort, &squad).await
    }

    /// Le garde-fou contre la double application : la suppression du panier
    /// sortant de la transaction, c'est la phase — et elle seule — qui empêche
    /// de rejouer le lot.
    #[tokio::test]
    async fn revalider_apres_succes_echoue_sans_rien_appliquer_deux_fois() {
        let teams = FakeTeamRepository::with_events(events_en_recrutement(1000));
        let baskets = FakeBasketRepository::with(panier(vec![ligne_joueur(PIETAILLE, 50)], 1));

        valider(&teams, &baskets, FakeSquadPort::empty())
            .await
            .expect("la première validation passe");
        let apres_succes = teams.appended();

        let erreur = valider(&teams, &baskets, FakeSquadPort::empty())
            .await
            .expect_err("la seconde est refusée");

        assert!(
            matches!(erreur, ValidateRecruitmentPhaseError::Domain(_)),
            "la phase est passée en Dismissals : {erreur:?}"
        );
        assert_eq!(
            teams.appended().len(),
            apres_succes.len(),
            "aucun événement supplémentaire"
        );
        assert_eq!(teams.batch_count(), 1, "un seul lot appendu");
    }

    #[tokio::test]
    async fn panier_vide_n_appende_que_la_transition() {
        let teams = FakeTeamRepository::with_events(events_en_recrutement(1000));
        let baskets = FakeBasketRepository::default();

        valider(&teams, &baskets, FakeSquadPort::empty())
            .await
            .unwrap();

        let appendus = teams.appended();
        assert_eq!(appendus.len(), 1);
        assert!(matches!(
            appendus[0],
            TeamDomainEvent::RecruitmentPhaseValidated
        ));
        assert_eq!(
            baskets.deleted(),
            vec![GamePhase::Recruitment],
            "le panier est purgé même vide"
        );
    }

    /// Le panier a été constitué quand le quota de Percuteurs était libre ;
    /// entre-temps l'équipe en a recruté deux. Le refus est **en bloc** : la
    /// Piétaille, pourtant saine, n'est pas appliquée non plus.
    #[tokio::test]
    async fn une_ligne_devenue_invalide_annule_tout_le_lot() {
        let teams = FakeTeamRepository::with_events(events_en_recrutement(1000));
        let fautive = ligne_joueur(PERCUTEUR, 90);
        let id_fautif = fautive.id().clone();
        let baskets =
            FakeBasketRepository::with(panier(vec![ligne_joueur(PIETAILLE, 50), fautive], 1));

        let erreur = valider(&teams, &baskets, FakeSquadPort(vec![PERCUTEUR, PERCUTEUR]))
            .await
            .expect_err("le quota de Percuteurs est saturé");

        let ValidateRecruitmentPhaseError::BasketNoLongerValid(rejetees) = erreur else {
            panic!("attendu BasketNoLongerValid, obtenu {erreur:?}");
        };
        assert_eq!(rejetees.len(), 1, "seule la ligne fautive est nommée");
        assert_eq!(rejetees[0].id, id_fautif);

        assert!(teams.appended().is_empty(), "rien n'est appliqué");
        assert!(
            baskets.deleted().is_empty(),
            "le panier survit au refus — le coach doit pouvoir le corriger"
        );
    }
}

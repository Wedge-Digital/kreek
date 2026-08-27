//! Le jet des erreurs coûteuses, tiré et appliqué.
//!
//! **L'orchestration ne décide de rien** : elle tire ce que le domaine lui
//! demande, dans l'ordre que le domaine impose, et lui laisse établir l'incident
//! et la perte.

use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::teams::domain::costly_mistakes::{dice_needed, incident_for};
use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::team::TeamDomainEvent;
use crate::app::teams::domain::value_objects::{IncidentType, Kpo};
use crate::app::teams::ports::{IDiceRoller, ITeamRepository, RepositoryError};

#[derive(Debug)]
pub struct ApplyCostlyMistakesCommand {
    pub team_id: TeamId,
}

/// Ce que le jet a donné — de quoi l'annoncer au coach sans relire l'agrégat.
#[derive(Debug, PartialEq, Eq)]
pub struct CostlyMistakesOutcome {
    pub roll: u8,
    pub damage_dice: Vec<u8>,
    pub incident: IncidentType,
    pub gp_lost: Kpo,
    pub treasury_before: Kpo,
    pub treasury_after: Kpo,
}

#[derive(Debug)]
pub enum ApplyCostlyMistakesError {
    TeamNotFound,
    /// Mauvaise phase — typiquement un second jet. Le contrôleur en fait un 409.
    Domain(DomainError),
    Repository(RepositoryError),
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ApplyCostlyMistakesCommand,
    team_repo: &dyn ITeamRepository,
    dice: &dyn IDiceRoller,
) -> Result<CostlyMistakesOutcome, ApplyCostlyMistakesError> {
    let team_id = cmd.team_id.to_string();
    let team = team_repo
        .find_by_id(&team_id)
        .await
        .map_err(ApplyCostlyMistakesError::Repository)?
        .ok_or(ApplyCostlyMistakesError::TeamNotFound)?;

    // Le dé est tiré **avant** la vérification de phase : un second POST en
    // tirera donc un inutile avant d'être refusé. Sans conséquence — il n'est
    // écrit nulle part, et l'ordre inverse obligerait à dupliquer ici la garde
    // que le domaine porte déjà.
    let roll = dice.d6();
    let damage_dice = lancer_les_degats(incident_for(team.treasury, roll), dice);

    let event = team
        .apply_costly_mistakes(roll, damage_dice.clone())
        .map_err(ApplyCostlyMistakesError::Domain)?;

    let TeamDomainEvent::CostlyMistakesApplied {
        incident, gp_lost, ..
    } = &event
    else {
        unreachable!("apply_costly_mistakes ne produit que cet événement")
    };
    let (incident, gp_lost) = (*incident, *gp_lost);

    team_repo
        .append(&team_id, &event, team.version)
        .await
        .map_err(ApplyCostlyMistakesError::Repository)?;

    let treasury_after = Kpo(team.treasury.0.saturating_sub(gp_lost.0));
    // Sur cible `kreek::`, sinon la ligne n'existe pas en production : une
    // contestation doit être vérifiable sans ouvrir l'event store.
    tracing::info!(
        team_id = %team_id,
        roll,
        ?damage_dice,
        ?incident,
        gp_lost = gp_lost.0,
        treasury_before = team.treasury.0,
        treasury_after = treasury_after.0,
        "erreurs coûteuses appliquées"
    );

    Ok(CostlyMistakesOutcome {
        roll,
        damage_dice,
        incident,
        gp_lost,
        treasury_before: team.treasury,
        treasury_after,
    })
}

/// Le domaine dit combien de dés il faut ; on les lance.
fn lancer_les_degats(incident: IncidentType, dice: &dyn IDiceRoller) -> Vec<u8> {
    match dice_needed(incident) {
        1 => vec![dice.d3()],
        2 => {
            let (a, b) = dice.two_d6();
            vec![a, b]
        }
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RosterId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::staff_counts::{
        ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
    };
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
    use crate::app::teams::domain::team::GamePhase;
    use crate::app::teams::domain::value_objects::{DedicatedFans, RosterName, TeamName};
    use crate::app::teams::use_cases::test_doubles::FakeTeamRepository;
    use std::sync::Mutex;

    const TEAM: &str = "00000000000000000000000001";

    /// Un dé qui rend ce qu'on lui dit.
    ///
    /// C'est ce qui permet de prouver qu'à 345 kPo un 1 donne un incident
    /// majeur et retire exactement 170 — la table de la carte 408 vérifiée de
    /// bout en bout, ce qu'un tirage en dur interdirait.
    struct DeTruque {
        d6: u8,
        d3: u8,
        deux_d6: (u8, u8),
        appels: Mutex<Vec<&'static str>>,
    }

    impl DeTruque {
        fn avec(d6: u8, d3: u8, deux_d6: (u8, u8)) -> Self {
            Self {
                d6,
                d3,
                deux_d6,
                appels: Mutex::new(vec![]),
            }
        }
        fn appels(&self) -> Vec<&'static str> {
            self.appels.lock().unwrap().clone()
        }
    }

    impl IDiceRoller for DeTruque {
        fn d6(&self) -> u8 {
            self.appels.lock().unwrap().push("d6");
            self.d6
        }
        fn d3(&self) -> u8 {
            self.appels.lock().unwrap().push("d3");
            self.d3
        }
        fn two_d6(&self) -> (u8, u8) {
            self.appels.lock().unwrap().push("2d6");
            self.deux_d6
        }
    }

    fn team_id() -> TeamId {
        TeamId::try_new(TEAM).unwrap()
    }

    /// Une équipe **en phase d'erreurs coûteuses**, avec la trésorerie voulue.
    fn equipe(treasury: u32, phase: GamePhase) -> Vec<TeamDomainEvent> {
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
                to_phase: phase,
                reason: None,
            },
        ]
    }

    async fn jeter(
        teams: &FakeTeamRepository,
        de: &DeTruque,
    ) -> Result<CostlyMistakesOutcome, ApplyCostlyMistakesError> {
        execute(ApplyCostlyMistakesCommand { team_id: team_id() }, teams, de).await
    }

    #[tokio::test]
    async fn a_345_kpo_un_jet_de_1_donne_un_majeur_de_170() {
        let teams = FakeTeamRepository::with_events(equipe(345, GamePhase::CostlyMistakes));
        let de = DeTruque::avec(1, 3, (6, 6));

        let issue = jeter(&teams, &de).await.unwrap();

        assert_eq!(issue.incident, IncidentType::Major);
        assert_eq!(issue.gp_lost, Kpo(170), "l'arrondi porte sur la perte");
        assert_eq!(issue.treasury_after, Kpo(175));
        assert_eq!(teams.appended().len(), 1, "un seul événement");
        assert_eq!(
            de.appels(),
            vec!["d6"],
            "un majeur ne demande aucun dé de dégâts"
        );
    }

    /// Une crise évitée produit **quand même** un événement : c'est lui qui
    /// referme la phase et rend l'équipe au jeu.
    #[tokio::test]
    async fn a_345_kpo_un_jet_de_5_est_une_crise_evitee_mais_produit_un_evenement() {
        let teams = FakeTeamRepository::with_events(equipe(345, GamePhase::CostlyMistakes));
        let de = DeTruque::avec(5, 3, (6, 6));

        let issue = jeter(&teams, &de).await.unwrap();

        assert_eq!(issue.incident, IncidentType::None);
        assert_eq!(issue.gp_lost, Kpo(0));
        assert_eq!(issue.treasury_after, Kpo(345), "rien n'est retiré");
        assert_eq!(teams.appended().len(), 1);
    }

    #[tokio::test]
    async fn a_560_kpo_un_jet_de_1_est_une_catastrophe_qui_ne_laisse_que_les_des() {
        let teams = FakeTeamRepository::with_events(equipe(560, GamePhase::CostlyMistakes));
        let de = DeTruque::avec(1, 3, (3, 4));

        let issue = jeter(&teams, &de).await.unwrap();

        assert_eq!(issue.incident, IncidentType::Catastrophe);
        assert_eq!(issue.damage_dice, vec![3, 4]);
        assert_eq!(issue.gp_lost, Kpo(490));
        assert_eq!(issue.treasury_after, Kpo(70));
        assert_eq!(
            de.appels(),
            vec!["d6", "2d6"],
            "les deux dés en un seul geste"
        );
    }

    #[tokio::test]
    async fn un_mineur_lance_un_d3_et_retire_dix_fois_le_de() {
        let teams = FakeTeamRepository::with_events(equipe(150, GamePhase::CostlyMistakes));
        let de = DeTruque::avec(1, 2, (6, 6));

        let issue = jeter(&teams, &de).await.unwrap();

        assert_eq!(issue.incident, IncidentType::Minor);
        assert_eq!(issue.damage_dice, vec![2]);
        assert_eq!(issue.gp_lost, Kpo(20));
        assert_eq!(de.appels(), vec!["d6", "d3"]);
    }

    /// Le second jet n'a besoin ni de verrou ni de jeton : le premier a reposé
    /// `ReadyToPlay`, donc la garde de phase du domaine refuse. L'idempotence
    /// sort du modèle.
    #[tokio::test]
    async fn hors_phase_le_jet_est_refuse_et_n_ecrit_rien() {
        let teams = FakeTeamRepository::with_events(equipe(345, GamePhase::ReadyToPlay));
        let de = DeTruque::avec(1, 3, (6, 6));

        let r = jeter(&teams, &de).await;

        assert!(matches!(r, Err(ApplyCostlyMistakesError::Domain(_))));
        assert!(teams.appended().is_empty(), "aucun événement écrit");
        assert_eq!(
            de.appels(),
            vec!["d6"],
            "le dé est tiré avant la garde, et jeté avec le refus"
        );
    }

    #[tokio::test]
    async fn une_equipe_introuvable_est_refusee() {
        let teams = FakeTeamRepository::with_events(vec![]);
        let de = DeTruque::avec(1, 3, (6, 6));

        assert!(matches!(
            jeter(&teams, &de).await,
            Err(ApplyCostlyMistakesError::TeamNotFound)
        ));
    }
}

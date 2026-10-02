//! L'ajustement de caisse décidé par un commissaire d'espace.
//!
//! **L'orchestration ne décide de rien** : elle charge, demande à l'agrégat, et
//! écrit. La seule règle — le solde couvre-t-il ce retrait — vit dans
//! `Team::adjust_treasury`, et les bornes du montant dans `AdjustmentAmount`.

use crate::app::teams::domain::error::DomainError;
use crate::app::teams::ports::{ITeamRepository, RepositoryError};
use crate::app::teams::use_cases::commands::AdjustTreasuryCommand;

#[derive(Debug)]
pub enum AdjustTreasuryError {
    TeamNotFound,
    /// Retrait non couvert par le solde. Le contrôleur en fait un message dans
    /// le pied du panneau, pas un code d'erreur HTTP.
    Domain(DomainError),
    Repository(RepositoryError),
}

/// # Il n'émet rien, et ce n'est pas un oubli
///
/// Dans `teams`, c'est le dépôt qui publie sur le bus interne, **après le
/// commit** — « le seul point qui les couvre tous », deux des quatre chemins
/// vers `ReadyToPlay` passant par des listeners. Déviation assumée du patron de
/// `players` et `match_report` ; émettre ici produirait un doublon.
///
/// # Il rend `()`, pas le nouveau solde
///
/// La réponse HTTP relit la fiche de toute façon. Rendre un solde créerait une
/// seconde source pour un chiffre que le relevé lit déjà de la dernière ligne
/// du grand livre — exactement ce que `TreasuryStatement::balance` refuse de
/// faire en ne le resommant jamais.
///
/// # Une écriture concurrente est refusée, jamais rejouée
///
/// `append` reçoit `team.version` ; si quelqu'un a écrit entre-temps, le dépôt
/// rend `ConcurrentWrite`, qui remonte tel quel. Rejouer en silence sur un état
/// qu'on n'a pas relu peut créditer une caisse que la recette d'un match publié
/// entre-temps a déjà remplie : le commissaire validerait un geste qu'il
/// n'aurait pas fait en voyant l'écran. Le refus coûte un clic.
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: AdjustTreasuryCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), AdjustTreasuryError> {
    let team_id = cmd.team_id.to_string();
    let team = team_repo
        .find_by_id(&team_id)
        .await
        .map_err(AdjustTreasuryError::Repository)?
        .ok_or(AdjustTreasuryError::TeamNotFound)?;

    let event = team
        .adjust_treasury(
            cmd.direction,
            cmd.amount,
            cmd.note,
            cmd.admin_id,
            cmd.admin_name,
        )
        .map_err(AdjustTreasuryError::Domain)?;

    team_repo
        .append(&team_id, &event, team.version)
        .await
        .map_err(AdjustTreasuryError::Repository)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RosterId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::staff_counts::{
        ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::coach_name::CoachName;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId, UserId};
    use crate::app::teams::domain::team::TeamDomainEvent;
    use crate::app::teams::domain::treasury::MovementDirection;
    use crate::app::teams::domain::value_objects::{
        AdjustmentAmount, AdjustmentNote, DedicatedFans, Kpo, RosterName, TeamName,
    };
    use crate::app::teams::use_cases::test_doubles::FakeTeamRepository;

    const TEAM: &str = "00000000000000000000000001";
    const MOTIF: &str = "Forfait des Griffons d'Argent — journée 3";

    fn team_id() -> TeamId {
        TeamId::try_new(TEAM).unwrap()
    }

    fn equipe(treasury: u32) -> Vec<TeamDomainEvent> {
        vec![TeamDomainEvent::TeamCreated {
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
        }]
    }

    fn commande(direction: MovementDirection, kpo: u32) -> AdjustTreasuryCommand {
        AdjustTreasuryCommand {
            team_id: team_id(),
            direction,
            amount: AdjustmentAmount::try_new(kpo).unwrap(),
            note: AdjustmentNote::try_new(MOTIF.to_string()).unwrap(),
            admin_id: UserId::try_new("00000000000000000000000007").unwrap(),
            admin_name: CoachName::try_new("Bagouze".to_string()).unwrap(),
        }
    }

    /// Un event store vide : l'équipe n'existe pas, et **rien ne doit être
    /// écrit** — pas même l'événement d'un ajustement qu'on croirait anodin.
    #[tokio::test]
    async fn une_equipe_introuvable_n_ecrit_rien() {
        let teams = FakeTeamRepository::default();

        let issue = execute(commande(MovementDirection::Credit, 120), &teams).await;

        assert!(matches!(issue, Err(AdjustTreasuryError::TeamNotFound)));
        assert!(teams.appended().is_empty());
    }

    /// Le refus du domaine remonte **tel quel**. Le traduire ici en une erreur
    /// applicative distincte ferait diverger deux formulations d'une seule
    /// règle, et le contrôleur n'a besoin que de savoir qu'elle a parlé.
    #[tokio::test]
    async fn un_retrait_non_couvert_remonte_le_refus_du_domaine() {
        let teams = FakeTeamRepository::with_events(equipe(85));

        let issue = execute(commande(MovementDirection::Debit, 90), &teams).await;

        assert!(matches!(
            issue,
            Err(AdjustTreasuryError::Domain(
                DomainError::InsufficientTreasury
            ))
        ));
        assert!(teams.appended().is_empty(), "un refus n'écrit rien");
    }

    /// **Le test qui compte.** Le motif et le nom ne vivent nulle part ailleurs
    /// que dans cet événement : perdus ici, ils ne se verraient qu'à la lecture
    /// du relevé, des semaines plus tard.
    #[tokio::test]
    async fn l_evenement_appende_porte_le_sens_le_motif_et_son_auteur() {
        let teams = FakeTeamRepository::with_events(equipe(85));

        execute(commande(MovementDirection::Credit, 120), &teams)
            .await
            .unwrap();

        let appendes = teams.appended();
        assert_eq!(appendes.len(), 1, "un seul événement");
        let TeamDomainEvent::TreasuryAdjusted {
            direction,
            amount,
            note,
            admin_name,
            ..
        } = &appendes[0]
        else {
            panic!("attendu TreasuryAdjusted, reçu {:?}", appendes[0])
        };
        assert_eq!(*direction, MovementDirection::Credit);
        assert_eq!(amount.into_inner(), 120);
        assert_eq!(note.as_ref(), MOTIF);
        assert_eq!(admin_name.clone().into_inner(), "Bagouze");
    }

    /// Un débit couvert suit le même chemin : c'est le sens qui change, pas
    /// l'orchestration.
    #[tokio::test]
    async fn un_debit_couvert_s_ecrit_aussi() {
        let teams = FakeTeamRepository::with_events(equipe(85));

        execute(commande(MovementDirection::Debit, 50), &teams)
            .await
            .unwrap();

        assert_eq!(teams.appended().len(), 1);
    }
}

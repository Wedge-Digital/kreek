//! Les faits de la journée que les agrégats attendent.
//!
//! Un module à part, et non une fonction publique dans un use case : `reopen` et
//! `record_answer` en ont tous deux besoin, et faire importer l'un depuis l'autre
//! serait un couplage sans raison d'être. La carte 517 la reprendra aussi.

use crate::app::competitions::domain::match_day::MatchDay;
use crate::app::competitions::domain::presence_survey::{EtatJournee, RencontreJournee};
use crate::app::competitions::ports::IMatchReportStatusPort;

/// Les faits que l'agrégat attend : les rencontres de la journée, et si l'une
/// d'elles porte un rapport publié.
///
/// **Un seul aller-retour au port**, sur tous les appariements à la fois : un
/// appel par rencontre ferait quinze allers-retours pour une réponse booléenne.
// arch:no-instrument — lecture de faits : assemble un `EtatJournee`, sans intention métier
pub async fn etat_de_la_journee(
    round: &MatchDay,
    report_status: &dyn IMatchReportStatusPort,
) -> Result<EtatJournee, String> {
    let ids: Vec<String> = round.pairings.iter().map(|p| p.id.to_string()).collect();
    let publies = report_status.find_published_pairings(&ids).await?;

    Ok(EtatJournee {
        figee: !publies.is_empty(),
        rencontres: round
            .pairings
            .iter()
            .map(|p| RencontreJournee {
                pairing: p.id,
                home: p.home_team_id,
                away: p.away_team_id,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{MatchDayType, Pairing};
    use crate::app::competitions::use_cases::presences::test_doubles::{journee, FauxRapports};
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId};
    use crate::app::shared_kernel::bloodbowl::team::TeamId;

    /// Un seul aller-retour au port, quels que soient les appariements : un appel
    /// par rencontre ferait quinze allers-retours pour une réponse booléenne.
    #[tokio::test]
    async fn l_etat_de_la_journee_interroge_le_port_une_seule_fois() {
        let jour = journee(
            MatchId::new(),
            MatchDayType::FixedDate,
            vec![Pairing {
                id: PairingId::new(),
                home_team_id: TeamId::new(),
                away_team_id: TeamId::new(),
            }],
        );

        let etat = etat_de_la_journee(&jour, &FauxRapports(false))
            .await
            .expect("état");

        assert!(!etat.figee);
        assert_eq!(etat.rencontres.len(), 1);
        assert_eq!(etat.rencontres[0].pairing, jour.pairings[0].id);
    }

    /// Une journée dont **un seul** appariement porte un rapport publié est figée
    /// en entier — R13. Ce n'est pas la rencontre qui se verrouille mais la
    /// journée, parce qu'un nouveau tirage la redistribuerait tout entière.
    #[tokio::test]
    async fn un_seul_rapport_publie_fige_la_journee_entiere() {
        let jour = journee(
            MatchId::new(),
            MatchDayType::FixedDate,
            (0..3)
                .map(|_| Pairing {
                    id: PairingId::new(),
                    home_team_id: TeamId::new(),
                    away_team_id: TeamId::new(),
                })
                .collect(),
        );

        let etat = etat_de_la_journee(&jour, &FauxRapports(true))
            .await
            .expect("état");

        assert!(etat.figee);
        assert_eq!(etat.rencontres.len(), 3, "les trois restent visibles");
    }

    /// Une journée sans appariement n'est jamais figée, et le port n'a rien à
    /// dire : `find_published_pairings` reçoit une liste vide.
    #[tokio::test]
    async fn une_journee_vide_n_est_pas_figee() {
        let jour = journee(MatchId::new(), MatchDayType::FixedDate, vec![]);

        let etat = etat_de_la_journee(&jour, &FauxRapports(true))
            .await
            .expect("état");

        assert!(
            !etat.figee,
            "aucun appariement, donc aucun rapport possible"
        );
        assert!(etat.rencontres.is_empty());
    }
}

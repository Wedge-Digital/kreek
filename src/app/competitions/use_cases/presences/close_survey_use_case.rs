//! Clore une campagne par décision.
//!
//! # Deux fichiers, et non un avec un booléen
//!
//! `close` et `reopen` n'ont ni les mêmes gardes ni les mêmes conséquences :
//! rouvrir réarme des jetons et exige une nouvelle échéance, clore n'en réarme
//! aucun et n'a besoin que de la date du jour. Les fondre derrière un
//! `rouvrir: bool` produirait un use case dont la moitié du corps est morte à
//! chaque appel.
//!
//! # La clôture est calculée, la décision est stockée
//!
//! R23 — `statut_de` croise l'échéance et la décision. Ce use case n'écrit donc
//! pas un statut : il pose la **décision**, et le statut s'en déduit. Clore une
//! campagne déjà échue est légitime et n'a rien d'un doublon : la décision prime
//! sur l'échéance, et c'est ce qui distingue « le délai est passé » de
//! « l'organisateur a tranché ».

use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::presence_survey_repository_port::{
    IPresenceSurveyRepository, PresenceSurveyRepositoryError,
};
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::MatchId;

#[derive(Debug)]
pub struct CloseSurveyCommand {
    pub round_id: MatchId,
    pub aujourd_hui: DateString,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CloseSurveyError {
    SurveyNotFound,
    Refus(DomainError),
    Database(String),
}

impl From<PresenceSurveyRepositoryError> for CloseSurveyError {
    fn from(e: PresenceSurveyRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: CloseSurveyCommand,
    survey_repo: &dyn IPresenceSurveyRepository,
) -> Result<(), CloseSurveyError> {
    let mut survey = survey_repo
        .find_by_round(&cmd.round_id.to_string())
        .await?
        .ok_or(CloseSurveyError::SurveyNotFound)?;

    survey
        .clore(&cmd.aujourd_hui)
        .map_err(CloseSurveyError::Refus)?;
    survey_repo.save(&survey).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::MatchDayType;
    use crate::app::competitions::domain::presence_survey::{Motif, SurveyStatus};
    use crate::app::competitions::use_cases::presences::test_doubles::*;
    use crate::app::shared_kernel::bloodbowl::ids::MatchId;

    #[tokio::test]
    async fn clore_pose_la_decision_et_la_persiste() {
        let round = MatchId::new();
        let jour = journee(round, MatchDayType::FixedDate, vec![]);
        let depot = FauxSurveyRepo::avec(campagne_ouverte(&jour, &[]));

        execute(
            CloseSurveyCommand {
                round_id: round,
                aujourd_hui: date("2026-10-03"),
            },
            &depot,
        )
        .await
        .expect("clôture");

        let close = depot.derniere_ecrite().expect("persistée");
        assert_eq!(
            close.statut(&date("2026-10-05")),
            SurveyStatus::Close(Motif::Decision),
            "la décision prime sur l'échéance, qui n'est pas encore passée"
        );
    }

    /// R23 — clore une campagne déjà échue n'est pas un doublon : la décision
    /// distingue « le délai est passé » de « l'organisateur a tranché ».
    #[tokio::test]
    async fn clore_une_campagne_deja_echue_change_le_motif() {
        let round = MatchId::new();
        let jour = journee(round, MatchDayType::FixedDate, vec![]);
        let campagne = campagne_ouverte(&jour, &[]);
        assert_eq!(
            campagne.statut(&date("2026-10-11")),
            SurveyStatus::Close(Motif::Echeance)
        );
        let depot = FauxSurveyRepo::avec(campagne);

        execute(
            CloseSurveyCommand {
                round_id: round,
                aujourd_hui: date("2026-10-11"),
            },
            &depot,
        )
        .await
        .expect("clôture");

        assert_eq!(
            depot
                .derniere_ecrite()
                .expect("persistée")
                .statut(&date("2026-10-11")),
            SurveyStatus::Close(Motif::Decision)
        );
    }

    #[tokio::test]
    async fn une_journee_sans_campagne_ne_se_clot_pas() {
        let refus = execute(
            CloseSurveyCommand {
                round_id: MatchId::new(),
                aujourd_hui: date("2026-10-05"),
            },
            &FauxSurveyRepo::vide(),
        )
        .await;

        assert_eq!(refus.unwrap_err(), CloseSurveyError::SurveyNotFound);
    }
}

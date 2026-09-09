//! Défaire un tirage — les appariements tombent, la campagne redevient close.
//!
//! # R13 est vérifiée **avant**, pas après
//!
//! `clear_round` conserve les rencontres dont un rapport est publié et les rend
//! dans son compte rendu. S'en contenter laisserait un tirage **à moitié défait** :
//! trois rencontres supprimées, une gardée, et une campagne qui ne sait plus où
//! elle en est. La garde passe donc en amont.
//!
//! Si `clear_round` rend malgré tout des rencontres conservées, c'est qu'un
//! rapport a été publié entre notre lecture et sa suppression. C'est une course,
//! et elle est signalée plutôt que tue.
//!
//! # Le gros du travail existait déjà
//!
//! `delete_pairing_use_case::clear_round` vide une journée, refuse ce qui est
//! rapporté et émet un `PairingDeleted` par rencontre. Ce use case l'enrobe : la
//! garde du sondage, l'appel, et la campagne remise à l'état d'avant tirage.

use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError,
};
use crate::app::competitions::domain::presence_survey_repository_port::{
    IPresenceSurveyRepository, PresenceSurveyRepositoryError,
};
use crate::app::competitions::ports::{IMatchReportStatusPort, ITeamInfoPort};
use crate::app::competitions::use_cases::admin::delete_pairing_use_case::clear_round;
use crate::app::competitions::use_cases::presences::etat_journee::etat_de_la_journee;
use crate::app::shared_kernel::bloodbowl::ids::MatchId;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct UndoDrawCommand {
    pub round_id: MatchId,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UndoOutcome {
    pub defaites: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UndoDrawError {
    SurveyNotFound,
    RoundNotFound,
    /// R13 — on ne défait pas un tirage dont un match est déjà rapporté.
    Refus(DomainError),
    /// Un rapport a été publié entre la garde et la suppression : le tirage est
    /// partiellement défait, et il faut le dire.
    PartiellementDefait {
        conservees: usize,
    },
    Database(String),
}

impl From<PresenceSurveyRepositoryError> for UndoDrawError {
    fn from(e: PresenceSurveyRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<MatchDayRepositoryError> for UndoDrawError {
    fn from(e: MatchDayRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

pub struct UndoDeps<'a> {
    pub survey_repo: &'a dyn IPresenceSurveyRepository,
    pub match_day_repo: &'a dyn IMatchDayRepository,
    pub report_status: &'a dyn IMatchReportStatusPort,
    pub teams: &'a dyn ITeamInfoPort,
    pub event_bus: &'a EventBus,
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: UndoDrawCommand,
    deps: UndoDeps<'_>,
) -> Result<UndoOutcome, UndoDrawError> {
    let mut survey = deps
        .survey_repo
        .find_by_round(&cmd.round_id.to_string())
        .await?
        .ok_or(UndoDrawError::SurveyNotFound)?;

    let round = deps
        .match_day_repo
        .find_by_id(&cmd.round_id.to_string())
        .await?
        .ok_or(UndoDrawError::RoundNotFound)?;
    let journee = etat_de_la_journee(&round, deps.report_status)
        .await
        .map_err(UndoDrawError::Database)?;
    if journee.figee {
        return Err(UndoDrawError::Refus(DomainError::RoundFrozenByReport));
    }

    let attendues = round.pairings.len();
    let conservees = clear_round(
        &cmd.round_id.to_string(),
        deps.match_day_repo,
        deps.report_status,
        deps.teams,
        deps.event_bus,
    )
    .await
    .map_err(|e| UndoDrawError::Database(format!("{e:?}")))?;

    if !conservees.is_empty() {
        return Err(UndoDrawError::PartiellementDefait {
            conservees: conservees.len(),
        });
    }

    survey.defaire_appariement();
    deps.survey_repo.save(&survey).await?;
    Ok(UndoOutcome {
        defaites: attendues,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{MatchDay, MatchDayType, Pairing};
    use crate::app::competitions::domain::presence_survey::Appariement;
    use crate::app::competitions::use_cases::presences::test_doubles::*;
    use crate::app::shared_kernel::bloodbowl::ids::PairingId;
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::common::services::event_bus::event_bus::new_bus;

    fn appariee(n: usize) -> MatchDay {
        journee(
            MatchId::new(),
            MatchDayType::FixedDate,
            (0..n)
                .map(|_| Pairing {
                    id: PairingId::new(),
                    home_team_id: TeamId::new(),
                    away_team_id: TeamId::new(),
                })
                .collect(),
        )
    }

    #[tokio::test]
    async fn defaire_supprime_les_rencontres_et_ramene_la_campagne_avant_le_tirage() {
        let round = appariee(3);
        let mut survey = campagne_ouverte(&round, &[]);
        survey.marquer_appariee(None);
        let depot = FauxSurveyRepo::avec(survey);
        let bus = new_bus();
        let mut abonne = bus.subscribe();

        let issue = execute(
            UndoDrawCommand { round_id: round.id },
            UndoDeps {
                survey_repo: &depot,
                match_day_repo: &FauxJournees::avec(round),
                report_status: &FauxRapports(false),
                teams: &FauxTeams(vec![]),
                event_bus: &bus,
            },
        )
        .await
        .expect("annulation");

        assert_eq!(issue.defaites, 3);
        for _ in 0..3 {
            assert!(abonne.try_recv().is_ok(), "un PairingDeleted par rencontre");
        }
        assert_eq!(
            depot.derniere_ecrite().expect("campagne").appariement(),
            &Appariement::Aucun
        );
    }

    /// R13 — on ne défait pas un tirage dont un match est déjà rapporté. La garde
    /// passe **avant** la suppression : `clear_round` conserverait les rencontres
    /// rapportées et supprimerait les autres, laissant un tirage à moitié défait.
    #[tokio::test]
    async fn une_journee_figee_refuse_l_annulation_et_ne_supprime_rien() {
        let round = appariee(3);
        let mut survey = campagne_ouverte(&round, &[]);
        survey.marquer_appariee(None);
        let depot = FauxSurveyRepo::avec(survey);
        let bus = new_bus();
        let mut abonne = bus.subscribe();

        let refus = execute(
            UndoDrawCommand { round_id: round.id },
            UndoDeps {
                survey_repo: &depot,
                match_day_repo: &FauxJournees::avec(round),
                report_status: &FauxRapports(true),
                teams: &FauxTeams(vec![]),
                event_bus: &bus,
            },
        )
        .await;

        assert_eq!(
            refus.unwrap_err(),
            UndoDrawError::Refus(DomainError::RoundFrozenByReport)
        );
        assert_eq!(depot.ecritures(), 0);
        assert!(
            abonne.try_recv().is_err(),
            "aucun PairingDeleted : rien n'a été supprimé"
        );
    }

    #[tokio::test]
    async fn une_journee_sans_campagne_ne_se_defait_pas() {
        let round = appariee(2);

        let refus = execute(
            UndoDrawCommand { round_id: round.id },
            UndoDeps {
                survey_repo: &FauxSurveyRepo::vide(),
                match_day_repo: &FauxJournees::avec(round),
                report_status: &FauxRapports(false),
                teams: &FauxTeams(vec![]),
                event_bus: &new_bus(),
            },
        )
        .await;

        assert_eq!(refus.unwrap_err(), UndoDrawError::SurveyNotFound);
    }
}

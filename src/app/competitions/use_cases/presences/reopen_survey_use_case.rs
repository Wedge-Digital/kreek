//! Rouvrir une campagne close, avec une nouvelle échéance.
//!
//! # Rouvrir réarme les anciens liens, sans rien réémettre
//!
//! Le jeton n'a pas de durée de vie propre : sa validité se lit sur l'état de la
//! campagne (R7). Rouvrir suffit donc à rendre les liens déjà reçus opérants — et
//! c'est aussi ce qui rend R13 nécessaire : rouvrir une journée dont un rapport
//! est publié ouvrirait la porte à des réponses sur un fait accompli.
//!
//! # C'est le premier appelant d'`IMatchReportStatusPort` de la campagne
//!
//! L'`EtatJournee` que `rouvrir` attend se construit de deux faits : les
//! appariements viennent de la journée, `figee` de `find_published_pairings` sur
//! leurs identifiants. Les cartes 516 et 517 reprendront ce patron — d'où
//! `etat_de_la_journee`, publique dans le module.

use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::match_day::MatchDay;
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError,
};
use crate::app::competitions::domain::presence_survey::{
    EtatJournee, RencontreJournee, SurveyDeadline,
};
use crate::app::competitions::domain::presence_survey_repository_port::{
    IPresenceSurveyRepository, PresenceSurveyRepositoryError,
};
use crate::app::competitions::ports::IMatchReportStatusPort;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::MatchId;

#[derive(Debug)]
pub struct ReopenSurveyCommand {
    pub round_id: MatchId,
    /// R23 — rouvrir **exige** une nouvelle échéance : la clôture étant calculée,
    /// rouvrir sans repousser la date rouvrirait sur une campagne close dans la
    /// seconde.
    pub deadline: SurveyDeadline,
    pub aujourd_hui: DateString,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReopenSurveyError {
    SurveyNotFound,
    RoundNotFound,
    /// Journée figée par un rapport publié (R13), échéance déjà passée (R23).
    /// Relayé tel quel : `DomainError` sait s'afficher.
    Refus(DomainError),
    Database(String),
}

impl From<PresenceSurveyRepositoryError> for ReopenSurveyError {
    fn from(e: PresenceSurveyRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<MatchDayRepositoryError> for ReopenSurveyError {
    fn from(e: MatchDayRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

pub struct ReopenDeps<'a> {
    pub survey_repo: &'a dyn IPresenceSurveyRepository,
    pub match_day_repo: &'a dyn IMatchDayRepository,
    pub report_status: &'a dyn IMatchReportStatusPort,
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ReopenSurveyCommand,
    deps: ReopenDeps<'_>,
) -> Result<(), ReopenSurveyError> {
    let mut survey = deps
        .survey_repo
        .find_by_round(&cmd.round_id.to_string())
        .await?
        .ok_or(ReopenSurveyError::SurveyNotFound)?;

    let round = deps
        .match_day_repo
        .find_by_id(&cmd.round_id.to_string())
        .await?
        .ok_or(ReopenSurveyError::RoundNotFound)?;
    let journee = etat_de_la_journee(&round, deps.report_status)
        .await
        .map_err(ReopenSurveyError::Database)?;

    survey
        .rouvrir(cmd.deadline, &journee, &cmd.aujourd_hui)
        .map_err(ReopenSurveyError::Refus)?;
    deps.survey_repo.save(&survey).await?;
    Ok(())
}

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
    use crate::app::competitions::domain::presence_survey::SurveyStatus;
    use crate::app::competitions::use_cases::presences::test_doubles::*;
    use crate::app::shared_kernel::bloodbowl::ids::PairingId;
    use crate::app::shared_kernel::bloodbowl::team::TeamId;

    fn commande(round: &MatchId, deadline: &str, aujourd_hui: &str) -> ReopenSurveyCommand {
        ReopenSurveyCommand {
            round_id: *round,
            deadline: echeance(deadline),
            aujourd_hui: date(aujourd_hui),
        }
    }

    fn appariee(round: MatchId) -> MatchDay {
        journee(
            round,
            MatchDayType::FixedDate,
            vec![Pairing {
                id: PairingId::new(),
                home_team_id: TeamId::new(),
                away_team_id: TeamId::new(),
            }],
        )
    }

    #[tokio::test]
    async fn rouvrir_remet_la_campagne_ouverte_avec_sa_nouvelle_echeance() {
        let round = MatchId::new();
        let jour = journee(round, MatchDayType::FixedDate, vec![]);
        let mut campagne = campagne_ouverte(&jour, &[]);
        campagne.clore(&date("2026-10-03")).unwrap();
        let depot = FauxSurveyRepo::avec(campagne);

        execute(
            commande(&round, "2026-10-20", "2026-10-05"),
            ReopenDeps {
                survey_repo: &depot,
                match_day_repo: &FauxJournees(Some(jour)),
                report_status: &FauxRapports(false),
            },
        )
        .await
        .expect("réouverture");

        let rouverte = depot.derniere_ecrite().expect("persistée");
        assert_eq!(rouverte.statut(&date("2026-10-05")), SurveyStatus::Ouverte);
        assert_eq!(rouverte.deadline(), &echeance("2026-10-20"));
    }

    /// R13 — rouvrir réarme les anciens liens (R7), donc rouvrir une journée dont
    /// un rapport est publié ouvrirait la porte à des réponses sur un fait accompli.
    #[tokio::test]
    async fn rouvrir_une_journee_figee_par_un_rapport_est_refuse() {
        let round = MatchId::new();
        let jour = appariee(round);
        let mut campagne = campagne_ouverte(&jour, &[]);
        campagne.clore(&date("2026-10-03")).unwrap();
        let depot = FauxSurveyRepo::avec(campagne);

        let refus = execute(
            commande(&round, "2026-10-20", "2026-10-05"),
            ReopenDeps {
                survey_repo: &depot,
                match_day_repo: &FauxJournees(Some(jour)),
                report_status: &FauxRapports(true),
            },
        )
        .await;

        assert_eq!(
            refus.unwrap_err(),
            ReopenSurveyError::Refus(DomainError::RoundFrozenByReport)
        );
        assert_eq!(depot.ecritures(), 0);
    }

    /// Une journée **appariée mais non jouée** reste rouvrable : c'est le chemin
    /// normal quand une défection arrive après le tirage.
    #[tokio::test]
    async fn rouvrir_une_journee_appariee_mais_non_jouee_reste_permis() {
        let round = MatchId::new();
        let jour = appariee(round);
        let mut campagne = campagne_ouverte(&jour, &[]);
        campagne.clore(&date("2026-10-03")).unwrap();
        let depot = FauxSurveyRepo::avec(campagne);

        execute(
            commande(&round, "2026-10-20", "2026-10-05"),
            ReopenDeps {
                survey_repo: &depot,
                match_day_repo: &FauxJournees(Some(jour)),
                report_status: &FauxRapports(false),
            },
        )
        .await
        .expect("le chemin normal après une défection");

        assert_eq!(depot.ecritures(), 1);
    }

    /// R23 — la clôture étant calculée, rouvrir sans repousser la date rouvrirait
    /// sur une campagne close dans la seconde.
    #[tokio::test]
    async fn rouvrir_sur_une_echeance_passee_est_refuse() {
        let round = MatchId::new();
        let jour = journee(round, MatchDayType::FixedDate, vec![]);
        let mut campagne = campagne_ouverte(&jour, &[]);
        campagne.clore(&date("2026-10-03")).unwrap();
        let depot = FauxSurveyRepo::avec(campagne);

        let refus = execute(
            commande(&round, "2026-10-04", "2026-10-05"),
            ReopenDeps {
                survey_repo: &depot,
                match_day_repo: &FauxJournees(Some(jour)),
                report_status: &FauxRapports(false),
            },
        )
        .await;

        assert_eq!(
            refus.unwrap_err(),
            ReopenSurveyError::Refus(DomainError::DeadlineInThePast {
                deadline: "2026-10-04".to_string()
            })
        );
    }

    /// Un seul aller-retour au port, quels que soient les appariements : un appel
    /// par rencontre ferait quinze allers-retours pour une réponse booléenne.
    #[tokio::test]
    async fn l_etat_de_la_journee_interroge_le_port_une_seule_fois() {
        let jour = appariee(MatchId::new());

        let etat = etat_de_la_journee(&jour, &FauxRapports(false))
            .await
            .expect("état");

        assert!(!etat.figee);
        assert_eq!(etat.rencontres.len(), 1);
        assert_eq!(etat.rencontres[0].pairing, jour.pairings[0].id);
    }
}

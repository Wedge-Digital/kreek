use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct CancelInducementsCommand {
    pub match_report_id: MatchReportId,
    pub cancelled_by: CoachId,
}

/// Où reprendre : le favori achète d'abord (R5).
#[derive(Debug)]
pub struct CancelInducementsOutcome {
    pub topdog_team_id: TeamId,
}

#[derive(Debug)]
pub enum CancelInducementsError {
    NotFound,
    /// Hors `PreMatch` — un `ReadyToPublish` compris : sa dépense de
    /// trésorerie est figée (R3).
    NotInPreMatchPhase,
    Domain(DomainError),
    /// Une autre écriture est passée entre le chargement et l'append (carte 623).
    Conflict,
    Repository(String),
}

impl CancelInducementsError {
    /// `ConcurrentWrite` garde son sens au lieu de se fondre dans `Repository`.
    fn from_repository(e: RepositoryError) -> Self {
        match e {
            RepositoryError::ConcurrentWrite => Self::Conflict,
            other => Self::Repository(other.to_string()),
        }
    }
}

/// Annule les coups de pouce des deux équipes (carte 626).
///
/// `InducementsCancelled` et les `TempPlayersReset` partent dans **un seul**
/// `append_many` : la projection suit dans la même transaction, et un rapport
/// ne peut pas se retrouver sans achats mais avec ses journaliers.
///
/// Les trois événements sont émis : le publisher fait des resets des
/// `JourneymenWithdrawn` pour `players`, et ignore le premier.
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: CancelInducementsCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
) -> Result<CancelInducementsOutcome, CancelInducementsError> {
    let mr_id = cmd.match_report_id.to_string();
    let pm = load_pre_match(repo, &mr_id).await?;
    let (updated, events) = pm
        .cancel_inducements(cmd.cancelled_by)
        .map_err(CancelInducementsError::Domain)?;
    persist_and_emit(repo, bus, &mr_id, events, pm.version).await?;
    Ok(CancelInducementsOutcome {
        topdog_team_id: updated.topdog_team_id().clone(),
    })
}

/// Un `PreMatch` seulement : un `ReadyToPublish` n'est **pas** converti par
/// `into_pre_match()`, contrairement aux autres écritures d'avant-match.
async fn load_pre_match(
    repo: &dyn IMatchReportRepository,
    mr_id: &str,
) -> Result<MatchReportPreMatch, CancelInducementsError> {
    let state = repo
        .find_by_id(mr_id)
        .await
        .map_err(CancelInducementsError::from_repository)?;
    match state.ok_or(CancelInducementsError::NotFound)? {
        MatchReportState::PreMatch(pm) => Ok(pm),
        _ => Err(CancelInducementsError::NotInPreMatchPhase),
    }
}

async fn persist_and_emit(
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
    mr_id: &str,
    events: Vec<MatchReportDomainEvent>,
    version: u64,
) -> Result<(), CancelInducementsError> {
    repo.append_many(mr_id, events.clone(), version)
        .await
        .map_err(CancelInducementsError::from_repository)?;
    for event in &events {
        emettre(bus, event.to_enveloppe(mr_id));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
    use crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish;
    use crate::app::match_report::domain::value_objects::{
        FanFactorMod, InducementCost, InducementPurchase, InducementQty, MatchGain,
        NoStallingBonuses, TempPlayer, TempPlayerId, TempPlayerKind,
    };
    use crate::app::match_report::use_cases::test_doubles::{
        pre_match, ConflictingRepository, RecordingRepository,
    };
    use crate::app::shared_kernel::bloodbowl::inducement_definition::InducementId;
    use crate::common::services::event_bus::event_bus::new_bus;

    fn bribe() -> InducementPurchase {
        InducementPurchase {
            uid: InducementId("BRIBE".into()),
            qty: InducementQty::try_new(1).unwrap(),
            unit_cost: InducementCost::try_new(100).unwrap(),
        }
    }

    fn journeyman(pm: &MatchReportPreMatch, id: &str) -> TempPlayer {
        TempPlayer {
            id: TempPlayerId(id.into()),
            team_id: pm.home_team_id.clone(),
            kind: TempPlayerKind::Journeyman {
                position_uid: "PIETAILLE".into(),
            },
            display_name: None,
        }
    }

    /// Le favori (domicile, à valeurs égales) a validé et aligné un journalier.
    fn validated_pre_match() -> MatchReportPreMatch {
        let mut pm = pre_match();
        pm.home_inducements = Some(vec![bribe()]);
        pm.home_temp_players = vec![journeyman(&pm, "JM01")];
        pm
    }

    fn command() -> CancelInducementsCommand {
        CancelInducementsCommand {
            match_report_id: MatchReportId::new(),
            cancelled_by: CoachId::new(),
        }
    }

    #[tokio::test]
    async fn l_annulation_appende_puis_emet_ses_evenements() {
        let pm = validated_pre_match();
        let topdog = pm.home_team_id.clone();
        let repo = RecordingRepository::holding(MatchReportState::PreMatch(pm));
        let bus = new_bus();
        let mut abonne = bus.subscribe();

        let outcome = execute(command(), &repo, &bus).await.expect("annulable");

        assert_eq!(outcome.topdog_team_id, topdog);
        let types: Vec<_> = repo.appended().iter().map(|e| e.type_name()).collect();
        assert_eq!(types, ["InducementsCancelled", "TempPlayersReset"]);
        let emis: Vec<_> = (0..2)
            .map(|_| abonne.try_recv().unwrap().event_type)
            .collect();
        assert_eq!(emis, ["InducementsCancelled", "TempPlayersReset"]);
    }

    /// Le reset nomme le journalier : sans lui, `players` le garderait.
    #[tokio::test]
    async fn le_reset_nomme_les_journaliers() {
        let repo = RecordingRepository::holding(MatchReportState::PreMatch(validated_pre_match()));

        execute(command(), &repo, &new_bus())
            .await
            .expect("annulable");

        let appended = repo.appended();
        assert!(matches!(
            &appended[1],
            MatchReportDomainEvent::TempPlayersReset { withdrawn, .. }
                if withdrawn == &vec![TempPlayerId("JM01".into())]
        ));
    }

    #[tokio::test]
    async fn un_conflit_d_ecriture_est_relaye() {
        let repo =
            ConflictingRepository::holding(MatchReportState::PreMatch(validated_pre_match()));
        let resultat = execute(command(), &repo, &new_bus()).await;
        assert!(
            matches!(resultat, Err(CancelInducementsError::Conflict)),
            "{resultat:?}"
        );
    }

    /// R3 : un `ReadyToPublish` n'est pas ramené en avant-match.
    #[tokio::test]
    async fn un_rapport_pret_a_publier_est_refuse() {
        let rtp = MatchReportReadyToPublish::from_pre_match(
            &validated_pre_match(),
            MatchGain::try_new(10_000).unwrap(),
            MatchGain::try_new(10_000).unwrap(),
            NoStallingBonuses::default(),
            FanFactorMod::try_new(0).unwrap(),
            FanFactorMod::try_new(0).unwrap(),
            None,
            None,
        );
        let repo = RecordingRepository::holding(MatchReportState::ReadyToPublish(rtp));
        let resultat = execute(command(), &repo, &new_bus()).await;
        assert!(
            matches!(resultat, Err(CancelInducementsError::NotInPreMatchPhase)),
            "{resultat:?}"
        );
        assert!(repo.appended().is_empty());
    }
}

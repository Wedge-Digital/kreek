use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::ports::IMatchReportStatusPort;
use crate::app::shared_kernel::common_types::EventId;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub enum DeletePairingError {
    /// Le rapport de match de cette rencontre est publié : la supprimer
    /// laisserait le match au classement tout en le faisant disparaître du
    /// calendrier.
    ReportPublished,
    /// L'état des rapports n'a pas pu être consulté — on refuse plutôt que de
    /// supprimer à l'aveugle.
    StatusUnavailable(String),
    Repository(String),
}

/// Supprime un pairing et annonce sa disparition.
///
/// L'annulation du rapport de match rattaché est la conséquence de l'événement
/// `PairingDeleted`, traitée par le BC `match_report` — ce use case ne connaît
/// pas les rapports, il vérifie seulement qu'aucun n'est publié.
pub async fn execute(
    pairing_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    status_port: &dyn IMatchReportStatusPort,
    event_bus: &EventBus,
) -> Result<(), DeletePairingError> {
    let published = status_port
        .find_published_pairings(std::slice::from_ref(&pairing_id.to_string()))
        .await
        .map_err(DeletePairingError::StatusUnavailable)?;

    if !published.is_empty() {
        return Err(DeletePairingError::ReportPublished);
    }

    match_day_repo
        .delete_pairing(pairing_id)
        .await
        .map_err(|e| DeletePairingError::Repository(e.to_string()))?;

    let _ = event_bus.send(
        CompetitionsDomainEvent::PairingDeleted {
            event_id: EventId::new(),
            pairing_id: pairing_id.to_string(),
        }
        .to_enveloppe(),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
    use crate::app::competitions::domain::match_day_repository_port::{
        MatchDayRepositoryError, NewPairingProjection, PairingDisplayDto,
    };
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    struct FakeStatusPort {
        published: Vec<String>,
        fails: bool,
    }

    #[async_trait]
    impl IMatchReportStatusPort for FakeStatusPort {
        async fn find_published_pairings(
            &self,
            pairing_ids: &[String],
        ) -> Result<Vec<String>, String> {
            if self.fails {
                return Err("BC injoignable".to_string());
            }
            Ok(pairing_ids
                .iter()
                .filter(|id| self.published.contains(id))
                .cloned()
                .collect())
        }
    }

    #[derive(Default)]
    struct FakeMatchDayRepo {
        deleted: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl IMatchDayRepository for FakeMatchDayRepo {
        async fn delete_pairing(&self, pairing_id: &str) -> Result<(), MatchDayRepositoryError> {
            self.deleted.lock().unwrap().push(pairing_id.to_string());
            Ok(())
        }
        async fn find_by_season(&self, _: &str) -> Result<Vec<MatchDay>, MatchDayRepositoryError> { Ok(vec![]) }
        async fn find_by_id(&self, _: &str) -> Result<Option<MatchDay>, MatchDayRepositoryError> { Ok(None) }
        async fn save_match_day(&self, _: &MatchDay) -> Result<(), MatchDayRepositoryError> { Ok(()) }
        async fn delete_match_day(&self, _: &str) -> Result<(), MatchDayRepositoryError> { Ok(()) }
        async fn save_pairing(&self, _: &str, _: &Pairing, _: &NewPairingProjection) -> Result<(), MatchDayRepositoryError> { Ok(()) }
        async fn find_pairing_id(&self, _: &str, _: &str, _: &str) -> Result<Option<String>, MatchDayRepositoryError> { Ok(None) }
        async fn clear_pairings(&self, _: &str) -> Result<(), MatchDayRepositoryError> { Ok(()) }
        async fn clear_all_pairings(&self, _: &str) -> Result<(), MatchDayRepositoryError> { Ok(()) }
        async fn ensure_match_days_from_structure(&self, _: &str, _: &[(String, String, String, Option<String>, Option<String>)]) -> Result<(), MatchDayRepositoryError> { Ok(()) }
        async fn list_resultats(&self, _: &str, _: Option<i32>, _: u32) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> { Ok(vec![]) }
        async fn list_calendrier(&self, _: &str, _: Option<i32>, _: u32) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> { Ok(vec![]) }
    }

    fn bus() -> EventBus {
        crate::common::services::event_bus::event_bus::new_bus()
    }

    #[tokio::test]
    async fn supprime_et_annonce_quand_aucun_rapport_publie() {
        let repo = FakeMatchDayRepo::default();
        let deleted = repo.deleted.clone();
        let port = FakeStatusPort { published: vec![], fails: false };
        let bus = bus();
        let mut rx = bus.subscribe();

        execute("pairing-1", &repo, &port, &bus).await.unwrap();

        assert_eq!(*deleted.lock().unwrap(), vec!["pairing-1".to_string()]);
        let envelope = rx.try_recv().expect("un PairingDeleted doit être émis");
        assert_eq!(envelope.event_type, "PairingDeleted");
        assert_eq!(envelope.emitter, "pairing-1");
    }

    #[tokio::test]
    async fn refuse_et_ne_supprime_rien_quand_le_rapport_est_publie() {
        let repo = FakeMatchDayRepo::default();
        let deleted = repo.deleted.clone();
        let port = FakeStatusPort { published: vec!["pairing-1".to_string()], fails: false };
        let bus = bus();
        let mut rx = bus.subscribe();

        let result = execute("pairing-1", &repo, &port, &bus).await;

        assert!(matches!(result, Err(DeletePairingError::ReportPublished)));
        assert!(deleted.lock().unwrap().is_empty(), "le pairing doit rester");
        assert!(rx.try_recv().is_err(), "aucun événement ne doit être émis");
    }

    /// Un autre pairing publié dans la base ne doit pas bloquer celui-ci : la
    /// réponse du port est filtrée sur les ids demandés.
    #[tokio::test]
    async fn un_autre_pairing_publie_ne_bloque_pas() {
        let repo = FakeMatchDayRepo::default();
        let deleted = repo.deleted.clone();
        let port = FakeStatusPort { published: vec!["pairing-2".to_string()], fails: false };

        execute("pairing-1", &repo, &port, &bus()).await.unwrap();

        assert_eq!(*deleted.lock().unwrap(), vec!["pairing-1".to_string()]);
    }

    #[tokio::test]
    async fn refuse_quand_le_statut_est_inconsultable() {
        let repo = FakeMatchDayRepo::default();
        let deleted = repo.deleted.clone();
        let port = FakeStatusPort { published: vec![], fails: true };

        let result = execute("pairing-1", &repo, &port, &bus()).await;

        assert!(matches!(result, Err(DeletePairingError::StatusUnavailable(_))));
        assert!(deleted.lock().unwrap().is_empty(), "pas de suppression à l'aveugle");
    }
}

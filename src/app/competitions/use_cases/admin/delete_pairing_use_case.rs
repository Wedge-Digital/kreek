use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::match_day::MatchDay;
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::ports::{IMatchReportStatusPort, ITeamInfoPort};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::EventId;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;
use std::collections::HashMap;

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

/// Rencontre conservée lors d'une suppression en masse, pour le compte-rendu
/// rendu à l'admin. DTO de lecture : aucun invariant à protéger.
pub struct KeptMatch {
    pub round_name: String,
    pub home_team_name: String,
    pub away_team_name: String,
}

/// Supprime un pairing et annonce sa disparition.
///
/// L'annulation du rapport de match rattaché est la conséquence de l'événement
/// `PairingDeleted`, traitée par le BC `match_report` — ce use case ne connaît
/// pas les rapports, il vérifie seulement qu'aucun n'est publié.
#[tracing::instrument(skip_all, fields(pairing_id = ?pairing_id))]
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

    delete_one(pairing_id, match_day_repo, event_bus).await
}

/// Vide les rencontres d'une journée.
#[tracing::instrument(skip_all, fields(round_id = ?round_id))]
pub async fn clear_round(
    round_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    status_port: &dyn IMatchReportStatusPort,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
) -> Result<Vec<KeptMatch>, DeletePairingError> {
    let days = load_round(round_id, match_day_repo).await?;
    delete_pairings_of(&days, match_day_repo, status_port, team_port, event_bus).await
}

/// Vide les rencontres de toute la saison.
#[tracing::instrument(skip_all, fields(season_id = ?season_id))]
pub async fn clear_season(
    season_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    status_port: &dyn IMatchReportStatusPort,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
) -> Result<Vec<KeptMatch>, DeletePairingError> {
    let days = match_day_repo
        .find_by_season(season_id)
        .await
        .map_err(|e| DeletePairingError::Repository(e.to_string()))?;

    delete_pairings_of(&days, match_day_repo, status_port, team_port, event_bus).await
}

/// Supprime une journée — mais seulement si aucune de ses rencontres n'est
/// conservée : la suppression de la journée cascade en base sur ses pairings,
/// ce qui contournerait le garde-fou.
#[tracing::instrument(skip_all, fields(round_id = ?round_id))]
pub async fn delete_round(
    round_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    status_port: &dyn IMatchReportStatusPort,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
) -> Result<Vec<KeptMatch>, DeletePairingError> {
    let days = load_round(round_id, match_day_repo).await?;
    let kept = delete_pairings_of(&days, match_day_repo, status_port, team_port, event_bus).await?;

    if kept.is_empty() {
        match_day_repo
            .delete_match_day(round_id)
            .await
            .map_err(|e| DeletePairingError::Repository(e.to_string()))?;
    }

    Ok(kept)
}

async fn load_round(
    round_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
) -> Result<Vec<MatchDay>, DeletePairingError> {
    let day = match_day_repo
        .find_by_id(round_id)
        .await
        .map_err(|e| DeletePairingError::Repository(e.to_string()))?;

    Ok(day.into_iter().collect())
}

/// Supprime toutes les rencontres de ces journées, sauf celles dont le rapport
/// est publié — rendues en compte-rendu plutôt que de faire échouer le lot.
async fn delete_pairings_of(
    days: &[MatchDay],
    match_day_repo: &dyn IMatchDayRepository,
    status_port: &dyn IMatchReportStatusPort,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
) -> Result<Vec<KeptMatch>, DeletePairingError> {
    let published = published_among(days, status_port).await?;
    let mut kept = Vec::new();

    for day in days {
        for pairing in &day.pairings {
            let pairing_id = pairing.id.to_string();
            if published.contains(&pairing_id) {
                kept.push((
                    day.name.clone(),
                    pairing.home_team_id.clone(),
                    pairing.away_team_id.clone(),
                ));
                continue;
            }
            delete_one(&pairing_id, match_day_repo, event_bus).await?;
        }
    }

    Ok(resolve_kept_names(kept, team_port).await)
}

async fn published_among(
    days: &[MatchDay],
    status_port: &dyn IMatchReportStatusPort,
) -> Result<Vec<String>, DeletePairingError> {
    let pairing_ids: Vec<String> = days
        .iter()
        .flat_map(|d| d.pairings.iter().map(|p| p.id.to_string()))
        .collect();

    status_port
        .find_published_pairings(&pairing_ids)
        .await
        .map_err(DeletePairingError::StatusUnavailable)
}

/// Un échec de résolution des noms ne doit pas faire échouer une suppression
/// déjà effectuée : on retombe sur les identifiants.
async fn resolve_kept_names(
    kept: Vec<(
        crate::app::competitions::domain::match_day::MatchDayName,
        TeamId,
        TeamId,
    )>,
    team_port: &dyn ITeamInfoPort,
) -> Vec<KeptMatch> {
    let team_ids: Vec<String> = kept
        .iter()
        .flat_map(|(_, home, away)| [home.to_string(), away.to_string()])
        .collect();

    let names: HashMap<String, String> = team_port
        .find_team_names(&team_ids)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|t| (t.team_id, t.team_name))
        .collect();

    kept.into_iter()
        .map(|(round_name, home, away)| KeptMatch {
            round_name: round_name.into_inner(),
            home_team_name: name_of(&names, home),
            away_team_name: name_of(&names, away),
        })
        .collect()
}

fn name_of(names: &HashMap<String, String>, team_id: TeamId) -> String {
    let id = team_id.to_string();
    names.get(&id).cloned().unwrap_or(id)
}

async fn delete_one(
    pairing_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    event_bus: &EventBus,
) -> Result<(), DeletePairingError> {
    match_day_repo
        .delete_pairing(pairing_id)
        .await
        .map_err(|e| DeletePairingError::Repository(e.to_string()))?;

    emettre(
        event_bus,
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
    use crate::app::competitions::ports::TeamInfoDto;
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

    struct FakeTeamInfoPort;

    #[async_trait]
    impl ITeamInfoPort for FakeTeamInfoPort {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_names(&self, team_ids: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(team_ids
                .iter()
                .map(|id| TeamInfoDto {
                    team_id: id.clone(),
                    team_name: format!("Équipe {id}"),
                    coach_id: String::new(),
                    coach_name: String::new(),
                    roster_name: String::new(),
                    logo_url: None,
                })
                .collect())
        }
    }

    #[derive(Default)]
    struct FakeMatchDayRepo {
        deleted: Arc<Mutex<Vec<String>>>,
        deleted_days: Arc<Mutex<Vec<String>>>,
        days: Vec<MatchDay>,
    }

    #[async_trait]
    impl IMatchDayRepository for FakeMatchDayRepo {
        async fn delete_pairing(&self, pairing_id: &str) -> Result<(), MatchDayRepositoryError> {
            self.deleted.lock().unwrap().push(pairing_id.to_string());
            Ok(())
        }
        async fn find_by_season(&self, _: &str) -> Result<Vec<MatchDay>, MatchDayRepositoryError> {
            Ok(self.days.clone())
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<MatchDay>, MatchDayRepositoryError> {
            Ok(self.days.first().cloned())
        }
        async fn save_match_day(&self, _: &MatchDay) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn delete_match_day(
            &self,
            match_day_id: &str,
        ) -> Result<(), MatchDayRepositoryError> {
            self.deleted_days
                .lock()
                .unwrap()
                .push(match_day_id.to_string());
            Ok(())
        }
        async fn save_pairing(
            &self,
            _: &str,
            _: &Pairing,
            _: &NewPairingProjection,
        ) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn find_pairing_id(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<Option<String>, MatchDayRepositoryError> {
            Ok(None)
        }
        async fn ensure_match_days_from_structure(
            &self,
            _: &str,
            _: &[(String, String, String, Option<String>, Option<String>)],
        ) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn list_resultats(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_calendrier(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_latest_completed_results(
            &self,
            _: &str,
            _: i64,
        ) -> Result<
            Vec<crate::app::competitions::domain::match_day_repository_port::LatestResultDto>,
            MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
    }

    fn bus() -> EventBus {
        crate::common::services::event_bus::event_bus::new_bus()
    }

    fn a_pairing() -> Pairing {
        Pairing {
            id: crate::app::shared_kernel::bloodbowl::ids::PairingId::new(),
            home_team_id: TeamId::new(),
            away_team_id: TeamId::new(),
        }
    }

    fn a_day(name: &str, pairings: Vec<Pairing>) -> MatchDay {
        use crate::app::competitions::domain::match_day::{MatchDayPosition, MatchDayType};
        use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
        MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: crate::app::competitions::domain::match_day::MatchDayName::try_new(
                name.to_string(),
            )
            .unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(0).unwrap(),
            pairings,
        }
    }

    fn repo_with(days: Vec<MatchDay>) -> FakeMatchDayRepo {
        FakeMatchDayRepo {
            days,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn supprime_et_annonce_quand_aucun_rapport_publie() {
        let repo = FakeMatchDayRepo::default();
        let deleted = repo.deleted.clone();
        let port = FakeStatusPort {
            published: vec![],
            fails: false,
        };
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
        let port = FakeStatusPort {
            published: vec!["pairing-1".to_string()],
            fails: false,
        };
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
        let port = FakeStatusPort {
            published: vec!["pairing-2".to_string()],
            fails: false,
        };

        execute("pairing-1", &repo, &port, &bus()).await.unwrap();

        assert_eq!(*deleted.lock().unwrap(), vec!["pairing-1".to_string()]);
    }

    #[tokio::test]
    async fn refuse_quand_le_statut_est_inconsultable() {
        let repo = FakeMatchDayRepo::default();
        let deleted = repo.deleted.clone();
        let port = FakeStatusPort {
            published: vec![],
            fails: true,
        };

        let result = execute("pairing-1", &repo, &port, &bus()).await;

        assert!(matches!(
            result,
            Err(DeletePairingError::StatusUnavailable(_))
        ));
        assert!(
            deleted.lock().unwrap().is_empty(),
            "pas de suppression à l'aveugle"
        );
    }

    // ── Suppressions en masse ────────────────────────────────────────────

    #[tokio::test]
    async fn vider_une_journee_epargne_les_rencontres_publiees() {
        let publie = a_pairing();
        let (a, b) = (a_pairing(), a_pairing());
        let repo = repo_with(vec![a_day(
            "Journée 3",
            vec![a.clone(), publie.clone(), b.clone()],
        )]);
        let deleted = repo.deleted.clone();
        let port = FakeStatusPort {
            published: vec![publie.id.to_string()],
            fails: false,
        };

        let kept = clear_round("round-1", &repo, &port, &FakeTeamInfoPort, &bus())
            .await
            .unwrap();

        assert_eq!(
            *deleted.lock().unwrap(),
            vec![a.id.to_string(), b.id.to_string()],
            "seules les rencontres non publiées sont supprimées"
        );
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].round_name, "Journée 3");
        assert_eq!(
            kept[0].home_team_name,
            format!("Équipe {}", publie.home_team_id)
        );
    }

    #[tokio::test]
    async fn vider_une_journee_sans_rapport_publie_supprime_tout() {
        let (a, b) = (a_pairing(), a_pairing());
        let repo = repo_with(vec![a_day("Journée 1", vec![a, b])]);
        let deleted = repo.deleted.clone();
        let port = FakeStatusPort {
            published: vec![],
            fails: false,
        };

        let kept = clear_round("round-1", &repo, &port, &FakeTeamInfoPort, &bus())
            .await
            .unwrap();

        assert_eq!(deleted.lock().unwrap().len(), 2);
        assert!(kept.is_empty());
    }

    #[tokio::test]
    async fn un_lot_entierement_publie_ne_supprime_rien() {
        let (a, b) = (a_pairing(), a_pairing());
        let repo = repo_with(vec![a_day("Journée 1", vec![a.clone(), b.clone()])]);
        let deleted = repo.deleted.clone();
        let port = FakeStatusPort {
            published: vec![a.id.to_string(), b.id.to_string()],
            fails: false,
        };
        let bus = bus();
        let mut rx = bus.subscribe();

        let kept = clear_round("round-1", &repo, &port, &FakeTeamInfoPort, &bus)
            .await
            .unwrap();

        assert!(deleted.lock().unwrap().is_empty());
        assert_eq!(kept.len(), 2);
        assert!(rx.try_recv().is_err(), "aucun événement ne doit être émis");
    }

    /// Supprimer la journée cascaderait en base sur ses pairings, garde-fou
    /// compris — elle doit survivre tant qu'une rencontre est conservée.
    #[tokio::test]
    async fn supprimer_une_journee_la_conserve_si_une_rencontre_resiste() {
        let publie = a_pairing();
        let repo = repo_with(vec![a_day("Journée 2", vec![a_pairing(), publie.clone()])]);
        let deleted_days = repo.deleted_days.clone();
        let port = FakeStatusPort {
            published: vec![publie.id.to_string()],
            fails: false,
        };

        let kept = delete_round("round-1", &repo, &port, &FakeTeamInfoPort, &bus())
            .await
            .unwrap();

        assert_eq!(kept.len(), 1);
        assert!(
            deleted_days.lock().unwrap().is_empty(),
            "la journée doit survivre"
        );
    }

    #[tokio::test]
    async fn supprimer_une_journee_la_supprime_si_rien_ne_resiste() {
        let repo = repo_with(vec![a_day("Journée 2", vec![a_pairing()])]);
        let deleted_days = repo.deleted_days.clone();
        let port = FakeStatusPort {
            published: vec![],
            fails: false,
        };

        let kept = delete_round("round-1", &repo, &port, &FakeTeamInfoPort, &bus())
            .await
            .unwrap();

        assert!(kept.is_empty());
        assert_eq!(*deleted_days.lock().unwrap(), vec!["round-1".to_string()]);
    }

    #[tokio::test]
    async fn vider_la_saison_traverse_toutes_les_journees() {
        let publie = a_pairing();
        let repo = repo_with(vec![
            a_day("Journée 1", vec![a_pairing(), a_pairing()]),
            a_day("Journée 2", vec![a_pairing(), publie.clone()]),
        ]);
        let deleted = repo.deleted.clone();
        let port = FakeStatusPort {
            published: vec![publie.id.to_string()],
            fails: false,
        };

        let kept = clear_season("season-1", &repo, &port, &FakeTeamInfoPort, &bus())
            .await
            .unwrap();

        assert_eq!(deleted.lock().unwrap().len(), 3);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].round_name, "Journée 2");
    }
}

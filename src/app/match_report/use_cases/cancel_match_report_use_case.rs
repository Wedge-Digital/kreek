//! Abandonner un rapport en cours.
//!
//! Le domaine sait annuler depuis les trois états d'avant publication, et le BC
//! `teams` écoute déjà l'app event pour libérer son verrou de saisie. Il ne
//! manquait qu'un chemin : le seul appelant de `cancel()` était le listener de
//! suppression d'appariement, si bien qu'abandonner un rapport exigeait qu'un
//! administrateur vide la journée — ce qui, pour un match programmé, efface la
//! rencontre du calendrier (carte 433).

use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct CancelMatchReportCommand {
    pub match_report_id: MatchReportId,
    /// Le nom de celui qui annule. Le coach ne saisit pas de motif — le geste
    /// est explicite, et la question ralentirait l'abandon d'un rapport ouvert
    /// par erreur. La raison dit donc **qui**, ce qui suffit au journal.
    pub cancelled_by: String,
}

#[derive(Debug, PartialEq)]
pub enum CancelMatchReportError {
    NotFound,
    /// Un rapport publié ne s'annule pas : défaire une publication a son propre
    /// chemin, la dépublication.
    NotCancellable(&'static str),
    Repository(String),
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: CancelMatchReportCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
) -> Result<(), CancelMatchReportError> {
    let mr_id = cmd.match_report_id.to_string();
    let state = repo
        .find_by_id(&mr_id)
        .await
        .map_err(|e| CancelMatchReportError::Repository(e.to_string()))?
        .ok_or(CancelMatchReportError::NotFound)?;

    let raison = format!("Annulé par {}", cmd.cancelled_by);
    let (version, event) = match state {
        MatchReportState::PreMatch(pm) => (pm.version, pm.cancel(raison)),
        MatchReportState::ReadyToPublish(rtp) => (rtp.version, rtp.cancel(raison)),
        // `Draft` en fait partie : le domaine l'autoriserait, mais un brouillon
        // ne verrouille rien et son écran a déjà un retour.
        autre => {
            let etat = etiquette(&autre);
            tracing::warn!(match_report_id = %mr_id, etat, "annulation refusée");
            return Err(CancelMatchReportError::NotCancellable(etat));
        }
    };

    repo.append(&mr_id, &event, version)
        .await
        .map_err(|e| CancelMatchReportError::Repository(e.to_string()))?;

    // Appender ne suffit pas : c'est le bus interne que le publisher écoute, et
    // sans cette émission aucun app event ne part — ni vers `competitions` pour
    // défaire la ligne de résultats, ni vers `teams` pour libérer le verrou de
    // saisie. Le rapport serait annulé et les deux équipes resteraient bloquées.
    emettre(bus, event.to_enveloppe(&mr_id));
    Ok(())
}

fn etiquette(state: &MatchReportState) -> &'static str {
    match state {
        MatchReportState::Draft(_) => "Draft",
        MatchReportState::PreMatch(_) => "PreMatch",
        MatchReportState::ReadyToPublish(_) => "ReadyToPublish",
        MatchReportState::Published(_) => "Published",
        MatchReportState::Cancelled(_) => "Cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::events::MatchReportDomainEvent;
    use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
    use crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish;
    use crate::app::match_report::domain::match_report_repository_port::{
        MatchActionRow, RepositoryError,
    };
    use crate::app::match_report::domain::value_objects::{
        DedicatedFans, MatchReportOrigin, TeamSide, TeamValue,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RoundId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
    use crate::common::services::event_bus::event_bus::new_bus;
    use std::sync::Mutex;

    /// `MatchReportState` n'est pas `Clone` : le dépôt le rend une fois, ce qui
    /// suffit — le use case ne le lit qu'une fois.
    struct DepotSimule {
        state: Mutex<Option<MatchReportState>>,
        appendus: Mutex<Vec<MatchReportDomainEvent>>,
    }

    impl DepotSimule {
        fn avec(state: MatchReportState) -> Self {
            Self {
                state: Mutex::new(Some(state)),
                appendus: Mutex::new(vec![]),
            }
        }
    }

    #[async_trait::async_trait]
    impl IMatchReportRepository for DepotSimule {
        async fn append(
            &self,
            _: &str,
            event: &MatchReportDomainEvent,
            _: u64,
        ) -> Result<u64, RepositoryError> {
            self.appendus.lock().unwrap().push(event.clone());
            Ok(1)
        }
        async fn find_space_id(&self, _: &str) -> Result<Option<String>, RepositoryError> {
            Ok(None)
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<MatchReportState>, RepositoryError> {
            Ok(self.state.lock().unwrap().take())
        }
        async fn append_many(
            &self,
            _: &str,
            _: Vec<MatchReportDomainEvent>,
            _: u64,
        ) -> Result<u64, RepositoryError> {
            Ok(1)
        }
        async fn find_id_by_pairing(&self, _: &str) -> Result<Option<String>, RepositoryError> {
            Ok(None)
        }
        async fn find_phases_by_pairings(
            &self,
            _: &[String],
        ) -> Result<Vec<(String, String)>, RepositoryError> {
            Ok(vec![])
        }
        async fn find_id_by_round_and_teams(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<Option<String>, RepositoryError> {
            Ok(None)
        }
        async fn find_actions_by_match_and_side(
            &self,
            _: &str,
            _: TeamSide,
        ) -> Result<Vec<MatchActionRow>, RepositoryError> {
            Ok(vec![])
        }
        async fn find_team_ids(
            &self,
            _: &str,
        ) -> Result<Option<(String, String)>, RepositoryError> {
            Ok(None)
        }
    }

    fn pre_match() -> MatchReportPreMatch {
        MatchReportPreMatch {
            id: MatchReportId::new(),
            space_id: SpaceId::new(),
            competition_id: CompetitionId::new(),
            season_id: SeasonId::new(),
            round_id: RoundId::new(),
            home_team_id: TeamId::new(),
            away_team_id: TeamId::new(),
            created_by: CoachId::new(),
            origin: MatchReportOrigin::Manual,
            pairing_id: None,
            home_fan_roll: None,
            away_fan_roll: None,
            home_dedicated_fans: DedicatedFans::default(),
            away_dedicated_fans: DedicatedFans::default(),
            home_team_value: Some(TeamValue::try_new(1000).unwrap()),
            away_team_value: Some(TeamValue::try_new(1000).unwrap()),
            home_inducements: None,
            away_inducements: None,
            star_engagements: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
            home_actions: vec![],
            away_actions: vec![],
            version: 3,
        }
    }

    fn pret_a_publier() -> MatchReportReadyToPublish {
        use crate::app::match_report::domain::value_objects::{FanFactorMod, MatchGain};
        MatchReportReadyToPublish::from_pre_match(
            &pre_match(),
            MatchGain::try_new(50_000).unwrap(),
            MatchGain::try_new(40_000).unwrap(),
            FanFactorMod::try_new(1).unwrap(),
            FanFactorMod::try_new(-1).unwrap(),
            None,
            None,
        )
    }

    fn commande() -> CancelMatchReportCommand {
        CancelMatchReportCommand {
            match_report_id: MatchReportId::new(),
            cancelled_by: "DevCoach".to_string(),
        }
    }

    fn raison(depot: &DepotSimule) -> String {
        match depot.appendus.lock().unwrap().first() {
            Some(MatchReportDomainEvent::MatchReportCancelled { reason, .. }) => reason.clone(),
            autre => panic!("attendu une annulation, obtenu {autre:?}"),
        }
    }

    /// L'état où l'on abandonne le plus souvent : on s'est trompé d'équipes et
    /// on s'en aperçoit à la saisie.
    #[tokio::test]
    async fn un_rapport_en_pre_match_s_annule() {
        let depot = DepotSimule::avec(MatchReportState::PreMatch(pre_match()));
        assert!(execute(commande(), &depot, &new_bus()).await.is_ok());
        assert_eq!(raison(&depot), "Annulé par DevCoach");
    }

    #[tokio::test]
    async fn un_rapport_pret_a_publier_s_annule() {
        let rtp = pret_a_publier();
        let depot = DepotSimule::avec(MatchReportState::ReadyToPublish(rtp));
        assert!(execute(commande(), &depot, &new_bus()).await.is_ok());
        assert_eq!(raison(&depot), "Annulé par DevCoach");
    }

    /// Défaire une publication a son propre chemin — la dépublication. Le refus
    /// laisse une ligne : c'est l'état qui l'explique, pas le code de retour.
    #[tokio::test]
    async fn un_rapport_publie_refuse_l_annulation() {
        let rtp = pret_a_publier();
        let (publie, _) = rtp.publish(CoachId::new());
        let publie = MatchReportState::Published(publie);
        let depot = DepotSimule::avec(publie);

        assert_eq!(
            execute(commande(), &depot, &new_bus()).await,
            Err(CancelMatchReportError::NotCancellable("Published"))
        );
        assert!(
            depot.appendus.lock().unwrap().is_empty(),
            "un refus ne doit rien écrire"
        );
    }
}

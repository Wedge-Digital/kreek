use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{EntryMode, MatchReportOrigin};
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct CreateMatchReportCommand {
    pub space_id: SpaceId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub created_by: CoachId,
    pub origin: MatchReportOrigin,
    pub pairing_id: Option<String>,
    /// Le mode de saisie, pour le chemin qui confirme aussitôt (`Manual`).
    /// Un brouillon du calendrier l'ignore : son mode se choisit quand il
    /// démarre (carte 631).
    pub entry_mode: EntryMode,
}

#[derive(Debug)]
pub enum CreateMatchReportError {
    SameTeam,
    /// Une autre écriture est passée entre le chargement et l'append — deux
    /// « Commencer » simultanés (carte 631, sur le modèle de la carte 623).
    Conflict,
    Repository(String),
}

impl CreateMatchReportError {
    /// `ConcurrentWrite` garde son sens : sans cette conversion, il devenait un
    /// `Repository` comme un autre, et le contrôleur rendait 500.
    fn from_repository(e: RepositoryError) -> Self {
        match e {
            RepositoryError::ConcurrentWrite => Self::Conflict,
            other => Self::Repository(other.to_string()),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: CreateMatchReportCommand,
    repo: &dyn IMatchReportRepository,
    event_bus: &EventBus,
) -> Result<MatchReportId, CreateMatchReportError> {
    // **Plus de déduplication ici** (carte 555).
    //
    // Elle existait pour départager deux créateurs concurrents : le contrôleur
    // du hors calendrier et `pairing_created_listener` créaient chacun un
    // rapport, et le premier arrivé devait retrouver celui de l'autre. Elle n'y
    // parvenait pas — chacun lisait avant que l'autre n'écrive, d'où 38
    // rencontres à deux rapports.
    //
    // Il n'y a plus qu'un créateur par cas métier : l'événement pour le
    // calendrier, le contrôleur pour le hors calendrier. L'aiguillage entre les
    // deux se fait dans le contrôleur, où il se lit.
    let id = MatchReportId::new();

    let (draft, event) = MatchReportDraft::create(
        id,
        cmd.space_id,
        cmd.competition_id,
        cmd.season_id,
        cmd.round_id,
        cmd.home_team_id,
        cmd.away_team_id,
        cmd.created_by,
        cmd.origin,
        cmd.pairing_id,
    )
    .map_err(|_| CreateMatchReportError::SameTeam)?;

    repo.append(&id.to_string(), &event, 0)
        .await
        .map_err(CreateMatchReportError::from_repository)?;

    // Le coach vient de choisir lui-même compétition, journée et équipes : lui
    // redemander la même sélection serait un aller-retour pour rien.
    //
    // # Ce que l'auto-confirmation garantit en plus depuis la carte 552
    //
    // Le chemin hors calendrier crée désormais un appariement avant son
    // rapport, ce qui réveille `pairing_created_listener` — lequel crée, lui
    // aussi, un brouillon. Les deux se courent après : selon celui qui arrive
    // le premier, le rapport ressort confirmé ou non.
    //
    // Confirmer ici rend l'état **déterministe** : le rapport est toujours en
    // `PreMatch` au retour. Le listener, arrivant après, retrouve ce rapport et
    // `confirm_existing` le laisse tel quel. Sans cela, l'appelant ne sait pas
    // dans quel état il récupère la main — et une confirmation de trop répond
    // `409`, ce que la suite e2e a montré sur une trentaine de tests.
    if cmd.origin == MatchReportOrigin::Manual {
        confirm_draft(draft, cmd.created_by, cmd.entry_mode, repo, event_bus).await?;
    }

    Ok(id)
}

/// Confirme la sélection d'un rapport **qui existe déjà**, s'il est encore un
/// brouillon (carte 555).
///
/// C'est le cas « la rencontre est au calendrier » : le brouillon a été créé au
/// tirage par `pairing_created_listener`, et le coach vient de demander à le
/// saisir. Son POST **est** la confirmation.
///
/// Sans elle, le coach repartirait sur un brouillon non confirmé et devrait
/// revalider une sélection qu'il vient d'envoyer — ce que la confirmation
/// suivante refuserait d'ailleurs si le calendrier avait retenu l'autre camp.
///
/// Les autres états sont rendus tels quels : un rapport déjà commencé n'a pas à
/// être reconfirmé.
#[tracing::instrument(skip_all, fields(match_report_id = ?match_report_id))]
pub async fn confirmer_si_brouillon(
    match_report_id: &str,
    confirmed_by: CoachId,
    mode: EntryMode,
    repo: &dyn IMatchReportRepository,
    event_bus: &EventBus,
) -> Result<(), CreateMatchReportError> {
    let state = repo
        .find_by_id(match_report_id)
        .await
        .map_err(CreateMatchReportError::from_repository)?
        .ok_or_else(|| CreateMatchReportError::Repository("rapport introuvable".into()))?;

    match state {
        MatchReportState::Draft(draft) => {
            confirm_draft(draft, confirmed_by, mode, repo, event_bus).await
        }
        _ => Ok(()),
    }
}

/// Confirme et démarre : `SelectionConfirmed` et `EntryModeChosen` partent
/// dans le **même** `append_many` — un démarrage n'existe pas sans son mode
/// (carte 631). Deux démarrages simultanés : le second échoue sur la version.
async fn confirm_draft(
    draft: MatchReportDraft,
    confirmed_by: CoachId,
    mode: EntryMode,
    repo: &dyn IMatchReportRepository,
    event_bus: &EventBus,
) -> Result<(), CreateMatchReportError> {
    let mr_id_str = draft.id.to_string();
    let expected_version = draft.version;
    let (_, events) = draft.confirm_selection(confirmed_by, mode);

    repo.append_many(&mr_id_str, events.clone(), expected_version)
        .await
        .map_err(CreateMatchReportError::from_repository)?;

    // Les domain events, sur le bus du BC : c'est le publisher qui fait de
    // `SelectionConfirmed` un `MatchReportConfirmed` pour `teams` et
    // `competitions` (carte 352) ; `EntryModeChosen` ne sort pas du BC.
    for event in &events {
        emettre(event_bus, event.to_enveloppe(&mr_id_str));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::events::MatchReportDomainEvent;
    use crate::app::match_report::domain::match_report_repository_port::{
        MatchActionRow, RepositoryError,
    };
    use crate::app::match_report::domain::match_report_state::rehydrate;
    use crate::app::match_report::domain::value_objects::TeamSide;
    use crate::common::services::event_bus::event_bus::new_bus;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeMatchReportRepo {
        events: Mutex<HashMap<String, Vec<MatchReportDomainEvent>>>,
    }

    #[async_trait]
    impl IMatchReportRepository for FakeMatchReportRepo {
        async fn find_entry_mode(
            &self,
            _: &str,
        ) -> Result<
            Option<crate::app::match_report::domain::match_report_repository_port::EntryModeRow>,
            RepositoryError,
        > {
            Ok(None)
        }
        async fn find_access_scope(
            &self,
            _: &str,
        ) -> Result<Option<(String, String, String)>, RepositoryError> {
            Ok(None)
        }

        async fn find_team_ids(
            &self,
            _: &str,
        ) -> Result<Option<(String, String)>, RepositoryError> {
            Ok(None)
        }
        /// Doublure : le contrôle d'appartenance est exercé par les tests de
        /// handler, sur une vraie base.
        async fn find_space_id(&self, _: &str) -> Result<Option<String>, RepositoryError> {
            Ok(None)
        }

        async fn append(
            &self,
            match_report_id: &str,
            event: &MatchReportDomainEvent,
            _expected_version: u64,
        ) -> Result<u64, RepositoryError> {
            let mut events = self.events.lock().unwrap();
            let entry = events.entry(match_report_id.to_string()).or_default();
            entry.push(event.clone());
            Ok(entry.len() as u64)
        }
        async fn find_by_id(
            &self,
            match_report_id: &str,
        ) -> Result<Option<MatchReportState>, RepositoryError> {
            let events = self.events.lock().unwrap();
            match events.get(match_report_id) {
                Some(evs) => Ok(Some(rehydrate(evs.clone()).expect("rehydrate"))),
                None => Ok(None),
            }
        }
        async fn append_many(
            &self,
            match_report_id: &str,
            new_events: Vec<MatchReportDomainEvent>,
            _expected_version: u64,
        ) -> Result<u64, RepositoryError> {
            let mut events = self.events.lock().unwrap();
            let entry = events.entry(match_report_id.to_string()).or_default();
            entry.extend(new_events);
            Ok(entry.len() as u64)
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
            round_id: &str,
            team_a: &str,
            team_b: &str,
        ) -> Result<Option<String>, RepositoryError> {
            let events = self.events.lock().unwrap();
            for (id, evs) in events.iter() {
                let Some(MatchReportDomainEvent::MatchReportCreated {
                    round_id: r,
                    home_team_id,
                    away_team_id,
                    ..
                }) = evs.first()
                else {
                    continue;
                };
                let (r, h, a) = (
                    r.to_string(),
                    home_team_id.to_string(),
                    away_team_id.to_string(),
                );
                let same_teams = (h == team_a && a == team_b) || (h == team_b && a == team_a);
                if r == round_id && same_teams {
                    return Ok(Some(id.clone()));
                }
            }
            Ok(None)
        }
        async fn find_actions_by_match_and_side(
            &self,
            _: &str,
            _: TeamSide,
        ) -> Result<Vec<MatchActionRow>, RepositoryError> {
            Ok(vec![])
        }
    }

    fn sample_cmd(origin: MatchReportOrigin) -> CreateMatchReportCommand {
        CreateMatchReportCommand {
            space_id: SpaceId::new(),
            competition_id: CompetitionId::new(),
            season_id: SeasonId::new(),
            round_id: RoundId::new(),
            home_team_id: TeamId::new(),
            away_team_id: TeamId::new(),
            created_by: CoachId::new(),
            origin,
            pairing_id: None,
            entry_mode: EntryMode::SoloEntry,
        }
    }

    /// La saisie où le coach a **déjà choisi** ressort confirmée, en un seul
    /// aller-retour.
    ///
    /// # Ce que ce test protège depuis la carte 552
    ///
    /// Le chemin hors calendrier crée un appariement avant son rapport, ce qui
    /// réveille `pairing_created_listener` — lequel crée un brouillon lui
    /// aussi. Sans cette confirmation, l'état de retour dépend de celui des
    /// deux qui arrive le premier, et l'appelant ne sait plus s'il doit
    /// confirmer ou non. Une confirmation de trop répond `409` : c'est ce que
    /// la suite e2e a montré sur une trentaine de tests.
    #[tokio::test]
    async fn la_saisie_deja_choisie_ressort_confirmee() {
        let repo = FakeMatchReportRepo::default();
        let bus = new_bus();

        let id = execute(sample_cmd(MatchReportOrigin::Manual), &repo, &bus)
            .await
            .unwrap();

        let state = repo.find_by_id(&id.to_string()).await.unwrap().unwrap();
        assert!(
            matches!(state, MatchReportState::PreMatch(_)),
            "état : {state:?}"
        );
    }

    /// La confirmation sort par le **bus du BC**, en domain event : c'est le
    /// publisher qui en fait `MatchReportConfirmed` (carte 352). Le bus reçu
    /// ici est le bus interne — le use case ne connaît plus l'autre.
    #[tokio::test]
    async fn the_confirmation_is_emitted_as_a_domain_event() {
        let repo = FakeMatchReportRepo::default();
        let bus = new_bus();
        let mut rx = bus.subscribe();

        execute(sample_cmd(MatchReportOrigin::Manual), &repo, &bus)
            .await
            .unwrap();

        let types: Vec<String> = std::iter::from_fn(|| rx.try_recv().ok())
            .map(|envelope| envelope.event_type)
            .collect();
        assert!(
            types.iter().any(|t| t == "SelectionConfirmed"),
            "émis : {types:?}"
        );
        assert!(
            !types.iter().any(|t| t == "MatchReportConfirmed"),
            "émis : {types:?}"
        );
    }

    /// Le démarrage part **avec son mode** : les deux événements sur le bus,
    /// dans l'ordre où ils sont appendés (carte 631).
    #[tokio::test]
    async fn the_entry_mode_is_emitted_with_the_confirmation() {
        let repo = FakeMatchReportRepo::default();
        let bus = new_bus();
        let mut rx = bus.subscribe();

        execute(sample_cmd(MatchReportOrigin::Manual), &repo, &bus)
            .await
            .unwrap();

        let types: Vec<String> = std::iter::from_fn(|| rx.try_recv().ok())
            .map(|envelope| envelope.event_type)
            .collect();
        assert_eq!(
            types,
            ["SelectionConfirmed", "EntryModeChosen"],
            "émis : {types:?}"
        );
    }

    /// Hors calendrier, celui qui crée est celui qui démarre.
    #[tokio::test]
    async fn the_creator_of_an_out_of_schedule_report_starts_it() {
        let repo = FakeMatchReportRepo::default();
        let cmd = sample_cmd(MatchReportOrigin::Manual);
        let creator = cmd.created_by;

        let id = execute(cmd, &repo, &new_bus()).await.unwrap();

        let Some(MatchReportState::PreMatch(pm)) = repo.find_by_id(&id.to_string()).await.unwrap()
        else {
            panic!("attendu PreMatch");
        };
        assert_eq!(
            (pm.started_by, pm.entry_mode),
            (creator, EntryMode::SoloEntry)
        );
    }

    /// Deux « Commencer » simultanés : le second échoue sur la version, et le
    /// conflit arrive jusqu'au contrôleur au lieu de devenir un 500 (carte 631).
    #[tokio::test]
    async fn a_conflict_on_the_start_is_relayed() {
        use crate::app::match_report::use_cases::test_doubles::ConflictingRepository;
        let (draft, _) = MatchReportDraft::create(
            MatchReportId::new(),
            SpaceId::new(),
            CompetitionId::new(),
            SeasonId::new(),
            RoundId::new(),
            TeamId::new(),
            TeamId::new(),
            CoachId::new(),
            MatchReportOrigin::Pairing,
            None,
        )
        .unwrap();
        let id = draft.id.to_string();
        let repo = ConflictingRepository::holding(MatchReportState::Draft(draft));

        let erreur =
            confirmer_si_brouillon(&id, CoachId::new(), EntryMode::SoloEntry, &repo, &new_bus())
                .await
                .expect_err("l'append doit échouer");
        assert!(
            matches!(erreur, CreateMatchReportError::Conflict),
            "{erreur:?}"
        );
    }

    #[tokio::test]
    async fn pairing_origin_stays_in_draft_until_explicitly_confirmed() {
        let repo = FakeMatchReportRepo::default();
        let bus = new_bus();

        let id = execute(sample_cmd(MatchReportOrigin::Pairing), &repo, &bus)
            .await
            .unwrap();

        let state = repo.find_by_id(&id.to_string()).await.unwrap().unwrap();
        assert!(matches!(state, MatchReportState::Draft(_)));
    }

    /// **Le use case n'est plus idempotent, et c'est voulu** (carte 555).
    ///
    /// Il l'était pour départager deux créateurs concurrents — le contrôleur du
    /// hors calendrier et `pairing_created_listener` — qui créaient chacun un
    /// rapport pour la même rencontre. La déduplication n'y parvenait pas :
    /// chacun lisait avant que l'autre n'écrive, d'où 38 rencontres à deux
    /// rapports en base.
    ///
    /// Il n'y a plus qu'un créateur par cas métier, et la garde est remontée
    /// **dans le contrôleur**, où elle se lit : `rapport_de_la_rencontre`
    /// aiguille vers l'ouverture du rapport existant au lieu d'en créer un.
    ///
    /// Ce test remplace `calling_execute_again_…_confirms_the_existing_draft`,
    /// qui affirmait l'inverse. Il documente une garantie **perdue**, pour que
    /// quiconque rappelle `execute` deux fois sache ce qu'il obtient.
    #[tokio::test]
    async fn deux_appels_creent_deux_rapports_la_garde_est_ailleurs() {
        let repo = FakeMatchReportRepo::default();
        let bus = new_bus();
        let premier = execute(sample_cmd(MatchReportOrigin::Pairing), &repo, &bus)
            .await
            .unwrap();
        let second = execute(sample_cmd(MatchReportOrigin::Pairing), &repo, &bus)
            .await
            .unwrap();

        assert_ne!(
            premier, second,
            "sans garde en amont, deux appels produisent deux rapports"
        );
    }
}

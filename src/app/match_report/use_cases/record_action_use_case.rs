use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{
    ActionId, ActionPlayer, HatredKeyword, MatchActionType, TeamSide, TempPlayerId, TurnNumber,
};
use crate::app::match_report::ports::{IKeywordCatalogPort, IPlayerDataPort};
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;

#[derive(Debug)]
pub struct RecordActionCommand {
    pub match_report_id: MatchReportId,
    pub team_side: TeamSide,
    pub turn: TurnNumber,
    pub player: ActionPlayer,
    pub action: MatchActionType,
    /// Le mot-clef haï, **validé en forme seulement**.
    ///
    /// Il voyage à côté de l'action et non dedans : `hatred_skill_uid` ne peut
    /// être rempli qu'après consultation du catalogue, ce que le contrôleur ne
    /// sait pas faire. C'est le use case qui les réunit, juste avant d'appeler
    /// le domaine.
    pub hatred: Option<HatredKeyword>,
    pub recorded_by: CoachId,
}

pub struct RecordActionOutcome {
    pub action_id: String,
}

#[derive(Debug)]
pub enum RecordActionError {
    NotFound,
    NotInPreMatchPhase,
    PlayerNotFound(String),
    TempPlayerNotFound(String),
    Domain(DomainError),
    /// L'uid porte le mot-clef refusé : sans lui, le journal dirait qu'un
    /// mot-clef a été refusé sans dire lequel, et le premier corpus incomplet
    /// coûterait une investigation.
    UnknownKeyword(String),
    Repository(String),
}

/// Réunit le choix du coach et ce que ce choix valait au moment présent.
///
/// Le mot-clef arrive validé **en forme** (`HatredKeyword`) ; son existence,
/// elle, se vérifie ici. `find_hateable` ne rend que des mots-clefs haïssables,
/// donc un `BLITZER` — qui existe pourtant au corpus — en sort comme un inconnu.
/// C'est voulu : l'écran ne le propose pas, une requête qui le porte vient
/// d'ailleurs, et lui inventer une erreur distincte documenterait au client une
/// nuance dont il n'a pas à connaître l'existence.
///
/// Une Haine posée sur autre chose qu'une blessure est ignorée sans bruit : la
/// commande ne permet pas au contrôleur de l'y attacher, et le cas n'est
/// atteignable que par un appel direct.
fn appliquer_haine(
    action: MatchActionType,
    hatred: Option<&HatredKeyword>,
    keywords: &dyn IKeywordCatalogPort,
) -> Result<MatchActionType, RecordActionError> {
    let (Some(mot), MatchActionType::Blesse { injury, .. }) = (hatred, &action) else {
        return Ok(action);
    };
    let dto = keywords
        .find_hateable(mot.as_ref())
        .ok_or_else(|| RecordActionError::UnknownKeyword(mot.to_string()))?;
    Ok(MatchActionType::Blesse {
        injury: injury.clone(),
        hatred: Some(mot.clone()),
        hatred_skill_uid: Some(dto.hate_skill_uid),
    })
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RecordActionCommand,
    repo: &dyn IMatchReportRepository,
    player_data: &dyn IPlayerDataPort,
    keywords: &dyn IKeywordCatalogPort,
) -> Result<RecordActionOutcome, RecordActionError> {
    // Le catalogue n'est consulté que si une Haine est déclarée, et le refus
    // tombe avant tout chargement d'agrégat.
    let action = appliquer_haine(cmd.action, cmd.hatred.as_ref(), keywords)?;

    let mr_id = cmd.match_report_id.to_string();
    let state = repo
        .find_by_id(&mr_id)
        .await
        .map_err(|e| RecordActionError::Repository(e.to_string()))?;
    let pm = match state.ok_or(RecordActionError::NotFound)? {
        MatchReportState::PreMatch(pm) => pm,
        MatchReportState::ReadyToPublish(rtp) => rtp.into_pre_match(),
        _ => return Err(RecordActionError::NotInPreMatchPhase),
    };

    let (display_name, position) =
        resolve_player_info(&cmd.player, cmd.team_side, &pm, player_data).await?;
    let action_id = ActionId(ulid::Ulid::new().to_string());
    // Le refus vient du domaine — le use case ne fait que le porter.
    let (_, event) = pm
        .record_action(
            cmd.team_side,
            cmd.turn,
            cmd.player,
            action,
            display_name,
            position,
            action_id.clone(),
            cmd.recorded_by,
        )
        .map_err(RecordActionError::Domain)?;

    let outcome = RecordActionOutcome {
        action_id: action_id.0.clone(),
    };
    repo.append(&mr_id, &event, pm.version)
        .await
        .map_err(|e| RecordActionError::Repository(e.to_string()))?;
    Ok(outcome)
}

async fn resolve_player_info(
    player: &ActionPlayer,
    side: TeamSide,
    pm: &crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch,
    player_data: &dyn IPlayerDataPort,
) -> Result<(String, String), RecordActionError> {
    match player {
        ActionPlayer::Regular(pid) => {
            let id = pid.to_string();
            let display = player_data
                .find_player_display(&id)
                .await
                .ok_or_else(|| RecordActionError::PlayerNotFound(id.clone()))?;
            let position = player_data
                .find_player_position(&id)
                .await
                .unwrap_or_default();
            Ok((display, position))
        }
        ActionPlayer::Temp(tid) => {
            let display = resolve_temp_display(tid, side, pm)?;
            Ok((display, String::new()))
        }
    }
}

fn resolve_temp_display(
    tid: &TempPlayerId,
    side: TeamSide,
    pm: &crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch,
) -> Result<String, RecordActionError> {
    pm.temp_players_for(side)
        .iter()
        .find(|tp| &tp.id == tid)
        .map(|tp| {
            tp.display_name
                .clone()
                .unwrap_or_else(|| format!("{:?}", tp.kind))
        })
        .ok_or_else(|| RecordActionError::TempPlayerNotFound(tid.0.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
    use crate::app::match_report::domain::value_objects::{
        DedicatedFans, MatchReportOrigin, TeamValue, TempPlayer, TempPlayerKind,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{
        CompetitionId, MatchReportId, RoundId, SeasonId,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};

    fn make_pm() -> MatchReportPreMatch {
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
            version: 1,
        }
    }

    // ── Haine (carte 401) ────────────────────────────────────────────────────

    use crate::app::match_report::domain::events::MatchReportDomainEvent;
    use crate::app::match_report::domain::match_report_repository_port::{
        MatchActionRow, RepositoryError,
    };
    use crate::app::match_report::domain::value_objects::InjuryType;
    use crate::app::match_report::ports::{KeywordDto, PositionCountDto};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Compte ses consultations : c'est ce compteur qui prouve que le catalogue
    /// n'est pas interrogé quand aucune Haine n'est déclarée.
    struct CatalogueEspion {
        appels: AtomicUsize,
    }

    impl CatalogueEspion {
        fn new() -> Self {
            Self {
                appels: AtomicUsize::new(0),
            }
        }
        fn appels(&self) -> usize {
            self.appels.load(Ordering::Relaxed)
        }
    }

    impl IKeywordCatalogPort for CatalogueEspion {
        fn list_hateable(&self) -> Vec<KeywordDto> {
            vec![]
        }
        fn find_hateable(&self, uid: &str) -> Option<KeywordDto> {
            self.appels.fetch_add(1, Ordering::Relaxed);
            (uid == "DARK_ELF").then(|| KeywordDto {
                uid: uid.to_string(),
                label: "Elfe Noir".to_string(),
                hate_skill_uid: "HAINE_DARK_ELF".to_string(),
            })
        }
    }

    fn blessure() -> MatchActionType {
        MatchActionType::Blesse {
            injury: InjuryType::Amoche,
            hatred: None,
            hatred_skill_uid: None,
        }
    }

    #[test]
    fn le_chemin_nominal_fige_le_mot_clef_et_sa_competence() {
        let mot = HatredKeyword::try_new("DARK_ELF").unwrap();
        let action = appliquer_haine(blessure(), Some(&mot), &CatalogueEspion::new()).unwrap();
        match action {
            MatchActionType::Blesse {
                hatred,
                hatred_skill_uid,
                ..
            } => {
                assert_eq!(hatred.map(|h| h.to_string()).as_deref(), Some("DARK_ELF"));
                assert_eq!(hatred_skill_uid.as_deref(), Some("HAINE_DARK_ELF"));
            }
            other => panic!("attendu une blessure, obtenu {other:?}"),
        }
    }

    #[test]
    fn un_mot_clef_inconnu_du_catalogue_est_refuse_avec_son_uid() {
        let mot = HatredKeyword::try_new("ESPECE_INVENTEE").unwrap();
        match appliquer_haine(blessure(), Some(&mot), &CatalogueEspion::new()) {
            Err(RecordActionError::UnknownKeyword(uid)) => assert_eq!(uid, "ESPECE_INVENTEE"),
            other => panic!("attendu UnknownKeyword, obtenu {other:?}"),
        }
    }

    /// `BLITZER` existe au corpus mais n'est pas haïssable. Le port ne le rend
    /// pas, donc il est refusé **comme un inconnu** — délibérément : l'écran ne
    /// le propose pas, une requête qui le porte vient d'ailleurs.
    #[test]
    fn un_mot_clef_non_haissable_est_refuse_comme_un_inconnu() {
        let mot = HatredKeyword::try_new("BLITZER").unwrap();
        assert!(matches!(
            appliquer_haine(blessure(), Some(&mot), &CatalogueEspion::new()),
            Err(RecordActionError::UnknownKeyword(_))
        ));
    }

    #[test]
    fn sans_haine_declaree_le_catalogue_n_est_pas_consulte() {
        let espion = CatalogueEspion::new();
        let action = appliquer_haine(MatchActionType::Touchdown, None, &espion).unwrap();
        assert!(matches!(action, MatchActionType::Touchdown));
        assert!(appliquer_haine(blessure(), None, &espion).is_ok());
        assert_eq!(espion.appels(), 0, "le catalogue ne doit pas être lu");
    }

    /// Un dépôt qui **panique à la moindre sollicitation**.
    ///
    /// C'est lui qui prouve « aucune écriture » : le refus tombe avant tout
    /// chargement d'agrégat, donc aucune de ces méthodes ne doit être atteinte.
    struct DepotIntouchable;

    #[async_trait]
    impl IMatchReportRepository for DepotIntouchable {
        async fn append(
            &self,
            _: &str,
            _: &MatchReportDomainEvent,
            _: u64,
        ) -> Result<u64, RepositoryError> {
            panic!("le dépôt ne doit pas être sollicité")
        }
        async fn find_space_id(&self, _: &str) -> Result<Option<String>, RepositoryError> {
            panic!("le dépôt ne doit pas être sollicité")
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<MatchReportState>, RepositoryError> {
            panic!("le dépôt ne doit pas être sollicité")
        }
        async fn append_many(
            &self,
            _: &str,
            _: Vec<MatchReportDomainEvent>,
            _: u64,
        ) -> Result<u64, RepositoryError> {
            panic!("le dépôt ne doit pas être sollicité")
        }
        async fn find_id_by_pairing(&self, _: &str) -> Result<Option<String>, RepositoryError> {
            panic!("le dépôt ne doit pas être sollicité")
        }
        async fn find_phases_by_pairings(
            &self,
            _: &[String],
        ) -> Result<Vec<(String, String)>, RepositoryError> {
            panic!("le dépôt ne doit pas être sollicité")
        }
        async fn find_id_by_round_and_teams(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<Option<String>, RepositoryError> {
            panic!("le dépôt ne doit pas être sollicité")
        }
        async fn find_actions_by_match_and_side(
            &self,
            _: &str,
            _: TeamSide,
        ) -> Result<Vec<MatchActionRow>, RepositoryError> {
            panic!("le dépôt ne doit pas être sollicité")
        }
    }

    struct JoueurAbsent;

    #[async_trait]
    impl IPlayerDataPort for JoueurAbsent {
        async fn count_available_players(&self, _: &str) -> Result<usize, String> {
            panic!("le port joueur ne doit pas être sollicité")
        }
        async fn find_player_display(&self, _: &str) -> Option<String> {
            panic!("le port joueur ne doit pas être sollicité")
        }
        async fn find_player_position(&self, _: &str) -> Option<String> {
            panic!("le port joueur ne doit pas être sollicité")
        }
        async fn find_player_counts_by_position(&self, _: &str) -> Vec<PositionCountDto> {
            panic!("le port joueur ne doit pas être sollicité")
        }
        async fn has_spent_spp_since_match(&self, _: &str, _: &str) -> Result<bool, String> {
            panic!("le port joueur ne doit pas être sollicité")
        }
    }

    #[tokio::test]
    async fn un_mot_clef_inconnu_ne_touche_pas_au_depot() {
        let cmd = RecordActionCommand {
            match_report_id: MatchReportId::new(),
            team_side: TeamSide::Home,
            turn: TurnNumber::try_new(1).unwrap(),
            player: ActionPlayer::Regular(
                crate::app::shared_kernel::bloodbowl::ids::PlayerId::new(),
            ),
            action: blessure(),
            hatred: Some(HatredKeyword::try_new("ESPECE_INVENTEE").unwrap()),
            recorded_by: CoachId::new(),
        };
        let r = execute(
            cmd,
            &DepotIntouchable,
            &JoueurAbsent,
            &CatalogueEspion::new(),
        )
        .await;
        assert!(matches!(r, Err(RecordActionError::UnknownKeyword(_))));
    }

    #[test]
    fn resolve_temp_display_finds_known_temp_player() {
        let mut pm = make_pm();
        let tid = TempPlayerId("TP01".into());
        pm.home_temp_players.push(TempPlayer {
            id: tid.clone(),
            team_id: pm.home_team_id.clone(),
            kind: TempPlayerKind::StarPlayer {
                ref_uid: "MORG".into(),
                position_uid: String::new(),
            },
            display_name: Some("Morg 'N' Thorg".into()),
        });
        let result = resolve_temp_display(&tid, TeamSide::Home, &pm);
        assert_eq!(result.unwrap(), "Morg 'N' Thorg");
    }

    #[test]
    fn resolve_temp_display_returns_error_for_unknown_id() {
        let pm = make_pm();
        let tid = TempPlayerId("UNKNOWN".into());
        let result = resolve_temp_display(&tid, TeamSide::Home, &pm);
        assert!(matches!(
            result,
            Err(RecordActionError::TempPlayerNotFound(_))
        ));
    }
}

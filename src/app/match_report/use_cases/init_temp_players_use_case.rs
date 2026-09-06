use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{
    TeamSide, TempPlayer, TempPlayerId, TempPlayerKind,
};
use crate::app::match_report::ports::{IPlayerDataPort, ITeamDataPort};
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct InitTempPlayersCommand {
    pub match_report_id: MatchReportId,
    pub team_id: TeamId,
}

#[derive(Debug)]
pub enum InitTempPlayersError {
    NotFound,
    NotInPreMatchPhase,
    JourneymanPositionUnavailable(String),
    PlayerCountUnavailable(String),
    Repository(String),
}

/// **Les deux événements sont émis sur le bus interne**, et pas seulement
/// appendus au store.
///
/// Le dépôt de `match_report` ne publie rien de lui-même : ses trois use cases
/// sortants — publication, dépublication, annulation — appellent `emettre()`
/// eux-mêmes. Celui-ci ne le faisait pas, parce qu'aucun de ses événements ne
/// franchissait la frontière du BC avant la carte 455.
///
/// La 455 a branché le publisher sur `TempPlayersInitialized` **sans que rien
/// n'alimente le bus** : le bras existait, il n'était jamais atteint, et aucun
/// journalier n'a jamais été créé. Ni le compilateur ni les tests unitaires ne
/// pouvaient le dire — chaque maillon était juste, c'est leur raccord qui
/// manquait. L'axe 12 de `check-arch` vérifie qu'une émission passe par
/// `emettre()`, jamais qu'un événement destiné à sortir est bien émis.
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: InitTempPlayersCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    player_data: &dyn IPlayerDataPort,
    bus: &EventBus,
) -> Result<(), InitTempPlayersError> {
    let mr_id = cmd.match_report_id.to_string();
    let pm = load_pm(repo, &mr_id).await?;

    let pm = reset_if_needed(pm, &cmd.team_id, repo, &mr_id, bus).await?;

    let stars = collect_stars(&pm, &cmd.team_id);
    let mercs = collect_mercs(&pm, &cmd.team_id);
    let journeymen = collect_journeymen(&cmd.team_id, team_data, player_data).await?;

    let players = stars.into_iter().chain(mercs).chain(journeymen).collect();
    let (_, event) = pm.init_temp_players(&cmd.team_id, players);

    let version = pm.version;
    repo.append(&mr_id, &event, version)
        .await
        .map_err(|e| InitTempPlayersError::Repository(e.to_string()))?;
    emettre(bus, event.to_enveloppe(&mr_id));
    Ok(())
}

async fn load_pm(
    repo: &dyn IMatchReportRepository,
    mr_id: &str,
) -> Result<
    crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch,
    InitTempPlayersError,
> {
    let state = repo
        .find_by_id(mr_id)
        .await
        .map_err(|e| InitTempPlayersError::Repository(e.to_string()))?
        .ok_or(InitTempPlayersError::NotFound)?;
    match state {
        MatchReportState::PreMatch(pm) => Ok(pm),
        MatchReportState::ReadyToPublish(rtp) => Ok(rtp.into_pre_match()),
        _ => Err(InitTempPlayersError::NotInPreMatchPhase),
    }
}

async fn reset_if_needed(
    pm: crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch,
    team_id: &TeamId,
    repo: &dyn IMatchReportRepository,
    mr_id: &str,
    bus: &EventBus,
) -> Result<
    crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch,
    InitTempPlayersError,
> {
    let side = if team_id == &pm.home_team_id {
        TeamSide::Home
    } else {
        TeamSide::Away
    };
    if pm.temp_players_for(side).is_empty() {
        return Ok(pm);
    }
    let (updated, reset_event) = pm.reset_temp_players(team_id);
    let version = updated.version - 1;
    repo.append(mr_id, &reset_event, version)
        .await
        .map_err(|e| InitTempPlayersError::Repository(e.to_string()))?;
    // Le retrait franchit la frontière comme la naissance : sans lui, un
    // repassage sur l'écran des coups de pouce laisserait des journaliers
    // orphelins dans l'effectif.
    emettre(bus, reset_event.to_enveloppe(mr_id));
    Ok(updated)
}

fn collect_stars(
    pm: &crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch,
    team_id: &TeamId,
) -> Vec<TempPlayer> {
    pm.star_player_uids_for(team_id)
        .into_iter()
        .map(|uid| TempPlayer {
            id: TempPlayerId(ulid::Ulid::new().to_string()),
            team_id: team_id.clone(),
            kind: TempPlayerKind::StarPlayer {
                ref_uid: uid.0.clone(),
                position_uid: String::new(),
            },
            display_name: Some(uid.0),
        })
        .collect()
}

fn collect_mercs(
    pm: &crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch,
    team_id: &TeamId,
) -> Vec<TempPlayer> {
    pm.purchases_for(team_id)
        .iter()
        .filter(|p| p.uid.0.starts_with("MERCO:"))
        .flat_map(|p| {
            let position_uid = p.uid.0.splitn(3, ':').nth(1).unwrap_or("").to_string();
            (0..p.qty.into_inner()).map(move |_| TempPlayer {
                id: TempPlayerId(ulid::Ulid::new().to_string()),
                team_id: team_id.clone(),
                kind: TempPlayerKind::Mercenary {
                    position_uid: position_uid.clone(),
                },
                display_name: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
    use crate::app::match_report::domain::value_objects::{
        DedicatedFans, InducementCost, InducementPurchase, InducementQty, MatchReportOrigin,
        TeamValue,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{
        CompetitionId, MatchReportId, RoundId, SeasonId,
    };
    use crate::app::shared_kernel::bloodbowl::inducement_definition::InducementId;
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};

    use crate::app::match_report::domain::match_report_repository_port::RepositoryError;
    use crate::app::match_report::domain::match_report_state::MatchReportState;
    use crate::app::match_report::ports::{
        JourneymanPositionDto, PositionCountDto, RosterPositionDto, TeamInfoDto,
    };
    use crate::common::services::event_bus::event_bus::new_bus;
    use std::sync::Mutex;

    struct DepotSimule {
        state: Mutex<Option<MatchReportState>>,
    }

    #[async_trait::async_trait]
    impl IMatchReportRepository for DepotSimule {
        async fn append(
            &self,
            _: &str,
            _: &crate::app::match_report::domain::events::MatchReportDomainEvent,
            _: u64,
        ) -> Result<u64, RepositoryError> {
            Ok(1)
        }
        async fn find_space_id(&self, _: &str) -> Result<Option<String>, RepositoryError> {
            Ok(None)
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<MatchReportState>, RepositoryError> {
            Ok(self.state.lock().unwrap().take())
        }
        async fn find_team_ids(
            &self,
            _: &str,
        ) -> Result<Option<(String, String)>, RepositoryError> {
            Ok(None)
        }
        async fn append_many(
            &self,
            _: &str,
            _: Vec<crate::app::match_report::domain::events::MatchReportDomainEvent>,
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
        ) -> Result<
            Vec<crate::app::match_report::domain::match_report_repository_port::MatchActionRow>,
            RepositoryError,
        > {
            Ok(vec![])
        }
    }

    struct EquipeSimulee;

    #[async_trait::async_trait]
    impl ITeamDataPort for EquipeSimulee {
        async fn is_team_ready_to_play(&self, _: &str) -> Result<bool, String> {
            Ok(true)
        }
        async fn is_team_in_player_improvement(&self, _: &str) -> Result<bool, String> {
            Ok(false)
        }
        async fn is_coach_of_team(&self, _: &str, _: &str) -> Result<bool, String> {
            Ok(true)
        }
        async fn find_team_info(&self, _: &str) -> Option<TeamInfoDto> {
            None
        }
        async fn find_team_value(&self, _: &str) -> Option<u32> {
            Some(1000)
        }
        async fn find_team_treasury(&self, _: &str) -> Option<u32> {
            Some(1000)
        }
        async fn find_journeyman_position(&self, _: &str) -> Option<JourneymanPositionDto> {
            Some(JourneymanPositionDto {
                position_uid: "DEMO_GRANIT__PIETAILLE".into(),
                position_name: "Piétaille".into(),
            })
        }
        async fn find_roster_positions(&self, _: &str) -> Vec<RosterPositionDto> {
            vec![]
        }
    }

    /// Dix alignables sur onze : il manque un journalier.
    struct JoueursSimules;

    #[async_trait::async_trait]
    impl IPlayerDataPort for JoueursSimules {
        async fn count_available_players(&self, _: &str) -> Result<usize, String> {
            Ok(10)
        }
        async fn find_player_display(&self, _: &str) -> Option<String> {
            None
        }
        async fn find_player_position(&self, _: &str) -> Option<String> {
            None
        }
        async fn find_player_counts_by_position(&self, _: &str) -> Vec<PositionCountDto> {
            vec![]
        }
        async fn has_spent_spp_since_match(&self, _: &str, _: &str) -> Result<bool, String> {
            Ok(false)
        }
    }

    /// **Le test qui manquait**, et qui aurait épargné une carte entière.
    ///
    /// La carte 455 a branché le publisher sur `TempPlayersInitialized` sans que
    /// rien n'alimente le bus interne : le bras existait, n'était jamais
    /// atteint, et aucun journalier n'a jamais été créé. Ni le compilateur ni
    /// les tests unitaires ne pouvaient le dire — chaque maillon était juste,
    /// c'est leur raccord qui manquait, et il n'est visible que d'ici.
    ///
    /// **Appender ne suffit pas** : c'est ce que ce test affirme. L'axe 12 de
    /// `check-arch` vérifie qu'une émission passe par `emettre()`, jamais qu'un
    /// événement destiné à sortir du BC est bien émis.
    #[tokio::test]
    async fn l_initialisation_emet_son_evenement_sur_le_bus() {
        let pm = make_pm();
        let team_id = pm.home_team_id.clone();
        let depot = DepotSimule {
            state: Mutex::new(Some(MatchReportState::PreMatch(pm))),
        };
        let bus = new_bus();
        let mut abonne = bus.subscribe();

        execute(
            InitTempPlayersCommand {
                match_report_id: MatchReportId::new(),
                team_id,
            },
            &depot,
            &EquipeSimulee,
            &JoueursSimules,
            &bus,
        )
        .await
        .expect("l'initialisation aboutit");

        let enveloppe = abonne
            .try_recv()
            .expect("l'événement doit atteindre le bus, pas seulement l'event store");
        assert_eq!(enveloppe.event_type, "TempPlayersInitialized");
    }

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

    #[test]
    fn collect_stars_creates_one_per_engagement() {
        let mut pm = make_pm();
        let uid = InducementId("MORG_N_THORG".into());
        pm.star_engagements
            .push((pm.home_team_id.clone(), uid.clone()));
        let stars = collect_stars(&pm, &pm.home_team_id.clone());
        assert_eq!(stars.len(), 1);
        assert!(
            matches!(&stars[0].kind, TempPlayerKind::StarPlayer { ref_uid, .. } if ref_uid == "MORG_N_THORG")
        );
        assert_eq!(stars[0].display_name, Some("MORG_N_THORG".into()));
    }

    #[test]
    fn collect_mercs_creates_qty_per_purchase() {
        let mut pm = make_pm();
        pm.home_inducements = Some(vec![InducementPurchase {
            uid: InducementId("MERCO:blitzeur:base".into()),
            qty: InducementQty::try_new(2).unwrap(),
            unit_cost: InducementCost::try_new(130).unwrap(),
        }]);
        let mercs = collect_mercs(&pm, &pm.home_team_id.clone());
        assert_eq!(mercs.len(), 2);
        assert!(
            matches!(&mercs[0].kind, TempPlayerKind::Mercenary { position_uid } if position_uid == "blitzeur")
        );
    }

    #[test]
    fn collect_mercs_extracts_position_uid_from_encoded_uid() {
        let mut pm = make_pm();
        pm.home_inducements = Some(vec![InducementPurchase {
            uid: InducementId("MERCO:witch-elf:lvl1".into()),
            qty: InducementQty::try_new(1).unwrap(),
            unit_cost: InducementCost::try_new(180).unwrap(),
        }]);
        let mercs = collect_mercs(&pm, &pm.home_team_id.clone());
        assert_eq!(mercs.len(), 1);
        assert!(
            matches!(&mercs[0].kind, TempPlayerKind::Mercenary { position_uid } if position_uid == "witch-elf")
        );
    }

    #[test]
    fn collect_mercs_ignores_non_mercenary_purchases() {
        let mut pm = make_pm();
        pm.home_inducements = Some(vec![InducementPurchase {
            uid: InducementId("BRIBE".into()),
            qty: InducementQty::try_new(1).unwrap(),
            unit_cost: InducementCost::try_new(100).unwrap(),
        }]);
        let mercs = collect_mercs(&pm, &pm.home_team_id.clone());
        assert!(mercs.is_empty());
    }
}

async fn collect_journeymen(
    team_id: &TeamId,
    team_data: &dyn ITeamDataPort,
    player_data: &dyn IPlayerDataPort,
) -> Result<Vec<TempPlayer>, InitTempPlayersError> {
    let count = player_data
        .count_available_players(&team_id.to_string())
        .await
        .map_err(|e| InitTempPlayersError::PlayerCountUnavailable(e))?;
    let n = 11usize.saturating_sub(count);
    if n == 0 {
        return Ok(vec![]);
    }
    let pos = team_data
        .find_journeyman_position(&team_id.to_string())
        .await
        .ok_or_else(|| InitTempPlayersError::JourneymanPositionUnavailable(team_id.to_string()))?;
    Ok((0..n)
        .map(|_| TempPlayer {
            id: TempPlayerId(ulid::Ulid::new().to_string()),
            team_id: team_id.clone(),
            kind: TempPlayerKind::Journeyman {
                position_uid: pos.position_uid.clone(),
            },
            display_name: None,
        })
        .collect())
}

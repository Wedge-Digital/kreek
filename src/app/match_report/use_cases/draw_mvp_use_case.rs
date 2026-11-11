//! Le tirage au sort du MVP (carte 638).
//!
//! Sur le modèle de `record_mvp_use_case` : les noms et postes des six
//! désignés sont résolus au moment présent, un rapport prêt à publier repasse
//! en avant-match (correction), et un conflit d'écriture se relaie. La règle —
//! six désignés distincts, un MVP par camp, au tour 16 — est celle du domaine
//! (`draw_mvp`).
//!
//! Le générateur est **reçu** : `StdRng::from_os_rng()` dans le contrôleur, une
//! graine fixe en test.

use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::mvp_draw::DesignatedPlayer;
use crate::app::match_report::domain::value_objects::{ActionId, ActionPlayer, TeamSide};
use crate::app::match_report::ports::IPlayerDataPort;
use crate::app::match_report::use_cases::conflict_retry::{lost_the_race, settle};
use crate::app::match_report::use_cases::event_emission::emit_all;
use crate::app::match_report::use_cases::record_action_use_case::resolve_player_info;
use crate::app::match_report::use_cases::record_mvp_use_case::{load_pre_match, RecordMvpError};
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::common::services::event_bus::event_bus::EventBus;
use rand::Rng;

#[derive(Debug)]
pub struct DrawMvpCommand {
    pub match_report_id: MatchReportId,
    pub team_side: TeamSide,
    /// Les joueurs désignés, dans l'ordre de leur désignation.
    pub designated: Vec<ActionPlayer>,
    pub recorded_by: CoachId,
}

/// Les refus sont ceux de la saisie manuelle : même résolution des joueurs,
/// mêmes conflits, et les refus du domaine.
pub type DrawMvpError = RecordMvpError;

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute<R: Rng + Send>(
    cmd: DrawMvpCommand,
    repo: &dyn IMatchReportRepository,
    player_data: &dyn IPlayerDataPort,
    rng: &mut R,
    bus: &EventBus,
) -> Result<(), DrawMvpError> {
    let action_id = ActionId(ulid::Ulid::new().to_string());
    // Carte 653 : une course perdue contre l'autre camp se rejoue une fois. Le
    // tirage est alors refait : le premier n'a été ni écrit, ni montré.
    let first = draw_once(&cmd, &action_id, repo, player_data, rng, bus).await;
    if !lost_the_race(COMMAND, &first) {
        return first;
    }
    settle(
        COMMAND,
        draw_once(&cmd, &action_id, repo, player_data, rng, bus).await,
    )
}

const COMMAND: &str = "draw_mvp";

/// Un essai : relire l'agrégat, tirer, appender à la version lue.
async fn draw_once<R: Rng + Send>(
    cmd: &DrawMvpCommand,
    action_id: &ActionId,
    repo: &dyn IMatchReportRepository,
    player_data: &dyn IPlayerDataPort,
    rng: &mut R,
    bus: &EventBus,
) -> Result<(), DrawMvpError> {
    let mr_id = cmd.match_report_id.to_string();
    let pm = load_pre_match(&mr_id, repo).await?;
    let designated = resolve_designated(cmd, &pm, player_data).await?;
    // Six désignés, distincts, un MVP par camp : les refus viennent du domaine.
    let (_, events) = pm
        .draw_mvp(
            cmd.team_side,
            designated,
            rng,
            action_id.clone(),
            cmd.recorded_by.clone(),
        )
        .map_err(DrawMvpError::Domain)?;
    // `ActionRecorded` puis `MvpDrawn`, dans une seule transaction, puis émis
    // dans cet ordre (carte 643) : un conflit n'émet rien.
    repo.append_many(&mr_id, events.clone(), pm.version)
        .await
        .map_err(DrawMvpError::from_repository)?;
    emit_all(bus, &mr_id, &events);
    Ok(())
}

/// Le nom et le poste de chaque désigné, figés au moment du tirage.
async fn resolve_designated(
    cmd: &DrawMvpCommand,
    pm: &MatchReportPreMatch,
    player_data: &dyn IPlayerDataPort,
) -> Result<Vec<DesignatedPlayer>, DrawMvpError> {
    let mut designated = Vec::with_capacity(cmd.designated.len());
    for player in &cmd.designated {
        let (display_name, position) = resolve_player_info(player, cmd.team_side, pm, player_data)
            .await
            .map_err(DrawMvpError::from_resolution)?;
        designated.push(DesignatedPlayer {
            player: player.clone(),
            display_name,
            position,
        });
    }
    Ok(designated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::error::DomainError;
    use crate::app::match_report::domain::events::MatchReportDomainEvent;
    use crate::app::match_report::domain::match_report_state::MatchReportState;
    use crate::app::match_report::domain::value_objects::MatchActionType;
    use crate::app::match_report::use_cases::test_doubles::{
        pre_match, KnownPlayers, RecordingRepository, ScriptedRepository,
    };
    use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
    use crate::common::services::event_bus::event_bus::{new_bus, EventBus};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn command(n: usize) -> DrawMvpCommand {
        DrawMvpCommand {
            match_report_id: MatchReportId::new(),
            team_side: TeamSide::Home,
            designated: (0..n)
                .map(|_| ActionPlayer::Regular(PlayerId::new()))
                .collect(),
            recorded_by: CoachId::new(),
        }
    }

    async fn run(
        repo: &dyn IMatchReportRepository,
        cmd: DrawMvpCommand,
    ) -> Result<(), DrawMvpError> {
        run_on(repo, cmd, &new_bus()).await
    }

    async fn run_on(
        repo: &dyn IMatchReportRepository,
        cmd: DrawMvpCommand,
        bus: &EventBus,
    ) -> Result<(), DrawMvpError> {
        let mut rng = StdRng::seed_from_u64(638);
        execute(cmd, repo, &KnownPlayers, &mut rng, bus).await
    }

    /// L'action tirée puis le tirage, appendés ensemble ; le tiré est l'un des six.
    #[tokio::test]
    async fn le_tirage_appende_l_action_puis_le_tirage() {
        let repo = RecordingRepository::holding(MatchReportState::PreMatch(pre_match()));
        let cmd = command(6);
        let six = cmd.designated.clone();
        run(&repo, cmd).await.unwrap();
        let appended = repo.appended();
        let [MatchReportDomainEvent::ActionRecorded {
            action_id,
            player,
            turn,
            action,
            ..
        }, MatchReportDomainEvent::MvpDrawn {
            action_id: drawn_id,
            designated,
            ..
        }] = appended.as_slice()
        else {
            panic!("attendu ActionRecorded puis MvpDrawn : {appended:?}");
        };
        assert_eq!(action_id, drawn_id);
        assert_eq!(turn.value(), 16);
        assert!(matches!(action, MatchActionType::Mvp));
        assert!(six.contains(player));
        assert_eq!(designated.len(), 6);
    }

    #[tokio::test]
    async fn cinq_designes_sont_refuses_sans_ecriture() {
        let repo = RecordingRepository::holding(MatchReportState::PreMatch(pre_match()));
        let r = run(&repo, command(5)).await;
        assert!(matches!(
            r,
            Err(RecordMvpError::Domain(DomainError::DrawNeedsSixPlayers(5)))
        ));
        assert!(repo.appended().is_empty());
    }

    fn two_reads() -> Vec<MatchReportState> {
        vec![
            MatchReportState::PreMatch(pre_match()),
            MatchReportState::PreMatch(pre_match()),
        ]
    }

    /// Carte 653 : deux conflits de suite sont relayés, après deux appends.
    #[tokio::test]
    async fn two_conflicts_are_relayed() {
        let repo = ScriptedRepository::reading(two_reads()).refusing(2);
        let bus = new_bus();
        let mut subscriber = bus.subscribe();
        let r = run_on(&repo, command(6), &bus).await;
        assert!(matches!(r, Err(RecordMvpError::Conflict)), "{:?}", r.err());
        assert_eq!(repo.append_attempts(), 2);
        assert!(subscriber.try_recv().is_err(), "un conflit n'émet rien");
    }

    /// Carte 653 : la course perdue se rejoue ; un seul tirage est écrit et
    /// émis — celui du réessai.
    #[tokio::test]
    async fn a_lost_race_is_retried_and_one_draw_lands() {
        let repo = ScriptedRepository::reading(two_reads()).refusing(1);
        let bus = new_bus();
        let mut subscriber = bus.subscribe();

        run_on(&repo, command(6), &bus)
            .await
            .expect("le réessai passe");

        assert_eq!(repo.appended().len(), 2, "ActionRecorded puis MvpDrawn");
        let emitted: Vec<_> = (0..2)
            .map(|_| subscriber.try_recv().unwrap().event_type)
            .collect();
        assert_eq!(emitted, ["ActionRecorded", "MvpDrawn"]);
        assert!(
            subscriber.try_recv().is_err(),
            "l'essai perdu n'a rien émis"
        );
    }

    /// Carte 643 : l'action tirée puis le tirage sortent dans l'ordre de
    /// l'append ; c'est l'`ActionRecorded` qui fera la notification.
    #[tokio::test]
    async fn the_draw_emits_the_action_then_the_draw() {
        let repo = RecordingRepository::holding(MatchReportState::PreMatch(pre_match()));
        let bus = new_bus();
        let mut subscriber = bus.subscribe();

        run_on(&repo, command(6), &bus).await.expect("tirage");

        let emitted: Vec<_> = (0..2)
            .map(|_| subscriber.try_recv().unwrap().event_type)
            .collect();
        assert_eq!(emitted, ["ActionRecorded", "MvpDrawn"]);
        assert!(subscriber.try_recv().is_err());
    }
}

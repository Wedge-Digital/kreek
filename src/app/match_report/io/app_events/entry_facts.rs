//! Les faits de saisie qui sortent du BC (carte 643) : une action, son
//! retrait, une sélection de coups de pouce validée, leur annulation.
//!
//! Personne ne les enregistre hors de `match_report` : ils s'annoncent, et le
//! canal temps réel (cartes 645, 646) en fera des toasts et des signaux. Chacun
//! porte l'espace et les deux équipes, pour le routage, et son auteur nommé.
//!
//! **La portée se lit sur la projection** (`find_space_id`,
//! `find_access_scope`), pas par la relecture de l'agrégat : une action en
//! rejouerait tout le flux à chaque saisie.

use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::value_objects::{
    InducementPurchase, MatchActionType, PurchaseKind, TeamSide, TurnNumber,
};
use crate::app::match_report::io::app_events::app_event_publisher::{
    build_action_type, PublisherPorts,
};
use crate::app::match_report::ports::{ICoachDataPort, ITeamDataPort};
use crate::app::match_report::use_cases::inducements_recap_service::{
    labelled_purchases, LabelledPurchase, RecapPorts,
};
use crate::app::shared_kernel::app_events::match_report_app_events::{
    InducementItemPayload, InducementKindPayload, MatchReportAppEvent, TeamSidePayload,
};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, EventId};
use crate::common::services::event_bus::app_event_publication::publier;
use crate::common::services::event_bus::event_bus::EventBus;

/// Où publier un fait : le rapport qui l'a produit, le bus, les ports.
pub(super) struct Target<'a> {
    pub match_report_id: &'a str,
    pub app_event_bus: &'a EventBus,
    pub ports: &'a PublisherPorts,
}

/// Ce que tout fait de saisie porte pour être routé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MatchScope {
    pub space_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
}

impl MatchScope {
    fn team_of(&self, side: TeamSide) -> &str {
        match side {
            TeamSide::Home => &self.home_team_id,
            TeamSide::Away => &self.away_team_id,
        }
    }
}

/// Une action appendée, telle que l'événement domaine la décrit.
pub(super) struct RecordedAction {
    pub team_side: TeamSide,
    pub turn: TurnNumber,
    pub action: MatchActionType,
    pub player_display_name: String,
    pub recorded_by: CoachId,
}

/// Une sélection de coups de pouce validée, telle que l'événement la décrit.
pub(super) struct RecordedSelection {
    pub team_id: TeamId,
    pub purchases: Vec<InducementPurchase>,
    pub recorded_by: CoachId,
}

/// L'espace et les deux équipes du rapport, lus sur la projection. Introuvables,
/// rien n'est publié : un fait sans destinataires ne se route pas.
pub(super) async fn match_scope(
    repo: &dyn IMatchReportRepository,
    match_report_id: &str,
) -> Option<MatchScope> {
    let space = repo.find_space_id(match_report_id).await;
    let scope = repo.find_access_scope(match_report_id).await;
    if let (Ok(Some(space_id)), Ok(Some((_, home_team_id, away_team_id)))) = (space, scope) {
        return Some(MatchScope {
            space_id,
            home_team_id,
            away_team_id,
        });
    }
    tracing::error!(
        match_report_id,
        "app_event_publisher: portée introuvable, fait de saisie non publié"
    );
    None
}

/// Le pseudonyme d'un coach, `None` si l'identité ne le connaît pas : c'est au
/// canal de décider comment l'annoncer, pas au publisher d'inventer un nom.
pub(super) async fn coach_name(coach_data: &dyn ICoachDataPort, coach: &CoachId) -> Option<String> {
    coach_data.find_coach_name(&coach.to_string()).await
}

async fn team_name(team_data: &dyn ITeamDataPort, team_id: &str) -> Option<String> {
    team_data.find_team_info(team_id).await.map(|i| i.team_name)
}

pub(super) async fn publish_action_recorded(to: &Target<'_>, fact: RecordedAction) {
    let (match_report_id, app_event_bus, ports) = (to.match_report_id, to.app_event_bus, to.ports);
    let Some(scope) = match_scope(ports.repo.as_ref(), match_report_id).await else {
        return;
    };
    let team = team_name(ports.team_data.as_ref(), scope.team_of(fact.team_side)).await;
    let author = coach_name(ports.coach_data.as_ref(), &fact.recorded_by).await;
    let event = action_recorded_event(match_report_id, scope, fact, team, author);
    publier(app_event_bus, event.to_enveloppe());
}

pub(super) async fn publish_action_deleted(
    to: &Target<'_>,
    team_side: TeamSide,
    deleted_by: CoachId,
) {
    let (match_report_id, app_event_bus, ports) = (to.match_report_id, to.app_event_bus, to.ports);
    let Some(scope) = match_scope(ports.repo.as_ref(), match_report_id).await else {
        return;
    };
    let event = MatchReportAppEvent::MatchActionDeleted {
        event_id: EventId::new(),
        match_report_id: match_report_id.to_string(),
        space_id: scope.space_id,
        home_team_id: scope.home_team_id,
        away_team_id: scope.away_team_id,
        team_side: side_payload(team_side),
        deleted_by: deleted_by.to_string(),
    };
    publier(app_event_bus, event.to_enveloppe());
}

pub(super) async fn publish_inducements_recorded(to: &Target<'_>, selection: RecordedSelection) {
    let (match_report_id, app_event_bus, ports) = (to.match_report_id, to.app_event_bus, to.ports);
    let Some(scope) = match_scope(ports.repo.as_ref(), match_report_id).await else {
        return;
    };
    let info = ports
        .team_data
        .find_team_info(&selection.team_id.to_string())
        .await;
    let (team, roster_id) =
        info.map_or((None, String::new()), |i| (Some(i.team_name), i.roster_id));
    let items = selection_items(&selection, &roster_id, ports).await;
    let author = coach_name(ports.coach_data.as_ref(), &selection.recorded_by).await;
    let event =
        inducements_recorded_event(match_report_id, scope, selection, (team, author), items);
    publier(app_event_bus, event.to_enveloppe());
}

pub(super) async fn publish_inducements_cancelled(to: &Target<'_>, cancelled_by: CoachId) {
    let (match_report_id, app_event_bus, ports) = (to.match_report_id, to.app_event_bus, to.ports);
    let Some(scope) = match_scope(ports.repo.as_ref(), match_report_id).await else {
        return;
    };
    let author = coach_name(ports.coach_data.as_ref(), &cancelled_by).await;
    let event = MatchReportAppEvent::MatchInducementsCancelled {
        event_id: EventId::new(),
        match_report_id: match_report_id.to_string(),
        space_id: scope.space_id,
        home_team_id: scope.home_team_id,
        away_team_id: scope.away_team_id,
        cancelled_by: cancelled_by.to_string(),
        cancelled_by_name: author,
    };
    publier(app_event_bus, event.to_enveloppe());
}

/// Les lignes nommées par le service du récapitulatif : libellés et vedettes
/// au catalogue, mercenaires par leur poste. Une sélection vide reste vide.
async fn selection_items(
    selection: &RecordedSelection,
    roster_id: &str,
    ports: &PublisherPorts,
) -> Vec<InducementItemPayload> {
    let recap_ports = RecapPorts {
        team_data: ports.team_data.as_ref(),
        coach_data: ports.coach_data.as_ref(),
        catalog: ports.catalog.as_ref(),
    };
    labelled_purchases(
        &selection.purchases,
        &selection.team_id,
        roster_id,
        &recap_ports,
    )
    .await
    .into_iter()
    .map(item_payload)
    .collect()
}

fn item_payload(line: LabelledPurchase) -> InducementItemPayload {
    InducementItemPayload {
        label: line.label,
        qty: line.qty,
        kind: match line.kind {
            PurchaseKind::Inducement => InducementKindPayload::Inducement,
            PurchaseKind::StarPlayer => InducementKindPayload::StarPlayer,
            PurchaseKind::Mercenary { skilled, .. } => InducementKindPayload::Mercenary { skilled },
        },
    }
}

fn side_payload(side: TeamSide) -> TeamSidePayload {
    match side {
        TeamSide::Home => TeamSidePayload::Home,
        TeamSide::Away => TeamSidePayload::Away,
    }
}

fn action_recorded_event(
    match_report_id: &str,
    scope: MatchScope,
    fact: RecordedAction,
    team_name: Option<String>,
    recorded_by_name: Option<String>,
) -> MatchReportAppEvent {
    MatchReportAppEvent::MatchActionRecorded {
        event_id: EventId::new(),
        match_report_id: match_report_id.to_string(),
        space_id: scope.space_id,
        home_team_id: scope.home_team_id,
        away_team_id: scope.away_team_id,
        team_side: side_payload(fact.team_side),
        team_name,
        turn: fact.turn.value(),
        action: build_action_type(&fact.action),
        player_display_name: fact.player_display_name,
        recorded_by: fact.recorded_by.to_string(),
        recorded_by_name,
    }
}

fn inducements_recorded_event(
    match_report_id: &str,
    scope: MatchScope,
    selection: RecordedSelection,
    (team_name, recorded_by_name): (Option<String>, Option<String>),
    items: Vec<InducementItemPayload>,
) -> MatchReportAppEvent {
    MatchReportAppEvent::MatchInducementsRecorded {
        event_id: EventId::new(),
        match_report_id: match_report_id.to_string(),
        space_id: scope.space_id,
        home_team_id: scope.home_team_id,
        away_team_id: scope.away_team_id,
        team_id: selection.team_id.to_string(),
        team_name,
        recorded_by: selection.recorded_by.to_string(),
        recorded_by_name,
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::events::MatchReportDomainEvent;
    use crate::app::match_report::domain::value_objects::{
        ActionId, ActionPlayer, InducementCost, InducementQty,
    };
    use crate::app::match_report::io::app_events::app_event_publisher::aiguiller;
    use crate::app::match_report::ports::{
        IReferenceCatalogPort, InducementLabelDto, JourneymanPositionDto, MatchRemindersDto,
        RosterPositionDto, SppScaleDto, StarPlayerDto, TeamInfoDto,
    };
    use crate::app::match_report::use_cases::test_doubles::{PlainCompetition, ScopedRepository};
    use crate::app::shared_kernel::app_events::match_report_app_events::ActionTypePayload;
    use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
    use crate::app::shared_kernel::bloodbowl::inducement_definition::InducementId;
    use crate::common::services::event_bus::event_bus::new_bus;
    use async_trait::async_trait;
    use std::sync::Arc;

    const SPACE: &str = "SPACE";

    /// Le rapport des tests : deux équipes, l'extérieur s'appelle London Rats.
    struct Match {
        home: TeamId,
        away: TeamId,
    }

    impl Match {
        fn new() -> Self {
            Self {
                home: TeamId::new(),
                away: TeamId::new(),
            }
        }

        fn scoped(&self) -> ScopedRepository {
            ScopedRepository::of(SPACE, &self.home.to_string(), &self.away.to_string())
        }

        fn ports(&self, repo: ScopedRepository) -> PublisherPorts {
            PublisherPorts {
                repo: Arc::new(repo),
                competition_data: Arc::new(PlainCompetition),
                team_data: Arc::new(Teams {
                    home: self.home.to_string(),
                }),
                coach_data: Arc::new(Coaches),
                catalog: Arc::new(Catalog),
            }
        }

        /// Ce que le publisher fait sortir de cet événement domaine, ou `None`.
        async fn published(
            &self,
            event: MatchReportDomainEvent,
            repo: ScopedRepository,
        ) -> Option<MatchReportAppEvent> {
            let bus = new_bus();
            let mut subscriber = bus.subscribe();
            aiguiller(event, "MR", &bus, &self.ports(repo)).await;
            let envelope = subscriber.try_recv().ok()?;
            assert!(subscriber.try_recv().is_err(), "un seul app event attendu");
            Some(serde_json::from_value(envelope.payload).unwrap())
        }
    }

    struct Teams {
        home: String,
    }

    #[async_trait]
    impl ITeamDataPort for Teams {
        async fn is_team_ready_to_play(&self, _: &str) -> Result<bool, String> {
            Ok(true)
        }
        async fn is_team_in_player_improvement(&self, _: &str) -> Result<bool, String> {
            Ok(false)
        }
        async fn is_coach_of_team(&self, _: &str, _: &str) -> Result<bool, String> {
            Ok(true)
        }
        async fn find_team_info(&self, team_id: &str) -> Option<TeamInfoDto> {
            let name = if team_id == self.home {
                "Nains de Fer"
            } else {
                "London Rats"
            };
            Some(TeamInfoDto {
                team_name: name.into(),
                roster_id: "ROSTER".into(),
                ..Default::default()
            })
        }
        async fn find_team_value(&self, _: &str) -> Option<u32> {
            None
        }
        async fn find_team_treasury(&self, _: &str) -> Option<u32> {
            None
        }
        async fn find_journeyman_position(&self, _: &str) -> Option<JourneymanPositionDto> {
            None
        }
        async fn find_roster_positions(&self, _: &str) -> Vec<RosterPositionDto> {
            vec![RosterPositionDto {
                position_uid: "rat-lineman".into(),
                position_name: "Trois-quarts".into(),
                base_cost: 50,
                max_qty: 16,
                is_journeyman: true,
                keywords: vec![],
            }]
        }
        async fn find_team_season(&self, _: &str) -> Option<String> {
            None
        }
    }

    struct Coaches;

    #[async_trait]
    impl ICoachDataPort for Coaches {
        async fn find_coach_name(&self, _: &str) -> Option<String> {
            Some("Morgane".into())
        }
    }

    struct Catalog;

    impl IReferenceCatalogPort for Catalog {
        fn find_inducement_labels(&self, uids: &[String]) -> Vec<InducementLabelDto> {
            uids.iter()
                .filter(|u| u.as_str() == "REROLL")
                .map(|u| InducementLabelDto {
                    uid: u.clone(),
                    name: "Relance".into(),
                    description: String::new(),
                })
                .collect()
        }
        fn find_star_players(&self, uids: &[String], _: &str) -> Vec<StarPlayerDto> {
            uids.iter()
                .filter(|u| u.as_str() == "MORG")
                .map(|u| StarPlayerDto {
                    uid: u.clone(),
                    name: "Morg 'n' Thorg".into(),
                    player_type: String::new(),
                    cost: 340,
                    ma: 6,
                    st: 6,
                    ag: String::new(),
                    pa: String::new(),
                    av: String::new(),
                    skills: vec![],
                    ability_name: String::new(),
                    ability_description: String::new(),
                })
                .collect()
        }
        fn find_spp_scale(&self, _: &str) -> SppScaleDto {
            SppScaleDto::default()
        }
        fn find_match_reminders(&self) -> MatchRemindersDto {
            MatchRemindersDto::default()
        }
    }

    fn action_recorded(action: MatchActionType, author: CoachId) -> MatchReportDomainEvent {
        MatchReportDomainEvent::ActionRecorded {
            action_id: ActionId("A1".into()),
            team_side: TeamSide::Away,
            turn: TurnNumber::try_new(4).unwrap(),
            player: ActionPlayer::Regular(PlayerId::new()),
            action,
            player_display_name: "Skritt".into(),
            player_position: String::new(),
            recorded_by: author,
        }
    }

    fn purchase(uid: &str, qty: u8) -> InducementPurchase {
        InducementPurchase {
            uid: InducementId(uid.into()),
            qty: InducementQty::try_new(qty).unwrap(),
            unit_cost: InducementCost::try_new(10).unwrap(),
        }
    }

    fn inducements_recorded(
        team_id: &TeamId,
        purchases: Vec<InducementPurchase>,
    ) -> MatchReportDomainEvent {
        MatchReportDomainEvent::InducementsRecorded {
            team_id: team_id.clone(),
            purchases,
            recorded_by: CoachId::new(),
        }
    }

    #[tokio::test]
    async fn a_recorded_action_leaves_with_author_and_team_names() {
        let m = Match::new();
        let author = CoachId::new();
        let event = m
            .published(
                action_recorded(MatchActionType::Touchdown, author),
                m.scoped(),
            )
            .await;
        let Some(MatchReportAppEvent::MatchActionRecorded {
            match_report_id,
            space_id,
            home_team_id,
            away_team_id,
            team_side,
            team_name,
            turn,
            action,
            player_display_name,
            recorded_by,
            recorded_by_name,
            ..
        }) = event
        else {
            panic!("MatchActionRecorded attendu : {event:?}");
        };
        assert_eq!((match_report_id.as_str(), space_id.as_str()), ("MR", SPACE));
        assert_eq!(home_team_id, m.home.to_string());
        assert_eq!(away_team_id, m.away.to_string());
        assert_eq!(team_side, TeamSidePayload::Away);
        assert_eq!(team_name.as_deref(), Some("London Rats"));
        assert_eq!(turn, 4);
        assert!(matches!(action, ActionTypePayload::Touchdown));
        assert_eq!(player_display_name, "Skritt");
        assert_eq!(recorded_by, author.to_string());
        assert_eq!(recorded_by_name.as_deref(), Some("Morgane"));
    }

    /// Un MVP, saisi ou tiré, sort comme une action `Mvp`.
    #[tokio::test]
    async fn a_recorded_mvp_leaves_as_an_mvp_action() {
        let m = Match::new();
        let event = m
            .published(
                action_recorded(MatchActionType::Mvp, CoachId::new()),
                m.scoped(),
            )
            .await;
        assert!(matches!(
            event,
            Some(MatchReportAppEvent::MatchActionRecorded {
                action: ActionTypePayload::Mvp,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn a_deleted_action_leaves_with_side_and_author() {
        let m = Match::new();
        let author = CoachId::new();
        let deleted = MatchReportDomainEvent::ActionDeleted {
            action_id: ActionId("A1".into()),
            team_side: TeamSide::Home,
            deleted_by: author,
        };
        let event = m.published(deleted, m.scoped()).await;
        let Some(MatchReportAppEvent::MatchActionDeleted {
            space_id,
            team_side,
            deleted_by,
            ..
        }) = event
        else {
            panic!("MatchActionDeleted attendu : {event:?}");
        };
        assert_eq!(space_id, SPACE);
        assert_eq!(team_side, TeamSidePayload::Home);
        assert_eq!(deleted_by, author.to_string());
    }

    #[tokio::test]
    async fn validated_inducements_carry_labels_quantities_and_kinds() {
        let m = Match::new();
        let selection = vec![
            purchase("REROLL", 2),
            purchase("MORG", 1),
            purchase("MERCO:rat-lineman:lvl1", 1),
        ];
        let event = m
            .published(inducements_recorded(&m.away, selection), m.scoped())
            .await;
        let Some(MatchReportAppEvent::MatchInducementsRecorded {
            team_id,
            team_name,
            recorded_by_name,
            items,
            ..
        }) = event
        else {
            panic!("MatchInducementsRecorded attendu : {event:?}");
        };
        assert_eq!(team_id, m.away.to_string());
        assert_eq!(team_name.as_deref(), Some("London Rats"));
        assert_eq!(recorded_by_name.as_deref(), Some("Morgane"));
        let item = |label: &str, qty, kind| InducementItemPayload {
            label: label.into(),
            qty,
            kind,
        };
        assert_eq!(
            items,
            [
                item("Relance", 2, InducementKindPayload::Inducement),
                item("Morg 'n' Thorg", 1, InducementKindPayload::StarPlayer),
                item(
                    "Trois-quarts",
                    1,
                    InducementKindPayload::Mercenary { skilled: true }
                ),
            ]
        );
    }

    /// Valider sans rien prendre est un fait : il sort, liste vide.
    #[tokio::test]
    async fn an_empty_selection_leaves_with_no_items() {
        let m = Match::new();
        let event = m
            .published(inducements_recorded(&m.home, vec![]), m.scoped())
            .await;
        assert!(matches!(
            event,
            Some(MatchReportAppEvent::MatchInducementsRecorded { items, .. }) if items.is_empty()
        ));
    }

    #[tokio::test]
    async fn cancelled_inducements_name_their_author() {
        let m = Match::new();
        let author = CoachId::new();
        let cancelled = MatchReportDomainEvent::InducementsCancelled {
            cancelled_by: author,
        };
        let event = m.published(cancelled, m.scoped()).await;
        let Some(MatchReportAppEvent::MatchInducementsCancelled {
            home_team_id,
            cancelled_by,
            cancelled_by_name,
            ..
        }) = event
        else {
            panic!("MatchInducementsCancelled attendu : {event:?}");
        };
        assert_eq!(home_team_id, m.home.to_string());
        assert_eq!(cancelled_by, author.to_string());
        assert_eq!(cancelled_by_name.as_deref(), Some("Morgane"));
    }

    /// Décision 10 : la notification d'un tirage est celle de son action.
    #[tokio::test]
    async fn a_drawn_mvp_produces_no_app_event() {
        let m = Match::new();
        let drawn = MatchReportDomainEvent::MvpDrawn {
            action_id: ActionId("A1".into()),
            team_side: TeamSide::Home,
            designated: vec![],
        };
        assert!(m.published(drawn, m.scoped()).await.is_none());
    }

    #[tokio::test]
    async fn an_unknown_scope_publishes_nothing() {
        let m = Match::new();
        let event = action_recorded(MatchActionType::Touchdown, CoachId::new());
        let published = m.published(event, ScopedRepository::unknown()).await;
        assert!(published.is_none());
    }
}

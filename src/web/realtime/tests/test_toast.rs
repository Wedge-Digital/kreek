//! La mise en forme des toasts de match : une phrase par fait, la sélection
//! vide, les noms absents, et le lien vers `/live` (carte 645).

use crate::app::shared_kernel::app_events::match_report_app_events::{
    ActionTypePayload, InducementItemPayload, InducementKindPayload, MatchReportAppEvent,
    TeamSidePayload,
};
use crate::app::shared_kernel::identity::ids::EventId;
use crate::web::realtime::toast::{match_link, notice_of, SEE_MATCH};

const SPACE: &str = "space-1";
const MATCH: &str = "match-1";
const HOME: &str = "home-1";
const AWAY: &str = "away-1";
const AUTHOR: &str = "author-1";

fn confirmed(author_name: Option<&str>) -> MatchReportAppEvent {
    MatchReportAppEvent::MatchReportConfirmed {
        event_id: EventId::new(),
        match_report_id: MATCH.into(),
        home_team_id: HOME.into(),
        away_team_id: AWAY.into(),
        space_id: SPACE.into(),
        pairing_id: None,
        season_id: "season-1".into(),
        round_id: "round-1".into(),
        competition_id: "competition-1".into(),
        confirmed_by: AUTHOR.into(),
        confirmed_by_name: author_name.map(String::from),
    }
}

fn action(
    action: ActionTypePayload,
    team: Option<&str>,
    author: Option<&str>,
) -> MatchReportAppEvent {
    MatchReportAppEvent::MatchActionRecorded {
        event_id: EventId::new(),
        match_report_id: MATCH.into(),
        space_id: SPACE.into(),
        home_team_id: HOME.into(),
        away_team_id: AWAY.into(),
        team_side: TeamSidePayload::Home,
        team_name: team.map(String::from),
        turn: 3,
        action,
        player_display_name: "Griff #9".into(),
        recorded_by: AUTHOR.into(),
        recorded_by_name: author.map(String::from),
    }
}

fn inducements(items: Vec<InducementItemPayload>, team: Option<&str>) -> MatchReportAppEvent {
    MatchReportAppEvent::MatchInducementsRecorded {
        event_id: EventId::new(),
        match_report_id: MATCH.into(),
        space_id: SPACE.into(),
        home_team_id: HOME.into(),
        away_team_id: AWAY.into(),
        team_id: HOME.into(),
        team_name: team.map(String::from),
        recorded_by: AUTHOR.into(),
        recorded_by_name: Some("Morgane".into()),
        items,
    }
}

fn item(label: &str, qty: u8, kind: InducementKindPayload) -> InducementItemPayload {
    InducementItemPayload {
        label: label.into(),
        qty,
        kind,
    }
}

fn title(event: &MatchReportAppEvent) -> String {
    notice_of(event).expect("ce fait fait un toast").toast.title
}

#[test]
fn the_start_names_its_author() {
    assert_eq!(
        title(&confirmed(Some("Morgane"))),
        "Saisie du match démarrée par Morgane"
    );
}

#[test]
fn an_unknown_author_is_left_out_not_invented() {
    assert_eq!(title(&confirmed(None)), "Saisie du match démarrée");
}

#[test]
fn an_action_names_its_player_team_and_author() {
    let event = action(
        ActionTypePayload::Touchdown,
        Some("London Rats"),
        Some("Morgane"),
    );
    assert_eq!(
        title(&event),
        "Touchdown : Griff #9 (London Rats) — par Morgane"
    );
}

#[test]
fn an_mvp_is_an_action_like_another() {
    let event = action(ActionTypePayload::Mvp, Some("London Rats"), Some("Morgane"));
    assert_eq!(title(&event), "MVP : Griff #9 (London Rats) — par Morgane");
}

#[test]
fn an_injury_reads_blessure() {
    let injury = ActionTypePayload::Blesse {
        injury: "Commotion".into(),
    };
    assert_eq!(
        title(&action(injury, Some("London Rats"), Some("Morgane"))),
        "Blessure : Griff #9 (London Rats) — par Morgane"
    );
}

#[test]
fn an_action_without_team_or_author_names_drops_both_clauses() {
    let event = action(ActionTypePayload::Passe, None, None);
    assert_eq!(title(&event), "Passe : Griff #9");
}

#[test]
fn the_inducement_selection_lists_what_was_taken() {
    let items = vec![
        item("Relance d'équipe", 2, InducementKindPayload::Inducement),
        item("Pot-de-vin", 1, InducementKindPayload::Inducement),
        item("Morg 'N' Thorg", 1, InducementKindPayload::StarPlayer),
        item(
            "Bloqueur nain",
            1,
            InducementKindPayload::Mercenary { skilled: true },
        ),
        item(
            "Coureur",
            1,
            InducementKindPayload::Mercenary { skilled: false },
        ),
    ];
    assert_eq!(
        title(&inducements(items, Some("London Rats"))),
        "London Rats : Relance d'équipe ×2, Pot-de-vin, Morg 'N' Thorg, \
         Bloqueur nain (Mercenaire Niv. 1), Coureur (Mercenaire) — validé par Morgane"
    );
}

#[test]
fn an_empty_selection_says_so() {
    assert_eq!(
        title(&inducements(vec![], Some("London Rats"))),
        "London Rats : aucun coup de pouce — validé par Morgane"
    );
}

#[test]
fn a_selection_of_an_unnamed_team_is_still_announced() {
    let items = vec![item("Pot-de-vin", 1, InducementKindPayload::Inducement)];
    assert_eq!(
        title(&inducements(items, None)),
        "Coups de pouce : Pot-de-vin — validé par Morgane"
    );
}

#[test]
fn the_cancellation_names_its_author() {
    let event = MatchReportAppEvent::MatchInducementsCancelled {
        event_id: EventId::new(),
        match_report_id: MATCH.into(),
        space_id: SPACE.into(),
        home_team_id: HOME.into(),
        away_team_id: AWAY.into(),
        cancelled_by: AUTHOR.into(),
        cancelled_by_name: Some("Morgane".into()),
    };
    assert_eq!(title(&event), "Coups de pouce annulés par Morgane");
}

#[test]
fn every_toast_links_to_the_live_page_through_its_subtitle() {
    let notice = notice_of(&confirmed(Some("Morgane"))).expect("un toast");

    assert_eq!(notice.toast.subtitle.as_deref(), Some(SEE_MATCH));
    assert_eq!(notice.toast.link, Some(match_link(SPACE, MATCH)));
    assert_eq!(
        match_link(SPACE, MATCH),
        "/app/space-1/match-report/match-1/live"
    );
    assert_eq!(notice.author, AUTHOR);
    assert_eq!(
        (notice.home_team_id.as_str(), notice.away_team_id.as_str()),
        (HOME, AWAY)
    );
}

#[test]
fn a_deleted_action_makes_no_toast() {
    let event = MatchReportAppEvent::MatchActionDeleted {
        event_id: EventId::new(),
        match_report_id: MATCH.into(),
        space_id: SPACE.into(),
        home_team_id: HOME.into(),
        away_team_id: AWAY.into(),
        team_side: TeamSidePayload::Away,
        deleted_by: AUTHOR.into(),
    };
    assert_eq!(notice_of(&event), None);
}

/// Le fan factor d'un camp ne fait que le signal (§ 12.7 de E22, carte 654).
#[test]
fn a_team_fan_factor_makes_no_toast() {
    let event = MatchReportAppEvent::MatchFanFactorRecorded {
        event_id: EventId::new(),
        match_report_id: MATCH.into(),
        space_id: SPACE.into(),
        home_team_id: HOME.into(),
        away_team_id: AWAY.into(),
        team_side: TeamSidePayload::Home,
        recorded_by: AUTHOR.into(),
    };
    assert_eq!(notice_of(&event), None);
}

//! Les lignes du widget de présence : le demandeur n'y figure pas, une personne
//! à plusieurs onglets n'y figure qu'une fois, et les libellés (carte 644).

use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::UserId;
use crate::web::ports::MatchRole;
use crate::web::realtime::presence_widget::{
    build_presence_lines, PresenceLineVm, PresenceWidgetTemplate,
};
use crate::web::realtime::registry::{ConnectionId, PresentConnection};
use askama::Template;

fn present(user_id: UserId, name: &str, role: MatchRole) -> PresentConnection {
    PresentConnection {
        id: ConnectionId::new(),
        user_id,
        coach_name: CoachName::try_new(name).unwrap(),
        role,
    }
}

fn line(name: &str, label: &'static str) -> PresenceLineVm {
    PresenceLineVm {
        coach_name: name.to_string(),
        label,
    }
}

#[test]
fn the_requester_is_not_listed_on_any_of_his_tabs() {
    let (me, morgane) = (UserId::new(), UserId::new());
    let connections = [
        present(me, "Yann", MatchRole::Follow),
        present(me, "Yann", MatchRole::Follow),
        present(morgane, "Morgane", MatchRole::Write),
    ];

    let lines = build_presence_lines(&connections, &me);

    assert_eq!(lines, vec![line("Morgane", "saisit")]);
}

#[test]
fn several_tabs_of_one_person_make_one_line_with_the_strongest_role() {
    let (me, morgane) = (UserId::new(), UserId::new());
    let connections = [
        present(morgane, "Morgane", MatchRole::Follow),
        present(morgane, "Morgane", MatchRole::Write),
    ];

    assert_eq!(
        build_presence_lines(&connections, &me),
        vec![line("Morgane", "saisit")]
    );
}

#[test]
fn followers_and_writers_are_named_and_sorted() {
    let me = UserId::new();
    let connections = [
        present(UserId::new(), "Yann", MatchRole::Follow),
        present(UserId::new(), "Morgane", MatchRole::Write),
    ];

    assert_eq!(
        build_presence_lines(&connections, &me),
        vec![line("Morgane", "saisit"), line("Yann", "suit le match")]
    );
}

#[test]
fn the_widget_renders_one_line_per_person() {
    let html = PresenceWidgetTemplate {
        lines: vec![line("Morgane", "saisit")],
    }
    .render()
    .unwrap();

    assert!(html.contains("presence-widget"), "{html}");
    assert!(html.contains("hx-disinherit=\"*\""), "{html}");
    assert!(html.contains("Morgane saisit"), "{html}");
}

#[test]
fn nobody_else_present_renders_no_line() {
    let html = PresenceWidgetTemplate { lines: vec![] }.render().unwrap();

    assert!(!html.contains("presence-widget-line"), "{html}");
}

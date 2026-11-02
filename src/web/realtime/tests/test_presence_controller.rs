//! La déclaration de présence, sans HTTP : le rôle inscrit est le minimum du
//! droit calculé par le port et de l'activité de la page ; un droit nul
//! refuse ; l'onglet doit être celui du demandeur (carte 644).

use crate::app::auth::domain::user::User;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::email::Email;
use crate::app::shared_kernel::identity::ids::{EntityId, UserId};
use crate::web::ports::{IMatchRolePort, MatchRole};
use crate::web::realtime::presence_controller::{
    declare_presence, parse_activity, role_to_record, PresenceForm, PresenceRefusal,
    PresenceRequest, PresenceTarget,
};
use crate::web::realtime::registry::{ConnectionId, ConnectionRegistry};
use async_trait::async_trait;

/// Rend toujours le même droit, quoi qu'on lui demande.
struct FixedRole(MatchRole);

#[async_trait]
impl IMatchRolePort for FixedRole {
    async fn match_role(&self, _: &User, _: &EntityId, _: &EntityId) -> MatchRole {
        self.0
    }
}

fn user() -> User {
    User::new(
        UserId::new(),
        CoachName::try_new("Morgane".to_string()).unwrap(),
        None,
        Email::try_new("morgane@example.com".to_string()).unwrap(),
        String::new(),
    )
}

fn request(connection_id: ConnectionId, activity: MatchRole) -> (PresenceRequest, EntityId) {
    let match_report_id = EntityId::new();
    let target = PresenceTarget {
        space_id: EntityId::new(),
        match_report_id,
        activity,
    };
    let request = PresenceRequest {
        connection_id,
        target: Some(target),
    };
    (request, match_report_id)
}

/// Le rôle que le registre a inscrit, pour ce droit et cette activité.
async fn recorded(right: MatchRole, activity: MatchRole) -> Result<MatchRole, PresenceRefusal> {
    let registry = ConnectionRegistry::new();
    let user = user();
    let opened = registry.open(user.id, user.coach_name.clone());
    let (request, match_report_id) = request(opened.id, activity);
    declare_presence(&registry, &FixedRole(right), &user, &request).await?;
    Ok(registry.present_on(&match_report_id)[0].role)
}

#[tokio::test]
async fn the_recorded_role_is_the_one_of_the_port() {
    assert_eq!(
        recorded(MatchRole::Write, MatchRole::Write).await,
        Ok(MatchRole::Write)
    );
    assert_eq!(
        recorded(MatchRole::Follow, MatchRole::Follow).await,
        Ok(MatchRole::Follow)
    );
}

/// Un admin qui lit `/live` suit le match : la page ne prétend pas saisir.
#[tokio::test]
async fn a_follow_page_never_gives_write() {
    assert_eq!(
        recorded(MatchRole::Write, MatchRole::Follow).await,
        Ok(MatchRole::Follow)
    );
}

/// Une page de saisie ouverte par qui n'a pas le droit d'écrire : il suit.
#[tokio::test]
async fn a_write_page_without_the_right_to_write_gives_follow() {
    assert_eq!(
        recorded(MatchRole::Follow, MatchRole::Write).await,
        Ok(MatchRole::Follow)
    );
}

#[tokio::test]
async fn no_right_is_refused_whatever_the_activity() {
    for activity in [MatchRole::Follow, MatchRole::Write] {
        assert_eq!(
            recorded(MatchRole::None, activity).await,
            Err(PresenceRefusal::Forbidden)
        );
    }
}

#[tokio::test]
async fn a_refused_declaration_records_nothing() {
    let registry = ConnectionRegistry::new();
    let user = user();
    let opened = registry.open(user.id, user.coach_name.clone());
    let (request, match_report_id) = request(opened.id, MatchRole::Write);

    let _ = declare_presence(&registry, &FixedRole(MatchRole::None), &user, &request).await;

    assert!(registry.present_on(&match_report_id).is_empty());
}

#[tokio::test]
async fn the_connection_of_another_user_is_unknown() {
    let registry = ConnectionRegistry::new();
    let owner = registry.open(UserId::new(), CoachName::try_new("Yann").unwrap());
    let (request, match_report_id) = request(owner.id, MatchRole::Follow);

    let outcome =
        declare_presence(&registry, &FixedRole(MatchRole::Write), &user(), &request).await;

    assert_eq!(outcome.err(), Some(PresenceRefusal::UnknownConnection));
    assert!(registry.present_on(&match_report_id).is_empty());
}

#[test]
fn the_recorded_role_is_the_minimum_of_right_and_activity() {
    assert_eq!(
        role_to_record(MatchRole::Write, MatchRole::Write),
        Some(MatchRole::Write)
    );
    assert_eq!(
        role_to_record(MatchRole::Write, MatchRole::Follow),
        Some(MatchRole::Follow)
    );
    assert_eq!(
        role_to_record(MatchRole::Follow, MatchRole::Write),
        Some(MatchRole::Follow)
    );
    assert_eq!(role_to_record(MatchRole::None, MatchRole::Write), None);
}

fn form(space_id: &str, match_report_id: &str, activity: &str) -> PresenceForm {
    PresenceForm {
        connection_id: ConnectionId::new().to_string(),
        space_id: space_id.to_string(),
        match_report_id: match_report_id.to_string(),
        activity: activity.to_string(),
    }
}

#[test]
fn an_empty_form_is_a_withdrawal() {
    let parsed = form("", "", "").parse().expect("lisible");
    assert_eq!(parsed.target, None);
}

#[test]
fn a_complete_form_names_its_match_and_activity() {
    let (space, mr) = (EntityId::new().to_string(), EntityId::new().to_string());
    let target = form(&space, &mr, "write").parse().unwrap().target.unwrap();
    assert_eq!(target.match_report_id.to_string(), mr);
    assert_eq!(target.activity, MatchRole::Write);
}

#[test]
fn an_unreadable_activity_or_a_half_given_match_is_unreadable() {
    let (space, mr) = (EntityId::new().to_string(), EntityId::new().to_string());
    assert!(form(&space, &mr, "admin").parse().is_none());
    assert!(form(&space, &mr, "").parse().is_none());
    assert!(form("", &mr, "follow").parse().is_none());
    assert!(form(&space, "pas-un-id", "follow").parse().is_none());
    assert_eq!(parse_activity("none"), None);
}

#[test]
fn an_unreadable_connection_is_unreadable() {
    let mut bad = form("", "", "");
    bad.connection_id = "pas-une-connexion".to_string();
    assert!(bad.parse().is_none());
}

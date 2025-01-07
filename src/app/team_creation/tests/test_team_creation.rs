use crate::app::team_creation::create_team;

#[test]
pub fn assert_team_creation_is_ok() {
    let team = create_team::Team::nouvelle_team("Les Bleus".to_string(), ulid::Ulid::new());
    assert_eq!(team.nom, "Les Bleus");
}
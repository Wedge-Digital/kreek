use app::team_creation;

mod framework;
mod app;

fn afficher_nom_team(team: &team_creation::create_team::Team) {
    println!("nom: {}", team.nom);
}

fn main() {
    println!("Hello, world!");
    let team = team_creation::create_team::Team::nouvelle_team("Les Bleus".to_string(), ulid::Ulid::new());
    team.afficher();
    afficher_nom_team(&team);
    team.afficher();
}


#[test]
fn it_works() {
    let result = team_creation::create_team::add(2, 2);
    assert_eq!(result, 4);
}



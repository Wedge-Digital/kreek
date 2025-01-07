use ulid::Ulid;
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub struct Team {
    pub entity_id: Ulid,
    pub coach_id: Ulid,
    pub nom: String,
}

impl Team {
    // Méthode d'instance
    pub fn afficher(&self) {
        println!("nom: {}, coach_id: {}, entity_id: {}" , self.nom, self.coach_id, self.entity_id);
    }

    pub fn nouvelle_team(nom: String, coach_id: Ulid) -> Team {
        let entity_id = Ulid::new();
        Team { nom, coach_id, entity_id }
    }
}

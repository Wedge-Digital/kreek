use crate::app::references::domain::port::IReferenceRepository;

pub struct RosterPickerItem {
    pub uid:  String,
    pub name: String,
}

pub struct InducementPickerItem {
    pub uid:  String,
    pub name: String,
    pub cost: u32,
}

pub struct StarPlayerPickerItem {
    pub uid:  String,
    pub name: String,
}

pub fn build_roster_items(repo: &dyn IReferenceRepository) -> Vec<RosterPickerItem> {
    let mut items: Vec<RosterPickerItem> = repo.list_teams()
        .iter()
        .map(|t| RosterPickerItem { uid: t.uid.clone(), name: t.name.clone() })
        .collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

pub fn build_inducement_items(repo: &dyn IReferenceRepository) -> Vec<InducementPickerItem> {
    repo.list_inducements()
        .iter()
        .map(|i| InducementPickerItem { uid: i.uid.clone(), name: i.name.clone(), cost: i.cost })
        .collect()
}

pub fn build_star_player_items(repo: &dyn IReferenceRepository) -> Vec<StarPlayerPickerItem> {
    repo.list_star_players()
        .iter()
        .map(|s| StarPlayerPickerItem { uid: s.uid.clone(), name: s.name.clone() })
        .collect()
}

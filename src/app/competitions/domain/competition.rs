use crate::app::shared_kernel::bloodbowl::competition_name::CompetitionName;
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, SpaceId};

pub struct Competition {
    pub id: CompetitionId,
    pub space_id: SpaceId,
    pub name: CompetitionName,
    pub logo: CloudinaryImage,
    pub admin_ids: Vec<CoachId>,
}

impl Competition {
    /// **Le créateur en est admin**, qu'il se soit désigné ou non (décision du
    /// 2026-10-03, carte 573) : tout membre peut créer une compétition, et
    /// l'assistant n'est ouvert qu'à ses admins. Sans cette règle, le créateur
    /// serait refusé dès l'étape 2.
    pub fn new(
        space_id: SpaceId,
        name: CompetitionName,
        logo: CloudinaryImage,
        admin_ids: Vec<CoachId>,
        created_by: CoachId,
    ) -> Self {
        Self {
            id: CompetitionId::new(),
            space_id,
            name,
            logo,
            admin_ids: with_admin(admin_ids, created_by),
        }
    }
}

/// La liste des admins après une modification de l'assistant.
///
/// **Un admin de la compétition qui modifie la liste y reste** (décision du
/// 2026-10-03, carte 573) : s'en retirer lui fermerait l'assistant au milieu
/// de la création — il serait refusé à l'étape suivante. Le créateur est le cas
/// qui compte, puisqu'il figure dans la liste dès la création. Un admin d'espace
/// absent de la liste n'y est pas ajouté : il n'en a pas besoin, et la liste
/// n'est pas la sienne.
pub fn admins_after_edit(
    current: &[CoachId],
    requested: Vec<CoachId>,
    editor: &CoachId,
) -> Vec<CoachId> {
    match current.contains(editor) {
        true => with_admin(requested, editor.clone()),
        false => requested,
    }
}

fn with_admin(mut admins: Vec<CoachId>, admin: CoachId) -> Vec<CoachId> {
    if !admins.contains(&admin) {
        admins.push(admin);
    }
    admins
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> CoachId {
        CoachId::try_new(&format!("{n:0>26}")).unwrap()
    }

    fn competition(admins: Vec<CoachId>, creator: CoachId) -> Competition {
        Competition::new(
            SpaceId::new(),
            CompetitionName::try_new("Ligue de Condate".to_string()).unwrap(),
            CloudinaryImage::try_new("https://res.cloudinary.com/x/image/upload/a.png".to_string())
                .unwrap(),
            admins,
            creator,
        )
    }

    #[test]
    fn the_creator_becomes_admin_without_asking() {
        assert_eq!(
            competition(vec![id(2)], id(1)).admin_ids,
            vec![id(2), id(1)]
        );
    }

    #[test]
    fn a_creator_who_named_themself_is_not_listed_twice() {
        assert_eq!(
            competition(vec![id(1), id(2)], id(1)).admin_ids,
            vec![id(1), id(2)]
        );
    }

    #[test]
    fn an_admin_who_edits_the_list_stays_in_it() {
        assert_eq!(
            admins_after_edit(&[id(1), id(2)], vec![id(2)], &id(1)),
            vec![id(2), id(1)]
        );
    }

    #[test]
    fn a_space_admin_outside_the_list_is_not_added() {
        assert_eq!(
            admins_after_edit(&[id(1)], vec![id(1)], &id(9)),
            vec![id(1)]
        );
    }
}

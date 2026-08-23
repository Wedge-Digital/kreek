use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::coach_icon::CoachIcon;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::CoachId;

#[derive(Debug, Clone, PartialEq)]
pub struct Coach {
    pub id: CoachId,
    pub name: CoachName,
    pub profile: SpaceProfile,
    /// Optionnelle, et le type l'impose : `CoachIcon` est un alias de
    /// `CloudinaryImage`, dont la validation exige une URL Cloudinary. La chaîne
    /// vide est refusée, il n'existe donc pas de « valeur neutre » à inventer
    /// pour un coach qui n'a pas d'avatar.
    ///
    /// Le champ était obligatoire, et le chargement traitait une icône absente
    /// comme un membre absent — les trente-huit membres de la base de démo, tous
    /// sans avatar, disparaissaient ainsi de l'agrégat.
    pub icon: Option<CoachIcon>,
}

impl Coach {
    pub fn new(
        id: CoachId,
        name: CoachName,
        profile: SpaceProfile,
        icon: Option<CoachIcon>,
    ) -> Self {
        Self {
            id,
            name,
            profile,
            icon,
        }
    }
}

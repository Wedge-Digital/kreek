use crate::app::shared_kernel::identity::charset::TEXTE_SAISI;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, EntityId};
use nutype::nutype;
use serde::{Deserialize, Serialize};

/// Le nom d'une équipe.
///
/// **100 caractères, comme le `TeamName` du BC `teams`.** Les deux ont
/// longtemps divergé — 50 ici, 100 là-bas — et
/// `teams/io/app_events/team_created_listener.rs` retombe sur « Unknown » en
/// silence quand la conversion échoue. Une équipe créée avec un nom de 60
/// caractères y perdait donc son nom, sans une ligne de journal. Les faire
/// converger supprime la cause ; le repli silencieux, lui, reste à traiter.
#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 100, regex = TEXTE_SAISI),
    derive(
        Debug,
        Clone,
        Serialize,
        Deserialize,
        PartialEq,
        Eq,
        Hash,
        Display,
        AsRef
    )
)]
pub struct TeamName(String);

pub type TeamId = EntityId;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub struct BaseTeamInfo {
    name: TeamName,
    coach_id: CoachId,
    logo_url: Option<CloudinaryImage>,
}

impl BaseTeamInfo {
    pub fn new(name: TeamName, coach_id: CoachId, logo_url: Option<CloudinaryImage>) -> Self {
        BaseTeamInfo {
            name,
            coach_id,
            logo_url,
        }
    }

    pub fn name(&self) -> &TeamName {
        &self.name
    }
    pub fn coach_id(&self) -> &CoachId {
        &self.coach_id
    }
    pub fn logo_url(&self) -> Option<&CloudinaryImage> {
        self.logo_url.as_ref()
    }
}

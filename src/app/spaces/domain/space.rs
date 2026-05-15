use std::fmt::Display;
use crate::app::shared_kernel::common_types::{CloudinaryImage, SpaceId};
use crate::app::shared_kernel::space_name::SpaceName;
use crate::app::spaces::domain::coach::Coach;

pub struct Space {
    pub id:      SpaceId,
    pub name:    SpaceName,
    pub logo:    CloudinaryImage,
    pub coaches: Vec<Coach>,
}

impl Space {
    pub fn new(id: SpaceId, name: SpaceName, logo: CloudinaryImage, coaches: Vec<Coach>) -> Self {
        Self { id, name, logo, coaches }
    }
}
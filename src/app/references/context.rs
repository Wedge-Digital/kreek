use crate::app::references::domain::port::IReferenceRepository;
use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;
use crate::app::references::io::repository::reference_data_error::ReferenceDataError;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub struct ReferencesContext {
    pub repository: Arc<dyn IReferenceRepository>,
}

impl ReferencesContext {
    pub fn new(refs_dir: &Path) -> Result<Self, ReferenceDataError> {
        Ok(Self {
            repository: Arc::new(InMemoryReferenceRepository::load_from_dir(refs_dir)?),
        })
    }

    /// Contexte bâti sur le jeu de données des tests unitaires.
    #[cfg(test)]
    pub fn for_tests() -> Self {
        Self {
            repository: Arc::new(InMemoryReferenceRepository::load_for_tests()),
        }
    }
}

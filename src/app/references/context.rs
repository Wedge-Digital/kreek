use std::sync::Arc;
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;

#[derive(Clone)]
pub struct ReferencesContext {
    pub repository: Arc<dyn IReferenceRepository>,
}

impl ReferencesContext {
    pub fn new() -> Self {
        Self {
            repository: Arc::new(InMemoryReferenceRepository::load()),
        }
    }
}

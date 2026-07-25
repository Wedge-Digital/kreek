use std::fmt;

/// Erreurs de chargement des données de référence depuis le disque.
///
/// Ces erreurs sont fatales au démarrage : sans données de référence,
/// l'application ne peut pas fonctionner. Elles vivent dans la couche IO —
/// le domaine ignore d'où viennent ses données.
#[derive(Debug)]
pub enum ReferenceDataError {
    FileUnreadable { file: String, cause: String },
    InvalidJson { file: String, cause: String },
}

impl fmt::Display for ReferenceDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReferenceDataError::FileUnreadable { file, cause } => {
                write!(f, "données de référence illisibles : {} ({})", file, cause)
            }
            ReferenceDataError::InvalidJson { file, cause } => {
                write!(f, "données de référence invalides : {} ({})", file, cause)
            }
        }
    }
}

impl std::error::Error for ReferenceDataError {}

use crate::app::match_report::ports::{IKeywordCatalogPort, KeywordDto};
use crate::app::references::domain::models::Keyword;
use crate::app::references::domain::port::IReferenceRepository;
use std::sync::Arc;

/// Traduit le référentiel vers le vocabulaire de `match_report`.
///
/// Il **filtre ici, une fois**, plutôt que de laisser chaque appelant écarter
/// les mots-clefs de poste : `list_hateable` et `find_hateable` ne rendent que
/// des mots-clefs haïssables, et un `BLITZER` en sort donc comme un uid inconnu.
/// C'est délibéré — l'écran ne le propose pas, donc une requête qui le porte
/// vient d'ailleurs, et lui inventer une erreur distincte documenterait au
/// client une nuance dont il n'a pas à connaître l'existence.
pub struct KeywordCatalogAdapter {
    reference_repo: Arc<dyn IReferenceRepository>,
}

impl KeywordCatalogAdapter {
    pub fn new(reference_repo: Arc<dyn IReferenceRepository>) -> Self {
        Self { reference_repo }
    }

    /// Un mot-clef haïssable **sans** `hate_skill_uid` est impossible : la garde
    /// de démarrage (carte 399) refuse le corpus qui en porterait un. Le
    /// `filter_map` est donc un total, pas un silence.
    fn vers_dto(mot: &Keyword) -> Option<KeywordDto> {
        if !mot.league_hate_selectable {
            return None;
        }
        Some(KeywordDto {
            uid: mot.uid.clone(),
            label: mot.label.clone(),
            hate_skill_uid: mot.hate_skill_uid.clone()?,
        })
    }
}

impl IKeywordCatalogPort for KeywordCatalogAdapter {
    fn list_hateable(&self) -> Vec<KeywordDto> {
        self.reference_repo
            .list_keywords()
            .iter()
            .filter_map(Self::vers_dto)
            .collect()
    }

    fn find_hateable(&self, uid: &str) -> Option<KeywordDto> {
        self.reference_repo
            .find_keyword_by_uid(uid)
            .and_then(Self::vers_dto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;

    fn adapter() -> KeywordCatalogAdapter {
        KeywordCatalogAdapter::new(Arc::new(InMemoryReferenceRepository::load_for_tests()))
    }

    #[test]
    fn le_catalogue_ne_rend_que_les_mots_clefs_haissables() {
        let mots = adapter().list_hateable();
        assert_eq!(mots.len(), 30, "seuls les 30 haïssables doivent sortir");
        assert!(
            mots.iter().all(|m| !m.hate_skill_uid.is_empty()),
            "chaque mot-clef rendu porte sa compétence"
        );
    }

    #[test]
    fn un_mot_clef_haissable_se_trouve_avec_sa_competence() {
        let mot = adapter()
            .find_hateable("DARK_ELF")
            .expect("DARK_ELF est haïssable au corpus");
        assert_eq!(mot.label, "Elfe Noir");
        assert_eq!(mot.hate_skill_uid, "HAINE_DARK_ELF");
    }

    /// `BLITZER` existe au corpus. Le port le traite comme un inconnu : c'est un
    /// rôle, et on ne hait pas un rôle.
    #[test]
    fn un_mot_clef_de_poste_est_introuvable_comme_un_inconnu() {
        let a = adapter();
        assert!(a.find_hateable("BLITZER").is_none());
        assert!(a.find_hateable("ESPECE_QUI_N_EXISTE_PAS").is_none());
    }
}

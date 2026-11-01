//! Le catalogue de références vu par l'impression de `match_report` (carte 605).
//!
//! Seul ce fichier importe `references` pour `match_report` : le BC ne connaît
//! que ses DTO. La disponibilité des star players est la règle du domaine de
//! `references` (`star_player_available_for`, carte 603) — appliquée ici, et
//! par le sélecteur de coups de pouce, au même endroit.

use crate::app::match_report::ports::{
    IReferenceCatalogPort, InducementLabelDto, MatchRemindersDto, SppScaleDto, StarPlayerDto,
};
use crate::app::references::domain::models::StarPlayer;
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::references::domain::star_player_availability::star_player_available_for;
use std::sync::Arc;

pub struct ReferenceCatalogAdapter {
    reference_repo: Arc<dyn IReferenceRepository>,
}

impl ReferenceCatalogAdapter {
    pub fn new(reference_repo: Arc<dyn IReferenceRepository>) -> Self {
        Self { reference_repo }
    }

    /// Les compétences d'un star player, en noms ; un uid inconnu du catalogue
    /// s'affiche tel quel plutôt que de disparaître.
    fn skill_names(&self, uids: &[String]) -> Vec<String> {
        uids.iter()
            .map(|uid| {
                self.reference_repo
                    .find_skill_by_uid(uid)
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| uid.clone())
            })
            .collect()
    }

    fn star_dto(&self, sp: &StarPlayer) -> StarPlayerDto {
        StarPlayerDto {
            uid: sp.uid.clone(),
            name: sp.name.clone(),
            player_type: sp.player_type.clone(),
            cost: sp.cost,
            ma: sp.ma,
            st: sp.st,
            ag: sp.ag.clone(),
            pa: sp.pa.clone(),
            av: sp.av.clone(),
            skills: self.skill_names(&sp.skills),
            ability_name: sp.special_ability_name.clone(),
            ability_description: sp.special_ability_description.clone(),
        }
    }
}

impl IReferenceCatalogPort for ReferenceCatalogAdapter {
    fn find_inducement_labels(&self, uids: &[String]) -> Vec<InducementLabelDto> {
        uids.iter()
            .filter_map(|uid| self.reference_repo.find_inducement_by_uid(uid))
            .map(|i| InducementLabelDto {
                uid: i.uid.clone(),
                name: i.name.clone(),
                description: i.description.clone(),
            })
            .collect()
    }

    fn find_star_players(&self, uids: &[String], roster_id: &str) -> Vec<StarPlayerDto> {
        uids.iter()
            .filter_map(|uid| self.reference_repo.find_star_player_by_uid(uid))
            .filter(|sp| star_player_available_for(sp, roster_id))
            .map(|sp| self.star_dto(sp))
            .collect()
    }

    fn find_spp_scale(&self, roster_id: &str) -> SppScaleDto {
        let s = self.reference_repo.spp_scale_for_roster(roster_id);
        SppScaleDto {
            touchdown: s.touchdown,
            casualty: s.casualty,
            interception: s.interception,
            pass: s.pass,
            mvp: s.mvp,
        }
    }

    fn find_match_reminders(&self) -> MatchRemindersDto {
        let r = self.reference_repo.match_reminders();
        MatchRemindersDto {
            mvp: r.mvp,
            fans: r.fans,
            weather: r.weather,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;

    fn adapter() -> ReferenceCatalogAdapter {
        ReferenceCatalogAdapter::new(Arc::new(InMemoryReferenceRepository::load_for_tests()))
    }

    fn uids(liste: &[&str]) -> Vec<String> {
        liste.iter().map(|u| u.to_string()).collect()
    }

    #[test]
    fn les_competences_d_un_star_player_sont_resolues_en_noms() {
        let stars = adapter().find_star_players(&uids(&["DEMO_ORVAL_LE_PATIENT"]), "DEMO_GRANIT");
        assert_eq!(stars.len(), 1);
        assert_eq!(stars[0].skills, vec!["Appui Ferme", "Masse Inébranlable"]);
        assert_eq!(stars[0].ability_name, "Pierre Levée");
    }

    /// R13 : Grumo ne joue que pour les Granitiers.
    #[test]
    fn un_star_player_hors_de_son_roster_n_est_pas_rendu() {
        let a = adapter();
        let liste = uids(&["DEMO_GRUMO_LE_TERNE"]);
        assert_eq!(a.find_star_players(&liste, "DEMO_GRANIT").len(), 1);
        assert!(a.find_star_players(&liste, "DEMO_ZEPHYR").is_empty());
    }

    /// Les Granitiers portent `BRAWLIN_BRUTES` : touchdown et sortie s'inversent.
    #[test]
    fn le_bareme_suit_la_regle_speciale_du_roster() {
        let a = adapter();
        assert_eq!(a.find_spp_scale("DEMO_GRANIT").touchdown, 2);
        assert_eq!(a.find_spp_scale("DEMO_ZEPHYR").touchdown, 4);
    }

    #[test]
    fn les_libelles_ignorent_un_uid_inconnu() {
        let labels =
            adapter().find_inducement_labels(&uids(&["DEMO_CHARIOT_DE_VIVRES", "INCONNU"]));
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, "Chariot de Vivres");
    }

    #[test]
    fn les_rappels_du_corpus_de_demonstration_sont_transmis() {
        let r = adapter().find_match_reminders();
        assert!(r.mvp.is_some());
        assert_eq!(r.weather.len(), 5);
    }
}

//! Qui peut administrer une compétition.
//!
//! # Une seule règle, deux appelants
//!
//! « Administrateur de la compétition, ou administrateur de l'espace » gardait
//! l'entrée dans `require_admin_access` et décidait de l'affichage du bouton
//! dans `competition_detail` — deux écritures, dont la seconde ne portait que
//! la première moitié. L'administrateur d'espace avait donc le droit d'entrer
//! sans qu'on le lui montre (carte 544).
//!
//! Les deux appellent désormais cette fonction. Elle ne décide **pas** de
//! l'accès à elle seule : `require_admin_access` lui ajoute la cohérence du
//! couple saison/compétition (carte 416), qui ne relève pas du droit mais du
//! chemin.

use crate::app::competitions::domain::competition_repository_port::CompetitionBaseInfo;
use crate::app::competitions::ports::ICompetitionSpaceMemberPort;
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};

/// Les deux listes d'administrateurs d'une compétition, empruntées.
///
/// Une struct plutôt que deux `&[String]` côte à côte : ce sont deux
/// paramètres du même type, et rien n'attraperait leur inversion à
/// l'appel. Elle affranchit aussi le service de `CompetitionBaseInfo` —
/// `PageBase`, qui porte les mêmes listes sans le même type, l'appelle
/// désormais aussi.
#[derive(Clone, Copy)]
pub struct AdminsDeLaCompetition<'a> {
    pub ids: &'a [String],
    pub names: &'a [String],
}

impl<'a> From<&'a CompetitionBaseInfo> for AdminsDeLaCompetition<'a> {
    fn from(info: &'a CompetitionBaseInfo) -> Self {
        Self {
            ids: &info.admin_ids,
            names: &info.admin_names,
        }
    }
}

/// L'ordre des deux questions n'est pas indifférent.
///
/// La compétition d'abord : ses administrateurs sont **déjà en mémoire**, la
/// page les porte. L'espace ensuite, qui coûte un aller-retour.
/// Un administrateur de compétition consultant sa propre compétition — le cas
/// fréquent — n'en déclenche aucun.
///
/// Le nom du coach compte autant que son identifiant : une compétition peut
/// désigner ses administrateurs par l'un ou par l'autre, et les deux listes
/// coexistent dans `CompetitionBaseInfo`.
// Sur une seule ligne : l'axe 11 n'examine que celle qui précède la fonction.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn peut_administrer(
    coach_id: &CoachId,
    coach_name: &str,
    space_id: &SpaceId,
    admins: AdminsDeLaCompetition<'_>,
    membres: &dyn ICompetitionSpaceMemberPort,
) -> bool {
    if est_admin_de_la_competition(coach_id, coach_name, admins) {
        return true;
    }
    matches!(
        membres.find_member_profile(coach_id, space_id).await,
        Some(SpaceProfile::SpaceAdmin)
    )
}

fn est_admin_de_la_competition(
    coach_id: &CoachId,
    coach_name: &str,
    admins: AdminsDeLaCompetition<'_>,
) -> bool {
    admins.ids.contains(&coach_id.to_string()) || admins.names.contains(&coach_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::ports::SpaceMemberDto;
    use crate::app::shared_kernel::identity::space_definition::SpaceDefinition;
    use async_trait::async_trait;

    // ── Doublures ────────────────────────────────────────────────────────────

    /// Le profil rendu quel que soit le coach : ces tests n'éprouvent que la
    /// combinaison des deux droits, jamais la résolution de l'appartenance.
    struct Membres(Option<SpaceProfile>);

    #[async_trait]
    impl ICompetitionSpaceMemberPort for Membres {
        async fn list_space_members(&self, _: &SpaceId) -> Vec<SpaceMemberDto> {
            Vec::new()
        }
        async fn find_member_profile(&self, _: &CoachId, _: &SpaceId) -> Option<SpaceProfile> {
            self.0.clone()
        }
        async fn find_all_spaces(&self) -> Vec<SpaceDefinition> {
            Vec::new()
        }
    }

    fn competition(admin_ids: &[&str], admin_names: &[&str]) -> CompetitionBaseInfo {
        CompetitionBaseInfo {
            name: "Coupe des Douves".to_string(),
            logo: None,
            admin_ids: admin_ids.iter().map(|s| s.to_string()).collect(),
            admin_names: admin_names.iter().map(|s| s.to_string()).collect(),
        }
    }

    async fn peut(profil: Option<SpaceProfile>, comp: &CompetitionBaseInfo) -> bool {
        let coach = CoachId::try_new("01JCAAAAAAAAAAAAAAAAAAAAAA").unwrap();
        let espace = SpaceId::try_new("01JSPBBBBBBBBBBBBBBBBBBBBB").unwrap();
        peut_administrer(&coach, "Sylvestre", &espace, comp.into(), &Membres(profil)).await
    }

    // ── Les quatre combinaisons ──────────────────────────────────────────────

    #[tokio::test]
    async fn admin_de_la_competition_par_identifiant() {
        let comp = competition(&["01JCAAAAAAAAAAAAAAAAAAAAAA"], &[]);
        assert!(peut(Some(SpaceProfile::SpaceUser), &comp).await);
    }

    #[tokio::test]
    async fn admin_de_la_competition_par_nom() {
        let comp = competition(&[], &["Sylvestre"]);
        assert!(peut(Some(SpaceProfile::SpaceUser), &comp).await);
    }

    /// Le cas de la carte 544 : aucun droit sur la compétition, mais le
    /// gouvernail de l'espace.
    #[tokio::test]
    async fn admin_de_l_espace_seulement() {
        let comp = competition(&["01JZZCCCCCCCCCCCCCCCCCCCCC"], &["Quelqu'un d'autre"]);
        assert!(peut(Some(SpaceProfile::SpaceAdmin), &comp).await);
    }

    #[tokio::test]
    async fn admin_des_deux() {
        let comp = competition(&["01JCAAAAAAAAAAAAAAAAAAAAAA"], &[]);
        assert!(peut(Some(SpaceProfile::SpaceAdmin), &comp).await);
    }

    #[tokio::test]
    async fn membre_simple_de_l_espace() {
        let comp = competition(&["01JZZCCCCCCCCCCCCCCCCCCCCC"], &["Quelqu'un d'autre"]);
        assert!(!peut(Some(SpaceProfile::SpaceUser), &comp).await);
    }

    /// Étranger à l'espace : `find_member_profile` ne rend rien, et le `None`
    /// ne doit pas se confondre avec un profil d'administrateur.
    #[tokio::test]
    async fn etranger_a_l_espace() {
        let comp = competition(&["01JZZCCCCCCCCCCCCCCCCCCCCC"], &["Quelqu'un d'autre"]);
        assert!(!peut(None, &comp).await);
    }

    /// Une compétition sans administrateur désigné n'ouvre à personne — sans
    /// cette épreuve, un `contains` sur une liste vide qui rendrait `true`
    /// passerait inaperçu.
    #[tokio::test]
    async fn competition_sans_administrateur() {
        let comp = competition(&[], &[]);
        assert!(!peut(Some(SpaceProfile::SpaceUser), &comp).await);
    }
}

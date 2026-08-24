use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, EventId, SpaceId};
use crate::app::shared_kernel::identity::space_name::SpaceName;
use crate::app::spaces::domain::coach::Coach;
use crate::app::spaces::domain::domain_event::SpacesDomainEvent;
use crate::app::spaces::domain::membership::{
    ChangementDAppartenance, NombreAdministrateurs, SpaceMembershipError,
};

/// Un espace et ses membres.
///
/// Les champs sont **privés**. Trois méthodes qui gardent un invariant ne
/// servent à rien tant que `space.coaches.push(…)` compile ailleurs.
pub struct Space {
    id: SpaceId,
    name: SpaceName,
    logo: CloudinaryImage,
    coaches: Vec<Coach>,
}

impl Space {
    /// Reconstruction depuis le dépôt. Ne valide rien : un espace hérité sans
    /// administrateur doit pouvoir se charger, sinon on le rend inaccessible au
    /// lieu de le réparer.
    pub fn new(id: SpaceId, name: SpaceName, logo: CloudinaryImage, coaches: Vec<Coach>) -> Self {
        Self {
            id,
            name,
            logo,
            coaches,
        }
    }

    pub fn id(&self) -> &SpaceId {
        &self.id
    }

    pub fn name(&self) -> &SpaceName {
        &self.name
    }

    pub fn logo(&self) -> &CloudinaryImage {
        &self.logo
    }

    /// Une **tranche**, pas un `Vec` : on lit, on ne mute que par les commandes.
    pub fn coaches(&self) -> &[Coach] {
        &self.coaches
    }

    /// Change le rôle d'un membre.
    ///
    /// `acteur` n'est pas décoratif : deux des règles portent sur lui et non sur
    /// la cible. Sans lui, le use case devrait les trancher — c'est-à-dire faire
    /// du métier.
    pub fn change_member_role(
        &mut self,
        acteur: &CoachId,
        cible: &CoachId,
        nouveau: SpaceProfile,
    ) -> Result<ChangementDAppartenance, SpaceMembershipError> {
        if acteur == cible {
            return Err(SpaceMembershipError::ActeurEstLaCible);
        }
        let actuel = self
            .membre(cible)
            .map(|c| c.profile.clone())
            .ok_or(SpaceMembershipError::PasMembre)?;

        // Reposter le rôle courant n'est pas une erreur — et ne doit pas
        // inscrire au journal un changement qui n'a pas eu lieu.
        if actuel == nouveau {
            return Ok(self.sans_changement());
        }

        if actuel == SpaceProfile::SpaceAdmin && self.est_dernier_administrateur(cible) {
            return Err(SpaceMembershipError::DernierAdministrateur);
        }

        if let Some(coach) = self.coaches.iter_mut().find(|c| &c.id == cible) {
            coach.profile = nouveau.clone();
        }

        let evenement = match nouveau {
            SpaceProfile::SpaceAdmin => SpacesDomainEvent::UserPromotedToSpaceAdmin {
                event_id: EventId::new(),
                user_id: *cible,
                space_id: self.id,
            },
            SpaceProfile::SpaceUser => SpacesDomainEvent::UserDemotedToSpaceUser {
                event_id: EventId::new(),
                user_id: *cible,
                space_id: self.id,
            },
        };
        Ok(self.avec(evenement))
    }

    /// Retire un membre de l'espace.
    ///
    /// **Aucune vérification sur ses équipes**, et c'est délibéré : un coach
    /// peut être retiré même s'il a une équipe engagée en compétition. La
    /// compétition se déroule, l'équipe reste, et la saisie des matchs n'est pas
    /// touchée — elle n'est ouverte qu'aux administrateurs. Ne pas ajouter de
    /// garde ici en croyant réparer un oubli.
    pub fn remove_member(
        &mut self,
        acteur: &CoachId,
        cible: &CoachId,
    ) -> Result<ChangementDAppartenance, SpaceMembershipError> {
        if acteur == cible {
            return Err(SpaceMembershipError::ActeurEstLaCible);
        }
        let profil = self
            .membre(cible)
            .map(|c| c.profile.clone())
            .ok_or(SpaceMembershipError::PasMembre)?;

        // L'invariant ne porte que sur les administrateurs : retirer un membre
        // ordinaire d'un espace qui n'en a qu'un doit réussir.
        if profil == SpaceProfile::SpaceAdmin && self.est_dernier_administrateur(cible) {
            return Err(SpaceMembershipError::DernierAdministrateur);
        }

        self.coaches.retain(|c| &c.id != cible);

        Ok(self.avec(SpacesDomainEvent::UserUnsubscribedFromSpace {
            event_id: EventId::new(),
            user_id: *cible,
            space_id: self.id,
        }))
    }

    /// Ajoute un membre à l'espace, **sans son consentement**.
    ///
    /// `nouveau` est un `Coach` complet et non un identifiant : l'agrégat stocke
    /// des `Coach`, qui portent un pseudo et une icône. Le use case le construit
    /// depuis le cache d'utilisateurs — le domaine ne va rien chercher lui-même.
    ///
    /// `acteur` **ne sert aucune règle** : rien n'interdit à un administrateur
    /// d'ajouter qui il veut. Il est là pour la trace, l'opération se passant du
    /// consentement de la cible.
    ///
    /// # Deux non-règles, décidées et non oubliées
    ///
    /// **Les deux profils sont attribuables** à l'ajout, Membre comme Admin.
    ///
    /// **Aucun plafond de membres par espace.** Un plafond ajouté plus tard, une
    /// fois des espaces au-delà du seuil en production, coûte bien plus qu'un
    /// plafond posé d'emblée. Ne pas en ajouter en croyant réparer un oubli.
    pub fn add_member(
        &mut self,
        acteur: &CoachId,
        nouveau: Coach,
    ) -> Result<ChangementDAppartenance, SpaceMembershipError> {
        if self.membre(&nouveau.id).is_some() {
            return Err(SpaceMembershipError::DejaMembre);
        }

        let evenement = SpacesDomainEvent::UserAddedToSpaceByAdmin {
            event_id: EventId::new(),
            user_id: nouveau.id,
            space_id: self.id,
            profile: nouveau.profile.clone(),
            added_by: *acteur,
        };
        self.coaches.push(nouveau);
        Ok(self.avec(evenement))
    }

    fn membre(&self, id: &CoachId) -> Option<&Coach> {
        self.coaches.iter().find(|c| &c.id == id)
    }

    fn nombre_d_administrateurs(&self) -> usize {
        self.coaches
            .iter()
            .filter(|c| c.profile == SpaceProfile::SpaceAdmin)
            .count()
    }

    fn est_dernier_administrateur(&self, id: &CoachId) -> bool {
        self.membre(id)
            .is_some_and(|c| c.profile == SpaceProfile::SpaceAdmin)
            && self.nombre_d_administrateurs() == 1
    }

    /// Le compte est lu **après** la mutation : c'est le seul instant où il est
    /// exact, et le seul où il a un sens pour l'appelant.
    fn avec(&self, evenement: SpacesDomainEvent) -> ChangementDAppartenance {
        ChangementDAppartenance {
            evenement: Some(evenement),
            administrateurs: self.compte(),
        }
    }

    fn sans_changement(&self) -> ChangementDAppartenance {
        ChangementDAppartenance {
            evenement: None,
            administrateurs: self.compte(),
        }
    }

    /// Le repli est **inatteignable**, et il tient à deux choses qui doivent
    /// rester vraies ensemble.
    ///
    /// `compte()` n'est appelé que sur un chemin de succès, lequel exige une
    /// cible membre — donc au moins un membre. Et depuis la migration
    /// `20260823000002`, tout espace **peuplé** a un administrateur : quatre
    /// n'en avaient aucun, personne ne pouvait les administrer.
    ///
    /// Un espace sans membre a bien zéro administrateur, mais aucune commande
    /// n'y réussit — toute cible y est `PasMembre`.
    ///
    /// Rendre `1` plutôt que paniquer : si l'une de ces deux prémisses tombe un
    /// jour, un compte faux se répare, un `panic` en production non.
    fn compte(&self) -> NombreAdministrateurs {
        NombreAdministrateurs::try_new(self.nombre_d_administrateurs())
            .unwrap_or(NombreAdministrateurs::try_new(1).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::identity::coach_name::CoachName;

    const LOGO: &str = "https://res.cloudinary.com/demo/image/upload/space.jpg";

    fn coach(nom: &str, profil: SpaceProfile) -> Coach {
        Coach::new(
            CoachId::new(),
            CoachName::try_new(nom).unwrap(),
            profil,
            None,
        )
    }

    fn espace(coaches: Vec<Coach>) -> Space {
        Space::new(
            SpaceId::new(),
            SpaceName::try_new("Tribu Celtique").unwrap(),
            CloudinaryImage::try_new(LOGO).unwrap(),
            coaches,
        )
    }

    fn type_de(c: &ChangementDAppartenance) -> Option<&'static str> {
        c.evenement.as_ref().map(|e| e.to_event_type())
    }

    // ── Promotion et rétrogradation ─────────────────────────────────────────

    #[test]
    fn promouvoir_un_membre_rend_l_evenement_et_incremente_le_compte() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let mut space = espace(vec![admin, membre]);

        let r = space
            .change_member_role(&a, &m, SpaceProfile::SpaceAdmin)
            .unwrap();

        assert_eq!(type_de(&r), Some("UserPromotedToSpaceAdmin"));
        assert_eq!(
            r.administrateurs,
            NombreAdministrateurs::try_new(2).unwrap()
        );
        assert_eq!(space.membre(&m).unwrap().profile, SpaceProfile::SpaceAdmin);
    }

    #[test]
    fn retrograder_un_admin_parmi_deux_rend_l_evenement_et_decremente() {
        let a1 = coach("Admin1", SpaceProfile::SpaceAdmin);
        let a2 = coach("Admin2", SpaceProfile::SpaceAdmin);
        let (i1, i2) = (a1.id, a2.id);
        let mut space = espace(vec![a1, a2]);

        let r = space
            .change_member_role(&i1, &i2, SpaceProfile::SpaceUser)
            .unwrap();

        assert_eq!(type_de(&r), Some("UserDemotedToSpaceUser"));
        assert_eq!(
            r.administrateurs,
            NombreAdministrateurs::try_new(1).unwrap()
        );
    }

    /// L'invariant : un espace a toujours au moins un administrateur.
    ///
    /// On vérifie l'**état**, pas seulement le type d'erreur — une
    /// implémentation qui muterait d'abord et validerait ensuite passerait le
    /// second contrôle et raterait le premier.
    #[test]
    fn retrograder_le_seul_administrateur_est_refuse_et_ne_mute_rien() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let mut space = espace(vec![admin, membre]);
        let avant = space.coaches().to_vec();

        let r = space.change_member_role(&m, &a, SpaceProfile::SpaceUser);

        assert_eq!(r.unwrap_err(), SpaceMembershipError::DernierAdministrateur);
        assert_eq!(space.coaches(), avant.as_slice());
    }

    #[test]
    fn reposter_le_role_courant_reussit_sans_evenement() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let mut space = espace(vec![admin, membre]);

        let r = space
            .change_member_role(&a, &m, SpaceProfile::SpaceUser)
            .unwrap();

        assert_eq!(type_de(&r), None, "rien ne s'est passé, rien ne s'inscrit");
        assert_eq!(
            r.administrateurs,
            NombreAdministrateurs::try_new(1).unwrap()
        );
    }

    // ── Retrait ─────────────────────────────────────────────────────────────

    #[test]
    fn retirer_un_membre_rend_l_evenement_de_desinscription() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let mut space = espace(vec![admin, membre]);

        let r = space.remove_member(&a, &m).unwrap();

        assert_eq!(type_de(&r), Some("UserUnsubscribedFromSpace"));
        assert_eq!(space.coaches().len(), 1);
        assert!(space.membre(&m).is_none());
    }

    #[test]
    fn retirer_le_seul_administrateur_est_refuse_et_ne_mute_rien() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let mut space = espace(vec![admin, membre]);
        let avant = space.coaches().to_vec();

        let r = space.remove_member(&m, &a);

        assert_eq!(r.unwrap_err(), SpaceMembershipError::DernierAdministrateur);
        assert_eq!(space.coaches(), avant.as_slice());
    }

    /// Le cas qu'une lecture rapide de l'invariant fait rater.
    ///
    /// Il porte sur les **administrateurs**, pas sur tous les retraits : retirer
    /// un membre ordinaire d'un espace qui n'a qu'un administrateur doit
    /// réussir. Une garde posée sur tous les retraits passerait tous les autres
    /// tests de ce fichier.
    #[test]
    fn retirer_un_membre_ordinaire_d_un_espace_a_un_seul_admin_reussit() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let mut space = espace(vec![admin, membre]);

        let r = space.remove_member(&a, &m);

        assert!(r.is_ok(), "l'invariant ne concerne que les administrateurs");
        assert_eq!(space.coaches().len(), 1);
    }

    // ── Règles portant sur l'acteur et la cible ─────────────────────────────

    #[test]
    fn on_ne_modifie_pas_son_propre_role() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let autre = coach("Autre", SpaceProfile::SpaceAdmin);
        let a = admin.id;
        let mut space = espace(vec![admin, autre]);

        let r = space.change_member_role(&a, &a, SpaceProfile::SpaceUser);

        assert_eq!(r.unwrap_err(), SpaceMembershipError::ActeurEstLaCible);
    }

    #[test]
    fn on_ne_se_retire_pas_soi_meme() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let autre = coach("Autre", SpaceProfile::SpaceAdmin);
        let a = admin.id;
        let mut space = espace(vec![admin, autre]);

        let r = space.remove_member(&a, &a);

        assert_eq!(r.unwrap_err(), SpaceMembershipError::ActeurEstLaCible);
    }

    #[test]
    fn une_cible_absente_de_l_espace_est_refusee() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let a = admin.id;
        let etranger = CoachId::new();
        let mut space = espace(vec![admin]);

        assert_eq!(
            space
                .change_member_role(&a, &etranger, SpaceProfile::SpaceAdmin)
                .unwrap_err(),
            SpaceMembershipError::PasMembre
        );
        assert_eq!(
            space.remove_member(&a, &etranger).unwrap_err(),
            SpaceMembershipError::PasMembre
        );
    }

    // ── Ajout par un administrateur ─────────────────────────────────────────

    #[test]
    fn ajouter_un_non_membre_en_membre_rend_l_evenement() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let a = admin.id;
        let mut space = espace(vec![admin]);
        let nouveau = coach("Nouveau", SpaceProfile::SpaceUser);
        let n = nouveau.id;

        let r = space.add_member(&a, nouveau).unwrap();

        assert_eq!(type_de(&r), Some("UserAddedToSpaceByAdmin"));
        assert_eq!(
            r.administrateurs,
            NombreAdministrateurs::try_new(1).unwrap()
        );
        assert_eq!(space.coaches().len(), 2);
        assert!(space.membre(&n).is_some());
    }

    #[test]
    fn ajouter_un_non_membre_en_admin_incremente_le_compte() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let a = admin.id;
        let mut space = espace(vec![admin]);

        let r = space
            .add_member(&a, coach("Second", SpaceProfile::SpaceAdmin))
            .unwrap();

        assert_eq!(
            r.administrateurs,
            NombreAdministrateurs::try_new(2).unwrap()
        );
    }

    #[test]
    fn ajouter_un_coach_deja_membre_est_refuse_et_ne_mute_rien() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let mut space = espace(vec![admin, membre]);
        let avant = space.coaches().to_vec();

        let deja = Coach::new(
            m,
            CoachName::try_new("Membre").unwrap(),
            SpaceProfile::SpaceUser,
            None,
        );
        let r = space.add_member(&a, deja);

        assert_eq!(r.unwrap_err(), SpaceMembershipError::DejaMembre);
        assert_eq!(space.coaches(), avant.as_slice());
    }

    /// Ajouter en Admin quelqu'un qui est déjà Membre **n'est pas** une
    /// promotion.
    ///
    /// Sans ce test, l'ajout deviendrait un chemin détourné pour changer un
    /// rôle — sans passer par `change_member_role`, donc sans sa règle du
    /// dernier administrateur ni sa vérification sur l'acteur.
    #[test]
    fn ajouter_avec_un_autre_profil_reste_un_refus_et_non_une_promotion() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let mut space = espace(vec![admin, membre]);

        let promu = Coach::new(
            m,
            CoachName::try_new("Membre").unwrap(),
            SpaceProfile::SpaceAdmin,
            None,
        );
        let r = space.add_member(&a, promu);

        assert_eq!(r.unwrap_err(), SpaceMembershipError::DejaMembre);
        assert_eq!(
            space.membre(&m).unwrap().profile,
            SpaceProfile::SpaceUser,
            "le profil ne doit pas avoir changé"
        );
    }

    #[test]
    fn l_evenement_porte_l_acteur_qui_a_ordonne_l_ajout() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let a = admin.id;
        let mut space = espace(vec![admin]);

        let r = space
            .add_member(&a, coach("Nouveau", SpaceProfile::SpaceUser))
            .unwrap();

        let Some(SpacesDomainEvent::UserAddedToSpaceByAdmin { added_by, .. }) = r.evenement else {
            panic!("l'événement attendu est UserAddedToSpaceByAdmin");
        };
        assert_eq!(
            added_by, a,
            "une opération sans consentement dit qui l'a ordonnée"
        );
    }

    // ── Le value object ─────────────────────────────────────────────────────

    #[test]
    fn zero_administrateur_n_est_pas_representable() {
        assert!(NombreAdministrateurs::try_new(0).is_err());
        assert!(NombreAdministrateurs::try_new(1).is_ok());
    }
}

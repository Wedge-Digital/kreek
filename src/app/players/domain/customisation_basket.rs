//! Le panier de customisation — un agrégat, pas un objet applicatif.
//!
//! Il porte tous les invariants de validité : bornes des caractéristiques,
//! doublon de compétence, prix plancher. La tension « le domaine n'appelle pas
//! de port » se résout par **hydratation** : l'agrégat *porte* les données dont
//! ses gardes ont besoin — caractéristiques résolues, compétences possédées,
//! catalogue. Le use case hydrate, puis tout est pur et synchrone : aucun
//! `async`, aucun port, aucune dépendance framework.
//!
//! Même patron que `teams::domain::recruitment_basket`, pour les mêmes raisons.

use crate::app::players::domain::error::DomainError;
use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::domain::player::{PlayerId, Spp, ValueKpo};
use crate::app::players::domain::value_objects::{
    BasketLineId, KpoDelta, SkillId, SppAmount, StatCrans,
};

/// Version du panier persisté, pour la garde d'écriture concurrente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasketVersion(pub u32);

/// Caractéristiques **résolues** du joueur — base du poste, séquelles et
/// augmentations comprises. L'agrégat reçoit un point de départ et raisonne en
/// deltas : il n'a pas à connaître le catalogue de postes.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedStats {
    pub ma: u8,
    pub st: u8,
    pub ag: u8,
    pub pa: u8,
    pub av: u8,
}

impl ResolvedStats {
    pub fn get(&self, stat: StatKind) -> u8 {
        match stat {
            StatKind::Ma => self.ma,
            StatKind::St => self.st,
            StatKind::Ag => self.ag,
            StatKind::Pa => self.pa,
            StatKind::Av => self.av,
        }
    }
}

/// L'état d'une action, tel que la vue a besoin de le connaître pour griser un
/// bouton.
///
/// `Forbidden` est **structurel** : vider le panier n'y changera rien — une
/// compétence déjà possédée le restera. `Blocked` est **conjoncturel** : une
/// borne atteinte à cause des lignes en attente se libère si on en retire une.
///
/// Type propre à `players`, alors que `teams` en a un homonyme : les BCs ne
/// partagent pas leurs types. Duplication assumée, imposée par la souveraineté.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionState {
    Allowed,
    Blocked { cause: DomainError },
    Forbidden { cause: DomainError },
}

/// Le **seul** état persisté du panier. Joueur, catalogue et caractéristiques
/// de base sont rechargés à chaque hydratation — c'est ce qui garantit qu'un
/// panier d'une heure est jugé contre le joueur d'aujourd'hui.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CustomisationLine {
    Skill {
        id: BasketLineId,
        skill_id: SkillId,
    },
    Stat {
        id: BasketLineId,
        stat: StatKind,
        crans: StatCrans,
    },
    Price {
        id: BasketLineId,
        delta: KpoDelta,
    },
    Spp {
        id: BasketLineId,
        amount: SppAmount,
    },
}

impl CustomisationLine {
    pub fn id(&self) -> &BasketLineId {
        match self {
            Self::Skill { id, .. }
            | Self::Stat { id, .. }
            | Self::Price { id, .. }
            | Self::Spp { id, .. } => id,
        }
    }
}

/// Une ligne refusée à la revalidation, avec son motif.
#[derive(Debug, Clone)]
pub struct RejectedLine {
    pub id: BasketLineId,
    pub cause: DomainError,
}

#[derive(Debug, Clone)]
pub struct CustomisationBasket {
    player_id: PlayerId,
    version: BasketVersion,
    lines: Vec<CustomisationLine>,

    // Hydratés, jamais persistés.
    base_stats: ResolvedStats,
    owned_skills: Vec<SkillId>,
    catalog_skills: Vec<SkillId>,
    current_value: ValueKpo,
    current_spp: Spp,
}

impl CustomisationBasket {
    #[allow(clippy::too_many_arguments)]
    pub fn hydrate(
        player_id: PlayerId,
        version: BasketVersion,
        lines: Vec<CustomisationLine>,
        base_stats: ResolvedStats,
        owned_skills: Vec<SkillId>,
        catalog_skills: Vec<SkillId>,
        current_value: ValueKpo,
        current_spp: Spp,
    ) -> Self {
        Self {
            player_id,
            version,
            lines,
            base_stats,
            owned_skills,
            catalog_skills,
            current_value,
            current_spp,
        }
    }

    pub fn player_id(&self) -> &PlayerId {
        &self.player_id
    }
    pub fn version(&self) -> BasketVersion {
        self.version
    }
    pub fn lines(&self) -> &[CustomisationLine] {
        &self.lines
    }
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    // ── Commandes ─────────────────────────────────────────────────────────────

    pub fn add_skill(&mut self, skill_id: SkillId) -> Result<BasketLineId, DomainError> {
        self.check_skill(&skill_id)?;
        let id = self.next_line_id();
        self.lines.push(CustomisationLine::Skill {
            id: id.clone(),
            skill_id,
        });
        Ok(id)
    }

    pub fn add_stat(
        &mut self,
        stat: StatKind,
        crans: StatCrans,
    ) -> Result<BasketLineId, DomainError> {
        self.check_stat(stat, crans)?;
        let id = self.next_line_id();
        self.lines.push(CustomisationLine::Stat {
            id: id.clone(),
            stat,
            crans,
        });
        Ok(id)
    }

    pub fn adjust_price(&mut self, delta: KpoDelta) -> Result<BasketLineId, DomainError> {
        self.check_price(delta)?;
        let id = self.next_line_id();
        self.lines.push(CustomisationLine::Price {
            id: id.clone(),
            delta,
        });
        Ok(id)
    }

    /// Le plafond de 100 est porté par `SppAmount` : il est par opération, donc
    /// entièrement dans le value object. Rien à revérifier ici.
    pub fn add_spp(&mut self, amount: SppAmount) -> Result<BasketLineId, DomainError> {
        let id = self.next_line_id();
        self.lines.push(CustomisationLine::Spp {
            id: id.clone(),
            amount,
        });
        Ok(id)
    }

    pub fn remove_line(&mut self, id: &BasketLineId) -> Result<(), DomainError> {
        let avant = self.lines.len();
        self.lines.retain(|l| l.id() != id);
        match self.lines.len() < avant {
            true => Ok(()),
            false => Err(DomainError::BasketLineNotFound),
        }
    }

    /// Rejoue toutes les lignes sur un panier vidé : chacune est jugée contre
    /// l'état accumulé des précédentes, et non contre l'état initial. Deux
    /// améliorations d'agilité depuis `2+` ne peuvent pas passer si la seconde
    /// franchit `1+`.
    ///
    /// **Tout ou rien** : une seule ligne refusée fait échouer l'ensemble.
    /// Appliquer les valides laisserait un panier partiellement consommé, dont
    /// le rechargement de la réponse effacerait la trace.
    pub fn validate_all(&self) -> Result<Vec<CustomisationLine>, Vec<RejectedLine>> {
        let mut rejoue = self.clone();
        rejoue.lines = Vec::new();

        let mut retenues = Vec::new();
        let mut refusees = Vec::new();

        for ligne in &self.lines {
            let issue = match ligne {
                CustomisationLine::Skill { skill_id, .. } => rejoue.add_skill(skill_id.clone()),
                CustomisationLine::Stat { stat, crans, .. } => rejoue.add_stat(*stat, *crans),
                CustomisationLine::Price { delta, .. } => rejoue.adjust_price(*delta),
                CustomisationLine::Spp { amount, .. } => rejoue.add_spp(*amount),
            };
            match issue {
                Ok(_) => retenues.push(ligne.clone()),
                Err(cause) => refusees.push(RejectedLine {
                    id: ligne.id().clone(),
                    cause,
                }),
            }
        }

        match refusees.is_empty() {
            true => Ok(retenues),
            false => Err(refusees),
        }
    }

    // ── Lecture, pour les view models ─────────────────────────────────────────

    /// La caractéristique telle qu'elle sera si le panier est validé.
    pub fn effective_stat(&self, stat: StatKind) -> u8 {
        let cumul: i16 = self
            .lines
            .iter()
            .filter_map(|l| match l {
                CustomisationLine::Stat { stat: s, crans, .. } if *s == stat => {
                    Some(crans.into_inner() as i16 * stat.improvement_step() as i16)
                }
                _ => None,
            })
            .sum();
        (self.base_stats.get(stat) as i16 + cumul).max(0) as u8
    }

    pub fn effective_value(&self) -> ValueKpo {
        let cumul: i64 = self
            .lines
            .iter()
            .filter_map(|l| match l {
                CustomisationLine::Price { delta, .. } => Some(delta.into_inner() as i64),
                _ => None,
            })
            .sum();
        ValueKpo((self.current_value.0 as i64 + cumul).max(0) as u32)
    }

    pub fn effective_spp(&self) -> Spp {
        let cumul: u32 = self
            .lines
            .iter()
            .filter_map(|l| match l {
                CustomisationLine::Spp { amount, .. } => Some(amount.into_inner() as u32),
                _ => None,
            })
            .sum();
        Spp(self.current_spp.0 + cumul)
    }

    /// Les compétences déjà possédées **ou** déjà au panier.
    pub fn pending_and_owned_skills(&self) -> Vec<SkillId> {
        let mut toutes = self.owned_skills.clone();
        toutes.extend(self.lines.iter().filter_map(|l| match l {
            CustomisationLine::Skill { skill_id, .. } => Some(skill_id.clone()),
            _ => None,
        }));
        toutes
    }

    pub fn action_for_stat(&self, stat: StatKind, sens: i8) -> ActionState {
        let crans = match StatCrans::try_new(sens) {
            Ok(c) => c,
            // Un sens nul n'est pas une action : rien à proposer.
            Err(_) => {
                return ActionState::Forbidden {
                    cause: DomainError::BasketLineNotFound,
                }
            }
        };
        match self.check_stat(stat, crans) {
            Ok(()) => ActionState::Allowed,
            // Une borne se libère si l'on retire une ligne du panier : c'est
            // conjoncturel, jamais structurel.
            Err(cause) => ActionState::Blocked { cause },
        }
    }

    pub fn action_for_skill(&self, skill_id: &SkillId) -> ActionState {
        match self.check_skill(skill_id) {
            Ok(()) => ActionState::Allowed,
            // Possédée de base ou acquise : vider le panier n'y changera rien.
            Err(DomainError::SkillAlreadyAcquired) if self.owned_skills.contains(skill_id) => {
                ActionState::Forbidden {
                    cause: DomainError::SkillAlreadyAcquired,
                }
            }
            Err(cause) => ActionState::Blocked { cause },
        }
    }

    // ── Gardes ────────────────────────────────────────────────────────────────

    fn check_skill(&self, skill_id: &SkillId) -> Result<(), DomainError> {
        if !self.catalog_skills.contains(skill_id) {
            return Err(DomainError::UnknownSkill);
        }
        match self.pending_and_owned_skills().contains(skill_id) {
            true => Err(DomainError::SkillAlreadyAcquired),
            false => Ok(()),
        }
    }

    /// La borne se juge sur l'état **effectif**, panier compris — c'est ce qui
    /// empêche deux améliorations successives de franchir la borne à deux.
    fn check_stat(&self, stat: StatKind, crans: StatCrans) -> Result<(), DomainError> {
        let courant = self.effective_stat(stat);
        match stat.apply_crans(courant, crans.into_inner()) {
            Some(_) => Ok(()),
            None => {
                let (min, max) = stat.bounds();
                let franchie = match crans.into_inner() * stat.improvement_step() > 0 {
                    true => max,
                    false => min,
                };
                Err(DomainError::StatOutOfBounds {
                    stat,
                    bound: franchie,
                })
            }
        }
    }

    /// Le plancher porte sur le **résultat**, pas sur le delta : un `-20` est
    /// licite ou non selon ce que le panier a déjà retiré.
    fn check_price(&self, delta: KpoDelta) -> Result<(), DomainError> {
        let resultat = self.effective_value().0 as i64 + delta.into_inner() as i64;
        match resultat >= 0 {
            true => Ok(()),
            false => Err(DomainError::NegativePlayerValue),
        }
    }

    /// Identifiant de ligne dérivé du rang : le panier vit peu, et un compteur
    /// suffit à distinguer ses lignes. Il meurt avec lui — l'identifiant durable
    /// d'une customisation appliquée est `CustomisationId`, posé à la validation.
    fn next_line_id(&self) -> BasketLineId {
        let mut rang = self.lines.len() + 1;
        loop {
            let candidat = BasketLineId::try_new(format!("l{rang}"))
                .expect("le format garantit une chaîne non vide");
            if !self.lines.iter().any(|l| *l.id() == candidat) {
                return candidat;
            }
            rang += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn competence(v: &str) -> SkillId {
        SkillId::try_new(v.to_string()).unwrap()
    }

    fn crans(v: i8) -> StatCrans {
        StatCrans::try_new(v).unwrap()
    }

    /// Base du poste : MV 7, FO 3, AG 3+, PA 5+, AR 8+.
    fn panier(possedees: Vec<&str>) -> CustomisationBasket {
        CustomisationBasket::hydrate(
            PlayerId("p1".into()),
            BasketVersion(1),
            vec![],
            ResolvedStats {
                ma: 7,
                st: 3,
                ag: 3,
                pa: 5,
                av: 8,
            },
            possedees.iter().map(|s| competence(s)).collect(),
            vec![
                competence("BLOCK"),
                competence("DODGE"),
                competence("GUARD"),
            ],
            ValueKpo(100),
            Spp(4),
        )
    }

    // ── Caractéristiques ──────────────────────────────────────────────────────

    /// L'invariant central : chaque ligne est jugée contre l'état accumulé des
    /// précédentes. AG part de 3+, deux améliorations la mènent à 1+, la
    /// troisième franchirait la borne.
    #[test]
    fn les_ameliorations_s_accumulent_jusqu_a_la_borne() {
        let mut p = panier(vec![]);
        assert!(p.add_stat(StatKind::Ag, crans(1)).is_ok());
        assert_eq!(p.effective_stat(StatKind::Ag), 2);
        assert!(p.add_stat(StatKind::Ag, crans(1)).is_ok());
        assert_eq!(p.effective_stat(StatKind::Ag), 1);

        assert_eq!(
            p.add_stat(StatKind::Ag, crans(1)).unwrap_err(),
            DomainError::StatOutOfBounds {
                stat: StatKind::Ag,
                bound: 1
            }
        );
    }

    /// Le pendant : une amplitude qui franchit la borne d'un coup est refusée
    /// aussi sûrement que deux crans successifs.
    #[test]
    fn une_amplitude_qui_franchit_la_borne_est_refusee() {
        let mut p = panier(vec![]);
        assert!(p.add_stat(StatKind::Ag, crans(2)).is_ok());
        let mut p = panier(vec![]);
        assert!(p.add_stat(StatKind::Ag, crans(3)).is_err());
    }

    #[test]
    fn la_degradation_a_sa_propre_borne() {
        let mut p = panier(vec![]);
        // AG 3+ dégradée de trois crans atteint 6+, la borne basse de qualité.
        assert!(p.add_stat(StatKind::Ag, crans(-3)).is_ok());
        assert_eq!(p.effective_stat(StatKind::Ag), 6);
        assert!(p.add_stat(StatKind::Ag, crans(-1)).is_err());
    }

    /// Retirer une ligne rouvre la marge : c'est ce qui rend la borne
    /// conjoncturelle, et non structurelle.
    #[test]
    fn retirer_une_ligne_libere_la_borne() {
        let mut p = panier(vec![]);
        let l1 = p.add_stat(StatKind::Ag, crans(2)).unwrap();
        assert!(p.add_stat(StatKind::Ag, crans(1)).is_err());

        p.remove_line(&l1).unwrap();
        assert_eq!(p.effective_stat(StatKind::Ag), 3);
        assert!(p.add_stat(StatKind::Ag, crans(1)).is_ok());
    }

    #[test]
    fn l_armure_monte_quand_on_l_ameliore() {
        let mut p = panier(vec![]);
        p.add_stat(StatKind::Av, crans(1)).unwrap();
        assert_eq!(p.effective_stat(StatKind::Av), 9);
    }

    // ── Compétences ───────────────────────────────────────────────────────────

    #[test]
    fn une_competence_inconnue_du_catalogue_est_refusee() {
        let mut p = panier(vec![]);
        assert_eq!(
            p.add_skill(competence("INCONNUE")).unwrap_err(),
            DomainError::UnknownSkill
        );
    }

    #[test]
    fn une_competence_deja_au_panier_est_refusee_en_doublon() {
        let mut p = panier(vec![]);
        p.add_skill(competence("BLOCK")).unwrap();
        assert_eq!(
            p.add_skill(competence("BLOCK")).unwrap_err(),
            DomainError::SkillAlreadyAcquired
        );
    }

    #[test]
    fn une_competence_deja_possedee_est_refusee() {
        let mut p = panier(vec!["BLOCK"]);
        assert_eq!(
            p.add_skill(competence("BLOCK")).unwrap_err(),
            DomainError::SkillAlreadyAcquired
        );
    }

    /// La distinction qui pilote le grisage : possédée = `Forbidden`, vider le
    /// panier n'y changera rien ; au panier = `Blocked`, retirer la ligne
    /// rouvre.
    #[test]
    fn possedee_est_interdite_au_panier_est_bloquee() {
        let p = panier(vec!["BLOCK"]);
        assert!(matches!(
            p.action_for_skill(&competence("BLOCK")),
            ActionState::Forbidden { .. }
        ));

        let mut p = panier(vec![]);
        p.add_skill(competence("DODGE")).unwrap();
        assert!(matches!(
            p.action_for_skill(&competence("DODGE")),
            ActionState::Blocked { .. }
        ));

        assert!(matches!(
            p.action_for_skill(&competence("GUARD")),
            ActionState::Allowed
        ));
    }

    // ── Prix et SPP ───────────────────────────────────────────────────────────

    /// Le plancher porte sur le résultat : deux retraits licites séparément
    /// peuvent être refusés ensemble.
    #[test]
    fn le_plancher_de_prix_se_juge_sur_le_cumul() {
        let mut p = panier(vec![]);
        assert!(p.adjust_price(KpoDelta::try_new(-60).unwrap()).is_ok());
        assert_eq!(p.effective_value().0, 40);
        assert_eq!(
            p.adjust_price(KpoDelta::try_new(-60).unwrap()).unwrap_err(),
            DomainError::NegativePlayerValue
        );
        assert!(p.adjust_price(KpoDelta::try_new(-40).unwrap()).is_ok());
        assert_eq!(p.effective_value().0, 0);
    }

    #[test]
    fn les_spp_s_accumulent_sans_plafond_de_total() {
        let mut p = panier(vec![]);
        p.add_spp(SppAmount::try_new(100).unwrap()).unwrap();
        p.add_spp(SppAmount::try_new(100).unwrap()).unwrap();
        // Le plafond de 100 est **par opération** : le total n'est pas borné.
        assert_eq!(p.effective_spp().0, 204);
    }

    // ── Retrait et validation ────────────────────────────────────────────────

    #[test]
    fn retirer_une_ligne_absente_est_une_erreur() {
        let mut p = panier(vec![]);
        assert_eq!(
            p.remove_line(&BasketLineId::try_new("l9".to_string()).unwrap())
                .unwrap_err(),
            DomainError::BasketLineNotFound
        );
    }

    #[test]
    fn validate_all_retient_tout_quand_tout_est_valide() {
        let mut p = panier(vec![]);
        p.add_skill(competence("BLOCK")).unwrap();
        p.add_stat(StatKind::Ma, crans(1)).unwrap();
        p.adjust_price(KpoDelta::try_new(-10).unwrap()).unwrap();
        p.add_spp(SppAmount::try_new(5).unwrap()).unwrap();

        assert_eq!(p.validate_all().unwrap().len(), 4);
    }

    /// Tout ou rien : une ligne devenue invalide fait échouer l'ensemble, même
    /// si les autres passeraient.
    #[test]
    fn validate_all_rejette_tout_si_une_ligne_tombe() {
        let mut p = panier(vec![]);
        p.add_skill(competence("BLOCK")).unwrap();
        p.add_stat(StatKind::Ag, crans(2)).unwrap();

        // Le joueur acquiert BLOCK entre-temps, par une autre voie.
        let devenu_invalide = CustomisationBasket::hydrate(
            p.player_id().clone(),
            p.version(),
            p.lines().to_vec(),
            ResolvedStats {
                ma: 7,
                st: 3,
                ag: 3,
                pa: 5,
                av: 8,
            },
            vec![competence("BLOCK")],
            vec![competence("BLOCK"), competence("DODGE")],
            ValueKpo(100),
            Spp(4),
        );

        let refusees = devenu_invalide.validate_all().unwrap_err();
        assert_eq!(refusees.len(), 1);
        assert_eq!(refusees[0].cause, DomainError::SkillAlreadyAcquired);
    }

    /// Le rejeu doit reproduire l'accumulation : deux améliorations d'AG
    /// validées ensemble restent valides, une troisième non.
    #[test]
    fn validate_all_rejoue_l_accumulation() {
        let mut p = panier(vec![]);
        p.add_stat(StatKind::Ag, crans(1)).unwrap();
        p.add_stat(StatKind::Ag, crans(1)).unwrap();
        assert!(p.validate_all().is_ok());

        // Trois crans cumulés franchiraient 1+ — refusé à l'ajout comme au rejeu.
        assert!(p.add_stat(StatKind::Ag, crans(1)).is_err());
    }

    #[test]
    fn les_identifiants_de_ligne_sont_uniques() {
        let mut p = panier(vec![]);
        let a = p.add_stat(StatKind::Ma, crans(1)).unwrap();
        let b = p.add_stat(StatKind::St, crans(1)).unwrap();
        p.remove_line(&a).unwrap();
        let c = p.add_stat(StatKind::Pa, crans(1)).unwrap();

        assert_ne!(
            b, c,
            "un identifiant réattribué écraserait une ligne vivante"
        );
    }
}

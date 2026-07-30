//! Le panier de renvois — un agrégat, pas un objet applicatif.
//!
//! Il ne porte qu'une règle, mais elle est subtile : on ne descend pas sous
//! onze joueurs **éligibles au prochain match**. Toutes les gardes de
//! composition du recrutement — plafond de 16, quota par poste, limites
//! croisées — sont ici sans objet : retirer ne peut violer aucune borne haute.
//!
//! **Pas de trésorerie** : un renvoi ne rembourse rien, l'agrégat n'a aucune
//! raison de la connaître.
//!
//! Même résolution de la tension « le domaine n'appelle pas de port » que le
//! panier de recrutement : l'agrégat *porte* l'effectif et le catalogue, hydratés
//! par le use case. Tout est ensuite pur et synchrone.

use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
use crate::app::teams::domain::basket::{
    BasketLineId, BasketVersion, OwnedStaff, Player, RejectedLine, RosterCatalog, Squad,
};
use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::value_objects::{Kpo, StaffType};

/// Onze joueurs sur le terrain : c'est le plancher, et la seule vraie règle de
/// cette phase.
const MIN_ELIGIBLE: usize = 11;

// ── Lignes du panier ──────────────────────────────────────────────────────────

/// Le **seul** état persisté. Effectif, catalogue et staff possédé sont
/// rechargés à chaque hydratation — un panier vieux de dix minutes est évalué
/// contre l'effectif d'aujourd'hui.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DismissalBasketLine {
    Player {
        id: BasketLineId,
        player_id: PlayerId,
    },
    Staff {
        id: BasketLineId,
        staff_type: StaffType,
    },
}

impl DismissalBasketLine {
    pub fn id(&self) -> &BasketLineId {
        match self {
            Self::Player { id, .. } | Self::Staff { id, .. } => id,
        }
    }
}

/// Ce qu'une ligne validée demande d'appliquer. Le panier ne construit pas
/// d'événement — c'est le use case (carte 268) qui le fait.
///
/// `value_at_dismissal` vient de l'effectif hydraté. Elle ne sert à aucun
/// calcul : elle documente ce que valait le joueur au moment du renvoi,
/// information que plus rien ne permettra de reconstituer une fois qu'il aura
/// quitté l'effectif.
#[derive(Debug, Clone, PartialEq)]
pub enum DismissalAppliedLine {
    Player {
        player_id: PlayerId,
        value_at_dismissal: Kpo,
    },
    Staff {
        staff_type: StaffType,
    },
}

// ── État d'une action, décidé par le domaine ──────────────────────────────────

/// Trois cas là où le recrutement en a trois autres.
///
/// `Marked` n'a pas d'équivalent au recrutement : une ligne s'annule ici
/// **depuis la ligne du joueur**, pas seulement depuis le panier. Le coach voit
/// le joueur marqué à sa place dans l'effectif, et l'y démarque.
///
/// Pas de `Forbidden` : aucun blocage de cette phase n'est définitif. Le
/// plancher se desserre dès qu'on démarque.
#[derive(Debug, Clone, PartialEq)]
pub enum DismissalActionState {
    Removable,
    Marked,
    Blocked { cause: DomainError },
}

// ── L'agrégat ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DismissalsBasket {
    team_id: String,
    version: BasketVersion,
    lines: Vec<DismissalBasketLine>,
    squad: Squad,
    catalog: RosterCatalog,
    owned_staff: OwnedStaff,
}

impl DismissalsBasket {
    pub fn hydrate(
        team_id: String,
        version: BasketVersion,
        lines: Vec<DismissalBasketLine>,
        squad: Squad,
        catalog: RosterCatalog,
        owned_staff: OwnedStaff,
    ) -> Self {
        Self {
            team_id,
            version,
            lines,
            squad,
            catalog,
            owned_staff,
        }
    }

    pub fn team_id(&self) -> &str {
        &self.team_id
    }
    pub fn version(&self) -> BasketVersion {
        self.version
    }
    pub fn lines(&self) -> &[DismissalBasketLine] {
        &self.lines
    }

    // ── Commandes ─────────────────────────────────────────────────────────

    pub fn mark_player(&mut self, player_id: PlayerId) -> Result<BasketLineId, DomainError> {
        self.check_not_already_marked(&player_id)?;
        self.check_eligible_floor(&player_id)?;

        let id = BasketLineId(ulid::Ulid::new().to_string());
        self.lines.push(DismissalBasketLine::Player {
            id: id.clone(),
            player_id,
        });
        Ok(id)
    }

    /// Le staff n'a pas de garde de doublon : marquer deux assistants sur trois
    /// est légitime, chaque ligne vaut une unité.
    pub fn mark_staff(&mut self, staff: StaffType) -> Result<BasketLineId, DomainError> {
        self.check_staff_owned(staff)?;

        let id = BasketLineId(ulid::Ulid::new().to_string());
        self.lines.push(DismissalBasketLine::Staff {
            id: id.clone(),
            staff_type: staff,
        });
        Ok(id)
    }

    /// Démarquer rend un éligible au compte : le plancher se desserre, et un
    /// marquage refusé l'instant d'avant redevient possible. Les gardes comptant
    /// les lignes en attente, il suffit d'enlever la ligne.
    pub fn remove_line(&mut self, id: &BasketLineId) -> Result<(), DomainError> {
        let avant = self.lines.len();
        self.lines.retain(|l| l.id() != id);
        if self.lines.len() == avant {
            return Err(DomainError::BasketLineNotFound);
        }
        Ok(())
    }

    /// Revalide **tout** le panier contre l'état du jour. Refus en bloc : une
    /// seule ligne fautive et rien n'est appliqué.
    ///
    /// La revalidation rejoue les lignes une à une dans un panier vidé, ce qui
    /// garantit qu'elle applique exactement les mêmes gardes que le marquage —
    /// et notamment que le plancher est évalué **en cumul**, pas ligne par
    /// ligne. Un joueur parti de l'effectif entre-temps fait échouer le lot
    /// entier, ce qui est le comportement voulu : le coach revoit un écran vrai.
    pub fn validate_all(&self) -> Result<Vec<DismissalAppliedLine>, Vec<RejectedLine>> {
        let mut rejouee = self.clone();
        rejouee.lines = Vec::new();

        let mut applied = Vec::new();
        let mut rejected = Vec::new();

        for ligne in &self.lines {
            let issue = match ligne {
                DismissalBasketLine::Player { player_id, .. } => rejouee
                    .mark_player(player_id.clone())
                    .map(|_| DismissalAppliedLine::Player {
                        player_id: player_id.clone(),
                        value_at_dismissal: rejouee.value_of(player_id),
                    }),
                DismissalBasketLine::Staff { staff_type, .. } => rejouee
                    .mark_staff(*staff_type)
                    .map(|_| DismissalAppliedLine::Staff {
                        staff_type: *staff_type,
                    }),
            };
            match issue {
                Ok(a) => applied.push(a),
                Err(cause) => rejected.push(RejectedLine {
                    id: ligne.id().clone(),
                    cause,
                }),
            }
        }

        if rejected.is_empty() {
            Ok(applied)
        } else {
            Err(rejected)
        }
    }

    // ── Lecture, pour les view models ─────────────────────────────────────

    pub fn action_for_player(&self, player_id: &PlayerId) -> DismissalActionState {
        if self.is_marked(player_id) {
            return DismissalActionState::Marked;
        }
        match self.check_eligible_floor(player_id) {
            Ok(()) => DismissalActionState::Removable,
            Err(cause) => DismissalActionState::Blocked { cause },
        }
    }

    pub fn action_for_staff(&self, staff: StaffType) -> DismissalActionState {
        match self.check_staff_owned(staff) {
            Ok(()) => DismissalActionState::Removable,
            Err(cause) => DismissalActionState::Blocked { cause },
        }
    }

    pub fn squad(&self) -> &Squad {
        &self.squad
    }

    pub fn catalog(&self) -> &RosterCatalog {
        &self.catalog
    }

    pub fn owned_staff(&self) -> OwnedStaff {
        self.owned_staff
    }

    /// Les éligibles qu'il resterait si le panier était appliqué. C'est ce
    /// nombre, et non l'effectif possédé, que le plancher regarde — sans quoi
    /// marquer dix joueurs d'un coup passerait.
    pub fn eligible_after_basket(&self) -> usize {
        self.squad
            .eligible_count()
            .saturating_sub(self.pending_eligible_players())
    }

    pub fn is_marked(&self, player_id: &PlayerId) -> bool {
        self.lines.iter().any(
            |l| matches!(l, DismissalBasketLine::Player { player_id: p, .. } if p == player_id),
        )
    }

    pub fn pending_staff_count(&self, staff: StaffType) -> u32 {
        self.lines
            .iter()
            .filter(|l| matches!(l, DismissalBasketLine::Staff { staff_type, .. } if *staff_type == staff))
            .count() as u32
    }

    // ── Comptages ─────────────────────────────────────────────────────────

    /// Seuls les marqués **disponibles** entament le plancher : renvoyer un
    /// absent ne retire personne au prochain match.
    fn pending_eligible_players(&self) -> usize {
        self.lines
            .iter()
            .filter_map(|l| match l {
                DismissalBasketLine::Player { player_id, .. } => self.squad.find(player_id),
                DismissalBasketLine::Staff { .. } => None,
            })
            .filter(|p| p.available_for_next_match)
            .count()
    }

    fn value_of(&self, player_id: &PlayerId) -> Kpo {
        self.squad
            .find(player_id)
            .map(|p: &Player| p.value_kpo)
            .unwrap_or(Kpo(0))
    }

    // ── Les deux gardes ───────────────────────────────────────────────────

    fn check_not_already_marked(&self, player_id: &PlayerId) -> Result<(), DomainError> {
        if self.is_marked(player_id) {
            return Err(DomainError::PlayerAlreadyMarked);
        }
        Ok(())
    }

    /// Le plancher des onze éligibles.
    ///
    /// Un joueur absent au prochain match ne compte pas parmi les éligibles :
    /// le renvoyer n'entame pas le plancher, et reste donc toujours possible,
    /// quel que soit l'état de l'effectif.
    fn check_eligible_floor(&self, player_id: &PlayerId) -> Result<(), DomainError> {
        let player = self
            .squad
            .find(player_id)
            .ok_or(DomainError::PlayerNotInSquad)?;
        if !player.available_for_next_match {
            return Ok(());
        }
        if self.eligible_after_basket() <= MIN_ELIGIBLE {
            return Err(DomainError::EligibleFloorReached);
        }
        Ok(())
    }

    /// Ne pas marquer plus que ce que l'équipe possède, lignes déjà en attente
    /// comprises.
    ///
    /// Le facteur fans y est bloqué sans garde supplémentaire : `count_of` le
    /// rend à zéro, parce qu'il ne se renvoie pas plus qu'il ne s'achète.
    fn check_staff_owned(&self, staff: StaffType) -> Result<(), DomainError> {
        if self.pending_staff_count(staff) >= self.owned_staff.count_of(staff) {
            return Err(DomainError::InsufficientStaff);
        }
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::teams::domain::basket::RosterLineId;

    fn catalogue_vide() -> RosterCatalog {
        RosterCatalog {
            positions: Vec::new(),
            cross_limits: Vec::new(),
            allowed_staff: Vec::new(),
            staff: Vec::new(),
            reroll_base_cost: Kpo(60),
        }
    }

    /// Les identifiants sont fixés et non tirés au hasard : un test qui échoue
    /// doit nommer le même joueur à chaque exécution. Le rembourrage à gauche
    /// tient sur les vingt-six caractères d'un ULID quel que soit le rang —
    /// écrire les zéros à la main ne débordait qu'à partir du dixième joueur,
    /// et seulement à l'exécution.
    fn id_de(n: u8) -> PlayerId {
        PlayerId::try_new(&format!("{n:0>26}")).unwrap()
    }

    fn joueur(n: u8, disponible: bool) -> Player {
        Player {
            player_id: id_de(n),
            roster_line: RosterLineId("DEMO_GRANIT__PIETAILLE".into()),
            personal_name: format!("Joueur {n}"),
            position_name: "Piétaille des Carrières".into(),
            spp: 0,
            value_kpo: Kpo(50),
            available_for_next_match: disponible,
        }
    }

    /// `disponibles` joueurs éligibles, puis `absents` indisponibles.
    fn panier(disponibles: u8, absents: u8) -> DismissalsBasket {
        let mut membres = Vec::new();
        for n in 0..disponibles {
            membres.push(joueur(n, true));
        }
        for n in disponibles..disponibles + absents {
            membres.push(joueur(n, false));
        }
        DismissalsBasket::hydrate(
            "team".into(),
            BasketVersion(0),
            Vec::new(),
            Squad { members: membres },
            catalogue_vide(),
            OwnedStaff::default(),
        )
    }

    fn panier_staff(owned: OwnedStaff) -> DismissalsBasket {
        DismissalsBasket::hydrate(
            "team".into(),
            BasketVersion(0),
            Vec::new(),
            Squad::default(),
            catalogue_vide(),
            owned,
        )
    }

    // ── Le plancher ───────────────────────────────────────────────────────

    #[test]
    fn douze_eligibles_marquer_un_disponible_passe() {
        let mut p = panier(12, 0);
        assert!(p.mark_player(id_de(0)).is_ok());
    }

    #[test]
    fn onze_eligibles_marquer_un_disponible_refuse() {
        let mut p = panier(11, 0);
        assert_eq!(
            p.mark_player(id_de(0)),
            Err(DomainError::EligibleFloorReached)
        );
    }

    /// Le plancher se resserre à chaque marquage : le premier passe parce qu'il
    /// resterait douze éligibles, le second échoue parce qu'il n'en resterait
    /// plus que onze. C'est l'interaction entre le plancher et le contenu du
    /// panier, seule vraie subtilité de cette phase.
    #[test]
    fn douze_eligibles_un_marque_le_second_refuse() {
        let mut p = panier(12, 0);
        p.mark_player(id_de(0)).unwrap();
        assert_eq!(p.eligible_after_basket(), 11);
        assert_eq!(
            p.mark_player(id_de(1)),
            Err(DomainError::EligibleFloorReached)
        );
    }

    /// Un absent ne compte pas parmi les éligibles : le renvoyer n'entame pas le
    /// plancher, même très en dessous.
    #[test]
    fn neuf_eligibles_marquer_un_absent_passe() {
        let mut p = panier(9, 3);
        assert!(p.mark_player(id_de(10)).is_ok());
        assert_eq!(
            p.eligible_after_basket(),
            9,
            "un absent ne change pas le compte"
        );
    }

    #[test]
    fn neuf_eligibles_marquer_un_disponible_refuse() {
        let mut p = panier(9, 3);
        assert_eq!(
            p.mark_player(id_de(0)),
            Err(DomainError::EligibleFloorReached)
        );
    }

    /// Démarquer rend un éligible et rouvre le marquage — la propriété qui rend
    /// le panier réversible tant que la phase n'est pas validée.
    #[test]
    fn demarquer_rend_un_eligible_et_rouvre_le_marquage() {
        let mut p = panier(12, 0);
        let ligne = p.mark_player(id_de(0)).unwrap();
        assert_eq!(
            p.mark_player(id_de(1)),
            Err(DomainError::EligibleFloorReached)
        );

        p.remove_line(&ligne).unwrap();

        assert_eq!(p.eligible_after_basket(), 12);
        assert!(p.mark_player(id_de(1)).is_ok());
    }

    // ── Le staff ──────────────────────────────────────────────────────────

    #[test]
    fn marquer_plus_de_staff_que_possede_refuse() {
        let mut p = panier_staff(OwnedStaff::default());
        assert_eq!(
            p.mark_staff(StaffType::Assistant),
            Err(DomainError::InsufficientStaff)
        );
    }

    #[test]
    fn staff_possede_deux_le_second_passe_le_troisieme_refuse() {
        let mut p = panier_staff(OwnedStaff {
            assistants: 2,
            ..OwnedStaff::default()
        });
        assert!(p.mark_staff(StaffType::Assistant).is_ok());
        assert!(p.mark_staff(StaffType::Assistant).is_ok());
        assert_eq!(
            p.mark_staff(StaffType::Assistant),
            Err(DomainError::InsufficientStaff)
        );
    }

    // ── Cas d'erreur et lot ───────────────────────────────────────────────

    #[test]
    fn joueur_absent_de_l_effectif_retourne_player_not_in_squad() {
        let mut p = panier(12, 0);
        // Le rang 99 n'est jamais peuplé : un joueur d'une autre équipe, ou un
        // identifiant forgé.
        assert_eq!(p.mark_player(id_de(99)), Err(DomainError::PlayerNotInSquad));
    }

    /// Sans cette garde, la seconde ligne compterait deux fois dans le plancher
    /// et le lot émettrait deux renvois pour un même joueur.
    #[test]
    fn marquer_deux_fois_le_meme_joueur_refuse() {
        let mut p = panier(14, 0);
        p.mark_player(id_de(0)).unwrap();
        assert_eq!(
            p.mark_player(id_de(0)),
            Err(DomainError::PlayerAlreadyMarked)
        );
    }

    /// Refus en bloc : le joueur disparu de l'effectif fait tomber le lot
    /// entier, y compris la ligne de staff qui, seule, serait passée.
    ///
    /// Le panier est rehydraté avec un effectif amputé, exactement ce qui se
    /// produit quand le joueur est renvoyé depuis un autre onglet entre le
    /// marquage et la validation.
    #[test]
    fn validate_all_une_ligne_invalide_n_applique_rien() {
        let assistant = OwnedStaff {
            assistants: 1,
            ..OwnedStaff::default()
        };
        let mut p = DismissalsBasket::hydrate(
            "team".into(),
            BasketVersion(0),
            Vec::new(),
            panier(14, 0).squad().clone(),
            catalogue_vide(),
            assistant,
        );
        p.mark_player(id_de(0)).unwrap();
        p.mark_staff(StaffType::Assistant).unwrap();
        assert!(p.validate_all().is_ok(), "les deux lignes sont valides");

        let ampute = Squad {
            members: p
                .squad()
                .members
                .iter()
                .filter(|m| m.player_id != id_de(0))
                .cloned()
                .collect(),
        };
        let p = DismissalsBasket::hydrate(
            "team".into(),
            BasketVersion(0),
            p.lines().to_vec(),
            ampute,
            catalogue_vide(),
            assistant,
        );

        let refus = p.validate_all().expect_err("le lot doit être refusé");
        assert_eq!(refus.len(), 1, "seule la ligne du joueur est fautive");
        assert_eq!(refus[0].cause, DomainError::PlayerNotInSquad);
    }

    #[test]
    fn panier_vide_donne_un_lot_vide_sans_erreur() {
        let p = panier(12, 0);
        assert_eq!(p.validate_all(), Ok(Vec::new()));
    }

    // ── L'état d'action, tel que la vue le lira ───────────────────────────

    #[test]
    fn action_pour_un_joueur_marque_est_marked() {
        let mut p = panier(14, 0);
        p.mark_player(id_de(0)).unwrap();
        assert_eq!(p.action_for_player(&id_de(0)), DismissalActionState::Marked);
        assert_eq!(
            p.action_for_player(&id_de(1)),
            DismissalActionState::Removable
        );
    }

    #[test]
    fn action_pour_un_joueur_au_plancher_est_bloquee() {
        let p = panier(11, 0);
        assert_eq!(
            p.action_for_player(&id_de(0)),
            DismissalActionState::Blocked {
                cause: DomainError::EligibleFloorReached
            }
        );
    }

    /// Le lot porte la valeur du joueur au moment du renvoi — elle ne sert à
    /// aucun calcul, mais rien ne permettrait de la retrouver ensuite.
    #[test]
    fn le_lot_porte_la_valeur_au_moment_du_renvoi() {
        let mut p = panier(14, 0);
        p.mark_player(id_de(0)).unwrap();
        assert_eq!(
            p.validate_all().unwrap(),
            vec![DismissalAppliedLine::Player {
                player_id: id_de(0),
                value_at_dismissal: Kpo(50),
            }]
        );
    }
}

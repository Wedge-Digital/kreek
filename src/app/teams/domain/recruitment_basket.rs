//! Le panier de recrutement — un agrégat, pas un objet applicatif.
//!
//! Il porte des invariants forts : plafond d'effectif, quota par poste, limites
//! croisées, trésorerie. La tension « le domaine n'appelle pas de port » se
//! résout par **hydratation** : l'agrégat *porte* les données dont ses gardes
//! ont besoin, comme `RosterSelectedTeam` porte son `Roster`. Le use case
//! hydrate, puis tout est **pur et synchrone** — aucun `async`, aucun port,
//! aucune dépendance framework.

use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
use crate::app::teams::domain::basket::{
    staff_uid, BasketLineId, BasketVersion, OwnedStaff, Player, RejectedLine, RosterCatalog,
    RosterLineId, Squad,
};
use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::value_objects::{Kpo, StaffType};

/// Effectif complet d'une équipe.
const MAX_SQUAD: usize = 16;

/// Quota de relances. Il n'est pas dans le corpus de référence — `staff_fr.json`
/// ne connaît que l'apothicaire, les meneuses, les assistants et le facteur
/// fans. La relance est tarifée par `reroll_base_cost` du roster et plafonnée
/// par cette constante.
const MAX_REROLLS: u32 = 8;

// ── Lignes du panier ──────────────────────────────────────────────────────────

/// Le **seul** état persisté du panier. Catalogue, effectif et trésorerie sont
/// rechargés à chaque hydratation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BasketLine {
    Player {
        id: BasketLineId,
        roster_line: RosterLineId,
        price: Kpo,
    },
    Staff {
        id: BasketLineId,
        staff_type: StaffType,
        price: Kpo,
    },
    /// Un journalier que le coach garde.
    ///
    /// Il porte le `player_id` et non une ligne de roster : le joueur existe
    /// déjà, on désigne **un homme**, pas un poste à pourvoir.
    Journeyman {
        id: BasketLineId,
        player_id: PlayerId,
        price: Kpo,
    },
}

impl BasketLine {
    pub fn id(&self) -> &BasketLineId {
        match self {
            Self::Player { id, .. } | Self::Staff { id, .. } | Self::Journeyman { id, .. } => id,
        }
    }

    fn price(&self) -> Kpo {
        match self {
            Self::Player { price, .. }
            | Self::Staff { price, .. }
            | Self::Journeyman { price, .. } => *price,
        }
    }
}

/// Le prix d'un journalier tel que le panier vient de l'inscrire.
///
/// Lu sur la ligne que `add_journeyman` a poussée, et non recalculé : les deux
/// doivent dire la même chose, et une seconde source les laisserait diverger.
fn prix_du_journalier(basket: &RecruitmentBasket, player_id: &PlayerId) -> Kpo {
    basket
        .lines
        .iter()
        .find_map(|l| match l {
            BasketLine::Journeyman {
                player_id: p,
                price,
                ..
            } if p == player_id => Some(*price),
            _ => None,
        })
        .unwrap_or(Kpo(0))
}

/// Ce qu'une ligne validée demande d'appliquer. Le panier ne construit pas
/// d'événement — c'est le use case (carte 263) qui le fait.
#[derive(Debug, Clone, PartialEq)]
pub enum AppliedLine {
    Player {
        roster_line: RosterLineId,
        base_value: Kpo,
        cost: Kpo,
    },
    Staff {
        staff_type: StaffType,
        cost: Kpo,
    },
    /// **Pas de `base_value`**, contrairement à `Player` : pour un journalier le
    /// prix **est** sa valeur courante. Un second champ qui la duplique
    /// inviterait à les faire diverger.
    Journeyman {
        player_id: PlayerId,
        cost: Kpo,
    },
}

// ── État d'une action, décidé par le domaine ──────────────────────────────────

/// `Blocked` et `Forbidden` sont distincts parce qu'un quota se libère et qu'un
/// roster n'acquiert jamais le droit à un apothicaire. C'est **le domaine** qui
/// décide de la cause ; la couche web ne fait que la formuler.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionState {
    Allowed,
    Blocked { cause: DomainError },
    Forbidden { cause: DomainError },
}

// ── L'agrégat ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RecruitmentBasket {
    team_id: String,
    version: BasketVersion,
    lines: Vec<BasketLine>,
    catalog: RosterCatalog,
    squad: Squad,
    owned_staff: OwnedStaff,
    treasury: Kpo,
}

/// Un journalier que le coach peut garder, et son prix.
///
/// **Juste ce qu'il faut pour valider.** Le nom, les SPP et l'amélioration
/// gagnée sont de l'affichage, et l'écran les lira ailleurs : les faire entrer
/// dans le domaine lui donnerait des champs dont aucune règle ne dépend.
#[derive(Debug, Clone, PartialEq)]
pub struct HireableJourneyman {
    pub player_id: PlayerId,
    pub price: Kpo,
}

impl RecruitmentBasket {
    pub fn hydrate(
        team_id: String,
        version: BasketVersion,
        lines: Vec<BasketLine>,
        catalog: RosterCatalog,
        squad: Squad,
        owned_staff: OwnedStaff,
        treasury: Kpo,
    ) -> Self {
        Self {
            team_id,
            version,
            lines,
            catalog,
            squad,
            owned_staff,
            treasury,
        }
    }

    pub fn team_id(&self) -> &str {
        &self.team_id
    }
    pub fn version(&self) -> BasketVersion {
        self.version
    }
    pub fn lines(&self) -> &[BasketLine] {
        &self.lines
    }

    // ── Commandes ─────────────────────────────────────────────────────────

    pub fn add_player(&mut self, line: RosterLineId) -> Result<BasketLineId, DomainError> {
        self.check_position_in_roster(&line)?;
        self.check_squad_max()?;
        self.check_position_quota(&line)?;
        self.check_cross_limits(&line)?;
        let price = self.price_of_position(&line);
        self.check_treasury(price)?;

        let id = BasketLineId(ulid::Ulid::new().to_string());
        self.lines.push(BasketLine::Player {
            id: id.clone(),
            roster_line: line,
            price,
        });
        Ok(id)
    }

    pub fn add_staff(&mut self, staff: StaffType) -> Result<BasketLineId, DomainError> {
        self.check_staff_buyable(staff)?;
        self.check_staff_allowed(staff)?;
        self.check_staff_quota(staff)?;
        let price = self.price_for(staff);
        self.check_treasury(price)?;

        let id = BasketLineId(ulid::Ulid::new().to_string());
        self.lines.push(BasketLine::Staff {
            id: id.clone(),
            staff_type: staff,
            price,
        });
        Ok(id)
    }

    /// Retirer une ligne libère son quota **et** sa part de trésorerie : les
    /// gardes comptant les lignes en attente, il suffit de l'enlever.
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
    /// garantit qu'elle applique exactement les mêmes gardes que l'ajout — et
    /// notamment que la trésorerie est vérifiée **en cumul**, pas ligne par
    /// ligne.
    /// Garde un journalier de l'effectif.
    ///
    /// **Trois gardes, dans cet ordre.** Le doublon d'abord : c'est la règle
    /// propre au journalier, et la moins coûteuse à vérifier. Puis la
    /// disponibilité, puis le plafond et la trésorerie, communes aux achats.
    ///
    /// **Un journalier ne s'ajoute pas deux fois**, et c'est une règle que les
    /// postes n'ont pas : un poste est un **type** — deux Trois-quarts sont
    /// deux joueurs — un journalier est **un homme**, et il n'y en a qu'un.
    ///
    /// Le garde-fou reste **pur** : le panier compare son contenu à l'effectif
    /// qu'on lui a donné à l'hydratation, il n'interroge rien.
    pub fn add_journeyman(&mut self, player_id: PlayerId) -> Result<BasketLineId, DomainError> {
        if self.journeyman_in_basket(&player_id) {
            return Err(DomainError::JourneymanAlreadyInBasket);
        }
        let Some(journalier) = self.squad.journeymen().find(|j| j.player_id == player_id) else {
            return Err(DomainError::JourneymanNoLongerAvailable);
        };
        let price = journalier.value_kpo;

        self.check_squad_max()?;
        self.check_treasury(price)?;

        let id = BasketLineId(ulid::Ulid::new().to_string());
        self.lines.push(BasketLine::Journeyman {
            id: id.clone(),
            player_id,
            price,
        });
        Ok(id)
    }

    pub fn validate_all(&self) -> Result<Vec<AppliedLine>, Vec<RejectedLine>> {
        let mut rejouee = self.clone();
        rejouee.lines = Vec::new();

        let mut applied = Vec::new();
        let mut rejected = Vec::new();

        for ligne in &self.lines {
            let issue = match ligne {
                BasketLine::Player { roster_line, .. } => rejouee
                    .add_player(roster_line.clone())
                    .map(|_| AppliedLine::Player {
                        roster_line: roster_line.clone(),
                        base_value: rejouee.price_of_position(roster_line),
                        cost: rejouee.price_of_position(roster_line),
                    }),
                BasketLine::Staff { staff_type, .. } => {
                    rejouee.add_staff(*staff_type).map(|_| AppliedLine::Staff {
                        staff_type: *staff_type,
                        cost: rejouee.price_for(*staff_type),
                    })
                }
                // Le prix est **relu sur l'effectif du jour**, pas repris de la
                // ligne : c'est ce qui fait tomber le journalier dont la valeur
                // a bougé depuis l'ajout au même endroit que les autres refus.
                BasketLine::Journeyman { player_id, .. } => rejouee
                    .add_journeyman(player_id.clone())
                    .map(|_| AppliedLine::Journeyman {
                        player_id: player_id.clone(),
                        cost: prix_du_journalier(&rejouee, player_id),
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

    pub fn action_for_position(&self, line: &RosterLineId) -> ActionState {
        if let Err(cause) = self.check_position_in_roster(line) {
            return ActionState::Forbidden { cause };
        }
        for garde in [
            self.check_squad_max(),
            self.check_position_quota(line),
            self.check_cross_limits(line),
            self.check_treasury(self.price_of_position(line)),
        ] {
            if let Err(cause) = garde {
                return ActionState::Blocked { cause };
            }
        }
        ActionState::Allowed
    }

    pub fn action_for_staff(&self, staff: StaffType) -> ActionState {
        // Un type non achetable ou refusé au roster ne le deviendra jamais.
        for interdit in [
            self.check_staff_buyable(staff),
            self.check_staff_allowed(staff),
        ] {
            if let Err(cause) = interdit {
                return ActionState::Forbidden { cause };
            }
        }
        for garde in [
            self.check_staff_quota(staff),
            self.check_treasury(self.price_for(staff)),
        ] {
            if let Err(cause) = garde {
                return ActionState::Blocked { cause };
            }
        }
        ActionState::Allowed
    }

    /// L'effectif **permanent** projeté — celui qui décide du plafond de seize.
    ///
    /// Les journaliers de l'effectif n'y sont pas : ils vont partir. Ceux du
    /// panier, si : les y mettre est justement ce que le coach est en train de
    /// faire.
    ///
    /// **Sans cette distinction, un coach à seize dont trois journaliers ne
    /// pourrait recruter personne** — alors que les recruter est exactement ce
    /// qui le libérerait. `seize_dont_trois_journaliers_autorisent_le_recrutement`
    /// échoue si quelqu'un « simplifie » cette méthode en `squad.size()`.
    pub fn projected_squad_size(&self) -> usize {
        self.squad.permanent_size() + self.pending_players()
    }

    /// Les journaliers que le coach peut encore garder : ceux de l'effectif qui
    /// ne sont pas déjà dans son panier.
    pub fn hireable_journeymen(&self) -> Vec<&Player> {
        self.squad
            .journeymen()
            .filter(|j| !self.journeyman_in_basket(&j.player_id))
            .collect()
    }

    /// Les journaliers **déjà dans le panier** — ceux que `hireable_journeymen`
    /// écarte. La vue a besoin des deux : l'un pour proposer, l'autre pour
    /// nommer les lignes déjà posées.
    pub fn journeymen_in_basket(&self) -> Vec<&Player> {
        self.squad
            .journeymen()
            .filter(|j| self.journeyman_in_basket(&j.player_id))
            .collect()
    }

    fn journeyman_in_basket(&self, player_id: &PlayerId) -> bool {
        self.lines
            .iter()
            .any(|l| matches!(l, BasketLine::Journeyman { player_id: p, .. } if p == player_id))
    }

    pub fn remaining_treasury(&self) -> Kpo {
        Kpo(self.treasury.0.saturating_sub(self.pending_total().0))
    }

    /// Le catalogue du roster, pour la vue. Immuable : l'agrégat le porte, il
    /// ne le modifie jamais.
    pub fn catalog(&self) -> &RosterCatalog {
        &self.catalog
    }

    /// Effectif **déjà possédé** à ce poste, sans les lignes en attente.
    pub fn owned_at(&self, line: &RosterLineId) -> usize {
        self.squad.count_at(line)
    }

    pub fn owned_staff(&self) -> OwnedStaff {
        self.owned_staff
    }

    pub fn pending_staff_count(&self, staff: StaffType) -> u32 {
        self.pending_staff(staff)
    }

    pub fn pending_for_position(&self, line: &RosterLineId) -> usize {
        self.lines
            .iter()
            .filter(|l| matches!(l, BasketLine::Player { roster_line, .. } if roster_line == line))
            .count()
    }

    // ── Prix ──────────────────────────────────────────────────────────────

    /// Le doublement de la relance hors création est une **règle de saison**,
    /// pas une donnée de référence : le catalogue donne le prix de base, c'est
    /// l'agrégat qui applique le facteur.
    pub fn price_for(&self, staff: StaffType) -> Kpo {
        match staff {
            StaffType::Reroll => Kpo(self.catalog.reroll_base_cost.0 * 2),
            autre => staff_uid(autre)
                .and_then(|uid| self.catalog.staff_entry(uid))
                .map(|e| e.price)
                .unwrap_or(Kpo(0)),
        }
    }

    fn price_of_position(&self, line: &RosterLineId) -> Kpo {
        self.catalog
            .position(line)
            .map(|p| p.cost)
            .unwrap_or(Kpo(0))
    }

    // ── Comptages : possédés **plus** en attente ───────────────────────────

    /// Les lignes du panier qui deviendront des **joueurs permanents** : un
    /// poste acheté, ou un journalier gardé. Le staff n'en est pas.
    fn pending_players(&self) -> usize {
        self.lines
            .iter()
            .filter(|l| matches!(l, BasketLine::Player { .. } | BasketLine::Journeyman { .. }))
            .count()
    }

    fn pending_staff(&self, staff: StaffType) -> u32 {
        self.lines
            .iter()
            .filter(|l| matches!(l, BasketLine::Staff { staff_type, .. } if *staff_type == staff))
            .count() as u32
    }

    fn pending_total(&self) -> Kpo {
        Kpo(self.lines.iter().map(|l| l.price().0).sum())
    }

    // ── Les huit gardes ───────────────────────────────────────────────────
    //
    // Chacune compte **possédés + en attente**. C'est ce qui fait qu'un panier
    // respecte les quotas au lieu de les contourner par empilement.

    fn check_position_in_roster(&self, line: &RosterLineId) -> Result<(), DomainError> {
        if self.catalog.position(line).is_none() {
            return Err(DomainError::PositionNotInRoster);
        }
        Ok(())
    }

    fn check_squad_max(&self) -> Result<(), DomainError> {
        if self.projected_squad_size() >= MAX_SQUAD {
            return Err(DomainError::MaxPlayersReached);
        }
        Ok(())
    }

    fn check_position_quota(&self, line: &RosterLineId) -> Result<(), DomainError> {
        let Some(position) = self.catalog.position(line) else {
            return Err(DomainError::PositionNotInRoster);
        };
        let total = self.squad.count_at(line) + self.pending_for_position(line);
        if total >= position.max_quantity as usize {
            return Err(DomainError::PositionQuotaReached);
        }
        Ok(())
    }

    fn check_cross_limits(&self, line: &RosterLineId) -> Result<(), DomainError> {
        for limite in &self.catalog.cross_limits {
            if !limite.position_uids.contains(line) {
                continue;
            }
            let total: usize = limite
                .position_uids
                .iter()
                .map(|uid| self.squad.count_at(uid) + self.pending_for_position(uid))
                .sum();
            if total >= limite.max as usize {
                return Err(DomainError::CrossLimitExceeded);
            }
        }
        Ok(())
    }

    /// Vérifie le **cumul** : chaque ligne peut passer seule et le total échouer.
    fn check_treasury(&self, additional: Kpo) -> Result<(), DomainError> {
        if self.pending_total().0 + additional.0 > self.treasury.0 {
            return Err(DomainError::InsufficientTreasury);
        }
        Ok(())
    }

    fn check_staff_buyable(&self, staff: StaffType) -> Result<(), DomainError> {
        if staff == StaffType::FansFactor {
            return Err(DomainError::StaffTypeNotBuyable);
        }
        Ok(())
    }

    /// La relance n'est pas une ligne de `allowed_staff` : tout roster en a.
    fn check_staff_allowed(&self, staff: StaffType) -> Result<(), DomainError> {
        let Some(uid) = staff_uid(staff) else {
            return Ok(());
        };
        if !self.catalog.allowed_staff.iter().any(|s| s == uid) {
            return Err(DomainError::StaffNotAllowedForRoster);
        }
        Ok(())
    }

    fn check_staff_quota(&self, staff: StaffType) -> Result<(), DomainError> {
        let total = self.owned_staff.count_of(staff) + self.pending_staff(staff);
        let max = match staff {
            StaffType::Reroll => MAX_REROLLS,
            autre => staff_uid(autre)
                .and_then(|uid| self.catalog.staff_entry(uid))
                .map(|e| e.max_quantity)
                .unwrap_or(0),
        };
        if total >= max {
            return Err(DomainError::StaffQuotaReached);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
    use crate::app::teams::domain::basket::{
        CatalogPosition, CrossLimit, Player, StaffCatalogEntry,
    };
    use crate::app::teams::domain::basket::{SquadEngagement, SquadPresence};

    const PIETAILLE: &str = "DEMO_GRANIT__PIETAILLE";
    const PERCUTEUR: &str = "DEMO_GRANIT__PERCUTEUR";
    const COLOSSE: &str = "DEMO_GRANIT__COLOSSE";
    /// Hors limite croisée : sert à éprouver le quota de poste seul, sans
    /// qu'une autre garde ne morde avant lui.
    const LANCEUR: &str = "DEMO_GRANIT__LANCEUR";

    fn ligne(uid: &str) -> RosterLineId {
        RosterLineId(uid.to_string())
    }

    /// Caractéristiques et compétences ne pèsent sur aucune garde : les tests
    /// du panier les fixent une fois pour toutes plutôt que de les répéter.
    fn poste(uid: &str, nom: &str, cout: u32, max: u8) -> CatalogPosition {
        CatalogPosition {
            uid: ligne(uid),
            position_name: nom.into(),
            cost: Kpo(cout),
            max_quantity: max,
            ma: 6,
            st: 3,
            ag: 3,
            pa: 4,
            av: 9,
            skills: vec![],
        }
    }

    fn catalogue() -> RosterCatalog {
        RosterCatalog {
            positions: vec![
                poste(PIETAILLE, "Piétaille", 50, 16),
                poste(PERCUTEUR, "Percuteur", 90, 4),
                poste(COLOSSE, "Colosse", 140, 1),
                poste(LANCEUR, "Lanceur", 80, 4),
            ],
            cross_limits: vec![CrossLimit {
                max: 2,
                position_uids: vec![ligne(PERCUTEUR), ligne(COLOSSE)],
            }],
            allowed_staff: vec![
                "APOTHECARY".into(),
                "CHEERLEADERS".into(),
                "COACH_ASSISTANTS".into(),
            ],
            staff: vec![
                StaffCatalogEntry {
                    uid: "APOTHECARY".into(),
                    price: Kpo(50),
                    max_quantity: 1,
                },
                StaffCatalogEntry {
                    uid: "CHEERLEADERS".into(),
                    price: Kpo(10),
                    max_quantity: 6,
                },
                StaffCatalogEntry {
                    uid: "COACH_ASSISTANTS".into(),
                    price: Kpo(10),
                    max_quantity: 6,
                },
                StaffCatalogEntry {
                    uid: "FAN_FACTOR".into(),
                    price: Kpo(5),
                    max_quantity: 2,
                },
            ],
            reroll_base_cost: Kpo(60),
        }
    }

    /// Le recrutement ne lit que la ligne de roster de chaque membre : le reste
    /// est rempli de valeurs neutres. Les identifiants restent distincts, sans
    /// quoi l'effectif ne représenterait qu'un seul joueur répété.
    fn effectif(lignes: &[(&str, usize)]) -> Squad {
        let mut members = Vec::new();
        for (uid, n) in lignes {
            for _ in 0..*n {
                let rang = members.len();
                members.push(Player {
                    player_id: identifiant(rang),
                    roster_line: ligne(uid),
                    jersey: Some(rang as u8 + 1),
                    personal_name: String::new(),
                    position_name: String::new(),
                    spp: 0,
                    value_kpo: Kpo(0),
                    presence: SquadPresence::Alignable,
                    engagement: crate::app::teams::domain::basket::SquadEngagement::Permanent,
                });
            }
        }
        Squad { members }
    }

    /// Même effectif, mais dont on choisit la présence de chaque membre.
    /// Sert aux règles qui distinguent l'occupant du perdu — les autres tests
    /// n'ont pas à connaître ce détail.
    fn effectif_avec(lignes: &[(&str, SquadPresence, usize)]) -> Squad {
        let mut members = Vec::new();
        for (uid, presence, n) in lignes {
            for _ in 0..*n {
                let rang = members.len();
                members.push(Player {
                    player_id: identifiant(rang),
                    roster_line: ligne(uid),
                    jersey: Some(rang as u8 + 1),
                    personal_name: String::new(),
                    position_name: String::new(),
                    spp: 0,
                    value_kpo: Kpo(0),
                    presence: *presence,
                    engagement: crate::app::teams::domain::basket::SquadEngagement::Permanent,
                });
            }
        }
        Squad { members }
    }

    fn identifiant(n: usize) -> PlayerId {
        PlayerId::try_new(&format!("{n:0>26}")).unwrap()
    }

    fn panier(squad: Squad, treasury: u32) -> RecruitmentBasket {
        RecruitmentBasket::hydrate(
            "t1".into(),
            BasketVersion(1),
            vec![],
            catalogue(),
            squad,
            OwnedStaff::default(),
            Kpo(treasury),
        )
    }

    // ── Journaliers (carte 457) ───────────────────────────────────────────

    /// Un effectif mêlant permanents et journaliers, tous alignables.
    fn effectif_mele(permanents: usize, journaliers: usize) -> Squad {
        let mut members = Vec::new();
        for i in 0..(permanents + journaliers) {
            members.push(Player {
                player_id: identifiant(i),
                roster_line: ligne(PIETAILLE),
                jersey: Some(i as u8 + 1),
                personal_name: String::new(),
                position_name: String::new(),
                spp: 0,
                value_kpo: Kpo(50),
                presence: SquadPresence::Alignable,
                engagement: match i < permanents {
                    true => SquadEngagement::Permanent,
                    false => SquadEngagement::Journalier,
                },
            });
        }
        Squad { members }
    }

    #[test]
    fn le_meme_journalier_ne_s_ajoute_pas_deux_fois() {
        let mut p = panier(effectif_mele(5, 1), 1000);
        let journalier = identifiant(5);

        assert!(p.add_journeyman(journalier.clone()).is_ok());
        assert_eq!(
            p.add_journeyman(journalier),
            Err(DomainError::JourneymanAlreadyInBasket)
        );
    }

    /// C'est la règle qu'un poste n'a pas : deux Trois-quarts sont deux
    /// joueurs, un journalier est un homme.
    #[test]
    fn deux_journaliers_differents_s_ajoutent() {
        let mut p = panier(effectif_mele(5, 2), 1000);

        assert!(p.add_journeyman(identifiant(5)).is_ok());
        assert!(p.add_journeyman(identifiant(6)).is_ok());
        assert_eq!(p.lines().len(), 2);
    }

    #[test]
    fn un_journalier_absent_des_recrutables_est_refuse() {
        let mut p = panier(effectif_mele(5, 1), 1000);

        // Un permanent n'est pas un journalier : on ne le « garde » pas.
        assert_eq!(
            p.add_journeyman(identifiant(0)),
            Err(DomainError::JourneymanNoLongerAvailable)
        );
        // Un inconnu non plus.
        assert_eq!(
            p.add_journeyman(identifiant(99)),
            Err(DomainError::JourneymanNoLongerAvailable)
        );
    }

    #[test]
    fn seize_permanents_bloquent_le_recrutement() {
        let mut p = panier(effectif_mele(16, 1), 1000);

        assert_eq!(
            p.add_journeyman(identifiant(16)),
            Err(DomainError::MaxPlayersReached)
        );
    }

    /// **Le test qui donne son sens au plafond.**
    ///
    /// Seize membres dont trois journaliers : treize places acquises, donc de
    /// la marge. Sans la distinction, ce coach ne pourrait recruter personne —
    /// alors que recruter ses journaliers est exactement ce qui le libérerait.
    ///
    /// Il échoue si quelqu'un « simplifie » `projected_squad_size` en
    /// `squad.size()`, ce qui compilerait et paraîtrait juste.
    #[test]
    fn seize_dont_trois_journaliers_autorisent_le_recrutement() {
        let mut p = panier(effectif_mele(13, 3), 1000);

        assert_eq!(p.projected_squad_size(), 13, "seuls les acquis comptent");
        assert!(p.add_journeyman(identifiant(13)).is_ok());
        assert!(p.add_journeyman(identifiant(14)).is_ok());
        assert!(p.add_journeyman(identifiant(15)).is_ok());
    }

    /// Un journalier du panier **devient** permanent : il entre au plafond dès
    /// qu'il y est posé, sans quoi le coach en garderait plus de seize.
    #[test]
    fn un_journalier_du_panier_compte_dans_le_plafond() {
        let mut p = panier(effectif_mele(15, 2), 1000);
        assert_eq!(p.projected_squad_size(), 15);

        p.add_journeyman(identifiant(15)).unwrap();
        assert_eq!(p.projected_squad_size(), 16, "il a pris une place acquise");

        assert_eq!(
            p.add_journeyman(identifiant(16)),
            Err(DomainError::MaxPlayersReached)
        );
    }

    #[test]
    fn un_journalier_trop_cher_est_refuse() {
        let mut p = panier(effectif_mele(5, 1), 10);

        assert_eq!(
            p.add_journeyman(identifiant(5)),
            Err(DomainError::InsufficientTreasury)
        );
    }

    /// Le rejeu de validation retrouve le journalier et rend sa ligne appliquée.
    #[test]
    fn validate_all_applique_un_journalier() {
        let mut p = panier(effectif_mele(5, 1), 1000);
        p.add_journeyman(identifiant(5)).unwrap();

        let applied = p.validate_all().expect("le panier est valide");
        assert_eq!(
            applied,
            vec![AppliedLine::Journeyman {
                player_id: identifiant(5),
                cost: Kpo(50),
            }]
        );
    }

    /// Le garde-fou à la validation : le journalier a disparu de l'effectif
    /// entre l'ajout et la clôture — un autre chemin l'a fait partir.
    #[test]
    fn validate_all_rejette_un_journalier_disparu() {
        let mut p = panier(effectif_mele(5, 1), 1000);
        p.add_journeyman(identifiant(5)).unwrap();

        // L'effectif est rechargé sans lui, les lignes du panier restent.
        let lignes = p.lines().to_vec();
        let sans_lui = RecruitmentBasket::hydrate(
            "t1".into(),
            BasketVersion(1),
            lignes,
            catalogue(),
            effectif_mele(5, 0),
            OwnedStaff::default(),
            Kpo(1000),
        );

        let rejets = sans_lui.validate_all().expect_err("il n'est plus là");
        assert_eq!(rejets.len(), 1);
        assert_eq!(rejets[0].cause, DomainError::JourneymanNoLongerAvailable);
    }

    /// Les recrutables sont ceux de l'effectif **moins** ceux déjà au panier :
    /// l'écran ne propose pas deux fois le même homme.
    #[test]
    fn les_recrutables_excluent_ceux_du_panier() {
        let mut p = panier(effectif_mele(5, 2), 1000);
        assert_eq!(p.hireable_journeymen().len(), 2);

        p.add_journeyman(identifiant(5)).unwrap();
        assert_eq!(p.hireable_journeymen().len(), 1);
        assert_eq!(p.journeymen_in_basket().len(), 1);
    }

    // ── 1 & 2 : plafond d'effectif, possédés puis mélange ─────────────────

    #[test]
    fn t01_seize_possedes_refuse() {
        let mut p = panier(effectif(&[(PIETAILLE, 16)]), 10_000);
        assert_eq!(
            p.add_player(ligne(PIETAILLE)),
            Err(DomainError::MaxPlayersReached)
        );
    }

    /// Le test qui compte : sans la prise en compte des lignes en attente, on
    /// contournerait le plafond par empilement.
    #[test]
    fn t02_quinze_possedes_plus_un_en_attente_refuse() {
        let mut p = panier(effectif(&[(PIETAILLE, 15)]), 10_000);
        p.add_player(ligne(PIETAILLE)).unwrap();
        assert_eq!(
            p.add_player(ligne(PIETAILLE)),
            Err(DomainError::MaxPlayersReached)
        );
    }

    // ── 3 & 4 : quota de poste ────────────────────────────────────────────

    #[test]
    fn t03_quota_de_poste_atteint_par_les_possedes_refuse() {
        let mut p = panier(effectif(&[(COLOSSE, 1)]), 10_000);
        assert_eq!(
            p.add_player(ligne(COLOSSE)),
            Err(DomainError::PositionQuotaReached)
        );
    }

    #[test]
    fn t04_quota_atteint_par_un_melange_possedes_et_attente_refuse() {
        // Le Lanceur est hors limite croisée : seul le quota de poste peut
        // mordre, ce qui rend le test univoque. Avec le Percuteur, la limite
        // croisée refusait dès le premier ajout et masquait la règle visée.
        let mut p = panier(effectif(&[(LANCEUR, 2)]), 10_000);
        p.add_player(ligne(LANCEUR)).unwrap();
        p.add_player(ligne(LANCEUR)).unwrap();
        assert_eq!(
            p.add_player(ligne(LANCEUR)),
            Err(DomainError::PositionQuotaReached),
            "2 possédés + 2 en attente atteignent le quota de 4"
        );
    }

    // ── Présence : ce qu'un mort ne garde pas ─────────────────────────────

    /// Le Colosse a un quota de 1. Mort, il ne l'occupe plus — sans quoi la
    /// place resterait bloquée pour toujours, ce statut étant terminal.
    #[test]
    fn t03bis_un_mort_ne_bloque_pas_le_quota_de_son_poste() {
        let mut p = panier(effectif_avec(&[(COLOSSE, SquadPresence::Perdu, 1)]), 10_000);
        assert!(
            p.add_player(ligne(COLOSSE)).is_ok(),
            "le quota du Colosse est rendu par la mort de son titulaire"
        );
    }

    /// La règle symétrique, et c'est elle qui interdit de filtrer sur
    /// « alignable » : un blessé revient au match suivant (BR12), sa place lui
    /// reste. La confondre avec le mort aurait fait recruter à sa place.
    #[test]
    fn t03ter_un_blesse_bloque_toujours_le_quota_de_son_poste() {
        let mut p = panier(
            effectif_avec(&[(COLOSSE, SquadPresence::Empeche, 1)]),
            10_000,
        );
        assert_eq!(
            p.add_player(ligne(COLOSSE)),
            Err(DomainError::PositionQuotaReached)
        );
    }

    #[test]
    fn t01bis_un_mort_rend_sa_place_dans_le_plafond_de_seize() {
        let mut p = panier(
            effectif_avec(&[
                (PIETAILLE, SquadPresence::Alignable, 15),
                (PIETAILLE, SquadPresence::Perdu, 1),
            ]),
            10_000,
        );
        assert!(
            p.add_player(ligne(PIETAILLE)).is_ok(),
            "seize membres dont un mort n'en font que quinze au plafond"
        );
    }

    #[test]
    fn t01ter_un_blesse_occupe_toujours_sa_place_dans_le_plafond() {
        let mut p = panier(
            effectif_avec(&[
                (PIETAILLE, SquadPresence::Alignable, 15),
                (PIETAILLE, SquadPresence::Empeche, 1),
            ]),
            10_000,
        );
        assert_eq!(
            p.add_player(ligne(PIETAILLE)),
            Err(DomainError::MaxPlayersReached)
        );
    }

    /// La limite croisée compte les occupants des deux postes, pas leurs
    /// membres : un mort n'y pèse pas davantage qu'ailleurs.
    #[test]
    fn t05bis_un_mort_ne_pese_pas_dans_la_limite_croisee() {
        let mut p = panier(
            effectif_avec(&[
                (PERCUTEUR, SquadPresence::Alignable, 1),
                (COLOSSE, SquadPresence::Perdu, 1),
            ]),
            10_000,
        );
        assert!(p.add_player(ligne(PERCUTEUR)).is_ok());
    }

    // ── 5 & 6 : limites croisées, poste hors roster ───────────────────────

    #[test]
    fn t05_limite_croisee_atteinte_sur_deux_postes_differents_refuse() {
        let mut p = panier(effectif(&[(PERCUTEUR, 1), (COLOSSE, 1)]), 10_000);
        assert_eq!(
            p.add_player(ligne(PERCUTEUR)),
            Err(DomainError::CrossLimitExceeded)
        );
    }

    #[test]
    fn t06_poste_absent_du_roster_refuse() {
        let mut p = panier(Squad::default(), 10_000);
        assert_eq!(
            p.add_player(ligne("AUTRE_ROSTER__TROLL")),
            Err(DomainError::PositionNotInRoster)
        );
    }

    // ── 7 : trésorerie en total ───────────────────────────────────────────

    /// Chaque ligne passe seule — 50 sur 120 de caisse — mais leur cumul non.
    /// C'est la vérification **en total**, pas ligne par ligne.
    #[test]
    fn t07_tresorerie_insuffisante_pour_le_total_refuse() {
        let mut p = panier(Squad::default(), 120);
        p.add_player(ligne(PIETAILLE)).unwrap();
        p.add_player(ligne(PIETAILLE)).unwrap();
        assert_eq!(
            p.add_player(ligne(PIETAILLE)),
            Err(DomainError::InsufficientTreasury),
            "150 demandés pour 120 en caisse"
        );
    }

    // ── 8 à 11 : staff ────────────────────────────────────────────────────

    #[test]
    fn t08_facteur_fans_non_achetable() {
        let mut p = panier(Squad::default(), 10_000);
        assert_eq!(
            p.add_staff(StaffType::FansFactor),
            Err(DomainError::StaffTypeNotBuyable)
        );
    }

    #[test]
    fn t09_apothicaire_sur_roster_non_autorise_refuse() {
        let mut catalogue = catalogue();
        catalogue.allowed_staff.retain(|s| s != "APOTHECARY");
        let mut p = RecruitmentBasket::hydrate(
            "t1".into(),
            BasketVersion(1),
            vec![],
            catalogue,
            Squad::default(),
            OwnedStaff::default(),
            Kpo(10_000),
        );
        assert_eq!(
            p.add_staff(StaffType::Apothecary),
            Err(DomainError::StaffNotAllowedForRoster)
        );
    }

    #[test]
    fn t10_apothicaire_sur_roster_autorise_accepte() {
        let mut p = panier(Squad::default(), 10_000);
        assert!(p.add_staff(StaffType::Apothecary).is_ok());
    }

    #[test]
    fn t11_la_neuvieme_relance_refuse() {
        let mut p = panier(Squad::default(), 10_000);
        for _ in 0..8 {
            p.add_staff(StaffType::Reroll).unwrap();
        }
        assert_eq!(
            p.add_staff(StaffType::Reroll),
            Err(DomainError::StaffQuotaReached)
        );
    }

    // ── 12 : le doublement de la relance ──────────────────────────────────

    #[test]
    fn t12_prix_de_relance_double_le_prix_de_base_du_roster() {
        let p = panier(Squad::default(), 10_000);
        assert_eq!(
            p.price_for(StaffType::Reroll),
            Kpo(120),
            "60 de base, doublé hors création"
        );
    }

    // ── 13 & 14 : retrait de ligne ────────────────────────────────────────

    #[test]
    fn t13_remove_line_libere_quota_et_tresorerie() {
        let mut p = panier(effectif(&[(COLOSSE, 0)]), 200);
        let id = p.add_player(ligne(COLOSSE)).unwrap();
        assert_eq!(p.remaining_treasury(), Kpo(60));
        assert_eq!(
            p.add_player(ligne(COLOSSE)),
            Err(DomainError::PositionQuotaReached)
        );

        p.remove_line(&id).unwrap();

        assert_eq!(p.remaining_treasury(), Kpo(200), "la trésorerie est rendue");
        assert!(p.add_player(ligne(COLOSSE)).is_ok(), "le quota est libéré");
    }

    #[test]
    fn t14_remove_line_sur_identifiant_inconnu_refuse() {
        let mut p = panier(Squad::default(), 200);
        assert_eq!(
            p.remove_line(&BasketLineId("inconnu".into())),
            Err(DomainError::BasketLineNotFound)
        );
    }

    // ── 15 & 16 : validation en bloc ──────────────────────────────────────

    /// Refus en bloc : une seule ligne devenue invalide et **rien** n'est
    /// appliqué, pas même les lignes saines.
    #[test]
    fn t15_une_ligne_invalide_et_rien_n_est_applique() {
        let mut p = panier(Squad::default(), 200);
        p.add_player(ligne(PIETAILLE)).unwrap();
        p.add_player(ligne(PIETAILLE)).unwrap();

        // La trésorerie fond entre la constitution du panier et sa validation.
        p.treasury = Kpo(60);

        let issue = p.validate_all();
        let rejets = issue.expect_err("le panier ne passe plus");
        assert_eq!(rejets.len(), 1, "la seconde ligne ne tient plus");
        assert_eq!(rejets[0].cause, DomainError::InsufficientTreasury);
    }

    #[test]
    fn t16_un_panier_vide_donne_un_lot_vide_sans_erreur() {
        let p = panier(Squad::default(), 200);
        assert_eq!(p.validate_all(), Ok(vec![]));
    }

    #[test]
    fn validate_all_rend_les_lignes_applicables() {
        let mut p = panier(Squad::default(), 500);
        p.add_player(ligne(PIETAILLE)).unwrap();
        p.add_staff(StaffType::Apothecary).unwrap();

        let applique = p.validate_all().expect("tout tient");

        assert_eq!(
            applique,
            vec![
                AppliedLine::Player {
                    roster_line: ligne(PIETAILLE),
                    base_value: Kpo(50),
                    cost: Kpo(50),
                },
                AppliedLine::Staff {
                    staff_type: StaffType::Apothecary,
                    cost: Kpo(50)
                },
            ]
        );
    }

    // ── 17 : le domaine décide de la cause ────────────────────────────────

    #[test]
    fn t17_action_for_position_retourne_la_cause_exacte() {
        let p = panier(effectif(&[(COLOSSE, 1)]), 10_000);
        assert_eq!(
            p.action_for_position(&ligne(COLOSSE)),
            ActionState::Blocked {
                cause: DomainError::PositionQuotaReached
            }
        );

        let pauvre = panier(Squad::default(), 10);
        assert_eq!(
            pauvre.action_for_position(&ligne(PIETAILLE)),
            ActionState::Blocked {
                cause: DomainError::InsufficientTreasury
            }
        );

        assert_eq!(
            p.action_for_position(&ligne("AUTRE__TROLL")),
            ActionState::Forbidden {
                cause: DomainError::PositionNotInRoster
            },
            "un poste hors roster ne le rejoindra jamais"
        );
        assert_eq!(
            p.action_for_position(&ligne(PIETAILLE)),
            ActionState::Allowed
        );
    }

    /// Un quota se libère, un roster n'acquiert jamais le droit à un
    /// apothicaire : les deux états ne se confondent pas.
    #[test]
    fn action_for_staff_distingue_bloque_et_interdit() {
        let mut p = panier(Squad::default(), 10_000);
        assert_eq!(
            p.action_for_staff(StaffType::FansFactor),
            ActionState::Forbidden {
                cause: DomainError::StaffTypeNotBuyable
            }
        );

        p.add_staff(StaffType::Apothecary).unwrap();
        assert_eq!(
            p.action_for_staff(StaffType::Apothecary),
            ActionState::Blocked {
                cause: DomainError::StaffQuotaReached
            },
            "quota atteint par la ligne en attente"
        );
    }
}

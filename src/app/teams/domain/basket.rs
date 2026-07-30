//! Ce que les deux paniers de phase partagent.
//!
//! Le recrutement et les renvois hydratent la même chose — l'effectif, le
//! catalogue du roster, le staff possédé — et numérotent leurs lignes de la même
//! façon. Ce vocabulaire vivait dans `recruitment_basket.rs`, où il était arrivé
//! le premier ; l'y laisser aurait fait du module « recrutement » le dictionnaire
//! des renvois, pour trois cartes de plus.
//!
//! Rien ici n'est spécifique à une phase. Ce qui l'est — les lignes, les gardes,
//! l'état d'une action — reste dans le module de son panier.

use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::value_objects::{Kpo, StaffType};

// ── Identifiants ──────────────────────────────────────────────────────────────

/// Identifiant d'une ligne de roster — `DEMO_GRANIT__PIETAILLE`, pas un ULID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RosterLineId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BasketLineId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BasketVersion(pub u32);

// ── Catalogue du roster ───────────────────────────────────────────────────────

/// Une compétence de base, telle qu'elle s'affiche. Aucun invariant : elle
/// voyage dans l'agrégat parce que le panier est la seule chose que la vue voit.
#[derive(Debug, Clone)]
pub struct SkillBadge {
    pub name: String,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct CatalogPosition {
    pub uid: RosterLineId,
    pub position_name: String,
    pub cost: Kpo,
    pub max_quantity: u8,
    /// Caractéristiques et compétences ne portent aucun invariant : elles ne
    /// sont là que parce que le panier est **la seule chose que la vue voit**.
    /// Les faire transiter autrement obligerait un handler à toucher un DTO de
    /// port, ce que la convention interdit.
    pub ma: u8,
    pub st: u8,
    pub ag: u8,
    pub pa: u8,
    pub av: u8,
    pub skills: Vec<SkillBadge>,
}

/// Limite de cumul entre postes — « pas plus de 3 parmi Ogre, Troll, Minotaure ».
#[derive(Debug, Clone)]
pub struct CrossLimit {
    pub max: u32,
    pub position_uids: Vec<RosterLineId>,
}

#[derive(Debug, Clone)]
pub struct StaffCatalogEntry {
    pub uid: String,
    pub price: Kpo,
    pub max_quantity: u32,
}

/// Le catalogue du roster, hydraté depuis `IRosterCatalogPort`.
#[derive(Debug, Clone)]
pub struct RosterCatalog {
    pub positions: Vec<CatalogPosition>,
    pub cross_limits: Vec<CrossLimit>,
    pub allowed_staff: Vec<String>,
    pub staff: Vec<StaffCatalogEntry>,
    pub reroll_base_cost: Kpo,
}

impl RosterCatalog {
    pub fn position(&self, line: &RosterLineId) -> Option<&CatalogPosition> {
        self.positions.iter().find(|p| &p.uid == line)
    }

    pub fn staff_entry(&self, uid: &str) -> Option<&StaffCatalogEntry> {
        self.staff.iter().find(|s| s.uid == uid)
    }
}

// ── L'effectif ────────────────────────────────────────────────────────────────

/// Un joueur de l'effectif, tel que `teams` le voit.
///
/// Ce n'est pas l'entité `Player` du BC `players` — `teams` ne la connaît pas et
/// n'en est pas propriétaire. C'est la projection que `ISquadPort` en rapporte,
/// portée par l'agrégat parce qu'un agrégat n'appelle pas de port.
///
/// Nom, poste et SPP ne servent aucune garde : ils sont là parce que le panier
/// est la seule chose que la vue voit, comme les caractéristiques de
/// `CatalogPosition`.
#[derive(Debug, Clone)]
pub struct Player {
    pub player_id: PlayerId,
    pub roster_line: RosterLineId,
    pub jersey: Option<u8>,
    pub personal_name: String,
    pub position_name: String,
    pub spp: u32,
    pub value_kpo: Kpo,
    /// Disponibilité au prochain match. Le recrutement l'ignore — un blessé
    /// occupe sa place et compte dans les quotas ; les renvois s'en servent pour
    /// le plancher des onze éligibles.
    pub available_for_next_match: bool,
}

/// L'effectif possédé, hydraté depuis `ISquadPort`. Tous les joueurs comptent
/// pour les quotas, disponibles ou non : un blessé occupe toujours sa place.
#[derive(Debug, Clone, Default)]
pub struct Squad {
    pub members: Vec<Player>,
}

impl Squad {
    pub fn size(&self) -> usize {
        self.members.len()
    }

    pub fn count_at(&self, line: &RosterLineId) -> usize {
        self.members
            .iter()
            .filter(|m| &m.roster_line == line)
            .count()
    }

    pub fn find(&self, id: &PlayerId) -> Option<&Player> {
        self.members.iter().find(|m| &m.player_id == id)
    }

    /// Les joueurs qui peuvent tenir une place au prochain match.
    pub fn eligible_count(&self) -> usize {
        self.members
            .iter()
            .filter(|m| m.available_for_next_match)
            .count()
    }
}

// ── Le staff possédé ──────────────────────────────────────────────────────────

/// Le staff déjà possédé par l'équipe, hydraté depuis l'agrégat `Team`.
#[derive(Debug, Clone, Copy, Default)]
pub struct OwnedStaff {
    pub rerolls: u32,
    pub apothecaries: u32,
    pub assistants: u32,
    pub cheerleaders: u32,
}

impl OwnedStaff {
    pub fn count_of(&self, staff: StaffType) -> u32 {
        match staff {
            StaffType::Reroll => self.rerolls,
            StaffType::Apothecary => self.apothecaries,
            StaffType::Assistant => self.assistants,
            StaffType::Cheerleader => self.cheerleaders,
            StaffType::FansFactor => 0,
        }
    }
}

/// Identifiant du staff dans le corpus de référence. La relance n'en a pas :
/// elle n'est pas une ligne de `staff_fr.json`.
pub fn staff_uid(staff: StaffType) -> Option<&'static str> {
    match staff {
        StaffType::Apothecary => Some("APOTHECARY"),
        StaffType::Assistant => Some("COACH_ASSISTANTS"),
        StaffType::Cheerleader => Some("CHEERLEADERS"),
        StaffType::FansFactor => Some("FAN_FACTOR"),
        StaffType::Reroll => None,
    }
}

// ── Refus ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct RejectedLine {
    pub id: BasketLineId,
    pub cause: DomainError,
}

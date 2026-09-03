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
    /// Ce que ce joueur est encore pour l'équipe. Voir `SquadPresence` : les
    /// deux questions qu'il pose n'ont pas la même réponse, et un booléen les
    /// confondait.
    pub presence: SquadPresence,
}

/// Ce qu'un membre de l'effectif est encore pour l'équipe.
///
/// Le vocabulaire est celui de `teams`, pas celui de `players` : l'adapter
/// traduit. `teams` n'a pas à connaître le mot « mort », seulement ses deux
/// conséquences — qui sont **indépendantes**, et c'est tout l'intérêt du type.
///
/// Un booléen `available_for_next_match` les confondait, et c'est ce qui a
/// laissé les morts occuper une place : filtrer dessus aurait aussi libéré
/// celle d'un blessé, qui revient au match suivant (BR12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquadPresence {
    /// Il jouera : il occupe une place et il est alignable.
    Alignable,
    /// Empêché au prochain match — blessure, retraite temporaire. Il garde sa
    /// place : on ne recrute pas quelqu'un d'autre pendant son absence.
    Empeche,
    /// Perdu pour l'équipe. Il ne garde rien : ni sa place au plafond de seize,
    /// ni celle du quota de son poste.
    Perdu,
}

impl SquadPresence {
    /// Compte-t-il dans le plafond de seize et dans le quota de son poste ?
    pub fn occupe_une_place(&self) -> bool {
        !matches!(self, Self::Perdu)
    }

    /// Peut-il tenir une place au coup d'envoi du prochain match ?
    ///
    /// Un perdu ne l'est pas non plus — d'où le journalier qu'il appelle, comme
    /// un blessé : le trou est réel jusqu'au recrutement.
    pub fn alignable(&self) -> bool {
        matches!(self, Self::Alignable)
    }
}

/// L'effectif rapporté par `ISquadPort`, **entier**.
///
/// Le port ne filtre pas — il ne le peut pas : la valeur d'équipe ne somme que
/// les alignables quand les quotas comptent tous les occupants, et un port qui
/// trancherait à la source servirait l'un en trahissant l'autre. C'est donc ce
/// type qui distingue, et `members` garde tout le monde.
#[derive(Debug, Clone, Default)]
pub struct Squad {
    pub members: Vec<Player>,
}

impl Squad {
    /// Les membres qui occupent encore une place. C'est aussi ce que l'équipe
    /// se voit d'elle-même : la liste des renvois est bâtie dessus, un joueur
    /// dont la place est déjà libre n'ayant plus rien à y faire.
    pub fn occupants(&self) -> impl Iterator<Item = &Player> {
        self.members
            .iter()
            .filter(|m| m.presence.occupe_une_place())
    }

    /// Ce qui compte pour le plafond de seize. Pas `members.len()` : un joueur
    /// perdu n'y figure plus, sans quoi il bloquerait un recrutement pour une
    /// place que personne n'occupe.
    pub fn size(&self) -> usize {
        self.occupants().count()
    }

    pub fn count_at(&self, line: &RosterLineId) -> usize {
        self.occupants().filter(|m| &m.roster_line == line).count()
    }

    /// Cherche parmi **tous** les membres, occupants ou non : le plancher des
    /// renvois doit pouvoir répondre sur un joueur qu'il ne propose plus.
    pub fn find(&self, id: &PlayerId) -> Option<&Player> {
        self.members.iter().find(|m| &m.player_id == id)
    }

    /// Les joueurs qui peuvent tenir une place au prochain match.
    pub fn eligible_count(&self) -> usize {
        self.members
            .iter()
            .filter(|m| m.presence.alignable())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::PlayerId;

    const PIETAILLE: &str = "DEMO_GRANIT__PIETAILLE";
    const COLOSSE: &str = "DEMO_GRANIT__COLOSSE";

    fn joueur(n: usize, ligne: &str, presence: SquadPresence) -> Player {
        Player {
            player_id: PlayerId::try_new(&format!("{n:0>26}")).unwrap(),
            roster_line: RosterLineId(ligne.into()),
            jersey: Some(n as u8 + 1),
            personal_name: format!("Joueur {n}"),
            position_name: "Poste".into(),
            spp: 0,
            value_kpo: Kpo(50),
            presence,
        }
    }

    /// Un alignable, un empêché, un perdu — chacun sur la même ligne de roster,
    /// pour que seule la présence explique les écarts.
    fn effectif_mixte() -> Squad {
        Squad {
            members: vec![
                joueur(0, PIETAILLE, SquadPresence::Alignable),
                joueur(1, PIETAILLE, SquadPresence::Empeche),
                joueur(2, PIETAILLE, SquadPresence::Perdu),
            ],
        }
    }

    #[test]
    fn le_perdu_ne_compte_pas_dans_la_taille_de_l_effectif() {
        assert_eq!(effectif_mixte().size(), 2, "trois membres, deux occupants");
    }

    #[test]
    fn le_perdu_ne_compte_pas_dans_le_quota_de_sa_ligne() {
        let squad = effectif_mixte();
        assert_eq!(squad.count_at(&RosterLineId(PIETAILLE.into())), 2);
        assert_eq!(squad.count_at(&RosterLineId(COLOSSE.into())), 0);
    }

    #[test]
    fn les_occupants_excluent_le_perdu_et_gardent_l_empeche() {
        let squad = effectif_mixte();
        let ids: Vec<String> = squad.occupants().map(|m| m.player_id.to_string()).collect();
        assert_eq!(ids.len(), 2);
        assert!(!ids.contains(
            &joueur(2, PIETAILLE, SquadPresence::Perdu)
                .player_id
                .to_string()
        ));
    }

    /// L'empêché garde sa place et c'est tout l'enjeu : filtrer sur
    /// « alignable » l'aurait libérée alors qu'il revient au match suivant.
    #[test]
    fn l_empeche_occupe_sa_place_sans_etre_alignable() {
        let squad = effectif_mixte();
        assert_eq!(squad.eligible_count(), 1, "seul l'alignable l'est");
        assert_eq!(squad.size(), 2, "l'empêché occupe pourtant sa place");
    }

    /// `find` cherche parmi **tous** les membres, occupants ou non : le plancher
    /// des renvois doit pouvoir répondre sur un joueur qu'il ne propose plus.
    #[test]
    fn find_trouve_encore_le_perdu() {
        let squad = effectif_mixte();
        let perdu = joueur(2, PIETAILLE, SquadPresence::Perdu).player_id;
        assert!(squad.find(&perdu).is_some());
    }

    #[test]
    fn un_effectif_sans_perdu_compte_tout_le_monde() {
        let squad = Squad {
            members: vec![
                joueur(0, PIETAILLE, SquadPresence::Alignable),
                joueur(1, PIETAILLE, SquadPresence::Empeche),
            ],
        };
        assert_eq!(squad.size(), 2);
        assert_eq!(squad.occupants().count(), 2);
    }
}

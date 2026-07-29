//! Les view models de la phase de recrutement.
//!
//! Tous se construisent depuis des types **domaine** : le panier porte ses
//! données une fois hydraté, aucun `builders.rs` n'est donc nécessaire.
//!
//! C'est ici, et nulle part ailleurs, qu'un `DomainError` devient une phrase
//! française. Le domaine décide qu'un bouton est bloqué et pourquoi ; la vue se
//! contente de le formuler. C'est ce qui permet de n'écrire chaque règle
//! qu'une fois.

use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::recruitment_basket::{
    ActionState, BasketLine, RecruitmentBasket, RosterLineId, SkillBadge,
};
use crate::app::teams::domain::team::Team;
use crate::app::teams::domain::value_objects::StaffType;
use crate::app::teams::routes::Routes;

/// Effectif maximum d'une équipe, pour l'en-tête. Le plafond qui *décide* vit
/// dans l'agrégat ; celui-ci ne fait que l'afficher.
const SQUAD_MAX: u8 = 16;

/// Les quatre personnels achetables, dans l'ordre de la maquette. Le facteur
/// fans n'en est pas : il ne s'achète pas.
const STAFF_ORDER: [StaffType; 4] = [
    StaffType::Reroll,
    StaffType::Apothecary,
    StaffType::Assistant,
    StaffType::Cheerleader,
];

// ── L'état d'un bouton ────────────────────────────────────────────────────────

/// `Blocked` et `Forbidden` restent distincts jusque dans la vue parce qu'ils
/// ne disent pas la même chose au coach : un quota se libère, un roster
/// n'acquiert jamais le droit à un apothicaire.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionVm {
    Enabled { label: String },
    Blocked { reason: String },
    Forbidden { explanation: String },
}

impl ActionVm {
    pub fn from_domain(state: &ActionState, label: &str) -> Self {
        match state {
            ActionState::Allowed => Self::Enabled {
                label: label.to_string(),
            },
            ActionState::Blocked { cause } => Self::Blocked {
                reason: raison_courte(cause).to_string(),
            },
            ActionState::Forbidden { cause } => Self::Forbidden {
                explanation: explication_longue(cause).to_string(),
            },
        }
    }

    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled { .. })
    }

    pub fn is_forbidden(&self) -> bool {
        matches!(self, Self::Forbidden { .. })
    }

    /// L'explication d'un interdit, vide sinon. Askama ne sait pas filtrer sur
    /// une variante d'énumération : le VM lui donne une chaîne.
    pub fn explanation_or_empty(&self) -> &str {
        match self {
            Self::Forbidden { explanation } => explanation,
            _ => "",
        }
    }

    /// Le texte porté par le bouton, quel que soit son état.
    pub fn label(&self) -> &str {
        match self {
            Self::Enabled { label } => label,
            Self::Blocked { reason } => reason,
            Self::Forbidden { .. } => "Indisponible",
        }
    }
}

/// Un blocage tient dans la largeur d'un bouton : c'est un état passager, le
/// coach voit dans le tableau ce qui le cause.
fn raison_courte(cause: &DomainError) -> &'static str {
    match cause {
        DomainError::MaxPlayersReached => "Effectif complet",
        DomainError::PositionQuotaReached | DomainError::StaffQuotaReached => "Quota atteint",
        DomainError::CrossLimitExceeded => "Limite atteinte",
        DomainError::InsufficientTreasury => "Trésorerie",
        _ => "Indisponible",
    }
}

/// Un interdit occupe une ligne entière, parce qu'il ne se lèvera pas : il faut
/// expliquer, pas signaler.
fn explication_longue(cause: &DomainError) -> &'static str {
    match cause {
        DomainError::StaffNotAllowedForRoster => "Ce roster n'a pas droit à ce personnel.",
        DomainError::StaffTypeNotBuyable => "Ce personnel ne s'achète pas.",
        DomainError::PositionNotInRoster => "Ce poste n'appartient pas au roster de l'équipe.",
        _ => "Indisponible pour cette équipe.",
    }
}

// ── Catalogue ─────────────────────────────────────────────────────────────────

pub struct RecruitmentCatalogVm {
    pub space_id: String,
    pub team_id: String,
    pub context: ContextVm,
    pub positions: Vec<PositionRowVm>,
    pub staff: Vec<StaffRowVm>,
    pub composition: Vec<CompositionRowVm>,
    pub squad_is_full: bool,
    /// Cuite dans les `hx-vals` de chaque bouton : le geste vaut pour l'état
    /// que le coach a sous les yeux, pas pour un autre.
    pub version: u32,
    pub concurrent_notice: bool,
}

impl RecruitmentCatalogVm {
    pub fn from_domain(team: &Team, basket: &RecruitmentBasket, space_id: &str) -> Self {
        Self {
            space_id: space_id.to_string(),
            team_id: team.id.to_string(),
            context: ContextVm::from_domain(team, basket),
            positions: PositionRowVm::all_from_domain(basket),
            staff: StaffRowVm::all_from_domain(basket),
            composition: CompositionRowVm::all_from_domain(basket),
            squad_is_full: basket.projected_squad_size() >= SQUAD_MAX as usize,
            version: basket.version().0,
            concurrent_notice: false,
        }
    }

    pub fn add_player_url(&self) -> String {
        Routes.recruitment_add_player(&self.space_id, &self.team_id)
    }

    pub fn add_staff_url(&self) -> String {
        Routes.recruitment_add_staff(&self.space_id, &self.team_id)
    }

    /// Même fragment, avec le bandeau de resynchronisation. Le geste n'a pas
    /// été appliqué, mais l'écran redevient vrai.
    pub fn with_concurrent_notice(mut self) -> Self {
        self.concurrent_notice = true;
        self
    }
}

pub struct ContextVm {
    pub roster_name: String,
    /// Trésorerie **réelle**, celle de l'équipe. Le reste après achats est
    /// affiché par le panier, sous son propre libellé : deux nombres distincts,
    /// jamais un seul mot qui changerait de sens.
    pub treasury_kpo: u32,
    /// Effectif **projeté** — possédé plus en attente — parce que c'est lui qui
    /// décide si le plafond est atteint.
    pub squad_count: u8,
    pub squad_max: u8,
    pub team_value_kpo: u32,
}

impl ContextVm {
    pub fn from_domain(team: &Team, basket: &RecruitmentBasket) -> Self {
        Self {
            roster_name: team.roster_name.to_string(),
            treasury_kpo: team.treasury.0,
            squad_count: basket.projected_squad_size() as u8,
            squad_max: SQUAD_MAX,
            team_value_kpo: team.team_value.0,
        }
    }
}

pub struct PositionRowVm {
    pub line_id: String,
    pub name: String,
    pub stats: String,
    pub skills: Vec<SkillBadgeVm>,
    pub owned: u8,
    pub pending: u8,
    pub max: u8,
    pub price_kpo: u32,
    pub action: ActionVm,
}

impl PositionRowVm {
    pub fn all_from_domain(basket: &RecruitmentBasket) -> Vec<Self> {
        basket
            .catalog()
            .positions
            .iter()
            .map(|p| Self {
                line_id: p.uid.0.clone(),
                name: p.position_name.clone(),
                stats: format_stats(p.ma, p.st, p.ag, p.pa, p.av),
                skills: SkillBadgeVm::all_from_domain(&p.skills),
                owned: basket.owned_at(&p.uid) as u8,
                pending: basket.pending_for_position(&p.uid) as u8,
                max: p.max_quantity,
                price_kpo: p.cost.0,
                action: ActionVm::from_domain(&basket.action_for_position(&p.uid), "Recruter"),
            })
            .collect()
    }
}

/// Même convention d'affichage que la fiche équipe : une pastille `skill-tag`
/// colorée par sa catégorie.
pub struct SkillBadgeVm {
    pub name: String,
    pub category_css: String,
}

impl SkillBadgeVm {
    pub fn all_from_domain(skills: &[SkillBadge]) -> Vec<Self> {
        skills
            .iter()
            .map(|s| Self {
                name: s.name.clone(),
                category_css: categorie_css(&s.category).to_string(),
            })
            .collect()
    }
}

/// Le corpus nomme la catégorie des mutations `MUTATIONS`, au pluriel, et
/// connaît aussi `DEVIOUS` et `TRAITS` — trois catégories que la table de
/// `players` ne couvre pas et qui y retombent en « général ».
fn categorie_css(categorie: &str) -> &'static str {
    match categorie {
        "STRENGTH" => "type-strength",
        "AGILITY" => "type-agility",
        "PASSING" => "type-passing",
        "MUTATIONS" | "MUTATION" => "type-mutation",
        "DEVIOUS" => "type-devious",
        "TRAITS" => "type-trait",
        _ => "type-general",
    }
}

/// `MA/ST/AG+/PA+/AV+` — les trois dernières sont des seuils de dé, d'où le
/// `+`. Format repris de la maquette validée.
fn format_stats(ma: u8, st: u8, ag: u8, pa: u8, av: u8) -> String {
    format!("{ma}/{st}/{ag}+/{pa}+/{av}+")
}

pub struct StaffRowVm {
    pub staff_uid: String,
    pub name: String,
    pub owned: u8,
    pub pending: u8,
    pub max: u8,
    pub price_kpo: u32,
    /// Renseigné pour la seule relance : le prix affiché est celui de saison,
    /// le double, et le prix de base est rappelé dessous.
    pub base_price_kpo: Option<u32>,
    pub action: ActionVm,
}

impl StaffRowVm {
    pub fn all_from_domain(basket: &RecruitmentBasket) -> Vec<Self> {
        STAFF_ORDER
            .iter()
            .map(|staff| Self {
                staff_uid: form_uid(*staff).to_string(),
                name: nom_staff(*staff).to_string(),
                owned: basket.owned_staff().count_of(*staff) as u8,
                pending: basket.pending_staff_count(*staff) as u8,
                max: quota_staff(basket, *staff),
                price_kpo: basket.price_for(*staff).0,
                base_price_kpo: match staff {
                    StaffType::Reroll => Some(basket.catalog().reroll_base_cost.0),
                    _ => None,
                },
                action: ActionVm::from_domain(&basket.action_for_staff(*staff), "Acheter"),
            })
            .collect()
    }
}

/// Le quota affiché. La relance n'est pas une ligne du corpus de référence :
/// son plafond est une constante du domaine, que l'agrégat expose à travers
/// l'état de son bouton — ici on affiche le plafond de jeu.
fn quota_staff(basket: &RecruitmentBasket, staff: StaffType) -> u8 {
    match staff {
        StaffType::Reroll => 8,
        autre => crate::app::teams::domain::recruitment_basket::staff_uid(autre)
            .and_then(|uid| basket.catalog().staff_entry(uid))
            .map(|e| e.max_quantity as u8)
            .unwrap_or(0),
    }
}

/// L'identifiant que le formulaire renvoie. Il désigne un `StaffType`, pas une
/// ligne de `staff_fr.json` : la relance n'a pas d'uid dans le corpus.
fn form_uid(staff: StaffType) -> &'static str {
    match staff {
        StaffType::Reroll => "REROLL",
        StaffType::Apothecary => "APOTHECARY",
        StaffType::Assistant => "ASSISTANT",
        StaffType::Cheerleader => "CHEERLEADER",
        StaffType::FansFactor => "FANS_FACTOR",
    }
}

pub fn staff_type_from_form(uid: &str) -> Option<StaffType> {
    match uid {
        "REROLL" => Some(StaffType::Reroll),
        "APOTHECARY" => Some(StaffType::Apothecary),
        "ASSISTANT" => Some(StaffType::Assistant),
        "CHEERLEADER" => Some(StaffType::Cheerleader),
        _ => None,
    }
}

fn nom_staff(staff: StaffType) -> &'static str {
    match staff {
        StaffType::Reroll => "Relance",
        StaffType::Apothecary => "Apothicaire",
        StaffType::Assistant => "Assistant entraîneur",
        StaffType::Cheerleader => "Pom-pom girl",
        StaffType::FansFactor => "Facteur fans",
    }
}

pub struct CompositionRowVm {
    pub name: String,
    pub owned: u8,
    pub pending: u8,
    pub max: u8,
    pub owned_pct: u8,
    pub pending_pct: u8,
}

impl CompositionRowVm {
    pub fn all_from_domain(basket: &RecruitmentBasket) -> Vec<Self> {
        basket
            .catalog()
            .positions
            .iter()
            .map(|p| {
                let owned = basket.owned_at(&p.uid) as u8;
                let pending = basket.pending_for_position(&p.uid) as u8;
                Self {
                    name: p.position_name.clone(),
                    owned,
                    pending,
                    max: p.max_quantity,
                    owned_pct: pourcentage(owned, p.max_quantity),
                    pending_pct: pourcentage(pending, p.max_quantity),
                }
            })
            .collect()
    }
}

/// Arrondi au pas de 5. Les styles en ligne étant interdits, la largeur de
/// barre est portée par une classe : vingt-et-une suffisent, là où un
/// pourcentage exact en demanderait cent-une.
fn pourcentage(part: u8, total: u8) -> u8 {
    if total == 0 {
        return 0;
    }
    let brut = ((part as u32 * 100) / total as u32).min(100);
    (brut.div_ceil(5) * 5).min(100) as u8
}

// ── Panier ────────────────────────────────────────────────────────────────────

/// En dessous de ce reste, le panier passe en couleur de risque.
const SEUIL_RESTE_FAIBLE: u32 = 50;

pub struct RecruitmentCartVm {
    pub space_id: String,
    pub team_id: String,
    pub lines: Vec<CartLineVm>,
    pub remaining_kpo: u32,
    pub is_low: bool,
    pub cta_label: String,
    pub version: u32,
    pub concurrent_notice: bool,
}

impl RecruitmentCartVm {
    pub fn from_domain(team: &Team, basket: &RecruitmentBasket, space_id: &str) -> Self {
        let remaining = basket.remaining_treasury().0;
        Self {
            space_id: space_id.to_string(),
            team_id: team.id.to_string(),
            lines: CartLineVm::all_from_domain(basket),
            remaining_kpo: remaining,
            is_low: remaining < SEUIL_RESTE_FAIBLE,
            cta_label: libelle_cta(basket.lines().len()),
            version: basket.version().0,
            concurrent_notice: false,
        }
    }

    pub fn remove_player_url(&self) -> String {
        Routes.recruitment_remove_player(&self.space_id, &self.team_id)
    }

    pub fn remove_staff_url(&self) -> String {
        Routes.recruitment_remove_staff(&self.space_id, &self.team_id)
    }

    pub fn validate_url(&self) -> String {
        Routes.validate_recruitment_phase(&self.space_id, &self.team_id)
    }

    pub fn with_concurrent_notice(mut self) -> Self {
        self.concurrent_notice = true;
        self
    }
}

fn libelle_cta(nombre: usize) -> String {
    match nombre {
        0 => "Terminer les achats sans rien acheter →".to_string(),
        1 => "Valider 1 achat →".to_string(),
        n => format!("Valider {n} achats →"),
    }
}

pub struct CartLineVm {
    pub line_id: String,
    pub label: String,
    pub price_kpo: u32,
    /// Le retrait a deux routes, une par famille de ligne. Le panier sait
    /// laquelle est laquelle ; le client n'a pas à le redéclarer.
    pub is_staff: bool,
    /// Vrai pour la dernière ligne : la maquette la souligne brièvement pour
    /// que le coach voie ce qu'il vient d'ajouter.
    pub is_last: bool,
}

impl CartLineVm {
    pub fn all_from_domain(basket: &RecruitmentBasket) -> Vec<Self> {
        let dernier = basket.lines().len().saturating_sub(1);
        basket
            .lines()
            .iter()
            .enumerate()
            .map(|(i, ligne)| match ligne {
                BasketLine::Player {
                    id,
                    roster_line,
                    price,
                } => Self {
                    line_id: id.0.clone(),
                    label: nom_du_poste(basket, roster_line),
                    price_kpo: price.0,
                    is_staff: false,
                    is_last: i == dernier,
                },
                BasketLine::Staff {
                    id,
                    staff_type,
                    price,
                } => Self {
                    line_id: id.0.clone(),
                    label: nom_staff(*staff_type).to_string(),
                    price_kpo: price.0,
                    is_staff: true,
                    is_last: i == dernier,
                },
            })
            .collect()
    }
}

/// Un poste disparu du catalogue depuis la constitution du panier reste
/// affiché sous son identifiant plutôt que d'escamoter la ligne : le coach doit
/// pouvoir la retirer.
fn nom_du_poste(basket: &RecruitmentBasket, line: &RosterLineId) -> String {
    basket
        .catalog()
        .position(line)
        .map(|p| p.position_name.clone())
        .unwrap_or_else(|| line.0.clone())
}

// ── Erreur ────────────────────────────────────────────────────────────────────

pub struct BasketErrorVm {
    pub message: String,
    /// Les lignes fautives, au refus en bloc. Vide pour une erreur simple.
    pub lines: Vec<String>,
}

impl BasketErrorVm {
    pub fn from_domain(cause: &DomainError) -> Self {
        Self {
            message: explication_longue_ou_courte(cause),
            lines: Vec::new(),
        }
    }

    /// Refus en bloc : rien n'a été appliqué, et on nomme les coupables. Sans
    /// ça le coach cliquerait « Valider » sans rien voir se passer.
    pub fn refus_en_bloc(lines: Vec<String>) -> Self {
        Self {
            message: "Le panier ne passe plus : rien n'a été acheté. Retirez ou \
                      corrigez les lignes ci-dessous, puis validez à nouveau."
                .into(),
            lines,
        }
    }

    /// La cause d'une ligne rejetée, formulée. Le domaine l'a décidée.
    pub fn raison_de(cause: &DomainError) -> String {
        explication_longue_ou_courte(cause)
    }
}

/// Un message d'erreur est lu isolément, sans la ligne du tableau qui lui
/// donnerait son contexte : il prend l'explication longue quand elle existe.
fn explication_longue_ou_courte(cause: &DomainError) -> String {
    match cause {
        DomainError::StaffNotAllowedForRoster
        | DomainError::StaffTypeNotBuyable
        | DomainError::PositionNotInRoster => explication_longue(cause).to_string(),
        DomainError::MaxPlayersReached => "L'effectif est complet : 16 joueurs au maximum.".into(),
        DomainError::PositionQuotaReached => "Le quota de ce poste est atteint.".into(),
        DomainError::StaffQuotaReached => "Le quota de ce personnel est atteint.".into(),
        DomainError::CrossLimitExceeded => {
            "La limite de cumul entre ces postes est atteinte.".into()
        }
        DomainError::InsufficientTreasury => "Trésorerie insuffisante.".into(),
        DomainError::BasketLineNotFound => "Cette ligne n'est plus dans le panier.".into(),
        autre => autre.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::teams::domain::recruitment_basket::{
        BasketLineId, BasketVersion, CatalogPosition, OwnedStaff, RosterCatalog, SquadMember,
        SquadSnapshot, StaffCatalogEntry,
    };
    use crate::app::teams::domain::value_objects::Kpo;

    const TROIS_QUART: &str = "WOOD_ELF__WOOD_ELF_LINEMAN";
    const DANSEUR: &str = "WOOD_ELF__WARDANCER";

    fn poste(uid: &str, nom: &str, cout: u32, max: u8) -> CatalogPosition {
        CatalogPosition {
            uid: RosterLineId(uid.into()),
            position_name: nom.into(),
            cost: Kpo(cout),
            max_quantity: max,
            ma: 7,
            st: 3,
            ag: 2,
            pa: 3,
            av: 8,
            skills: vec![SkillBadge {
                name: "Bloc".into(),
                category: "AGILITY".into(),
            }],
        }
    }

    /// Roster à deux postes, avec apothicaire **refusé** : c'est ce qui permet
    /// d'éprouver `Forbidden` sans le fabriquer à la main.
    fn catalogue() -> RosterCatalog {
        RosterCatalog {
            positions: vec![
                poste(TROIS_QUART, "Trois-quart", 65, 16),
                poste(DANSEUR, "Danseur de Guerre", 130, 2),
            ],
            cross_limits: vec![],
            allowed_staff: vec!["CHEERLEADERS".into()],
            staff: vec![StaffCatalogEntry {
                uid: "CHEERLEADERS".into(),
                price: Kpo(10),
                max_quantity: 6,
            }],
            reroll_base_cost: Kpo(50),
        }
    }

    fn panier(effectif: &[&str], lignes: Vec<BasketLine>, tresorerie: u32) -> RecruitmentBasket {
        RecruitmentBasket::hydrate(
            "team-1".into(),
            BasketVersion(3),
            lignes,
            catalogue(),
            SquadSnapshot {
                members: effectif
                    .iter()
                    .map(|l| SquadMember {
                        roster_line: RosterLineId((*l).into()),
                    })
                    .collect(),
            },
            OwnedStaff::default(),
            Kpo(tresorerie),
        )
    }

    fn ligne_joueur(uid: &str, prix: u32) -> BasketLine {
        BasketLine::Player {
            id: BasketLineId(format!("l-{uid}-{prix}")),
            roster_line: RosterLineId(uid.into()),
            price: Kpo(prix),
        }
    }

    fn poste_vm(basket: &RecruitmentBasket, uid: &str) -> PositionRowVm {
        PositionRowVm::all_from_domain(basket)
            .into_iter()
            .find(|p| p.line_id == uid)
            .expect("poste présent")
    }

    #[test]
    fn les_caracteristiques_suivent_le_format_de_la_maquette() {
        let vm = poste_vm(&panier(&[], vec![], 1000), TROIS_QUART);
        assert_eq!(vm.stats, "7/3/2+/3+/8+");
    }

    /// Le quota compte l'effectif possédé **et** les lignes en attente : c'est
    /// ce qui empêche d'empiler un poste saturé dans le panier.
    #[test]
    fn un_poste_sature_par_le_panier_affiche_quota_atteint() {
        let basket = panier(&[DANSEUR], vec![ligne_joueur(DANSEUR, 130)], 1000);
        let vm = poste_vm(&basket, DANSEUR);

        assert_eq!(vm.owned, 1);
        assert_eq!(vm.pending, 1);
        assert_eq!(
            vm.action,
            ActionVm::Blocked {
                reason: "Quota atteint".into()
            }
        );
    }

    #[test]
    fn une_tresorerie_insuffisante_le_dit_sans_parler_de_quota() {
        let vm = poste_vm(&panier(&[], vec![], 60), DANSEUR);
        assert_eq!(
            vm.action,
            ActionVm::Blocked {
                reason: "Trésorerie".into()
            }
        );
    }

    /// Un quota se libère, un roster n'acquiert jamais le droit à un
    /// apothicaire : les deux états restent distincts jusque dans la vue.
    #[test]
    fn un_staff_refuse_au_roster_est_interdit_et_non_bloque() {
        let staff = StaffRowVm::all_from_domain(&panier(&[], vec![], 1000));
        let apo = staff.iter().find(|s| s.staff_uid == "APOTHECARY").unwrap();

        assert!(apo.action.is_forbidden());
        assert_eq!(apo.action.label(), "Indisponible");
        assert_eq!(
            apo.action,
            ActionVm::Forbidden {
                explanation: "Ce roster n'a pas droit à ce personnel.".into()
            }
        );
    }

    /// La relance est tarifée au double hors création, et le prix de base est
    /// rappelé — c'est lui que comptera la valeur d'équipe.
    #[test]
    fn la_relance_affiche_le_prix_de_saison_et_rappelle_sa_base() {
        let staff = StaffRowVm::all_from_domain(&panier(&[], vec![], 1000));
        let relance = staff.iter().find(|s| s.staff_uid == "REROLL").unwrap();

        assert_eq!(relance.price_kpo, 100);
        assert_eq!(relance.base_price_kpo, Some(50));
        assert_eq!(relance.max, 8);
    }

    #[test]
    fn le_libelle_du_bouton_de_validation_accorde_son_pluriel() {
        assert_eq!(libelle_cta(0), "Terminer les achats sans rien acheter →");
        assert_eq!(libelle_cta(1), "Valider 1 achat →");
        assert_eq!(libelle_cta(4), "Valider 4 achats →");
    }

    #[test]
    fn la_composition_distingue_la_part_possedee_de_la_part_en_attente() {
        let basket = panier(&[DANSEUR], vec![ligne_joueur(DANSEUR, 130)], 1000);
        let rows = CompositionRowVm::all_from_domain(&basket);
        let danseur = rows.iter().find(|r| r.name == "Danseur de Guerre").unwrap();

        assert_eq!((danseur.owned, danseur.pending, danseur.max), (1, 1, 2));
        assert_eq!((danseur.owned_pct, danseur.pending_pct), (50, 50));
    }

    /// Le panier nomme ses lignes depuis le catalogue du jour ; une ligne qui
    /// n'y correspond plus reste affichée sous son identifiant, pour que le
    /// coach puisse la retirer.
    #[test]
    fn une_ligne_dont_le_poste_a_disparu_reste_retirable() {
        let basket = panier(&[], vec![ligne_joueur("POSTE_DISPARU", 40)], 1000);
        let lignes = CartLineVm::all_from_domain(&basket);

        assert_eq!(lignes.len(), 1);
        assert_eq!(lignes[0].label, "POSTE_DISPARU");
        assert!(lignes[0].is_last);
    }
}

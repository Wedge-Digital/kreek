//! Les view models de la phase de renvois.
//!
//! Fichier distinct de `view_models.rs`, qui porte déjà les sept VMs du
//! recrutement : les deux phases ne partagent que des libellés de staff, pas
//! leurs projections. Ce qui est commun est importé, pas recopié.
//!
//! Comme au recrutement, c'est ici — et nulle part ailleurs — qu'un
//! `DomainError` devient une phrase française. Le domaine décide qu'un bouton
//! est bloqué et pourquoi ; la vue se contente de le formuler.

use crate::app::teams::domain::basket::Player;
use crate::app::teams::domain::dismissals_basket::{
    DismissalActionState, DismissalBasketLine, DismissalsBasket,
};
use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::team::Team;
use crate::app::teams::domain::value_objects::StaffType;
use crate::app::teams::io::web::view_models::{form_uid, nom_staff, STAFF_ORDER};
use crate::app::teams::routes::Routes;

/// Le plancher, pour l'affichage. Celui qui *décide* vit dans l'agrégat ;
/// celui-ci ne fait que le nommer.
const MIN_ELIGIBLE: u8 = 11;

// ── L'état d'un bouton ────────────────────────────────────────────────────────

/// Trois cas, contre deux au recrutement.
///
/// `Marked` porte la ligne qui l'a marqué : c'est ce qui permet au bouton
/// « Annuler » de la ligne du joueur de poster exactement le même `line_id` que
/// le « × » du panier, donc de passer par le même use case.
#[derive(Debug, Clone, PartialEq)]
pub enum DismissalActionVm {
    Removable { label: String },
    Marked { label: String, line_id: String },
    Blocked { reason: String },
}

impl DismissalActionVm {
    pub fn is_removable(&self) -> bool {
        matches!(self, Self::Removable { .. })
    }
    pub fn is_marked(&self) -> bool {
        matches!(self, Self::Marked { .. })
    }
    pub fn label(&self) -> &str {
        match self {
            Self::Removable { label } | Self::Marked { label, .. } => label,
            Self::Blocked { reason } => reason,
        }
    }
    /// La ligne à annuler, vide hors de l'état marqué. Askama ne sait pas
    /// filtrer sur une variante : le VM lui donne une chaîne.
    pub fn line_id_or_empty(&self) -> &str {
        match self {
            Self::Marked { line_id, .. } => line_id,
            _ => "",
        }
    }
}

/// Un blocage tient dans la largeur d'un bouton. Il n'y en a qu'un ici qui
/// mérite un nom — le plancher ; les autres ne devraient pas s'afficher.
fn raison_courte(cause: &DomainError) -> String {
    match cause {
        DomainError::EligibleFloorReached => format!("Minimum {MIN_ELIGIBLE}"),
        DomainError::InsufficientStaff => "Aucun".to_string(),
        _ => "Indisponible".to_string(),
    }
}

fn action_vm(
    state: &DismissalActionState,
    label: &str,
    line_id: Option<String>,
) -> DismissalActionVm {
    match state {
        DismissalActionState::Removable => DismissalActionVm::Removable {
            label: label.to_string(),
        },
        DismissalActionState::Marked => DismissalActionVm::Marked {
            label: "Annuler".to_string(),
            line_id: line_id.unwrap_or_default(),
        },
        DismissalActionState::Blocked { cause } => DismissalActionVm::Blocked {
            reason: raison_courte(cause),
        },
    }
}

// ── L'effectif ────────────────────────────────────────────────────────────────

pub struct DismissalsRosterVm {
    pub space_id: String,
    pub team_id: String,
    pub team_name: String,
    pub context: DismissalsContextVm,
    pub players: Vec<PlayerRowVm>,
    pub staff: Vec<StaffDismissalRowVm>,
    /// Cuite dans les `hx-vals` de chaque bouton : le geste vaut pour l'état
    /// que le coach a sous les yeux, pas pour un autre.
    pub version: u32,
    pub concurrent_notice: bool,
}

impl DismissalsRosterVm {
    pub fn from_domain(team: &Team, basket: &DismissalsBasket, space_id: &str) -> Self {
        Self {
            space_id: space_id.to_string(),
            team_id: team.id.to_string(),
            team_name: team.name.to_string(),
            context: DismissalsContextVm::from_domain(team, basket),
            players: PlayerRowVm::all_from_domain(basket),
            staff: StaffDismissalRowVm::all_from_domain(basket),
            version: basket.version().0,
            concurrent_notice: false,
        }
    }

    pub fn mark_player_url(&self) -> String {
        Routes.dismissals_mark_player(&self.space_id, &self.team_id)
    }
    pub fn unmark_player_url(&self) -> String {
        Routes.dismissals_unmark_player(&self.space_id, &self.team_id)
    }
    pub fn mark_staff_url(&self) -> String {
        Routes.dismissals_mark_staff(&self.space_id, &self.team_id)
    }

    pub fn with_concurrent_notice(mut self) -> Self {
        self.concurrent_notice = true;
        self
    }
}

pub struct DismissalsContextVm {
    pub roster_name: String,
    /// Affichée bien qu'aucun renvoi ne la touche : le coach a besoin de la
    /// voir pour comprendre qu'elle **ne bougera pas**.
    pub treasury_kpo: u32,
    /// Effectif après renvois — c'est ce que le panier promet.
    pub squad_count: u8,
    /// Distinct de `squad_count`, et c'est lui qui gouverne le plancher.
    /// Les confondre rendrait le blocage incompréhensible.
    pub eligible_count: u8,
    pub eligible_is_low: bool,
}

impl DismissalsContextVm {
    pub fn from_domain(team: &Team, basket: &DismissalsBasket) -> Self {
        let eligible = basket.eligible_after_basket() as u8;
        Self {
            roster_name: team.roster_name.to_string(),
            treasury_kpo: team.treasury.0,
            squad_count: squad_after(basket),
            eligible_count: eligible,
            eligible_is_low: eligible < MIN_ELIGIBLE,
        }
    }
}

/// L'effectif tel qu'il sera si le panier est validé.
fn squad_after(basket: &DismissalsBasket) -> u8 {
    let marques = basket
        .lines()
        .iter()
        .filter(|l| matches!(l, DismissalBasketLine::Player { .. }))
        .count();
    basket.squad().size().saturating_sub(marques) as u8
}

pub struct PlayerRowVm {
    pub player_id: String,
    /// `None` tant qu'aucun maillot n'a été attribué : la vue affiche un tiret
    /// plutôt qu'un zéro, qui se lirait comme un numéro.
    pub number: Option<u8>,
    pub name: String,
    pub position: String,
    pub spp: u32,
    pub value_kpo: u32,
    pub is_available: bool,
    /// La ligne est barrée, jamais estompée : c'est la trace de ce que le coach
    /// vient de décider, elle doit rester lisible.
    pub is_marked: bool,
    pub action: DismissalActionVm,
}

impl PlayerRowVm {
    pub fn all_from_domain(basket: &DismissalsBasket) -> Vec<Self> {
        // `occupants()` et non `members` : un joueur dont la place est déjà
        // libre n'a plus rien à faire ici. Le renvoyer ne libérerait rien, et
        // l'afficher laisserait croire qu'il faut le faire.
        basket
            .squad()
            .occupants()
            .map(|p| Self::from_domain(p, basket))
            .collect()
    }

    fn from_domain(player: &Player, basket: &DismissalsBasket) -> Self {
        let etat = basket.action_for_player(&player.player_id);
        let ligne = basket.line_id_for(&player.player_id).map(|id| id.0.clone());
        Self {
            player_id: player.player_id.to_string(),
            number: player.jersey,
            name: nom_affiche(player),
            position: player.position_name.clone(),
            spp: player.spp,
            value_kpo: player.value_kpo.0,
            is_available: player.presence.alignable(),
            is_marked: matches!(etat, DismissalActionState::Marked),
            action: action_vm(&etat, "Renvoyer", ligne),
        }
    }

    /// « Disponible » ou « Absent » — le seul vocabulaire de participation que
    /// `teams` connaisse, l'adapter ayant traduit celui de `players`.
    pub fn status_label(&self) -> &'static str {
        if self.is_available {
            "Disponible"
        } else {
            "Absent"
        }
    }
}

/// Un joueur qu'aucun coach n'a nommé s'affiche par son poste — même convention
/// que la table de `players`, où un nom vide n'a jamais laissé de case blanche.
fn nom_affiche(player: &Player) -> String {
    if player.personal_name.is_empty() {
        player.position_name.clone()
    } else {
        player.personal_name.clone()
    }
}

pub struct StaffDismissalRowVm {
    pub staff_uid: String,
    pub name: String,
    /// Ce qu'il **restera** après application du panier.
    pub remaining: u8,
    /// Affiché « −N » : ce que le panier va retirer.
    pub pending: u8,
    pub unit_value_kpo: u32,
    pub action: DismissalActionVm,
}

impl StaffDismissalRowVm {
    /// Le staff qu'une équipe ne peut pas posséder n'apparaît pas — non parce
    /// que le renvoi serait interdit, mais parce qu'il n'y a rien à renvoyer.
    /// Le facteur fans, lui, ne se renvoie jamais.
    pub fn all_from_domain(basket: &DismissalsBasket) -> Vec<Self> {
        STAFF_ORDER
            .iter()
            .filter(|s| {
                basket.owned_staff().count_of(**s) > 0 || basket.pending_staff_count(**s) > 0
            })
            .map(|s| Self::from_domain(*s, basket))
            .collect()
    }

    fn from_domain(staff: StaffType, basket: &DismissalsBasket) -> Self {
        let possede = basket.owned_staff().count_of(staff);
        let en_attente = basket.pending_staff_count(staff);
        Self {
            staff_uid: form_uid(staff).to_string(),
            name: nom_staff(staff).to_string(),
            remaining: possede.saturating_sub(en_attente) as u8,
            pending: en_attente as u8,
            unit_value_kpo: prix_unitaire(basket, staff),
            action: action_vm(&basket.action_for_staff(staff), "Renvoyer", None),
        }
    }
}

/// La valeur affichée est celle du catalogue, sans le doublement du
/// recrutement : ce n'est pas un prix d'achat, c'est ce que l'équipe perd.
fn prix_unitaire(basket: &DismissalsBasket, staff: StaffType) -> u32 {
    match staff {
        StaffType::Reroll => basket.catalog().reroll_base_cost.0,
        autre => crate::app::teams::domain::basket::staff_uid(autre)
            .and_then(|uid| basket.catalog().staff_entry(uid))
            .map(|e| e.price.0)
            .unwrap_or(0),
    }
}

// ── Le panier ─────────────────────────────────────────────────────────────────

pub struct DismissalsCartVm {
    pub space_id: String,
    pub team_id: String,
    pub lines: Vec<DismissalCartLineVm>,
    pub squad_after: u8,
    pub eligible_after: u8,
    /// Le nombre de journaliers qu'il faudra aligner. Zéro dès qu'on a onze
    /// éligibles — l'alerte ne signale plus une conséquence des renvois, que le
    /// plancher interdit, mais un déficit **déjà causé par les blessures**.
    pub journeymen_needed: u8,
    pub cta_label: String,
    /// Rouge seulement quand le bouton va réellement détruire quelque chose.
    pub cta_destructive: bool,
    pub version: u32,
    pub concurrent_notice: bool,
}

impl DismissalsCartVm {
    pub fn from_domain(basket: &DismissalsBasket, space_id: &str) -> Self {
        let eligible = basket.eligible_after_basket() as u8;
        let lignes = DismissalCartLineVm::all_from_domain(basket);
        Self {
            space_id: space_id.to_string(),
            team_id: basket.team_id().to_string(),
            cta_label: libelle_cta(lignes.len()),
            cta_destructive: !lignes.is_empty(),
            lines: lignes,
            squad_after: squad_after(basket),
            eligible_after: eligible,
            journeymen_needed: MIN_ELIGIBLE.saturating_sub(eligible),
            version: basket.version().0,
            concurrent_notice: false,
        }
    }

    pub fn unmark_player_url(&self) -> String {
        Routes.dismissals_unmark_player(&self.space_id, &self.team_id)
    }
    pub fn unmark_staff_url(&self) -> String {
        Routes.dismissals_unmark_staff(&self.space_id, &self.team_id)
    }
    pub fn validate_url(&self) -> String {
        Routes.validate_dismissals_phase(&self.space_id, &self.team_id)
    }

    pub fn with_concurrent_notice(mut self) -> Self {
        self.concurrent_notice = true;
        self
    }
}

fn libelle_cta(nombre: usize) -> String {
    match nombre {
        0 => "Valider sans renvoyer personne →".to_string(),
        1 => "Valider 1 renvoi →".to_string(),
        n => format!("Valider {n} renvois →"),
    }
}

pub struct DismissalCartLineVm {
    pub line_id: String,
    pub label: String,
    /// Le poste pour un joueur, « staff » pour le reste : de quoi distinguer
    /// deux lignes homonymes sans alourdir le panier.
    pub detail: String,
    /// L'annulation a deux routes, une par famille de ligne, parce qu'elles ne
    /// rendent pas le même fragment. Le panier sait laquelle est laquelle.
    pub is_staff: bool,
    /// Vrai pour la dernière : la maquette la souligne brièvement, pour que le
    /// coach voie ce qu'il vient de marquer.
    pub is_last: bool,
}

impl DismissalCartLineVm {
    pub fn all_from_domain(basket: &DismissalsBasket) -> Vec<Self> {
        let dernier = basket.lines().len().saturating_sub(1);
        basket
            .lines()
            .iter()
            .enumerate()
            .map(|(i, ligne)| match ligne {
                DismissalBasketLine::Player { id, player_id } => {
                    let joueur = basket.squad().find(player_id);
                    Self {
                        line_id: id.0.clone(),
                        label: joueur
                            .map(|p| p.personal_name.clone())
                            .unwrap_or_else(|| "Joueur retiré de l'effectif".to_string()),
                        detail: joueur.map(|p| p.position_name.clone()).unwrap_or_default(),
                        is_staff: false,
                        is_last: i == dernier,
                    }
                }
                DismissalBasketLine::Staff { id, staff_type } => Self {
                    line_id: id.0.clone(),
                    label: nom_staff(*staff_type).to_string(),
                    detail: "staff".to_string(),
                    is_staff: true,
                    is_last: i == dernier,
                },
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
    use crate::app::teams::domain::basket::SquadPresence;
    use crate::app::teams::domain::basket::{
        BasketVersion, OwnedStaff, RosterCatalog, RosterLineId, Squad, StaffCatalogEntry,
    };
    use crate::app::teams::domain::value_objects::Kpo;

    fn catalogue() -> RosterCatalog {
        RosterCatalog {
            positions: Vec::new(),
            cross_limits: Vec::new(),
            allowed_staff: vec!["APOTHECARY".into()],
            staff: vec![StaffCatalogEntry {
                uid: "APOTHECARY".into(),
                price: Kpo(50),
                max_quantity: 1,
            }],
            reroll_base_cost: Kpo(60),
        }
    }

    fn id_de(n: u8) -> PlayerId {
        PlayerId::try_new(&format!("{n:0>26}")).unwrap()
    }

    fn joueur(n: u8, disponible: bool) -> Player {
        joueur_present(
            n,
            if disponible {
                SquadPresence::Alignable
            } else {
                SquadPresence::Empeche
            },
        )
    }

    fn joueur_present(n: u8, presence: SquadPresence) -> Player {
        Player {
            player_id: id_de(n),
            roster_line: RosterLineId("DEMO_GRANIT__PIETAILLE".into()),
            jersey: Some(n + 1),
            personal_name: format!("Joueur {n}"),
            position_name: "Piétaille".into(),
            spp: 3,
            value_kpo: Kpo(50),
            presence,
        }
    }

    fn panier(disponibles: u8, absents: u8, staff: OwnedStaff) -> DismissalsBasket {
        panier_avec(disponibles, absents, staff, Vec::new())
    }

    /// Même panier, plus des membres dont on choisit la présence. Passe par
    /// l'hydratation comme les autres : `squad` est privé, et c'est très bien —
    /// un test qui force l'état interne n'éprouve pas le chemin réel.
    fn panier_avec(
        disponibles: u8,
        absents: u8,
        staff: OwnedStaff,
        extras: Vec<Player>,
    ) -> DismissalsBasket {
        let mut membres: Vec<Player> = (0..disponibles).map(|n| joueur(n, true)).collect();
        membres.extend((disponibles..disponibles + absents).map(|n| joueur(n, false)));
        membres.extend(extras);
        DismissalsBasket::hydrate(
            "team".into(),
            BasketVersion(0),
            Vec::new(),
            Squad { members: membres },
            catalogue(),
            staff,
        )
    }

    /// La feuille des renvois ne propose que les occupants. Un mort n'y figure
    /// plus : sa place est déjà libre, le renvoyer ne rendrait rien, et
    /// l'afficher laisserait croire qu'il faut le faire.
    #[test]
    fn un_perdu_ne_figure_pas_dans_la_liste_des_renvois() {
        let p = panier_avec(
            3,
            0,
            OwnedStaff::default(),
            vec![joueur_present(9, SquadPresence::Perdu)],
        );

        let lignes = PlayerRowVm::all_from_domain(&p);

        assert_eq!(lignes.len(), 3, "le perdu n'est pas une ligne renvoyable");
        assert!(!lignes.iter().any(|l| l.player_id == id_de(9).to_string()));
    }

    /// Le pendant : un absent reste proposé. Il occupe sa place, et le renvoyer
    /// est justement le moyen de la libérer.
    #[test]
    fn un_empeche_figure_toujours_dans_la_liste_des_renvois() {
        let p = panier(2, 1, OwnedStaff::default());

        let lignes = PlayerRowVm::all_from_domain(&p);

        assert_eq!(lignes.len(), 3);
        assert_eq!(
            lignes.iter().filter(|l| !l.is_available).count(),
            1,
            "l'absent est affiché, marqué comme tel"
        );
    }

    #[test]
    fn le_bouton_dit_renvoyer_puis_annuler_une_fois_marque() {
        let mut p = panier(13, 0, OwnedStaff::default());
        let lignes = PlayerRowVm::all_from_domain(&p);
        assert!(lignes[0].action.is_removable());
        assert_eq!(lignes[0].action.label(), "Renvoyer");
        assert!(!lignes[0].is_marked);

        let ligne = p.mark_player(id_de(0)).unwrap();

        let lignes = PlayerRowVm::all_from_domain(&p);
        assert!(lignes[0].action.is_marked());
        assert_eq!(lignes[0].action.label(), "Annuler");
        assert!(lignes[0].is_marked, "la ligne est barrée");
        assert_eq!(
            lignes[0].action.line_id_or_empty(),
            ligne.0,
            "le bouton de la ligne poste le même identifiant que le panier"
        );
    }

    #[test]
    fn au_plancher_le_bouton_dit_minimum_11() {
        let p = panier(11, 0, OwnedStaff::default());
        let lignes = PlayerRowVm::all_from_domain(&p);
        assert_eq!(lignes[0].action.label(), "Minimum 11");
    }

    /// Un absent reste renvoyable sous le plancher : c'est la règle 25, vue
    /// depuis l'écran.
    #[test]
    fn sous_le_plancher_seuls_les_absents_restent_renvoyables() {
        let p = panier(9, 2, OwnedStaff::default());
        let lignes = PlayerRowVm::all_from_domain(&p);

        assert_eq!(lignes[0].action.label(), "Minimum 11");
        assert!(lignes[9].action.is_removable(), "le premier absent");
        assert_eq!(lignes[9].status_label(), "Absent");
    }

    #[test]
    fn l_entete_distingue_effectif_et_eligibles() {
        let mut p = panier(12, 2, OwnedStaff::default());
        p.mark_player(id_de(0)).unwrap();

        let vm = DismissalsCartVm::from_domain(&p, "space");
        assert_eq!(vm.squad_after, 13, "quatorze moins le marqué");
        assert_eq!(vm.eligible_after, 11, "douze disponibles moins le marqué");
        assert_eq!(vm.journeymen_needed, 0, "onze éligibles suffisent");
    }

    /// L'alerte informe d'un déficit déjà causé par les blessures — le plancher
    /// interdit au coach de le creuser lui-même.
    #[test]
    fn l_alerte_journaliers_compte_ce_qui_manque_aux_onze() {
        let p = panier(9, 3, OwnedStaff::default());
        let vm = DismissalsCartVm::from_domain(&p, "space");
        assert_eq!(vm.eligible_after, 9);
        assert_eq!(vm.journeymen_needed, 2);
    }

    #[test]
    fn le_cta_ne_devient_rouge_qu_avec_une_ligne_en_attente() {
        let mut p = panier(13, 0, OwnedStaff::default());
        let vide = DismissalsCartVm::from_domain(&p, "space");
        assert!(!vide.cta_destructive);
        assert_eq!(vide.cta_label, "Valider sans renvoyer personne →");

        p.mark_player(id_de(0)).unwrap();
        let plein = DismissalsCartVm::from_domain(&p, "space");
        assert!(plein.cta_destructive);
        assert_eq!(plein.cta_label, "Valider 1 renvoi →");
    }

    /// Le staff qu'on ne possède pas n'a pas de ligne : il n'y a rien à
    /// renvoyer, et une ligne « Aucun » n'apprendrait rien.
    #[test]
    fn seul_le_staff_possede_apparait() {
        let p = panier(13, 0, OwnedStaff::default());
        assert!(StaffDismissalRowVm::all_from_domain(&p).is_empty());

        let p = panier(
            13,
            0,
            OwnedStaff {
                apothecaries: 1,
                ..OwnedStaff::default()
            },
        );
        let lignes = StaffDismissalRowVm::all_from_domain(&p);
        assert_eq!(lignes.len(), 1);
        assert_eq!(lignes[0].name, "Apothicaire");
        assert_eq!(lignes[0].remaining, 1);
        assert_eq!(lignes[0].unit_value_kpo, 50);
    }

    #[test]
    fn marquer_du_staff_decremente_le_restant_et_affiche_l_attente() {
        let mut p = panier(
            13,
            0,
            OwnedStaff {
                apothecaries: 1,
                ..OwnedStaff::default()
            },
        );
        p.mark_staff(StaffType::Apothecary).unwrap();

        let lignes = StaffDismissalRowVm::all_from_domain(&p);
        assert_eq!(lignes[0].remaining, 0);
        assert_eq!(lignes[0].pending, 1);
        assert_eq!(lignes[0].action.label(), "Aucun", "plus rien à renvoyer");
    }

    /// Un joueur sans nom n'a jamais de case blanche : c'est son poste qui
    /// s'affiche, comme dans la table de `players`.
    #[test]
    fn un_joueur_sans_nom_s_affiche_par_son_poste() {
        let mut membres = vec![joueur(0, true)];
        membres[0].personal_name = String::new();
        let p = DismissalsBasket::hydrate(
            "team".into(),
            BasketVersion(0),
            Vec::new(),
            Squad { members: membres },
            catalogue(),
            OwnedStaff::default(),
        );
        assert_eq!(PlayerRowVm::all_from_domain(&p)[0].name, "Piétaille");
    }

    #[test]
    fn le_panier_nomme_ses_lignes() {
        let mut p = panier(
            13,
            0,
            OwnedStaff {
                apothecaries: 1,
                ..OwnedStaff::default()
            },
        );
        p.mark_player(id_de(0)).unwrap();
        p.mark_staff(StaffType::Apothecary).unwrap();

        let lignes = DismissalCartLineVm::all_from_domain(&p);
        assert_eq!(lignes[0].label, "Joueur 0");
        assert_eq!(lignes[0].detail, "Piétaille");
        assert!(!lignes[0].is_staff);
        assert_eq!(lignes[1].label, "Apothicaire");
        assert!(lignes[1].is_staff);
        assert!(lignes[1].is_last);
    }
}

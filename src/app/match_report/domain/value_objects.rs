use crate::app::match_report::domain::error::DomainError;
use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
use crate::app::shared_kernel::bloodbowl::inducement_definition::InducementId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RosterPositionUid(pub String);

impl RosterPositionUid {
    pub fn try_new(s: &str) -> Result<Self, &'static str> {
        if s.is_empty() {
            Err("position uid vide")
        } else {
            Ok(Self(s.to_string()))
        }
    }
}

impl fmt::Display for RosterPositionUid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsStarPlayer(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchReportOrigin {
    Manual,
    Pairing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct D3Roll(u8);

impl D3Roll {
    pub fn try_new(value: u8) -> Result<Self, DomainError> {
        if (1..=3).contains(&value) {
            Ok(Self(value))
        } else {
            Err(DomainError::InvalidD3Roll(value))
        }
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

#[nutype(
    validate(less_or_equal = 3000),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Serialize,
        Deserialize,
        Display
    )
)]
pub struct TeamValue(u32);

/// Copie du `teams::DedicatedFans` de chaque équipe au moment de l'enregistrement
/// du facteur fans — même borne (≤20), dupliquée ici car `match_report` ne peut
/// pas importer le type domaine de `teams` (règle de souveraineté des BCs).
#[nutype(
    validate(less_or_equal = 20),
    default = 0,
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)
)]
pub struct DedicatedFans(u32);

/// Une quantité **achetée** de coups de pouce identiques.
///
/// Dix est déjà généreux pour un achat. Ce plafond n'a rien à voir avec celui
/// d'un roster, qui vit sur une autre échelle — un trois-quarts s'aligne à
/// seize. Confondre les deux a fait disparaître des mercenaires payés
/// (carte 406) : le plafond porte désormais son propre type.
#[nutype(
    validate(greater_or_equal = 1, less_or_equal = 10),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Serialize,
        Deserialize
    )
)]
pub struct InducementQty(u8);

/// Le plafond que le corpus pose sur un coup de pouce ou un poste.
///
/// Il vient du référentiel, pas d'une saisie : il n'a pas à être borné par le
/// haut, et l'y borner revenait à disqualifier des postes légitimes. Seize pour
/// un trois-quarts, quatre pour un percuteur, un pour un colosse.
///
/// Il ne protège aucun invariant que `validate_max_qty` ne vérifie déjà — mais
/// un `u8` nu se serait confondu de nouveau avec une quantité d'achat, ce qui
/// est précisément le défaut qu'on corrige.
#[nutype(
    validate(greater_or_equal = 1),
    derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Serialize,
        Deserialize
    )
)]
pub struct InducementMaxQty(u8);

#[nutype(
    validate(greater_or_equal = 1),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct InducementCost(u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InducementPurchase {
    pub uid: InducementId,
    pub qty: InducementQty,
    pub unit_cost: InducementCost,
}

impl InducementPurchase {
    pub fn total_cost(&self) -> u32 {
        self.unit_cost.into_inner() * self.qty.into_inner() as u32
    }
}

#[derive(Debug, Clone)]
pub struct AllowedInducementSpec {
    pub uid: InducementId,
    /// Le plafond du **corpus**, jamais une quantité d'achat — cf.
    /// [`InducementMaxQty`].
    pub max_qty: InducementMaxQty,
    pub unit_cost: InducementCost,
    pub is_star_player: IsStarPlayer,
}

// ── step5 : après-match ───────────────────────────────────────────────────────

#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct MatchGain(u32);

/// Ce que les coups de pouce retirent réellement à la trésorerie d'une équipe.
///
/// Zéro est un cas courant — et même le plus courant côté underdog, dont la
/// petite monnaie couvre souvent tout —, d'où l'absence de la borne `> 0` que
/// porte `MatchGain`.
///
/// Calculé au pré-match, quand les valeurs d'équipe et les achats des deux
/// camps sont encore connus : après la publication, l'écart de TV est perdu et
/// le montant ne serait plus reconstructible.
#[nutype(
    default = 0,
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)
)]
pub struct InducementSpending(u32);

#[nutype(
    validate(greater_or_equal = -2, less_or_equal = 2),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct FanFactorMod(i8);

// ── step3-4 : actions match ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TempPlayerId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnNumber(u8);

impl TurnNumber {
    pub fn try_new(value: u8) -> Result<Self, DomainError> {
        if (1..=16).contains(&value) {
            Ok(Self(value))
        } else {
            Err(DomainError::InvalidTurn(value))
        }
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamSide {
    Home,
    Away,
}

/// Verdict sur la possibilité de corriger un rapport déjà publié.
///
/// Calculé hors du domaine — il dépend de l'état d'autres BCs (phase de jeu des
/// équipes, SPP déjà dépensés) — puis remis au domaine, seul habilité à décider
/// si la dépublication est permise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionEligibility {
    Eligible,
    Blocked(CorrectionBlocker),
}

/// Ce qui empêche la correction. Porte un `TeamSide` et non un nom d'équipe :
/// le domaine ignore les chaînes d'affichage, la résolution du nom appartient à
/// la couche de présentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionBlocker {
    /// Un joueur de ce camp a dépensé des SPP depuis le match.
    SppAlreadySpent { side: TeamSide },
    /// Ce camp a quitté la phase d'amélioration des joueurs.
    PhaseAdvanced { side: TeamSide },
    /// Une des consultations nécessaires au verdict n'a pas abouti. On échoue
    /// fermé : autoriser une correction qui aurait dû être refusée laisserait
    /// des données incohérentes, alors qu'un refus indu ne fait que retarder.
    EligibilityUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionPlayer {
    Regular(PlayerId),
    Temp(TempPlayerId),
}

/// La forme d'un identifiant de mot-clef du corpus : `DARK_ELF`, `BEASTMAN`.
///
/// **Ce n'est pas du texte saisi**, et `TEXTE_SAISI` ne s'y applique donc pas —
/// l'y soumettre laisserait passer « elfe noir » là où on attend `DARK_ELF`.
/// C'est aussi pourquoi cette expression vit ici et non dans
/// `shared_kernel::identity::charset` : la voisiner avec les charsets de saisie
/// inviterait à la confondre avec eux.
///
/// **Le piège nutype** (cf. `CLAUDE.md`) : passée par une constante plutôt
/// qu'en littéral, l'expression n'est compilée qu'au premier usage — une faute
/// de syntaxe ne casse pas `cargo build`, elle panique en production. Le test
/// `un_uid_de_mot_clef_respecte_la_forme_du_corpus` touche la constante ; c'est
/// lui qui referme le trou, et il faut le maintenir.
static UID_MOT_CLEF: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"^[A-Z][A-Z0-9_]*$").unwrap());

/// Le mot-clef qu'un joueur se met à haïr après avoir été blessé.
///
/// Le VO valide **la forme, jamais l'existence** : un uid absent du corpus est
/// syntaxiquement correct, et c'est le use case qui le refuse (carte 401). Un
/// value object qui interrogerait un port cesserait d'être un objet du domaine.
#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 40, regex = UID_MOT_CLEF),
    derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, AsRef)
)]
pub struct HatredKeyword(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchActionType {
    Touchdown,
    Passe,
    Interception,
    Agression,
    Lancer,
    Sortie,
    Mvp,
    /// `hatred` est **le choix** du coach ; `hatred_skill_uid` est **ce qu'il
    /// valait ce jour-là**. Ce n'est pas une redondance : résoudre le lien plus
    /// tard, à la publication, ferait dépendre un fait passé de l'état présent
    /// du référentiel — un corpus qui change de compétence entre la saisie et la
    /// publication réécrirait le sens d'une action déjà enregistrée.
    ///
    /// Les deux champs sont en `#[serde(default)]` parce que les actions sont
    /// persistées en JSON, dans l'event store comme dans
    /// `match_report_actions.action_json`. Sans lui, la relecture des blessures
    /// déjà écrites échouerait, et le rejeu de tout rapport ancien avec.
    Blesse {
        injury: InjuryType,
        #[serde(default)]
        hatred: Option<HatredKeyword>,
        // arch:ok — instantané dénormalisé, comme `player_display_name` et
        // `player_position` : la valeur est figée au moment du choix.
        #[serde(default)]
        hatred_skill_uid: Option<String>,
    },
}

/// La catégorie de SPP que vaut une action, quand elle en vaut une.
///
/// Elle nomme les cinq lignes du barème du corpus — le barème dit *combien*,
/// cette énumération dit *de quoi*. Les deux sont séparés parce que le premier
/// change d'un roster à l'autre (`BRAWLIN_BRUTES`) et le second jamais.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SppCategory {
    Touchdown,
    Pass,
    Interception,
    Casualty,
    Mvp,
}

impl MatchActionType {
    /// `None` pour les actions qui ne rapportent rien : l'agression, et la
    /// blessure — qui est **subie**, et ne saurait créditer sa victime.
    ///
    /// Cette correspondance n'existe qu'ici. Le récapitulatif la lit pour
    /// calculer ce qu'il affiche, avant publication ; après publication, c'est
    /// `app_event_publisher` qui achemine les mêmes actions vers `players`. Les
    /// deux chemins décrivent le même match et doivent en dire autant — les
    /// figer dans un seul tableau est le seul moyen de s'en assurer.
    pub fn spp_category(&self) -> Option<SppCategory> {
        match self {
            MatchActionType::Touchdown => Some(SppCategory::Touchdown),
            // BR2 — Passe et Lancer sont la même notion domaine
            MatchActionType::Passe | MatchActionType::Lancer => Some(SppCategory::Pass),
            MatchActionType::Interception => Some(SppCategory::Interception),
            MatchActionType::Sortie => Some(SppCategory::Casualty),
            MatchActionType::Mvp => Some(SppCategory::Mvp),
            MatchActionType::Agression => None,
            MatchActionType::Blesse { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InjuryType {
    Commotion,
    Amoche,
    BlessureSerieuse,
    Sequel { stat: SequelStat },
    Mort,
}

impl InjuryType {
    /// Amoché, Blessure Sérieuse, Séquelle laissent une rancune.
    ///
    /// Une Commotion est trop légère pour en laisser une, et une Mort ne laisse
    /// personne pour haïr. La règle est **ici et nulle part ailleurs** : la
    /// constante que portera le template (carte 402) n'en est qu'un reflet,
    /// chargé de masquer la section. Un écart entre les deux se solde par un
    /// refus, jamais par une donnée fausse.
    pub fn peut_donner_haine(&self) -> bool {
        matches!(
            self,
            InjuryType::Amoche | InjuryType::BlessureSerieuse | InjuryType::Sequel { .. }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SequelStat {
    MinusAv,
    MinusMa,
    MinusPa,
    MinusAg,
    MinusSt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempPlayer {
    pub id: TempPlayerId,
    pub team_id: TeamId,
    pub kind: TempPlayerKind,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TempPlayerKind {
    StarPlayer {
        ref_uid: String,
        position_uid: String,
    },
    Mercenary {
        position_uid: String,
    },
    Journeyman {
        position_uid: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchAction {
    pub id: ActionId,
    pub turn: TurnNumber,
    pub player: ActionPlayer,
    pub action: MatchActionType,
    pub player_display_name: String, // arch:ok texte libre dénormalisé (snapshot pour l'historique)
    #[serde(default)]
    pub player_position: String, // arch:ok texte libre dénormalisé (snapshot pour l'historique)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_value_ord() {
        let a = TeamValue::try_new(1000).unwrap();
        let b = TeamValue::try_new(1100).unwrap();
        assert!(b > a);
        assert!(a < b);
        assert_eq!(
            TeamValue::try_new(1000).unwrap(),
            TeamValue::try_new(1000).unwrap()
        );
    }

    #[test]
    fn inducement_purchase_total_cost() {
        let p = InducementPurchase {
            uid: InducementId("BRIBE".to_string()),
            qty: InducementQty::try_new(2).unwrap(),
            unit_cost: InducementCost::try_new(50).unwrap(),
        };
        assert_eq!(p.total_cost(), 100); // 2 × 50 kPo
    }

    #[test]
    fn d3roll_accepte_1_2_3() {
        assert!(D3Roll::try_new(1).is_ok());
        assert!(D3Roll::try_new(2).is_ok());
        assert!(D3Roll::try_new(3).is_ok());
        assert_eq!(D3Roll::try_new(2).unwrap().value(), 2);
    }

    #[test]
    fn d3roll_rejette_0_et_4() {
        assert!(D3Roll::try_new(0).is_err());
        assert!(D3Roll::try_new(4).is_err());
        assert!(D3Roll::try_new(255).is_err());
    }

    #[test]
    fn turn_number_accepts_1_and_16() {
        assert!(TurnNumber::try_new(1).is_ok());
        assert!(TurnNumber::try_new(16).is_ok());
        assert_eq!(TurnNumber::try_new(8).unwrap().value(), 8);
    }

    #[test]
    fn turn_number_rejects_0() {
        assert_eq!(
            TurnNumber::try_new(0).unwrap_err(),
            DomainError::InvalidTurn(0)
        );
    }

    #[test]
    fn turn_number_rejects_17() {
        assert_eq!(
            TurnNumber::try_new(17).unwrap_err(),
            DomainError::InvalidTurn(17)
        );
    }
}

use crate::app::players::domain::player::TeamId;
use nutype::nutype;
use serde::{Deserialize, Serialize};

// ── Contexte de match (embarqué dans chaque event, zéro appel inter-BC en lecture) ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchReportId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchContext {
    pub match_report_id: MatchReportId,
    pub round_id: RoundId,
    pub round_label: String, // arch:ok texte libre dénormalisé (zéro appel inter-BC en lecture)
    pub opponent_team_id: TeamId,
    pub opponent_team_name: String, // arch:ok texte libre dénormalisé (zéro appel inter-BC en lecture)
}

// ── SPP gagné par une action (résolu en amont via references, jamais calculé ici) ──

#[nutype(
    validate(greater_or_equal = 1),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct SppEarned(u32);

// ── Statut de participation ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerParticipationStatus {
    Available,
    MissingNextGame,
    Retired,
    Dead,
}

// ── Blessures ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatKind {
    Ma,
    St,
    Ag,
    Pa,
    Av,
}

impl StatKind {
    /// Ce qu'un cran d'**amélioration** fait à la valeur brute.
    ///
    /// MV, FO et AR montent quand le joueur progresse. AG et PA sont des
    /// nombres cibles à atteindre au dé : ils **descendent**.
    ///
    /// AR s'affiche avec un « + » comme AG et PA mais se comporte à l'inverse —
    /// en règles 2020 l'adversaire doit atteindre la cible pour blesser, donc
    /// une armure haute protège mieux. Le suffixe ne dit rien de la direction ;
    /// cette table est la seule à faire foi.
    ///
    /// Elle vit **ici**, dans le domaine, et non dans `player_stats_service` où
    /// elle se trouvait : le panier de customisation en a besoin pour juger si
    /// une amélioration franchit une borne, et deux tables auraient fini par
    /// diverger.
    pub fn improvement_step(self) -> i8 {
        match self {
            Self::Ma | Self::St | Self::Av => 1,
            Self::Ag | Self::Pa => -1,
        }
    }

    /// Bornes **inclusives** de la valeur brute résolue. Une modification qui
    /// en sortirait est refusée.
    pub fn bounds(self) -> (u8, u8) {
        match self {
            Self::Ma => (0, 9),
            Self::St => (0, 9),
            Self::Ag => (1, 6),
            Self::Pa => (1, 6),
            Self::Av => (2, 12),
        }
    }

    /// La valeur brute obtenue en appliquant `crans` d'amélioration (négatif
    /// pour une dégradation), bornes comprises. `None` si le résultat sort des
    /// bornes — c'est le refus, pas un écrêtage silencieux.
    pub fn apply_crans(self, current: u8, crans: i8) -> Option<u8> {
        let (min, max) = self.bounds();
        let cible = current as i16 + crans as i16 * self.improvement_step() as i16;
        match cible >= min as i16 && cible <= max as i16 {
            true => Some(cible as u8),
            false => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjuryType {
    Commotion,
    Amoche,
    BlessureSerieuse,
    Sequel { stat: StatKind },
    Mort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInjuryRecord {
    pub injury_type: InjuryType,
    pub context: MatchContext,
}

#[nutype(
    validate(greater_or_equal = 1),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct StatMalus(u8);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StatAdjustment {
    pub stat: StatKind,
    pub malus: StatMalus,
}

// ── Compteurs de carrière (même style que Spp/ValueKpo dans player.rs) ─────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TouchdownCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PassCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InterceptionCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CasualtyCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MvpCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FoulCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PersistentInjuryCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MatchesPlayedCount(pub u16);

#[cfg(test)]
mod stat_kind_tests {
    use super::StatKind;

    /// Le test qui protège toute la fonctionnalité : améliorer l'agilité
    /// **descend** le seuil de dé, améliorer l'armure le monte. Le suffixe « + »
    /// que les deux partagent à l'affichage ne dit rien de la direction.
    #[test]
    fn ameliorer_descend_les_seuils_de_de_et_monte_le_reste() {
        assert_eq!(StatKind::Ag.improvement_step(), -1);
        assert_eq!(StatKind::Pa.improvement_step(), -1);
        assert_eq!(StatKind::Ma.improvement_step(), 1);
        assert_eq!(StatKind::St.improvement_step(), 1);
        assert_eq!(StatKind::Av.improvement_step(), 1);
    }

    #[test]
    fn les_bornes_couvrent_les_cinq_caracteristiques() {
        assert_eq!(StatKind::Ma.bounds(), (0, 9));
        assert_eq!(StatKind::St.bounds(), (0, 9));
        assert_eq!(StatKind::Ag.bounds(), (1, 6));
        assert_eq!(StatKind::Pa.bounds(), (1, 6));
        assert_eq!(StatKind::Av.bounds(), (2, 12));
    }

    #[test]
    fn apply_crans_ameliore_dans_le_bon_sens() {
        // AG 3+ améliorée d'un cran devient 2+.
        assert_eq!(StatKind::Ag.apply_crans(3, 1), Some(2));
        // AR 8+ améliorée d'un cran devient 9+ — plus haut protège mieux.
        assert_eq!(StatKind::Av.apply_crans(8, 1), Some(9));
        // MV 7 dégradé d'un cran devient 6.
        assert_eq!(StatKind::Ma.apply_crans(7, -1), Some(6));
    }

    /// Hors bornes = refus, jamais écrêtage : rendre `Some(1)` là où l'on
    /// demandait `0+` ferait croire à une application réussie.
    #[test]
    fn apply_crans_refuse_hors_bornes_sans_ecreter() {
        assert_eq!(StatKind::Ag.apply_crans(1, 1), None); // 1+ ne peut pas mieux
        assert_eq!(StatKind::Ag.apply_crans(6, -1), None); // 6+ ne peut pas pire
        assert_eq!(StatKind::Ma.apply_crans(9, 1), None);
        assert_eq!(StatKind::Ma.apply_crans(0, -1), None);
        assert_eq!(StatKind::Av.apply_crans(12, 1), None);
        assert_eq!(StatKind::Av.apply_crans(2, -1), None);
    }

    #[test]
    fn apply_crans_accepte_une_amplitude_superieure_a_un() {
        assert_eq!(StatKind::Ag.apply_crans(5, 3), Some(2));
        assert_eq!(StatKind::Ag.apply_crans(5, 5), None);
    }
}

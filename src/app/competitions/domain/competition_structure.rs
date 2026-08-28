use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::match_day::MatchDayName;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ranking_group_id::RankingGroupId;
use crate::app::shared_kernel::identity::charset::TEXTE_SAISI;
use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UseRankingGroups(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UsePlayoffsPhase(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FinalPhaseMatchForThirdPlace(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UseSchedule(pub bool);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionStructure {
    pub ranking_group: RankingGroupConfig,
    pub play_offs_phase: PlayOffsPhase,
    pub schedule: ScheduleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DispatchType {
    Automatic,
    Manual,
    #[serde(other)]
    Unknown,
}

impl Default for DispatchType {
    fn default() -> Self {
        Self::Automatic
    }
}

/// La configuration des poules d'une compétition.
///
/// **Champs privés, une seule porte.** Un smart constructor posé sur des champs
/// publics ne garde rien : on l'évite par un littéral, et `Deserialize` l'évite
/// tout seul. Or ces réglages arrivent en JSON depuis le navigateur.
///
/// `#[serde(try_from = ...)]` est la même réponse que celle déjà retenue pour
/// `TiebreakConfig`, et pour la même raison, écrite là-bas :
///
/// > sans lui, un `Deserialize` nu reconstruirait le newtype sans passer par
/// > `try_new`, et n'importe quel payload JSON contournerait les invariants.
///
/// `Serialize` reste dérivé — sérialiser ne contourne aucun invariant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "RankingGroupConfigData")]
pub struct RankingGroupConfig {
    use_ranking_groups: UseRankingGroups,
    dispatch_type: DispatchType,
    ranking_groups: Vec<RankingGroup>,
}

/// Le miroir désérialisable, **privé au module** : il n'existe que pour que
/// `serde` ait une cible avant validation. L'exposer rouvrirait la porte qu'on
/// vient de fermer.
#[derive(Deserialize)]
struct RankingGroupConfigData {
    use_ranking_groups: UseRankingGroups,
    #[serde(default)]
    dispatch_type: DispatchType,
    ranking_groups: Vec<RankingGroup>,
}

impl TryFrom<RankingGroupConfigData> for RankingGroupConfig {
    type Error = DomainError;

    fn try_from(data: RankingGroupConfigData) -> Result<Self, Self::Error> {
        Self::try_new(
            data.use_ranking_groups,
            data.dispatch_type,
            data.ranking_groups,
        )
    }
}

impl RankingGroupConfig {
    /// Refuse **deux poules de même nom** et **deux poules de même
    /// identifiant**. Rien d'autre.
    ///
    /// Une liste vide est valide, avec ou sans le drapeau : retirer toutes les
    /// poules est un usage prévu, et le drapeau ne commande pas la liste.
    pub fn try_new(
        use_ranking_groups: UseRankingGroups,
        dispatch_type: DispatchType,
        ranking_groups: Vec<RankingGroup>,
    ) -> Result<Self, DomainError> {
        Self::refuser_les_doublons(&ranking_groups)?;
        Ok(Self {
            use_ranking_groups,
            dispatch_type,
            ranking_groups,
        })
    }

    fn refuser_les_doublons(groupes: &[RankingGroup]) -> Result<(), DomainError> {
        let mut noms: HashSet<&str> = HashSet::new();
        let mut ids: HashSet<&str> = HashSet::new();
        for g in groupes {
            if !ids.insert(g.id.as_ref()) {
                return Err(DomainError::DuplicatePoolId {
                    id: g.id.as_ref().to_string(),
                });
            }
            if !noms.insert(g.name.as_ref()) {
                return Err(DomainError::DuplicatePoolName {
                    name: g.name.as_ref().to_string(),
                });
            }
        }
        Ok(())
    }

    pub fn use_ranking_groups(&self) -> bool {
        self.use_ranking_groups.0
    }

    pub fn dispatch_type(&self) -> &DispatchType {
        &self.dispatch_type
    }

    /// `&[RankingGroup]`, jamais `&mut Vec` : aucune référence mutable ne sort
    /// de l'agrégat, sans quoi les doublons rentreraient par la fenêtre.
    pub fn groups(&self) -> &[RankingGroup] {
        &self.ranking_groups
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingGroup {
    pub id: RankingGroupId,
    pub name: RankingGroupName,
}

/// Le nom d'un groupe de classement.
///
/// Il portait un `NameVo` nu — le type générique que quatre autres noms
/// partageaient. Lui donner le sien est ce que demande la règle « pas de type
/// primitif nu » du `CLAUDE.md`, un cran au-dessus : un `String` validé qui
/// désigne n'importe quel nom n'est guère mieux qu'un `String`.
#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = TEXTE_SAISI),
    derive(
        Debug,
        Clone,
        Serialize,
        Deserialize,
        PartialEq,
        Eq,
        Hash,
        Display,
        AsRef
    )
)]
pub struct RankingGroupName(String);

#[nutype(
    validate(less_or_equal = 100),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)
)]
pub struct QualifiedTeamPerPool(u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayOffsPhase {
    pub use_playoffs_phase: UsePlayoffsPhase,
    pub qualified_team_per_pool: QualifiedTeamPerPool,
    pub final_phase_match_for_third_place: FinalPhaseMatchForThirdPlace,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleType {
    #[default]
    Unknown,
    FixedDate,
    TimeFrame,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub use_schedule: UseSchedule,
    #[serde(default)]
    pub schedule_type: ScheduleType,
    #[serde(default)]
    pub schedule_start_date: DateString,
    #[serde(default)]
    pub play_off_start_date: DateString,
    #[serde(default)]
    pub play_off_end_date: DateString,
    #[serde(default)]
    pub schedule_end_date: DateString,
    pub scheduled_dates: Vec<ScheduledDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScheduledDate {
    FixedDate {
        name: MatchDayName,
        multiplexe_date: DateString,
    },
    TimeFrame {
        name: MatchDayName,
        start_date: DateString,
        end_date: DateString,
    },
}

#[cfg(test)]
mod ranking_group_config_tests {
    //! L'encapsulation de la configuration des poules (carte 417).
    use super::*;

    fn groupe(id: &str, nom: &str) -> RankingGroup {
        RankingGroup {
            id: RankingGroupId::try_new(id.to_string()).unwrap(),
            name: RankingGroupName::try_new(nom.to_string()).unwrap(),
        }
    }

    fn construire(groupes: Vec<RankingGroup>) -> Result<RankingGroupConfig, DomainError> {
        RankingGroupConfig::try_new(UseRankingGroups(true), DispatchType::Automatic, groupes)
    }

    #[test]
    fn try_new_refuse_deux_poules_de_meme_nom() {
        match construire(vec![groupe("ga", "Poule A"), groupe("gb", "Poule A")]) {
            Err(DomainError::DuplicatePoolName { name }) => assert_eq!(name, "Poule A"),
            autre => panic!("attendu un doublon de nom : {autre:?}"),
        }
    }

    #[test]
    fn try_new_refuse_deux_poules_de_meme_id() {
        match construire(vec![groupe("ga", "Poule A"), groupe("ga", "Poule B")]) {
            Err(DomainError::DuplicatePoolId { id }) => assert_eq!(id, "ga"),
            autre => panic!("attendu un doublon d'identifiant : {autre:?}"),
        }
    }

    /// Retirer toutes les poules est un usage prévu.
    #[test]
    fn try_new_accepte_une_liste_vide() {
        let config =
            RankingGroupConfig::try_new(UseRankingGroups(false), DispatchType::Automatic, vec![])
                .expect("une liste vide est valide");
        assert!(config.groups().is_empty());
    }

    /// **Le drapeau ne commande pas la liste.** Les lier obligerait à les
    /// modifier ensemble, et un écran qui décoche « en poules » sans vider la
    /// liste — ou l'inverse — se ferait refuser sans raison métier.
    #[test]
    fn try_new_accepte_une_liste_vide_avec_le_drapeau_actif() {
        let config = construire(vec![]).expect("le drapeau seul ne suffit pas à exiger une poule");
        assert!(config.use_ranking_groups());
        assert!(config.groups().is_empty());
    }

    #[test]
    fn try_new_accepte_des_poules_distinctes() {
        let config = construire(vec![groupe("ga", "Poule A"), groupe("gb", "Poule B")]).unwrap();
        assert_eq!(config.groups().len(), 2);
        assert!(matches!(config.dispatch_type(), DispatchType::Automatic));
    }

    /// **Le test qui garde l'encapsulation.** Sans `#[serde(try_from)]`, un
    /// `Deserialize` nu reconstruirait la structure sans passer par `try_new`,
    /// et n'importe quel payload JSON contournerait les invariants — or ces
    /// réglages arrivent en JSON depuis le navigateur.
    ///
    /// Sans ce test, retirer l'attribut au détour d'un refactor ne casserait
    /// rien de visible.
    #[test]
    fn deserialize_passe_par_try_new() {
        let json = r#"{
            "use_ranking_groups": true,
            "dispatch_type": "automatic",
            "ranking_groups": [
                {"id": "ga", "name": "Poule A"},
                {"id": "gb", "name": "Poule A"}
            ]
        }"#;

        let issue = serde_json::from_str::<RankingGroupConfig>(json);

        assert!(
            issue.is_err(),
            "un JSON à deux homonymes a été accepté : l'attribut serde a-t-il sauté ?"
        );
    }

    /// Contre-épreuve : sans elle, un JSON refusé pour une faute de forme se
    /// lirait comme un refus d'invariant, et le test ci-dessus passerait au vert
    /// même si `try_from` était retiré.
    #[test]
    fn deserialize_accepte_une_configuration_valide() {
        let json = r#"{
            "use_ranking_groups": true,
            "dispatch_type": "automatic",
            "ranking_groups": [
                {"id": "ga", "name": "Poule A"},
                {"id": "gb", "name": "Poule B"}
            ]
        }"#;

        let config = serde_json::from_str::<RankingGroupConfig>(json).expect("JSON valide");

        assert_eq!(config.groups().len(), 2);
    }

    /// Le champ est `#[serde(default)]` : les structures écrites avant son
    /// introduction n'en portent pas, et neuf saisons de production sont dans ce
    /// cas potentiel.
    #[test]
    fn deserialize_tolere_un_dispatch_type_absent() {
        let json = r#"{"use_ranking_groups": false, "ranking_groups": []}"#;

        let config = serde_json::from_str::<RankingGroupConfig>(json).expect("JSON historique");

        assert!(matches!(config.dispatch_type(), DispatchType::Automatic));
    }
}

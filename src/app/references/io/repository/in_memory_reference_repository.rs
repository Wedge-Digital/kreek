use crate::app::references::domain::models::{
    Inducement, League, PlayerPosition, Skill, SkillCategory, SkillCostLevel, SpecialRule, Staff,
    StarPlayer, Team,
};
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::references::io::repository::reference_data_error::ReferenceDataError;
use crate::app::shared_kernel::bloodbowl::ids::RosterId;
use crate::app::shared_kernel::bloodbowl::inducement_definition::{
    InducementCost, InducementDefinition, InducementId, InducementName,
};
use crate::app::shared_kernel::bloodbowl::roster_definition::RosterDefinition;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::path::Path;
// ── Raw JSON wrappers (deserialization only) ──────────────────────────────────

#[derive(Deserialize)]
struct InducementsFile {
    inducements: Vec<Inducement>,
}

#[derive(Deserialize)]
struct StarPlayersFile {
    star_players: Vec<StarPlayer>,
}

#[derive(Deserialize)]
struct TeamsFile {
    teams: Vec<Team>,
}

#[derive(Deserialize)]
struct SkillsFile {
    skills: Vec<Skill>,
}

#[derive(Deserialize)]
struct SkillCatFile {
    skill_categories: Vec<SkillCategory>,
}

#[derive(Deserialize)]
struct SpecialRulesFile {
    special_rules: Vec<SpecialRule>,
}

#[derive(Deserialize)]
struct StaffFile {
    staff: Vec<Staff>,
}

#[derive(Deserialize)]
struct LeaguesFile {
    leagues: Vec<League>,
}

// skill_cost.json est un tableau JSON de premier niveau, pas d'objet wrapper
type SkillCostFile = Vec<SkillCostLevel>;

#[derive(Deserialize)]
struct ImprovementValuesFile {
    improvement_values: ImprovementValues,
}

/// Valeur ajoutée à un joueur par une amélioration, en **kPo**, quelle que soit
/// l'origine de l'amélioration — bonus de création ou achat en SPP. Elle vivait
/// en dur dans deux endroits qui divergeaient (carte 249).
#[derive(Deserialize)]
pub struct ImprovementValues {
    pub skill: SkillImprovementValues,
    pub stat: StatImprovementValues,
}

#[derive(Deserialize)]
pub struct SkillImprovementValues {
    pub primary: u32,
    pub secondary: u32,
}

#[derive(Deserialize)]
pub struct StatImprovementValues {
    pub ma: u32,
    pub st: u32,
    pub ag: u32,
    pub pa: u32,
    pub av: u32,
}

// ── In-memory repository ──────────────────────────────────────────────────────

pub struct InMemoryReferenceRepository {
    inducements: Vec<Inducement>,
    star_players: Vec<StarPlayer>,
    teams: Vec<Team>,
    skills: Vec<Skill>,
    skill_categories: Vec<SkillCategory>,
    special_rules: Vec<SpecialRule>,
    staff: Vec<Staff>,
    leagues: Vec<League>,
    skill_cost_matrix: Vec<SkillCostLevel>,
    improvement_values: ImprovementValues,
}

impl InMemoryReferenceRepository {
    /// Charge l'intégralité des données de référence depuis `dir`, une fois,
    /// au démarrage. Tout est ensuite servi depuis la mémoire.
    pub fn load_from_dir(dir: &Path) -> Result<Self, ReferenceDataError> {
        Ok(Self {
            inducements: read_json::<InducementsFile>(dir, "inducements_fr.json")?.inducements,
            star_players: read_json::<StarPlayersFile>(dir, "star_players_fr.json")?.star_players,
            teams: read_json::<TeamsFile>(dir, "teams_fr.json")?.teams,
            skills: read_json::<SkillsFile>(dir, "skills_fr.json")?.skills,
            skill_categories: read_json::<SkillCatFile>(dir, "skill_cat_fr.json")?.skill_categories,
            special_rules: read_json::<SpecialRulesFile>(dir, "special_rules_fr.json")?
                .special_rules,
            staff: read_json::<StaffFile>(dir, "staff_fr.json")?.staff,
            leagues: read_json::<LeaguesFile>(dir, "leagues_fr.json")?.leagues,
            skill_cost_matrix: read_json::<SkillCostFile>(dir, "skill_cost.json")?,
            improvement_values: read_json::<ImprovementValuesFile>(dir, "improvement_values.json")?
                .improvement_values,
        })
    }

    /// Jeu de données des tests unitaires : le jeu de démonstration versionné,
    /// jamais les données réelles — celles-ci ne sont pas dans le dépôt.
    /// Résolu depuis la racine de la crate pour rester indépendant du
    /// répertoire courant.
    #[cfg(test)]
    pub fn load_for_tests() -> Self {
        const DIR: &str = "assets/references.example";
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(DIR);
        Self::load_from_dir(&dir)
            .unwrap_or_else(|e| panic!("jeu de données « {DIR} » invalide : {e}"))
    }
}

fn read_json<T: DeserializeOwned>(dir: &Path, file: &str) -> Result<T, ReferenceDataError> {
    let path = dir.join(file);
    let raw = std::fs::read_to_string(&path).map_err(|e| ReferenceDataError::FileUnreadable {
        file: path.display().to_string(),
        cause: e.to_string(),
    })?;
    serde_json::from_str(&raw).map_err(|e| ReferenceDataError::InvalidJson {
        file: path.display().to_string(),
        cause: e.to_string(),
    })
}

impl IReferenceRepository for InMemoryReferenceRepository {
    fn list_roster_definitions(&self) -> Vec<RosterDefinition> {
        let mut rosters: Vec<RosterDefinition> = self
            .teams
            .iter()
            .map(|team: &Team| RosterDefinition {
                id: RosterId(team.uid.clone()),
                name: team.name.clone(),
            })
            .collect();
        rosters.sort_unstable_by(|a, b| a.id.0.cmp(&b.id.0));
        rosters
    }

    fn list_inducements(&self) -> Vec<InducementDefinition> {
        let mut inducements: Vec<InducementDefinition> = self
            .inducements
            .iter()
            .map(|inducement| InducementDefinition {
                id: InducementId(inducement.uid.clone()),
                cost: InducementCost::try_new(inducement.cost).expect("invalid inducement cost"),
                name: InducementName(inducement.name.clone()),
            })
            .collect();
        inducements.sort_unstable_by(|a, b| a.id.0.cmp(&b.id.0));
        inducements
    }

    fn list_star_players(&self) -> &[StarPlayer] {
        &self.star_players
    }
    fn list_teams(&self) -> &[Team] {
        &self.teams
    }
    fn list_skills(&self) -> &[Skill] {
        &self.skills
    }
    fn list_skill_categories(&self) -> &[SkillCategory] {
        &self.skill_categories
    }
    fn list_special_rules(&self) -> &[SpecialRule] {
        &self.special_rules
    }
    fn list_staff(&self) -> &[Staff] {
        &self.staff
    }
    fn list_leagues(&self) -> &[League] {
        &self.leagues
    }

    fn find_inducement_by_uid(&self, uid: &str) -> Option<&Inducement> {
        self.inducements.iter().find(|x| x.uid == uid)
    }
    fn find_star_player_by_uid(&self, uid: &str) -> Option<&StarPlayer> {
        self.star_players.iter().find(|x| x.uid == uid)
    }
    fn find_team_by_uid(&self, uid: &str) -> Option<&Team> {
        self.teams.iter().find(|x| x.uid == uid)
    }
    fn find_skill_by_uid(&self, uid: &str) -> Option<&Skill> {
        self.skills.iter().find(|x| x.uid == uid)
    }
    fn find_position_by_uid(&self, uid: &str) -> Option<&PlayerPosition> {
        self.teams
            .iter()
            .flat_map(|t| t.available_players.iter())
            .find(|p| p.uid == uid)
    }

    fn skill_cost_matrix(&self) -> &[SkillCostLevel] {
        &self.skill_cost_matrix
    }

    fn touchdown_spp(&self) -> u8 {
        3
    }
    fn pass_spp(&self) -> u8 {
        1
    }
    fn interception_spp(&self) -> u8 {
        2
    }
    fn casualty_spp(&self) -> u8 {
        2
    }
    fn mvp_spp(&self) -> u8 {
        4
    }

    fn improvement_skill_value_delta(&self, is_secondary_access: bool) -> u32 {
        if is_secondary_access {
            self.improvement_values.skill.secondary
        } else {
            self.improvement_values.skill.primary
        }
    }
    fn improvement_stat_value_delta_ma(&self) -> u32 {
        self.improvement_values.stat.ma
    }
    fn improvement_stat_value_delta_st(&self) -> u32 {
        self.improvement_values.stat.st
    }
    fn improvement_stat_value_delta_ag(&self) -> u32 {
        self.improvement_values.stat.ag
    }
    fn improvement_stat_value_delta_pa(&self) -> u32 {
        self.improvement_values.stat.pa
    }
    fn improvement_stat_value_delta_av(&self) -> u32 {
        self.improvement_values.stat.av
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::references::domain::consistency::check_consistency;

    /// Doit rester aligné sur `FIVE_CHAOS_GODS` (special_rule_selector.rs),
    /// que le sélecteur de règle à choix résout depuis le jeu de données.
    const CHOICE_RULE_UIDS: [&str; 5] = [
        "FAVOURED_OF_KHORNE",
        "FAVOURED_OF_NURGLE",
        "FAVOURED_OF_SLAANESH",
        "FAVOURED_OF_TZEENTCH",
        "FAVOURED_OF_UNDIVIDED",
    ];

    fn fixture_dir(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn load_from_dir_populates_every_collection() {
        let repo = InMemoryReferenceRepository::load_for_tests();
        assert!(!repo.list_teams().is_empty());
        assert!(!repo.list_star_players().is_empty());
        assert!(!repo.list_skills().is_empty());
        assert!(!repo.list_skill_categories().is_empty());
        assert!(!repo.list_special_rules().is_empty());
        assert!(!repo.list_staff().is_empty());
        assert!(!repo.list_leagues().is_empty());
        assert!(!repo.list_inducements().is_empty());
        assert!(!repo.skill_cost_matrix().is_empty());
    }

    /// `expect_err` exigerait `Debug` sur le repository — on déstructure à la main.
    fn expect_load_error(dir: &str, context: &str) -> ReferenceDataError {
        match InMemoryReferenceRepository::load_from_dir(&fixture_dir(dir)) {
            Err(e) => e,
            Ok(_) => panic!("{context}"),
        }
    }

    #[test]
    fn missing_directory_names_the_faulty_file() {
        let err = expect_load_error("references_empty", "un répertoire vide doit échouer");
        match err {
            ReferenceDataError::FileUnreadable { file, .. } => {
                assert!(
                    file.ends_with("inducements_fr.json"),
                    "fichier signalé : {file}"
                )
            }
            other => panic!("erreur inattendue : {other}"),
        }
    }

    #[test]
    fn malformed_json_names_the_faulty_file() {
        let err = expect_load_error("references_invalid", "un JSON malformé doit échouer");
        match err {
            ReferenceDataError::InvalidJson { file, .. } => {
                assert!(
                    file.ends_with("inducements_fr.json"),
                    "fichier signalé : {file}"
                )
            }
            other => panic!("erreur inattendue : {other}"),
        }
    }

    // ── Jeu de démonstration versionné ────────────────────────────────────────

    #[test]
    fn example_dataset_is_referentially_consistent() {
        let repo = InMemoryReferenceRepository::load_for_tests();
        let violations = check_consistency(&repo);
        assert!(violations.is_empty(), "incohérences : {violations:?}");
    }

    /// Les uids que le code de production interroge en dur. Un jeu de données
    /// qui ne les fournit pas dégrade silencieusement des fonctionnalités.
    #[test]
    fn example_dataset_honours_hardcoded_uids() {
        let repo = InMemoryReferenceRepository::load_for_tests();
        for uid in [
            "APOTHECARY",
            "CHEERLEADERS",
            "COACH_ASSISTANTS",
            "FAN_FACTOR",
        ] {
            assert!(
                repo.list_staff().iter().any(|s| s.uid == uid),
                "staff manquant : {uid}"
            );
        }
        for uid in CHOICE_RULE_UIDS {
            assert!(
                repo.list_special_rules().iter().any(|r| r.uid == uid),
                "règle spéciale manquante : {uid}"
            );
        }
        for id in ["GENERAL", "AGILITY", "STRENGTH", "PASSING"] {
            assert!(
                repo.list_skill_categories().iter().any(|c| c.id == id),
                "catégorie manquante : {id}"
            );
        }
    }

    /// Le jeu de démonstration sert de spécification exécutable du schéma :
    /// il doit exercer les variations que la donnée réelle n'expose pas.
    #[test]
    fn example_dataset_exercises_optional_schema_fields() {
        let repo = InMemoryReferenceRepository::load_for_tests();

        let teams = repo.list_teams();
        assert!(
            teams.iter().any(|t| t.logo.is_some()),
            "aucun logo renseigné"
        );
        assert!(teams.iter().any(|t| t.logo.is_none()), "aucun logo omis");

        // `leagues` est optionnel au sens du schéma, mais de fait obligatoire :
        // une équipe ne peut pas être soumise sans ligue (LeagueNotSelected), et
        // l'auto-assignation n'opère que sur une entrée unique
        // (player_table_widget.rs). Un roster de démo sans ligue serait
        // injouable — c'est arrivé, l'e2e l'a détecté.
        for team in teams {
            assert_eq!(
                team.leagues.len(),
                1,
                "le roster {} doit déclarer exactement une ligue",
                team.uid
            );
        }

        let positions: Vec<_> = teams
            .iter()
            .flat_map(|t| t.available_players.iter())
            .collect();
        assert!(
            positions.iter().any(|p| p.is_journeyman),
            "aucun journeyman"
        );
        assert!(
            positions.iter().any(|p| p.skills.is_empty()),
            "aucune position sans skill"
        );
        assert!(
            positions.iter().any(|p| !p.skills.is_empty()),
            "aucune position avec skills"
        );

        let inducements = repo.list_inducements();
        assert!(!inducements.is_empty());
        let raw: Vec<_> = inducements
            .iter()
            .filter_map(|i| repo.find_inducement_by_uid(&i.id.0))
            .collect();
        assert!(
            raw.iter().any(|i| i.reduced_cost.is_some()),
            "aucun reducedCost"
        );
        assert!(
            raw.iter().any(|i| i.reduced_cost.is_none()),
            "aucun reducedCost nul"
        );
        assert!(
            raw.iter().any(|i| !i.restricted_to.is_empty()),
            "aucun restrictedTo"
        );

        assert!(repo
            .list_star_players()
            .iter()
            .any(|s| !s.plays_for.is_empty()));
        assert!(repo
            .list_star_players()
            .iter()
            .any(|s| s.plays_for.is_empty()));

        let costs = repo.skill_cost_matrix();
        assert!(
            costs.iter().any(|c| c.chosen_elite.is_some()),
            "aucun chosenElite"
        );
        assert!(
            costs.iter().any(|c| c.chosen_elite.is_none()),
            "aucun chosenElite omis"
        );
        assert!(
            costs.iter().any(|c| c.random_elite.is_some()),
            "aucun randomElite"
        );
    }

    #[test]
    fn spp_scale_matches_blood_bowl_standard_barème() {
        let repo = InMemoryReferenceRepository::load_for_tests();
        assert_eq!(repo.touchdown_spp(), 3);
        assert_eq!(repo.pass_spp(), 1);
        assert_eq!(repo.interception_spp(), 2);
        assert_eq!(repo.casualty_spp(), 2);
        assert_eq!(repo.mvp_spp(), 4);
    }

    #[test]
    fn improvement_value_delta_matches_official_table() {
        let repo = InMemoryReferenceRepository::load_for_tests();
        assert_eq!(repo.improvement_skill_value_delta(false), 20);
        assert_eq!(repo.improvement_skill_value_delta(true), 40);
        assert_eq!(repo.improvement_stat_value_delta_ma(), 20);
        assert_eq!(repo.improvement_stat_value_delta_st(), 60);
        assert_eq!(repo.improvement_stat_value_delta_ag(), 30);
        assert_eq!(repo.improvement_stat_value_delta_pa(), 20);
        assert_eq!(repo.improvement_stat_value_delta_av(), 10);
    }
}

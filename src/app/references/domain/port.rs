use crate::app::references::domain::models::{
    Inducement, League, PlayerPosition, Skill, SkillCategory, SkillCostLevel, SpecialRule, Staff,
    StarPlayer, Team,
};
use crate::app::shared_kernel::bloodbowl::inducement_definition::InducementDefinition;
use crate::app::shared_kernel::bloodbowl::roster_definition::RosterDefinition;

pub trait IReferenceRepository: Send + Sync {
    fn list_roster_definitions(&self) -> Vec<RosterDefinition>;
    fn list_inducements(&self) -> Vec<InducementDefinition>;
    fn list_star_players(&self) -> &[StarPlayer];
    fn list_teams(&self) -> &[Team];
    fn list_skills(&self) -> &[Skill];
    fn list_skill_categories(&self) -> &[SkillCategory];
    fn list_special_rules(&self) -> &[SpecialRule];
    fn list_staff(&self) -> &[Staff];
    fn list_leagues(&self) -> &[League];

    fn find_inducement_by_uid(&self, uid: &str) -> Option<&Inducement>;
    fn find_star_player_by_uid(&self, uid: &str) -> Option<&StarPlayer>;
    fn find_team_by_uid(&self, uid: &str) -> Option<&Team>;
    fn find_skill_by_uid(&self, uid: &str) -> Option<&Skill>;
    fn find_position_by_uid(&self, uid: &str) -> Option<&PlayerPosition>;
    fn skill_cost_matrix(&self) -> &[SkillCostLevel];

    // ── Barème SPP par type d'action (règle Blood Bowl standard, fixe) ──────────
    fn touchdown_spp(&self) -> u8;
    fn pass_spp(&self) -> u8;
    fn interception_spp(&self) -> u8;
    fn casualty_spp(&self) -> u8;
    fn mvp_spp(&self) -> u8;

    // ── Valeur ajoutée par amélioration achetée en SPP (fixe) ───────────────────
    // Unité : Po (pas kPo) — cohérent avec `players::domain::player::ValueKpo`,
    // qui malgré son nom stocke des Po bruts (ex. ValueKpo(100_000) → "100 000
    // Po"). Attention au pont vers `teams::domain::value_objects::Kpo`, qui LUI
    // stocke déjà des kPo (Kpo(873) → "873 kPo") — diviser par 1000 en
    // traversant la frontière players → teams (cf. carte 180).
    fn improvement_skill_value_delta(&self, is_secondary_access: bool) -> u32;
    fn improvement_stat_value_delta_ma(&self) -> u32;
    fn improvement_stat_value_delta_st(&self) -> u32;
    fn improvement_stat_value_delta_ag(&self) -> u32;
    fn improvement_stat_value_delta_pa(&self) -> u32;
    fn improvement_stat_value_delta_av(&self) -> u32;
}

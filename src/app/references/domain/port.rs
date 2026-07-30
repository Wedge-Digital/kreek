use crate::app::references::domain::models::{
    Inducement, League, PlayerPosition, Skill, SkillCategory, SkillCostLevel, SpecialRule,
    SppScale, Staff, StarPlayer, Team,
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

    // ── Barème SPP ─────────────────────────────────────────────────────────────
    /// Le barème d'acquisition de SPP de l'équipe à laquelle ce poste appartient.
    ///
    /// Il n'est **pas** fixe : la règle spéciale `BRAWLIN_BRUTES` inverse la
    /// valeur du touchdown et de la sortie. C'est `references` qui résout le
    /// roster depuis la ligne de poste, parce que le corpus lui appartient.
    ///
    /// Rendu entier plutôt qu'action par action : une seule résolution, et un
    /// match ne peut pas mélanger deux barèmes.
    fn spp_scale_for_roster_line(&self, roster_line_id: &str) -> SppScale;

    /// Le même barème, désigné cette fois par le roster lui-même.
    ///
    /// Le récapitulatif de match raisonne par équipe, pas par joueur : il
    /// connaît le roster des deux camps et rien de leurs lignes de poste.
    /// Un barème absent retombe sur `normal`, comme pour la ligne de poste.
    fn spp_scale_for_roster(&self, roster_uid: &str) -> SppScale;

    // ── Valeur ajoutée par une amélioration ─────────────────────────────────────
    // Unité : kPo, comme partout ailleurs dans le back. Lue dans
    // `improvement_values.json`, et non plus codée en dur — la même table sert
    // les deux origines d'une compétence, bonus de création et achat en SPP,
    // qui divergeaient jusqu'à la carte 249.
    fn improvement_skill_value_delta(&self, is_secondary_access: bool) -> u32;
    fn improvement_stat_value_delta_ma(&self) -> u32;
    fn improvement_stat_value_delta_st(&self) -> u32;
    fn improvement_stat_value_delta_ag(&self) -> u32;
    fn improvement_stat_value_delta_pa(&self) -> u32;
    fn improvement_stat_value_delta_av(&self) -> u32;
}

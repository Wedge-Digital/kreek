use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::domain::player::Player;
use crate::app::references::domain::port::IReferenceRepository;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPlayerStats {
    pub ma: u8,
    pub st: u8,
    pub ag: u8,
    pub pa: u8,
    pub av: u8,
}

/// Résout les stats finales d'un joueur : stat de base du poste (`references`)
/// combinée avec les `stat_adjustments` accumulés (séquelles). L'agrégat `Player`
/// reste pur — il ne stocke que le delta, jamais la valeur résolue (BR13).
pub fn resolve_stats(player: &Player, ref_repo: &dyn IReferenceRepository) -> Option<ResolvedPlayerStats> {
    let base = ref_repo.find_position_by_uid(player.roster_line_id.as_ref())?;
    let mut stats = ResolvedPlayerStats {
        ma: base.ma,
        st: base.st,
        ag: base.ag,
        pa: base.pa,
        av: base.av,
    };
    for adj in &player.stat_adjustments {
        match adj.stat {
            // MA/ST/AV : plus haut = meilleur (AV 2020 : le adversaire doit ATTEINDRE
            // la cible pour blesser, donc plus haut = plus dur à blesser) → le malus DIMINUE la valeur.
            // Cohérent avec le nommage `SequelStat::MinusAv` côté match_report.
            StatKind::Ma => stats.ma = stats.ma.saturating_sub(adj.malus),
            StatKind::St => stats.st = stats.st.saturating_sub(adj.malus),
            StatKind::Av => stats.av = stats.av.saturating_sub(adj.malus),
            // AG/PA : nombres cibles de dé à atteindre pour réussir une action,
            // plus bas = meilleur → le malus AUGMENTE la valeur.
            StatKind::Ag => stats.ag = stats.ag.saturating_add(adj.malus),
            StatKind::Pa => stats.pa = stats.pa.saturating_add(adj.malus),
        }
    }
    Some(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::events::PlayerDomainEvent;
    use crate::app::players::domain::match_impact::StatAdjustment;
    use crate::app::players::domain::player::{PlayerId, Spp, TeamId, ValueKpo};
    use crate::app::players::domain::value_objects::{PositionNameVo, RosterLineId};
    use crate::app::references::domain::models::PlayerPosition;
    use crate::app::references::domain::models::{Inducement, League, Skill, SkillCategory, SkillCostLevel, SpecialRule, Staff, StarPlayer, Team};
    use crate::app::shared_kernel::common_types::SpaceId;
    use crate::app::shared_kernel::inducement_definition::InducementDefinition;
    use crate::app::shared_kernel::roster_definition::RosterDefinition;

    struct FakeRefRepo {
        blitzer: PlayerPosition,
    }

    impl FakeRefRepo {
        fn new() -> Self {
            Self {
                blitzer: PlayerPosition {
                    uid: "BLITZER".into(),
                    position_name: "Frappeur".into(),
                    cost: 90_000,
                    ma: 7,
                    st: 3,
                    ag: 3,
                    pa: 5,
                    av: 8,
                    skills: vec![],
                    primary_access: vec![],
                    secondary_access: vec![],
                    max_quantity: 16,
                    is_journalier: false,
                },
            }
        }
    }

    impl IReferenceRepository for FakeRefRepo {
        fn list_roster_definitions(&self) -> Vec<RosterDefinition> { vec![] }
        fn list_inducements(&self) -> Vec<InducementDefinition> { vec![] }
        fn list_star_players(&self) -> &[StarPlayer] { &[] }
        fn list_teams(&self) -> &[Team] { &[] }
        fn list_skills(&self) -> &[Skill] { &[] }
        fn list_skill_categories(&self) -> &[SkillCategory] { &[] }
        fn list_special_rules(&self) -> &[SpecialRule] { &[] }
        fn list_staff(&self) -> &[Staff] { &[] }
        fn list_leagues(&self) -> &[League] { &[] }
        fn find_inducement_by_uid(&self, _uid: &str) -> Option<&Inducement> { None }
        fn find_star_player_by_uid(&self, _uid: &str) -> Option<&StarPlayer> { None }
        fn find_team_by_uid(&self, _uid: &str) -> Option<&Team> { None }
        fn find_skill_by_uid(&self, _uid: &str) -> Option<&Skill> { None }
        fn find_position_by_uid(&self, uid: &str) -> Option<&PlayerPosition> {
            if uid == "BLITZER" {
                Some(&self.blitzer)
            } else {
                None
            }
        }
        fn skill_cost_matrix(&self) -> &[SkillCostLevel] { &[] }
        fn touchdown_spp(&self) -> u8 { 3 }
        fn pass_spp(&self) -> u8 { 1 }
        fn interception_spp(&self) -> u8 { 2 }
        fn casualty_spp(&self) -> u8 { 2 }
        fn mvp_spp(&self) -> u8 { 4 }
    }

    fn sample_player() -> Player {
        let created = PlayerDomainEvent::PlayerCreated {
            player_id:      PlayerId("p1".into()),
            team_id:        TeamId("t1".into()),
            space_id:       SpaceId::new(),
            position_name:  PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey:         None,
            base_skills:    vec![],
            starting_spp:   Spp(0),
            starting_value: ValueKpo(90_000),
        };
        Player::from_events(&[created]).unwrap()
    }

    #[test]
    fn resolve_stats_returns_base_stats_when_no_adjustment() {
        let player = sample_player();
        let stats = resolve_stats(&player, &FakeRefRepo::new()).unwrap();
        assert_eq!(stats, ResolvedPlayerStats { ma: 7, st: 3, ag: 3, pa: 5, av: 8 });
    }

    #[test]
    fn resolve_stats_applies_ma_st_av_malus_as_decrease() {
        let mut player = sample_player();
        player.stat_adjustments.push(StatAdjustment { stat: StatKind::Ma, malus: 1 });
        player.stat_adjustments.push(StatAdjustment { stat: StatKind::St, malus: 1 });
        player.stat_adjustments.push(StatAdjustment { stat: StatKind::Av, malus: 1 });
        let stats = resolve_stats(&player, &FakeRefRepo::new()).unwrap();
        assert_eq!(stats.ma, 6);
        assert_eq!(stats.st, 2);
        assert_eq!(stats.av, 7);
    }

    #[test]
    fn resolve_stats_applies_ag_pa_malus_as_increase() {
        let mut player = sample_player();
        player.stat_adjustments.push(StatAdjustment { stat: StatKind::Ag, malus: 1 });
        let stats = resolve_stats(&player, &FakeRefRepo::new()).unwrap();
        assert_eq!(stats.ag, 4);
    }

    #[test]
    fn resolve_stats_returns_none_for_unknown_position() {
        let mut player = sample_player();
        player.roster_line_id = RosterLineId::try_new("UNKNOWN".to_string()).unwrap();
        assert!(resolve_stats(&player, &FakeRefRepo::new()).is_none());
    }
}

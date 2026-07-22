use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::domain::player::Player;
use crate::app::players::ports::ISkillCatalogPort;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPlayerStats {
    pub ma: u8,
    pub st: u8,
    pub ag: u8,
    pub pa: u8,
    pub av: u8,
}

/// Résout les stats finales d'un joueur : stat de base du poste (`references`,
/// via le port `ISkillCatalogPort`) combinée avec les `stat_adjustments`
/// accumulés (séquelles) et les `stat_increases` achetés en SPP. L'agrégat
/// `Player` reste pur — il ne stocke que les deltas, jamais la valeur résolue (BR13).
pub fn resolve_stats(player: &Player, catalog: &dyn ISkillCatalogPort) -> Option<ResolvedPlayerStats> {
    let base = catalog.find_position(player.roster_line_id.as_ref())?;
    let mut stats = ResolvedPlayerStats {
        ma: base.ma,
        st: base.st,
        ag: base.ag,
        pa: base.pa,
        av: base.av,
    };
    for adj in &player.stat_adjustments {
        apply_malus(&mut stats, adj.stat, adj.malus.into_inner());
    }
    for increase in &player.stat_increases {
        apply_increase(&mut stats, increase.stat);
    }
    Some(stats)
}

fn apply_malus(stats: &mut ResolvedPlayerStats, stat: StatKind, malus: u8) {
    match stat {
        // MA/ST/AV : plus haut = meilleur (AV 2020 : le adversaire doit ATTEINDRE
        // la cible pour blesser, donc plus haut = plus dur à blesser) → le malus DIMINUE la valeur.
        // Cohérent avec le nommage `SequelStat::MinusAv` côté match_report.
        StatKind::Ma => stats.ma = stats.ma.saturating_sub(malus),
        StatKind::St => stats.st = stats.st.saturating_sub(malus),
        StatKind::Av => stats.av = stats.av.saturating_sub(malus),
        // AG/PA : nombres cibles de dé à atteindre pour réussir une action,
        // plus bas = meilleur → le malus AUGMENTE la valeur.
        StatKind::Ag => stats.ag = stats.ag.saturating_add(malus),
        StatKind::Pa => stats.pa = stats.pa.saturating_add(malus),
    }
}

/// Une augmentation SPP va toujours dans le sens de l'amélioration —
/// inverse de `apply_malus` pour AG/PA (nombres cibles : plus bas = meilleur).
fn apply_increase(stats: &mut ResolvedPlayerStats, stat: StatKind) {
    match stat {
        StatKind::Ma => stats.ma = stats.ma.saturating_add(1),
        StatKind::St => stats.st = stats.st.saturating_add(1),
        StatKind::Av => stats.av = stats.av.saturating_add(1),
        StatKind::Ag => stats.ag = stats.ag.saturating_sub(1),
        StatKind::Pa => stats.pa = stats.pa.saturating_sub(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::events::PlayerDomainEvent;
    use crate::app::players::domain::match_impact::{StatAdjustment, StatMalus};
    use crate::app::players::domain::player::{PlayerId, Spp, StatIncrease, TeamId, ValueKpo};
    use crate::app::players::domain::value_objects::{PositionNameVo, RosterLineId, SppCost};
    use crate::app::players::ports::{PositionAccessDto, PositionCatalogEntryDto, SkillCatalogEntryDto, SkillCostLevelDto};
    use crate::app::shared_kernel::common_types::SpaceId;

    struct FakeSkillCatalog;

    impl ISkillCatalogPort for FakeSkillCatalog {
        fn find_skill(&self, _skill_id: &str) -> Option<SkillCatalogEntryDto> { None }
        fn find_position(&self, roster_line_id: &str) -> Option<PositionCatalogEntryDto> {
            if roster_line_id == "BLITZER" {
                Some(PositionCatalogEntryDto {
                    position_name: "Frappeur".into(),
                    cost: 90_000,
                    ma: 7, st: 3, ag: 3, pa: 5, av: 8,
                    base_skills: vec![],
                    primary_categories: vec![],
                    secondary_categories: vec![],
                })
            } else {
                None
            }
        }
        fn position_access(&self, _roster_line_id: &str) -> Option<PositionAccessDto> { None }
        fn cost_for_level(&self, _level: u8, _is_elite: bool) -> Option<SkillCostLevelDto> { None }
        fn skill_value_delta(&self, _is_secondary_access: bool) -> u32 { 0 }
        fn stat_value_delta(&self, _stat: StatKind) -> u32 { 0 }
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
        let stats = resolve_stats(&player, &FakeSkillCatalog).unwrap();
        assert_eq!(stats, ResolvedPlayerStats { ma: 7, st: 3, ag: 3, pa: 5, av: 8 });
    }

    #[test]
    fn resolve_stats_applies_ma_st_av_malus_as_decrease() {
        let mut player = sample_player();
        player.stat_adjustments.push(StatAdjustment { stat: StatKind::Ma, malus: StatMalus::try_new(1).unwrap() });
        player.stat_adjustments.push(StatAdjustment { stat: StatKind::St, malus: StatMalus::try_new(1).unwrap() });
        player.stat_adjustments.push(StatAdjustment { stat: StatKind::Av, malus: StatMalus::try_new(1).unwrap() });
        let stats = resolve_stats(&player, &FakeSkillCatalog).unwrap();
        assert_eq!(stats.ma, 6);
        assert_eq!(stats.st, 2);
        assert_eq!(stats.av, 7);
    }

    #[test]
    fn resolve_stats_applies_ag_pa_malus_as_increase() {
        let mut player = sample_player();
        player.stat_adjustments.push(StatAdjustment { stat: StatKind::Ag, malus: StatMalus::try_new(1).unwrap() });
        let stats = resolve_stats(&player, &FakeSkillCatalog).unwrap();
        assert_eq!(stats.ag, 4);
    }

    #[test]
    fn resolve_stats_returns_none_for_unknown_position() {
        let mut player = sample_player();
        player.roster_line_id = RosterLineId::try_new("UNKNOWN".to_string()).unwrap();
        assert!(resolve_stats(&player, &FakeSkillCatalog).is_none());
    }

    #[test]
    fn resolve_stats_applies_ma_st_av_increase_as_increase() {
        let mut player = sample_player();
        player.stat_increases.push(StatIncrease { stat: StatKind::Ma, spp_cost: SppCost::try_new(1).unwrap(), value_delta: ValueKpo(0) });
        player.stat_increases.push(StatIncrease { stat: StatKind::St, spp_cost: SppCost::try_new(1).unwrap(), value_delta: ValueKpo(0) });
        player.stat_increases.push(StatIncrease { stat: StatKind::Av, spp_cost: SppCost::try_new(1).unwrap(), value_delta: ValueKpo(0) });
        let stats = resolve_stats(&player, &FakeSkillCatalog).unwrap();
        assert_eq!(stats.ma, 8);
        assert_eq!(stats.st, 4);
        assert_eq!(stats.av, 9);
    }

    #[test]
    fn resolve_stats_applies_ag_pa_increase_as_decrease() {
        let mut player = sample_player();
        player.stat_increases.push(StatIncrease { stat: StatKind::Ag, spp_cost: SppCost::try_new(1).unwrap(), value_delta: ValueKpo(0) });
        player.stat_increases.push(StatIncrease { stat: StatKind::Pa, spp_cost: SppCost::try_new(1).unwrap(), value_delta: ValueKpo(0) });
        let stats = resolve_stats(&player, &FakeSkillCatalog).unwrap();
        assert_eq!(stats.ag, 2);
        assert_eq!(stats.pa, 4);
    }
}

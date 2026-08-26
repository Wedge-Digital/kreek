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
pub fn resolve_stats(
    player: &Player,
    catalog: &dyn ISkillCatalogPort,
) -> Option<ResolvedPlayerStats> {
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
    // Troisième source, après les séquelles et les augmentations : les
    // ajustements donnés par un commissaire. Leur offset est déjà brut et
    // signé — le domaine a traduit au moment de la commande.
    for custo in &player.stat_customisations {
        apply_offset(&mut stats, custo.stat, custo.offset as i16);
    }
    Some(stats)
}

/// Un malus va toujours dans le sens de la dégradation — soit l'inverse d'un
/// cran d'amélioration, dont `StatKind::improvement_step()` détient le sens.
///
/// La table des directions vivait ici ; elle est descendue dans le domaine
/// (`StatKind`) parce que le panier de customisation en a besoin lui aussi, et
/// que deux tables auraient fini par diverger. Ce service **compose** — base,
/// séquelles, augmentations — le domaine dit dans quel sens.
fn apply_malus(stats: &mut ResolvedPlayerStats, stat: StatKind, malus: u8) {
    apply_offset(
        stats,
        stat,
        -(stat.improvement_step() as i16) * malus as i16,
    );
}

/// Une augmentation SPP va toujours dans le sens de l'amélioration.
fn apply_increase(stats: &mut ResolvedPlayerStats, stat: StatKind) {
    apply_offset(stats, stat, stat.improvement_step() as i16);
}

/// Applique un offset **brut** à la caractéristique, en saturant à zéro comme
/// le faisaient les deux fonctions d'origine. Les bornes de `StatKind` ne sont
/// pas appliquées ici : ce service **résout** ce que les événements ont
/// produit, il ne les juge pas. Le refus hors bornes appartient au panier, en
/// amont de l'écriture.
fn apply_offset(stats: &mut ResolvedPlayerStats, stat: StatKind, offset: i16) {
    let champ = match stat {
        StatKind::Ma => &mut stats.ma,
        StatKind::St => &mut stats.st,
        StatKind::Ag => &mut stats.ag,
        StatKind::Pa => &mut stats.pa,
        StatKind::Av => &mut stats.av,
    };
    *champ = (*champ as i16 + offset).max(0) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::events::PlayerDomainEvent;
    use crate::app::players::domain::match_impact::{StatAdjustment, StatMalus};
    use crate::app::players::domain::player::{
        PlayerId, Spp, StatCustomisation, StatIncrease, TeamId, ValueKpo,
    };
    use crate::app::players::domain::value_objects::{PositionNameVo, RosterLineId, SppCost};
    use crate::app::players::ports::{
        PositionAccessDto, PositionCatalogEntryDto, SkillCatalogEntryDto, SkillCostLevelDto,
        SppScaleDto,
    };
    use crate::app::shared_kernel::identity::ids::SpaceId;

    struct FakeSkillCatalog;

    impl ISkillCatalogPort for FakeSkillCatalog {
        fn find_skill(&self, _skill_id: &str) -> Option<SkillCatalogEntryDto> {
            None
        }
        fn list_all_skills(&self) -> Vec<SkillCatalogEntryDto> {
            vec![]
        }
        fn find_position(&self, roster_line_id: &str) -> Option<PositionCatalogEntryDto> {
            if roster_line_id == "BLITZER" {
                Some(PositionCatalogEntryDto {
                    position_name: "Frappeur".into(),
                    cost: 90_000,
                    ma: 7,
                    st: 3,
                    ag: 3,
                    pa: 5,
                    av: 8,
                    base_skills: vec![],
                    primary_categories: vec![],
                    secondary_categories: vec![],
                    keywords: vec![],
                })
            } else {
                None
            }
        }
        fn position_access(&self, _roster_line_id: &str) -> Option<PositionAccessDto> {
            None
        }
        fn cost_for_level(&self, _level: u8, _is_elite: bool) -> Option<SkillCostLevelDto> {
            None
        }
        fn skill_value_delta(&self, _is_secondary_access: bool, _is_elite: bool) -> u32 {
            0
        }
        fn stat_value_delta(&self, _stat: StatKind) -> u32 {
            0
        }
        fn spp_scale_for_roster_line(&self, _roster_line_id: &str) -> SppScaleDto {
            SppScaleDto {
                touchdown: 3,
                pass: 1,
                interception: 2,
                casualty: 2,
                mvp: 4,
            }
        }
    }

    fn sample_player() -> Player {
        let created = PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey: None,
            base_skills: vec![],
            starting_spp: Spp(0),
            starting_value: ValueKpo(90),
        };
        Player::from_events(&[created]).unwrap()
    }

    #[test]
    fn resolve_stats_returns_base_stats_when_no_adjustment() {
        let player = sample_player();
        let stats = resolve_stats(&player, &FakeSkillCatalog).unwrap();
        assert_eq!(
            stats,
            ResolvedPlayerStats {
                ma: 7,
                st: 3,
                ag: 3,
                pa: 5,
                av: 8
            }
        );
    }

    #[test]
    fn resolve_stats_applies_ma_st_av_malus_as_decrease() {
        let mut player = sample_player();
        player.stat_adjustments.push(StatAdjustment {
            stat: StatKind::Ma,
            malus: StatMalus::try_new(1).unwrap(),
        });
        player.stat_adjustments.push(StatAdjustment {
            stat: StatKind::St,
            malus: StatMalus::try_new(1).unwrap(),
        });
        player.stat_adjustments.push(StatAdjustment {
            stat: StatKind::Av,
            malus: StatMalus::try_new(1).unwrap(),
        });
        let stats = resolve_stats(&player, &FakeSkillCatalog).unwrap();
        assert_eq!(stats.ma, 6);
        assert_eq!(stats.st, 2);
        assert_eq!(stats.av, 7);
    }

    #[test]
    fn resolve_stats_applies_ag_pa_malus_as_increase() {
        let mut player = sample_player();
        player.stat_adjustments.push(StatAdjustment {
            stat: StatKind::Ag,
            malus: StatMalus::try_new(1).unwrap(),
        });
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
        player.stat_increases.push(StatIncrease {
            stat: StatKind::Ma,
            spp_cost: SppCost::try_new(1).unwrap(),
            value_delta: ValueKpo(0),
        });
        player.stat_increases.push(StatIncrease {
            stat: StatKind::St,
            spp_cost: SppCost::try_new(1).unwrap(),
            value_delta: ValueKpo(0),
        });
        player.stat_increases.push(StatIncrease {
            stat: StatKind::Av,
            spp_cost: SppCost::try_new(1).unwrap(),
            value_delta: ValueKpo(0),
        });
        let stats = resolve_stats(&player, &FakeSkillCatalog).unwrap();
        assert_eq!(stats.ma, 8);
        assert_eq!(stats.st, 4);
        assert_eq!(stats.av, 9);
    }

    #[test]
    fn resolve_stats_applies_ag_pa_increase_as_decrease() {
        let mut player = sample_player();
        player.stat_increases.push(StatIncrease {
            stat: StatKind::Ag,
            spp_cost: SppCost::try_new(1).unwrap(),
            value_delta: ValueKpo(0),
        });
        player.stat_increases.push(StatIncrease {
            stat: StatKind::Pa,
            spp_cost: SppCost::try_new(1).unwrap(),
            value_delta: ValueKpo(0),
        });
        let stats = resolve_stats(&player, &FakeSkillCatalog).unwrap();
        assert_eq!(stats.ag, 2);
        assert_eq!(stats.pa, 4);
    }

    /// Non-régression du déplacement de la table des directions vers `StatKind`
    /// (carte 302). Les valeurs résolues doivent être **exactement** celles
    /// d'avant : c'est le seul test qui prouve que le déplacement n'a rien
    /// changé, le compilateur ne pouvant rien en dire.
    ///
    /// Base du poste BLITZER : MV 7, FO 3, AG 3+, PA 5+, AR 8+.
    #[test]
    fn le_deplacement_de_la_table_ne_change_aucune_valeur_resolue() {
        // Base nue.
        let joueur = joueur_avec(vec![], vec![]);
        let s = resolve_stats(&joueur, &FakeSkillCatalog).unwrap();
        assert_eq!((s.ma, s.st, s.ag, s.pa, s.av), (7, 3, 3, 5, 8));

        // Une augmentation par caractéristique : MV/FO/AR montent, AG/PA descendent.
        let joueur = joueur_avec(
            vec![
                StatKind::Ma,
                StatKind::St,
                StatKind::Ag,
                StatKind::Pa,
                StatKind::Av,
            ],
            vec![],
        );
        let s = resolve_stats(&joueur, &FakeSkillCatalog).unwrap();
        assert_eq!((s.ma, s.st, s.ag, s.pa, s.av), (8, 4, 2, 4, 9));

        // Une séquelle par caractéristique : l'exact inverse.
        let joueur = joueur_avec(
            vec![],
            vec![
                StatKind::Ma,
                StatKind::St,
                StatKind::Ag,
                StatKind::Pa,
                StatKind::Av,
            ],
        );
        let s = resolve_stats(&joueur, &FakeSkillCatalog).unwrap();
        assert_eq!((s.ma, s.st, s.ag, s.pa, s.av), (6, 2, 4, 6, 7));
    }

    /// La troisième source : les customisations se composent avec les deux
    /// autres, sans les remplacer.
    #[test]
    fn les_customisations_se_cumulent_avec_les_autres_sources() {
        let mut joueur = joueur_avec(vec![StatKind::Ag], vec![StatKind::Ma]);
        joueur.stat_customisations.push(StatCustomisation {
            stat: StatKind::Ag,
            offset: -1,
        });
        joueur.stat_customisations.push(StatCustomisation {
            stat: StatKind::Ma,
            offset: 2,
        });

        let s = resolve_stats(&joueur, &FakeSkillCatalog).unwrap();
        // AG : 3 base, -1 augmentation, -1 customisation → 1+
        assert_eq!(s.ag, 1);
        // MV : 7 base, -1 séquelle, +2 customisation → 8
        assert_eq!(s.ma, 8);
    }

    fn joueur_avec(augmentations: Vec<StatKind>, sequelles: Vec<StatKind>) -> Player {
        let created = PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey: None,
            base_skills: vec![],
            starting_spp: Spp(0),
            starting_value: ValueKpo(100),
        };
        let mut joueur = Player::from_events(&[created]).unwrap();
        for stat in augmentations {
            joueur.stat_increases.push(StatIncrease {
                stat,
                spp_cost: SppCost::try_new(0).unwrap(),
                value_delta: ValueKpo(0),
            });
        }
        for stat in sequelles {
            joueur.stat_adjustments.push(StatAdjustment {
                stat,
                malus: StatMalus::try_new(1).unwrap(),
            });
        }
        joueur
    }
}

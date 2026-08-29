//! Les view models des panneaux qui dépendent d'un **port** autant que du
//! domaine.
//!
//! Le `CLAUDE.md` réserve `from_domain()` aux VMs purement domaine ; ceux qui
//! croisent un DTO de port vivent ici, pour que `view_models` n'importe jamais
//! les types du port.

use crate::app::competitions::domain::competition_rules::RankingRules;
use crate::app::competitions::domain::competition_rules::TierRule;
use crate::app::competitions::domain::competition_structure::RankingGroupConfig;
use crate::app::competitions::domain::group_repository_port::GroupWithTeams;
use crate::app::competitions::io::web::admin::settings::pools_panel::{PoolRowVm, PoolsVm};
use crate::app::competitions::io::web::admin::settings::ranking_panel::{
    BonusVm, RankingVm, TiebreakRowVm,
};
use crate::app::competitions::io::web::admin::settings::tiers_panel::{ChipVm, TierVm};
use crate::app::competitions::ports::{ICompetitionReferencePort, ITiebreakCatalogPort};

/// Le barème et ses critères de départage, joints au catalogue.
///
/// # Une jointure **ordonnée**, et sa règle de fin
///
/// L'ordre vient de la `TiebreakConfig` enregistrée — c'est lui qui porte la
/// priorité. Les libellés viennent du catalogue. Et **les critères du catalogue
/// absents de la configuration s'ajoutent à la fin, désactivés**.
///
/// Sans cette dernière règle, un critère ajouté au catalogue serait invisible
/// pour toutes les compétitions existantes : leur configuration ne le
/// mentionnant pas, il n'apparaîtrait jamais à l'écran, et personne ne pourrait
/// l'activer. Le catalogue grandirait sans que rien ne le montre.
pub fn build_ranking_vm(rules: &RankingRules, catalog: &dyn ITiebreakCatalogPort) -> RankingVm {
    let connus = catalog.all();
    let libelle = |code: &str| {
        connus
            .iter()
            .find(|c| c.code == code)
            .map(|c| c.label.clone())
            // Un code enregistré que le catalogue ne connaît plus : on le montre
            // sous son code plutôt que de l'escamoter. Le faire disparaître
            // changerait la priorité des suivants sans le dire.
            .unwrap_or_else(|| code.to_string())
    };

    let mut lignes: Vec<TiebreakRowVm> = rules
        .tiebreakers
        .settings()
        .iter()
        .map(|s| TiebreakRowVm {
            code: s.code.as_ref().to_string(),
            label: libelle(s.code.as_ref()),
            activated: s.activated.0,
        })
        .collect();

    for critere in &connus {
        if !lignes.iter().any(|l| l.code == critere.code) {
            lignes.push(TiebreakRowVm {
                code: critere.code.clone(),
                label: critere.label.clone(),
                activated: false,
            });
        }
    }

    RankingVm {
        win_points: rules.win_points.into_inner(),
        draw_points: rules.draw_points.into_inner(),
        lose_points: rules.lose_points.into_inner(),
        offensive: BonusVm {
            activated: rules.offensive_bonus.activated.0,
            threshold: rules.offensive_bonus.min_td.into_inner(),
            points: rules.offensive_bonus.points.into_inner(),
        },
        defensive: BonusVm {
            activated: rules.defensive_bonus.activated.0,
            threshold: rules.defensive_bonus.max_td_conceded.into_inner(),
            points: rules.defensive_bonus.points.into_inner(),
        },
        aggressive: BonusVm {
            activated: rules.aggressive_bonus.activated.0,
            threshold: rules.aggressive_bonus.min_casualties.into_inner(),
            points: rules.aggressive_bonus.points.into_inner(),
        },
        tiebreakers: lignes,
        recompute: None,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::competition_rules::{
        Activated, AggressiveBonus, DefensiveBonus, MaxTdConceded, MinCasualties, MinTd,
        OffensiveBonus, RankingPoints, TiebreakCode, TiebreakConfig, TiebreakSetting,
    };
    use crate::app::competitions::ports::TiebreakCriterionDto;

    struct FakeCatalogue(Vec<(&'static str, &'static str)>);

    impl ITiebreakCatalogPort for FakeCatalogue {
        fn all(&self) -> Vec<TiebreakCriterionDto> {
            self.0
                .iter()
                .map(|(code, label)| TiebreakCriterionDto {
                    code: code.to_string(),
                    label: label.to_string(),
                })
                .collect()
        }
    }

    fn reglage(code: &str, actif: bool) -> TiebreakSetting {
        TiebreakSetting {
            code: TiebreakCode::try_new(code).unwrap(),
            activated: Activated(actif),
        }
    }

    fn bareme(tiebreakers: TiebreakConfig) -> RankingRules {
        RankingRules {
            win_points: RankingPoints::try_new(3).unwrap(),
            draw_points: RankingPoints::try_new(1).unwrap(),
            lose_points: RankingPoints::try_new(0).unwrap(),
            offensive_bonus: OffensiveBonus {
                activated: Activated(true),
                min_td: MinTd::try_new(2).unwrap(),
                points: RankingPoints::try_new(1).unwrap(),
            },
            defensive_bonus: DefensiveBonus {
                activated: Activated(false),
                points: RankingPoints::try_new(1).unwrap(),
                max_td_conceded: MaxTdConceded::try_new(1).unwrap(),
            },
            aggressive_bonus: AggressiveBonus {
                activated: Activated(false),
                points: RankingPoints::try_new(1).unwrap(),
                min_casualties: MinCasualties::try_new(3).unwrap(),
            },
            tiebreakers,
        }
    }

    /// **L'ordre vient de la configuration, les libellés du catalogue.** Le
    /// catalogue est délibérément donné dans un ordre différent : c'est ce qui
    /// prouve que la jointure ne le suit pas.
    #[test]
    fn la_jointure_suit_l_ordre_de_la_configuration_pas_du_catalogue() {
        let config =
            TiebreakConfig::try_new(vec![reglage("nb_td", true), reglage("diff_td", false)])
                .unwrap();
        let catalogue = FakeCatalogue(vec![
            ("diff_td", "Différence de touchdowns"),
            ("nb_td", "Touchdowns marqués"),
        ]);

        let vm = build_ranking_vm(&bareme(config), &catalogue);

        let codes: Vec<&str> = vm.tiebreakers.iter().map(|t| t.code.as_str()).collect();
        assert_eq!(codes, vec!["nb_td", "diff_td"]);
        assert_eq!(vm.tiebreakers[0].label, "Touchdowns marqués");
        assert!(vm.tiebreakers[0].activated);
        assert!(!vm.tiebreakers[1].activated);
    }

    /// **Un critère ajouté au catalogue rejoint la fin, désactivé.**
    ///
    /// Sans cette règle il serait invisible pour toutes les compétitions
    /// existantes — leur configuration ne le mentionnant pas, il n'apparaîtrait
    /// jamais à l'écran et personne ne pourrait l'activer. Le catalogue
    /// grandirait sans que rien ne le montre.
    #[test]
    fn un_critere_du_catalogue_absent_de_la_configuration_rejoint_la_fin_desactive() {
        let config = TiebreakConfig::try_new(vec![reglage("nb_td", true)]).unwrap();
        let catalogue = FakeCatalogue(vec![
            ("nb_td", "Touchdowns marqués"),
            ("nouveau", "Confrontation directe"),
        ]);

        let vm = build_ranking_vm(&bareme(config), &catalogue);

        assert_eq!(vm.tiebreakers.len(), 2);
        assert_eq!(vm.tiebreakers[1].code, "nouveau");
        assert_eq!(vm.tiebreakers[1].label, "Confrontation directe");
        assert!(
            !vm.tiebreakers[1].activated,
            "un critère jamais configuré arrive éteint"
        );
    }

    /// Un code enregistré que le catalogue ne connaît plus **reste affiché**,
    /// sous son code. L'escamoter changerait la priorité des suivants sans le
    /// dire.
    #[test]
    fn un_code_inconnu_du_catalogue_reste_affiche_sous_son_code() {
        let config =
            TiebreakConfig::try_new(vec![reglage("disparu", true), reglage("nb_td", true)])
                .unwrap();
        let catalogue = FakeCatalogue(vec![("nb_td", "Touchdowns marqués")]);

        let vm = build_ranking_vm(&bareme(config), &catalogue);

        assert_eq!(vm.tiebreakers.len(), 2);
        assert_eq!(vm.tiebreakers[0].code, "disparu");
        assert_eq!(vm.tiebreakers[0].label, "disparu");
        assert_eq!(vm.tiebreakers[1].code, "nb_td");
    }

    #[test]
    fn les_trois_bonus_portent_leur_seuil_respectif() {
        let config = TiebreakConfig::try_new(vec![reglage("nb_td", true)]).unwrap();
        let vm = build_ranking_vm(&bareme(config), &FakeCatalogue(vec![]));

        assert_eq!((vm.win_points, vm.draw_points, vm.lose_points), (3, 1, 0));
        assert!(vm.offensive.activated);
        assert_eq!(vm.offensive.threshold, 2, "TD d'écart");
        assert_eq!(vm.defensive.threshold, 1, "TD encaissés au plus");
        assert_eq!(vm.aggressive.threshold, 3, "sorties");
        assert!(vm.recompute.is_none(), "aucun rejeu n'a eu lieu");
    }
}

/// Les poules déclarées, **jointes à leurs affectations d'équipes**.
///
/// # Pourquoi le compteur ne vient pas du JSONB
///
/// La déclaration ne sait pas qui joue où : seule `competition_group_teams` le
/// sait, et c'est elle que la cascade videra. Un compteur lu dans la structure
/// afficherait « 0 équipe à réaffecter » sur une poule qui en porte six — et le
/// commissaire retirerait la poule en croyant que c'est sans conséquence.
///
/// L'ordre vient de la **déclaration**, pas de la table : c'est lui que l'écran
/// montre et que l'enregistrement réécrit.
pub fn build_pools_vm(config: &RankingGroupConfig, affectations: &[GroupWithTeams]) -> PoolsVm {
    PoolsVm {
        use_pools: config.use_ranking_groups(),
        pools: config
            .groups()
            .iter()
            .map(|g| {
                let assigned_teams = affectations
                    .iter()
                    .find(|a| a.group_id == g.id.as_ref())
                    .map(|a| a.team_ids.len() as u32)
                    // Une poule déclarée que la table ne connaît pas encore :
                    // la projection est paresseuse, elle n'a lieu qu'à
                    // l'ouverture de l'onglet Poules. Zéro est la vérité.
                    .unwrap_or(0);
                PoolRowVm {
                    id: g.id.as_ref().to_string(),
                    name: g.name.as_ref().to_string(),
                    assigned_teams,
                    assigned_label: match assigned_teams {
                        0 => "aucune équipe".to_string(),
                        1 => "1 équipe".to_string(),
                        n => format!("{n} équipes"),
                    },
                }
            })
            .collect(),
    }
}

/// Les tiers, leurs uid résolus en noms lisibles.
///
/// # Un uid non résolu s'affiche tel quel
///
/// Un coup de pouce retiré du corpus doit **se voir**, pas s'évaporer. Le faire
/// disparaître ferait croire au commissaire qu'il ne l'avait jamais autorisé —
/// et l'enregistrement suivant le supprimerait pour de bon, sans qu'il l'ait
/// décidé.
pub fn build_tiers_vm(tiers: &[TierRule], refs: &dyn ICompetitionReferencePort) -> Vec<TierVm> {
    tiers
        .iter()
        .enumerate()
        .map(|(rang, t)| {
            let puces = |uids: &[String], resoudre: &dyn Fn(&str) -> Option<String>| {
                uids.iter()
                    .map(|uid| ChipVm {
                        uid: uid.clone(),
                        label: resoudre(uid).unwrap_or_else(|| uid.clone()),
                    })
                    .collect::<Vec<_>>()
            };
            TierVm {
                // 1-indexé : c'est ce que portent `.tier-1`, `.tier-2`, `.tier-3`.
                index: (rang as u8) + 1,
                name: t.name.as_ref().to_string(),
                budget_kpo: t.budget.0,
                starting_xp: t.starting_xp.into_inner(),
                roster_names: t
                    .rosters
                    .iter()
                    .map(|uid| refs.find_roster_name(uid).unwrap_or_else(|| uid.clone()))
                    .collect(),
                inducements: puces(&t.inducements, &|uid| refs.find_inducement_name(uid)),
                star_players: puces(&t.star_players, &|uid| refs.find_star_player_name(uid)),
                // Le nom du tier fait l'affaire : il est unique dans une
                // compétition, et le sélecteur n'en demande pas plus.
                picker_instance_id: t.name.as_ref().to_string(),
                selected_inducements: t.inducements.join(","),
                selected_star_players: t.star_players.join(","),
                frozen_json: serde_json::json!({
                    "name": t.name.as_ref(),
                    "budget": t.budget.0,
                    "starting_xp": t.starting_xp.into_inner(),
                    "rosters": t.rosters,
                    "inducements": t.inducements,
                    "star_players": t.star_players,
                })
                .to_string(),
            }
        })
        .collect()
}

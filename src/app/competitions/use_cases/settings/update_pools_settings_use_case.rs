//! Renommer, ajouter et retirer des poules sur une saison **en cours**.
//!
//! # Ce que le retrait défait vraiment
//!
//! Les poules vivent à deux endroits : la déclaration dans le JSONB de la
//! structure, et la table `competition_groups` — qui porte les affectations
//! d'équipes. La seconde n'est jamais purgée par le projecteur, qui ne fait
//! qu'`INSERT … ON CONFLICT DO UPDATE`. Sans suppression explicite, retirer une
//! poule serait **cosmétique** : sa ligne et ses équipes resteraient.
//!
//! D'où `save_structure_and_prune_groups`, qui écrit et supprime dans une seule
//! transaction, et rend le nombre d'affectations défaites par la cascade.
//!
//! # Ce que le retrait ne défait pas
//!
//! Les **points** des équipes désaffectées. Une poule est un regroupement de
//! classement, pas une appartenance : la retirer ne change ni les matchs joués,
//! ni les lignes de classement.

use crate::app::competitions::domain::competition_structure::{
    CompetitionStructure, DispatchType, RankingGroup, RankingGroupConfig, RankingGroupName,
    UseRankingGroups,
};
use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonRepositoryError,
};
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::app::shared_kernel::bloodbowl::ranking_group_id::RankingGroupId;
use crate::app::shared_kernel::identity::id_service::IdService;

/// Une poule soumise. `id` absent = poule neuve, à qui le **serveur** attribue
/// son identifiant.
#[derive(Debug)]
pub struct PoolInput {
    pub id: Option<RankingGroupId>,
    pub name: RankingGroupName,
}

#[derive(Debug)]
pub struct UpdatePoolsSettingsCommand {
    pub season_id: SeasonId,
    pub use_pools: UseRankingGroups,
    pub pools: Vec<PoolInput>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PoolsSettingsOutcome {
    /// Les équipes que le retrait a désaffectées. C'est ce que l'écran annonce :
    /// « 6 équipes à réaffecter ».
    pub unassigned_teams: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdatePoolsSettingsError {
    SeasonNotFound,
    /// Doublon de nom ou d'identifiant. **Le refus vient du domaine** (carte
    /// 417) : ce use case ne le rejuge pas, il le relaie.
    InvalidPools(DomainError),
    Database(String),
}

impl From<SeasonRepositoryError> for UpdatePoolsSettingsError {
    fn from(e: SeasonRepositoryError) -> Self {
        match e {
            SeasonRepositoryError::SeasonNotFound => Self::SeasonNotFound,
            autre => Self::Database(autre.to_string()),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute<T: IdService>(
    cmd: UpdatePoolsSettingsCommand,
    season_repo: &dyn ISeasonRepository,
    id_service: &T,
) -> Result<PoolsSettingsOutcome, UpdatePoolsSettingsError> {
    let courante = season_repo
        .find_structure(&cmd.season_id)
        .await?
        .ok_or(UpdatePoolsSettingsError::SeasonNotFound)?;

    let groupes: Vec<RankingGroup> = cmd
        .pools
        .into_iter()
        .map(|p| RankingGroup {
            id: p.id.unwrap_or_else(|| identifiant_neuf(id_service)),
            name: p.name,
        })
        .collect();
    let gardes: Vec<String> = groupes.iter().map(|g| g.id.as_ref().to_string()).collect();

    let config = RankingGroupConfig::try_new(
        cmd.use_pools,
        // Le mode de répartition n'est pas rouvert par ce panneau : il est repris
        // tel quel, comme le calendrier et la phase finale.
        courante.ranking_group.dispatch_type().clone(),
        groupes,
    )
    .map_err(UpdatePoolsSettingsError::InvalidPools)?;

    // **Le `schedule` est conservé.** `save_structure_*` écrit la structure
    // entière : le reconstruire à vide effacerait le calendrier de la saison,
    // sans erreur et sans trace.
    //
    // `play_offs_phase` était conservé pour la même raison jusqu'à la carte 412,
    // qui l'a retiré du modèle — il ne reste donc qu'un champ à préserver.
    let structure = CompetitionStructure {
        ranking_group: config,
        schedule: courante.schedule,
    };

    let defaites = season_repo
        .save_structure_and_prune_groups(&cmd.season_id, &structure, &gardes)
        .await?;

    Ok(PoolsSettingsOutcome {
        unassigned_teams: defaites as u32,
    })
}

/// L'identifiant d'une poule neuve, **engendré côté serveur**.
///
/// Le magicien de création le laisse fabriquer par le navigateur
/// (`new-competition-phase-3.html`, `genId()`). Un identifiant de domaine minté
/// par le client n'est contrôlé ni en forme, ni en unicité, ni en provenance.
///
/// La forme est imposée par `RankingGroupId`, qui valide `^g[0-9a-z]+$` : un
/// ULID nu — majuscule et sans préfixe — **est refusé**. D'où le `g` et les
/// minuscules, qui reproduisent la forme du magicien sans lui laisser la main.
fn identifiant_neuf<T: IdService>(id_service: &T) -> RankingGroupId {
    let brut = format!("g{}", id_service.generate_id().to_string().to_lowercase());
    RankingGroupId::try_new(brut).expect("« g » + un ULID en minuscules satisfait ^g[0-9a-z]+$")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
    use crate::app::competitions::domain::competition_notifications::CompetitionNotifications;
    use crate::app::competitions::domain::competition_rules::CompetitionRules;
    use crate::app::competitions::domain::competition_season::CompetitionSeason;
    use crate::app::competitions::domain::competition_structure::{
        ScheduleConfig, ScheduleType, UseSchedule,
    };
    use crate::app::competitions::domain::season_repository_port::{SeasonBaseInfo, SeasonFull};
    use crate::app::shared_kernel::bloodbowl::date_string::DateString;
    use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
    use crate::app::shared_kernel::identity::id_service::FakeIdService;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct FakeSeasonRepo {
        structure: Option<CompetitionStructure>,
        /// La structure et les identifiants gardés, tels que reçus.
        ecrit: Mutex<Option<(CompetitionStructure, Vec<String>)>>,
        defaites: u64,
    }

    #[async_trait]
    impl ISeasonRepository for FakeSeasonRepo {
        async fn save(&self, _: &CompetitionSeason) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn find_latest_season_id(
            &self,
            _: &CompetitionId,
        ) -> Result<Option<SeasonId>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn find_space_id(
            &self,
            _: &SeasonId,
        ) -> Result<Option<String>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn find_base_info(
            &self,
            _: &SeasonId,
        ) -> Result<Option<SeasonBaseInfo>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn find_rules(
            &self,
            _: &SeasonId,
        ) -> Result<Option<CompetitionRules>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn save_rules(
            &self,
            _: &SeasonId,
            _: &str,
            _: &CompetitionRules,
        ) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn find_structure(
            &self,
            _: &SeasonId,
        ) -> Result<Option<CompetitionStructure>, SeasonRepositoryError> {
            Ok(self.structure.clone())
        }
        async fn save_structure(
            &self,
            _: &SeasonId,
            _: &CompetitionStructure,
        ) -> Result<(), SeasonRepositoryError> {
            unimplemented!("ce use case passe par save_structure_and_prune_groups")
        }
        async fn save_structure_and_prune_groups(
            &self,
            _: &SeasonId,
            structure: &CompetitionStructure,
            kept_ids: &[String],
        ) -> Result<u64, SeasonRepositoryError> {
            *self.ecrit.lock().unwrap() = Some((structure.clone(), kept_ids.to_vec()));
            Ok(self.defaites)
        }
        async fn find_invitations(
            &self,
            _: &SeasonId,
        ) -> Result<Option<CompetitionInvitations>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn save_invitations(
            &self,
            _: &SeasonId,
            _: &CompetitionInvitations,
            _: &CompetitionNotifications,
        ) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn save_visibility(
            &self,
            _: &SeasonId,
            _: &CompetitionInvitations,
        ) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn find_notifications(
            &self,
            _: &SeasonId,
        ) -> Result<Option<CompetitionNotifications>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn save_notifications(
            &self,
            _: &SeasonId,
            _: &CompetitionNotifications,
        ) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn set_ready(&self, _: &SeasonId) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn find_full(
            &self,
            _: &SeasonId,
        ) -> Result<Option<SeasonFull>, SeasonRepositoryError> {
            Ok(None)
        }
    }

    // ── Fixtures ─────────────────────────────────────────────────────────────

    const DATE_DEBUT: &str = "2026-09-01";

    fn groupe(id: &str, nom: &str) -> RankingGroup {
        RankingGroup {
            id: RankingGroupId::try_new(id.to_string()).unwrap(),
            name: RankingGroupName::try_new(nom.to_string()).unwrap(),
        }
    }

    /// La structure d'origine porte **un calendrier renseigné et une phase
    /// finale à trois qualifiés** : deux valeurs distinctes de leur défaut, pour
    /// qu'un test qui les perdrait s'en aperçoive.
    fn structure_origine(groupes: Vec<RankingGroup>) -> CompetitionStructure {
        CompetitionStructure {
            ranking_group: RankingGroupConfig::try_new(
                UseRankingGroups(true),
                DispatchType::Manual,
                groupes,
            )
            .unwrap(),
            schedule: ScheduleConfig {
                use_schedule: UseSchedule(true),
                schedule_type: ScheduleType::default(),
                schedule_start_date: DateString::try_new(DATE_DEBUT.to_string()).unwrap(),
                schedule_end_date: DateString::default(),
                scheduled_dates: vec![],
            },
        }
    }

    fn depot(groupes: Vec<RankingGroup>, defaites: u64) -> FakeSeasonRepo {
        FakeSeasonRepo {
            structure: Some(structure_origine(groupes)),
            ecrit: Mutex::new(None),
            defaites,
        }
    }

    fn nom(v: &str) -> RankingGroupName {
        RankingGroupName::try_new(v.to_string()).unwrap()
    }

    fn commande(pools: Vec<PoolInput>) -> UpdatePoolsSettingsCommand {
        UpdatePoolsSettingsCommand {
            season_id: SeasonId::new(),
            use_pools: UseRankingGroups(!pools.is_empty()),
            pools,
        }
    }

    // ── Les scénarios ────────────────────────────────────────────────────────

    /// **Le test le plus important de la carte.** Son échec ne produirait aucune
    /// erreur : juste un calendrier vide, découvert des jours plus tard.
    ///
    /// `save_structure_and_prune_groups` écrit la structure **entière** ; la
    /// reconstruire sans relire le calendrier et la phase finale les effacerait
    /// en silence.
    #[tokio::test]
    async fn le_calendrier_et_la_phase_finale_survivent() {
        let depot = depot(vec![groupe("ga", "Poule A")], 0);

        execute(
            commande(vec![PoolInput {
                id: Some(RankingGroupId::try_new("ga".to_string()).unwrap()),
                name: nom("Renommée"),
            }]),
            &depot,
            &FakeIdService::new(),
        )
        .await
        .expect("renommage");

        let (ecrite, _) = depot.ecrit.lock().unwrap().clone().expect("écriture");
        assert_eq!(
            ecrite.schedule.schedule_start_date.as_ref(),
            DATE_DEBUT,
            "le calendrier a été effacé"
        );
        assert!(ecrite.schedule.use_schedule.0);
        // Le mode de répartition n'est pas rouvert par ce panneau : il est repris.
        assert!(matches!(
            ecrite.ranking_group.dispatch_type(),
            DispatchType::Manual
        ));
    }

    /// **Une poule neuve reçoit son identifiant du serveur.** Un identifiant de
    /// domaine minté par le navigateur ne serait contrôlé ni en forme, ni en
    /// unicité, ni en provenance.
    #[tokio::test]
    async fn une_poule_neuve_recoit_un_identifiant_engendre() {
        let depot = depot(vec![], 0);

        execute(
            commande(vec![PoolInput {
                id: None,
                name: nom("Poule neuve"),
            }]),
            &depot,
            &FakeIdService::new(),
        )
        .await
        .expect("ajout");

        let (ecrite, gardes) = depot.ecrit.lock().unwrap().clone().expect("écriture");
        let id = ecrite.ranking_group.groups()[0].id.as_ref().to_string();
        assert!(
            id.starts_with('g')
                && id
                    .chars()
                    .skip(1)
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            "l'identifiant doit satisfaire ^g[0-9a-z]+$ : « {id} »"
        );
        assert_eq!(gardes, vec![id], "il est gardé, donc pas supprimé aussitôt");
    }

    /// Une poule existante **garde** son identifiant : le renommage ne la
    /// recrée pas, sinon ses équipes seraient désaffectées au passage.
    #[tokio::test]
    async fn renommer_une_poule_conserve_son_identifiant() {
        let depot = depot(vec![groupe("ga", "Poule A")], 0);

        execute(
            commande(vec![PoolInput {
                id: Some(RankingGroupId::try_new("ga".to_string()).unwrap()),
                name: nom("Renommée"),
            }]),
            &depot,
            &FakeIdService::new(),
        )
        .await
        .unwrap();

        let (ecrite, gardes) = depot.ecrit.lock().unwrap().clone().expect("écriture");
        assert_eq!(ecrite.ranking_group.groups()[0].id.as_ref(), "ga");
        assert_eq!(ecrite.ranking_group.groups()[0].name.as_ref(), "Renommée");
        assert_eq!(gardes, vec!["ga".to_string()]);
    }

    /// **Retirer toutes les poules n'est pas un cas particulier** : `kept_ids`
    /// vide, tout part. Aucune branche à écrire — et c'est le signe que la forme
    /// est juste.
    #[tokio::test]
    async fn retirer_toutes_les_poules_est_autorise() {
        let depot = depot(vec![groupe("ga", "Poule A"), groupe("gb", "Poule B")], 6);

        let issue = execute(commande(vec![]), &depot, &FakeIdService::new())
            .await
            .expect("retrait total");

        assert_eq!(
            issue,
            PoolsSettingsOutcome {
                unassigned_teams: 6
            }
        );
        let (ecrite, gardes) = depot.ecrit.lock().unwrap().clone().expect("écriture");
        assert!(gardes.is_empty(), "rien n'est gardé");
        assert!(ecrite.ranking_group.groups().is_empty());
        assert!(!ecrite.ranking_group.use_ranking_groups());
    }

    /// Le refus des doublons vient du **domaine** (carte 417), pas de ce use
    /// case. Il le relaie sans le rejuger.
    #[tokio::test]
    async fn un_doublon_de_nom_est_refuse_par_le_domaine() {
        let depot = depot(vec![], 0);

        let issue = execute(
            commande(vec![
                PoolInput {
                    id: None,
                    name: nom("Même nom"),
                },
                PoolInput {
                    id: Some(RankingGroupId::try_new("gb".to_string()).unwrap()),
                    name: nom("Même nom"),
                },
            ]),
            &depot,
            &FakeIdService::new(),
        )
        .await;

        assert!(
            matches!(
                issue,
                Err(UpdatePoolsSettingsError::InvalidPools(
                    DomainError::DuplicatePoolName { .. }
                ))
            ),
            "attendu un doublon de nom : {issue:?}"
        );
        assert!(
            depot.ecrit.lock().unwrap().is_none(),
            "un refus ne doit rien écrire"
        );
    }

    #[tokio::test]
    async fn une_saison_sans_structure_est_refusee() {
        let depot = FakeSeasonRepo {
            structure: None,
            ecrit: Mutex::new(None),
            defaites: 0,
        };

        let issue = execute(commande(vec![]), &depot, &FakeIdService::new()).await;

        assert_eq!(issue, Err(UpdatePoolsSettingsError::SeasonNotFound));
    }
}

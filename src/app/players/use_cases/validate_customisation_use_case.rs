//! Application du panier de customisation, et son annulation.

use crate::app::players::domain::customisation_basket::{CustomisationLine, RejectedLine};
use crate::app::players::domain::error::DomainError;
use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::Player;
use crate::app::players::domain::value_objects::{CustomisationId, SkillName};
use crate::app::players::ports::{
    ICustomisationBasketRepository, IPlayerRepository, ISkillCatalogPort, RepositoryError,
};
use crate::app::players::use_cases::commands::{
    CancelCustomisationCommand, ValidateCustomisationCommand,
};
use crate::app::players::use_cases::customisation_basket_hydration_service::{
    hydrate, HydrationError,
};
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub enum ValidateCustomisationError {
    PlayerNotFound,
    NothingToApply,
    /// Le panier a changé entre le rendu du panneau et la validation. Ni une
    /// faute d'utilisateur, ni un bug : le panneau re-rendu porte l'état réel.
    ConcurrentWrite,
    /// Une ou plusieurs lignes ne sont plus applicables — **rien** n'est
    /// appliqué.
    LinesRejected(Vec<RejectedLine>),
    /// Autant d'identifiants que de lignes : c'est le handler qui les engendre,
    /// une divergence est un bug d'appelant.
    IdentifiantsManquants,
    Domain(DomainError),
    Hydration(HydrationError),
    Repository(RepositoryError),
}

impl From<RepositoryError> for ValidateCustomisationError {
    fn from(e: RepositoryError) -> Self {
        Self::Repository(e)
    }
}

impl From<HydrationError> for ValidateCustomisationError {
    fn from(e: HydrationError) -> Self {
        match e {
            HydrationError::PlayerNotFound => Self::PlayerNotFound,
            autre => Self::Hydration(autre),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ValidateCustomisationCommand,
    player_repo: &dyn IPlayerRepository,
    basket_repo: &dyn ICustomisationBasketRepository,
    catalog: &dyn ISkillCatalogPort,
    event_bus: &EventBus,
) -> Result<(), ValidateCustomisationError> {
    let (basket, player) = hydrate(&cmd.player_id, player_repo, basket_repo, catalog).await?;

    // Avant toute chose : le panier est-il bien celui que l'appelant a vu ?
    // Il a compté ses lignes sur une lecture antérieure pour engendrer un
    // identifiant par ligne ; si le contenu a bougé depuis, ses identifiants ne
    // correspondent plus à rien.
    if basket.version().0 != cmd.expected_version {
        return Err(ValidateCustomisationError::ConcurrentWrite);
    }

    if basket.is_empty() {
        return Err(ValidateCustomisationError::NothingToApply);
    }

    // Revalidation intégrale : une ligne valide à l'ajout peut ne plus l'être —
    // compétence acquise entre-temps, borne atteinte par une autre voie. Tout
    // ou rien.
    let retenues = basket
        .validate_all()
        .map_err(ValidateCustomisationError::LinesRejected)?;

    if cmd.customisation_ids.len() != retenues.len() {
        return Err(ValidateCustomisationError::IdentifiantsManquants);
    }

    let events = construire_evenements(&player, &retenues, &cmd, catalog)?;

    // **Supprimer avant d'appendre.** Les deux tables sont écrites par deux
    // transactions : une panne entre elles perd la saisie sans rien appliquer,
    // là où l'ordre inverse écrirait deux fois des customisations sur des
    // données de jeu — ce qui ne se découvrirait que bien plus tard.
    basket_repo.delete(&cmd.player_id.0).await?;

    let entrees: Vec<_> = events
        .iter()
        .enumerate()
        .map(|(rang, event)| {
            (
                player.id.clone(),
                player.team_id.clone(),
                event.clone(),
                player.version + 1 + rang as i32,
            )
        })
        .collect();
    player_repo.append_batch(entrees).await?;

    for event in &events {
        let _ = event_bus.send(event.to_enveloppe(&player.id.0));
    }

    Ok(())
}

/// Traduit chaque ligne retenue en événement domaine, via les méthodes de
/// `Player` — c'est le domaine qui traduit les crans en offset brut.
fn construire_evenements(
    player: &Player,
    lignes: &[CustomisationLine],
    cmd: &ValidateCustomisationCommand,
    catalog: &dyn ISkillCatalogPort,
) -> Result<Vec<PlayerDomainEvent>, ValidateCustomisationError> {
    lignes
        .iter()
        .zip(cmd.customisation_ids.iter())
        .map(|(ligne, id)| evenement_pour(player, ligne, id.clone(), &cmd.author, catalog))
        .collect()
}

fn evenement_pour(
    player: &Player,
    ligne: &CustomisationLine,
    id: CustomisationId,
    author: &str,
    catalog: &dyn ISkillCatalogPort,
) -> Result<PlayerDomainEvent, ValidateCustomisationError> {
    let issue = match ligne {
        CustomisationLine::Skill { skill_id, .. } => {
            // Le nom vient du catalogue : le domaine ne sait pas le résoudre,
            // et l'événement doit le porter pour être rejouable seul.
            let nom = catalog
                .find_skill(skill_id.as_ref())
                .and_then(|s| SkillName::try_new(s.name).ok())
                .ok_or(ValidateCustomisationError::Domain(
                    DomainError::UnknownSkill,
                ))?;
            player.customise_skill(id, skill_id.clone(), nom, author.to_string())
        }
        CustomisationLine::Stat { stat, crans, .. } => {
            player.customise_stat(id, *stat, *crans, author.to_string())
        }
        CustomisationLine::Price { delta, .. } => {
            player.customise_value(id, *delta, author.to_string())
        }
        CustomisationLine::Spp { amount, .. } => {
            player.customise_spp(id, *amount, author.to_string())
        }
    };
    issue.map_err(ValidateCustomisationError::Domain)
}

/// Supprime le panier. Ni joueur chargé, ni domaine appelé : rien n'a été
/// engagé, il n'y a rien à défaire.
///
/// **Idempotente** — un panier déjà absent n'est pas une erreur, et deux clics
/// ne doivent pas produire un message d'échec.
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn cancel(
    cmd: CancelCustomisationCommand,
    basket_repo: &dyn ICustomisationBasketRepository,
) -> Result<(), RepositoryError> {
    basket_repo.delete(&cmd.player_id.0).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::match_impact::StatKind;
    use crate::app::players::domain::player::{PlayerId, Spp, TeamId, ValueKpo};
    use crate::app::players::domain::value_objects::{
        PositionNameVo, RosterLineId, SkillId, StatCrans,
    };
    use crate::app::players::ports::{
        CustomisationBasketState, PositionAccessDto, PositionCatalogEntryDto, SkillCatalogEntryDto,
        SkillCostLevelDto, SppScaleDto,
    };
    use crate::app::players::use_cases::commands::{
        AddCustomisationSkillCommand, AddCustomisationStatCommand,
    };
    use crate::app::players::use_cases::customisation_basket_mutation;
    use crate::app::shared_kernel::identity::ids::SpaceId;
    use std::sync::Mutex;

    // ── Doublures ─────────────────────────────────────────────────────────────

    struct FakeCatalog;
    impl ISkillCatalogPort for FakeCatalog {
        fn find_skill(&self, skill_id: &str) -> Option<SkillCatalogEntryDto> {
            self.list_all_skills()
                .into_iter()
                .find(|s| s.skill_id == skill_id)
        }
        fn list_all_skills(&self) -> Vec<SkillCatalogEntryDto> {
            ["BLOCK", "DODGE", "GUARD"]
                .iter()
                .map(|uid| SkillCatalogEntryDto {
                    skill_id: uid.to_string(),
                    name: format!("Compétence {uid}"),
                    category: "GENERAL".into(),
                    category_label: "Général".into(),
                    description: format!("Description de {uid}"),
                    is_elite: false,
                })
                .collect()
        }
        fn find_position(&self, _: &str) -> Option<PositionCatalogEntryDto> {
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
            })
        }
        fn position_access(&self, _: &str) -> Option<PositionAccessDto> {
            None
        }
        fn cost_for_level(&self, _: u8, _: bool) -> Option<SkillCostLevelDto> {
            None
        }
        fn skill_value_delta(&self, _: bool) -> u32 {
            0
        }
        fn stat_value_delta(&self, _: StatKind) -> u32 {
            0
        }
        fn spp_scale_for_roster_line(&self, _: &str) -> SppScaleDto {
            SppScaleDto {
                touchdown: 3,
                pass: 1,
                interception: 2,
                casualty: 2,
                mvp: 4,
            }
        }
    }

    #[derive(Default)]
    struct FakeBasketRepo {
        etat: Mutex<Option<CustomisationBasketState>>,
        suppressions: Mutex<u32>,
    }

    #[async_trait::async_trait]
    impl ICustomisationBasketRepository for FakeBasketRepo {
        async fn load(&self, _: &str) -> Result<Option<CustomisationBasketState>, RepositoryError> {
            Ok(self.etat.lock().unwrap().clone())
        }
        async fn save(
            &self,
            basket: &CustomisationBasketState,
            expected_version: u32,
        ) -> Result<u32, RepositoryError> {
            let mut etat = self.etat.lock().unwrap();
            let courante = etat.as_ref().map(|e| e.version).unwrap_or(0);
            if courante != expected_version {
                return Err(RepositoryError::ConcurrentWrite);
            }
            let nouvelle = expected_version + 1;
            *etat = Some(CustomisationBasketState {
                version: nouvelle,
                ..basket.clone()
            });
            Ok(nouvelle)
        }
        async fn delete(&self, _: &str) -> Result<(), RepositoryError> {
            *self.etat.lock().unwrap() = None;
            *self.suppressions.lock().unwrap() += 1;
            Ok(())
        }
    }

    struct FakePlayerRepo {
        flux: Mutex<Vec<PlayerDomainEvent>>,
        echoue_a_l_append: bool,
    }

    impl FakePlayerRepo {
        fn neuf() -> Self {
            Self {
                flux: Mutex::new(vec![PlayerDomainEvent::PlayerCreated {
                    player_id: PlayerId("p1".into()),
                    team_id: TeamId("t1".into()),
                    space_id: SpaceId::new(),
                    position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
                    roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
                    jersey: None,
                    base_skills: vec![],
                    starting_spp: Spp(0),
                    starting_value: ValueKpo(100),
                }]),
                echoue_a_l_append: false,
            }
        }
        fn appendus(&self) -> Vec<PlayerDomainEvent> {
            self.flux
                .lock()
                .unwrap()
                .iter()
                .filter(|e| !matches!(e, PlayerDomainEvent::PlayerCreated { .. }))
                .cloned()
                .collect()
        }
    }

    #[async_trait::async_trait]
    impl IPlayerRepository for FakePlayerRepo {
        async fn append(
            &self,
            _: &PlayerId,
            _: &TeamId,
            event: &PlayerDomainEvent,
            _: i32,
        ) -> Result<(), RepositoryError> {
            if self.echoue_a_l_append {
                return Err(RepositoryError::ConcurrentWrite);
            }
            self.flux.lock().unwrap().push(event.clone());
            Ok(())
        }
        async fn find_by_id(&self, _: &PlayerId) -> Result<Option<Player>, RepositoryError> {
            Ok(Player::from_events(&self.flux.lock().unwrap()))
        }
        async fn find_by_team_id(&self, _: &TeamId) -> Result<Vec<Player>, RepositoryError> {
            Ok(vec![])
        }
        async fn find_events_by_id(
            &self,
            _: &PlayerId,
        ) -> Result<Vec<PlayerDomainEvent>, RepositoryError> {
            Ok(vec![])
        }
        async fn has_spent_spp_since_match(
            &self,
            _: &TeamId,
            _: &str,
        ) -> Result<bool, RepositoryError> {
            Ok(false)
        }
    }

    // ── Aides ─────────────────────────────────────────────────────────────────

    fn joueur() -> PlayerId {
        PlayerId("p1".into())
    }

    fn bus() -> EventBus {
        crate::common::services::event_bus::event_bus::new_bus()
    }

    fn ids(n: usize) -> Vec<CustomisationId> {
        (0..n)
            .map(|i| CustomisationId::try_new(format!("c{i}")).unwrap())
            .collect()
    }

    async fn ajouter_stat(
        repo: &FakePlayerRepo,
        panier: &FakeBasketRepo,
        crans: i8,
        version: u32,
    ) -> Result<(), customisation_basket_mutation::CustomisationBasketError> {
        customisation_basket_mutation::add_stat(
            AddCustomisationStatCommand {
                player_id: joueur(),
                stat: StatKind::Ag,
                crans: StatCrans::try_new(crans).unwrap(),
                expected_version: version,
            },
            "s1",
            repo,
            panier,
            &FakeCatalog,
        )
        .await
    }

    // ── Mutations ─────────────────────────────────────────────────────────────

    /// La première mutation **crée** le panier : il n'y a pas d'endpoint
    /// d'ouverture.
    #[tokio::test]
    async fn la_premiere_mutation_cree_le_panier() {
        let repo = FakePlayerRepo::neuf();
        let panier = FakeBasketRepo::default();

        assert!(panier.load("p1").await.unwrap().is_none());
        ajouter_stat(&repo, &panier, 1, 0).await.unwrap();
        assert_eq!(panier.load("p1").await.unwrap().unwrap().version, 1);
    }

    /// Une mutation refusée ne touche pas les lignes déjà présentes.
    #[tokio::test]
    async fn une_mutation_refusee_laisse_le_panier_intact() {
        let repo = FakePlayerRepo::neuf();
        let panier = FakeBasketRepo::default();

        // AG 3+ → 1+ en deux crans : la limite est atteinte.
        ajouter_stat(&repo, &panier, 2, 0).await.unwrap();
        let avant = panier.load("p1").await.unwrap().unwrap();

        let erreur = ajouter_stat(&repo, &panier, 1, 1).await.unwrap_err();
        assert!(matches!(
            erreur,
            customisation_basket_mutation::CustomisationBasketError::Domain(
                DomainError::StatOutOfBounds { .. }
            )
        ));

        let apres = panier.load("p1").await.unwrap().unwrap();
        assert_eq!(apres.version, avant.version);
        assert_eq!(apres.state, avant.state);
    }

    #[tokio::test]
    async fn une_version_perimee_est_un_conflit() {
        let repo = FakePlayerRepo::neuf();
        let panier = FakeBasketRepo::default();
        ajouter_stat(&repo, &panier, 1, 0).await.unwrap();

        let erreur = ajouter_stat(&repo, &panier, 1, 0).await.unwrap_err();
        assert!(matches!(
            erreur,
            customisation_basket_mutation::CustomisationBasketError::ConcurrentWrite
        ));
    }

    // ── Validation ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn valider_un_panier_vide_ne_fait_rien() {
        let repo = FakePlayerRepo::neuf();
        let panier = FakeBasketRepo::default();

        let erreur = execute(
            ValidateCustomisationCommand {
                player_id: joueur(),
                author: "Bagouze".into(),
                customisation_ids: vec![],
                expected_version: 0,
            },
            &repo,
            &panier,
            &FakeCatalog,
            &bus(),
        )
        .await
        .unwrap_err();

        assert!(matches!(erreur, ValidateCustomisationError::NothingToApply));
    }

    /// Un événement **par ligne**, comme la phase 1 l'exige.
    #[tokio::test]
    async fn la_validation_produit_un_evenement_par_ligne() {
        let repo = FakePlayerRepo::neuf();
        let panier = FakeBasketRepo::default();

        customisation_basket_mutation::add_skill(
            AddCustomisationSkillCommand {
                player_id: joueur(),
                skill_id: SkillId::try_new("BLOCK".to_string()).unwrap(),
                expected_version: 0,
            },
            "s1",
            &repo,
            &panier,
            &FakeCatalog,
        )
        .await
        .unwrap();
        ajouter_stat(&repo, &panier, 1, 1).await.unwrap();

        execute(
            ValidateCustomisationCommand {
                player_id: joueur(),
                author: "Bagouze".into(),
                customisation_ids: ids(2),
                expected_version: 2,
            },
            &repo,
            &panier,
            &FakeCatalog,
            &bus(),
        )
        .await
        .unwrap();

        let events = repo.appendus();
        assert_eq!(events.len(), 2);
        assert!(events
            .iter()
            .any(|e| matches!(e, PlayerDomainEvent::PlayerSkillCustomised { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, PlayerDomainEvent::PlayerStatCustomised { .. })));
    }

    /// Le panier est **supprimé**, pas vidé : son existence commande
    /// l'affichage du mode.
    #[tokio::test]
    async fn la_validation_supprime_le_panier() {
        let repo = FakePlayerRepo::neuf();
        let panier = FakeBasketRepo::default();
        ajouter_stat(&repo, &panier, 1, 0).await.unwrap();

        execute(
            ValidateCustomisationCommand {
                player_id: joueur(),
                author: "Bagouze".into(),
                customisation_ids: ids(1),
                expected_version: 1,
            },
            &repo,
            &panier,
            &FakeCatalog,
            &bus(),
        )
        .await
        .unwrap();

        assert!(panier.load("p1").await.unwrap().is_none());
    }

    /// La garde qui rend le comptage d'identifiants sûr. Le handler compte les
    /// lignes sur une lecture antérieure à celle du use case ; sans elle, un
    /// panier modifié entre les deux ferait sortir `IdentifiantsManquants`,
    /// code qui annonce un bug d'appelant pour une simple écriture concurrente.
    ///
    /// Ici le panier est à la version 2 et l'appelant en attendait 1 : refus,
    /// et **rien n'est appliqué** — le panier survit à la tentative.
    #[tokio::test]
    async fn une_validation_sur_une_version_perimee_est_un_conflit_pas_un_bug() {
        let repo = FakePlayerRepo::neuf();
        let panier = FakeBasketRepo::default();
        ajouter_stat(&repo, &panier, 1, 0).await.unwrap();
        ajouter_stat(&repo, &panier, 1, 1).await.unwrap();

        let erreur = execute(
            ValidateCustomisationCommand {
                player_id: joueur(),
                author: "Bagouze".into(),
                customisation_ids: ids(1),
                expected_version: 1,
            },
            &repo,
            &panier,
            &FakeCatalog,
            &bus(),
        )
        .await
        .unwrap_err();

        assert!(
            matches!(erreur, ValidateCustomisationError::ConcurrentWrite),
            "obtenu {erreur:?}"
        );
        assert!(
            panier.load("p1").await.unwrap().is_some(),
            "un conflit ne doit rien consommer"
        );
    }

    /// Tout ou rien : une ligne devenue invalide fait tout échouer, et **rien**
    /// n'est appliqué — ni événement, ni suppression du panier.
    #[tokio::test]
    async fn une_ligne_devenue_invalide_fait_tout_echouer() {
        let repo = FakePlayerRepo::neuf();
        let panier = FakeBasketRepo::default();

        customisation_basket_mutation::add_skill(
            AddCustomisationSkillCommand {
                player_id: joueur(),
                skill_id: SkillId::try_new("BLOCK".to_string()).unwrap(),
                expected_version: 0,
            },
            "s1",
            &repo,
            &panier,
            &FakeCatalog,
        )
        .await
        .unwrap();

        // Le joueur acquiert BLOCK entre-temps, par une autre voie.
        let p = repo.find_by_id(&joueur()).await.unwrap().unwrap();
        let event = p
            .customise_skill(
                CustomisationId::try_new("ailleurs".to_string()).unwrap(),
                SkillId::try_new("BLOCK".to_string()).unwrap(),
                SkillName::try_new("Bloc".to_string()).unwrap(),
                "Autre".into(),
            )
            .unwrap();
        repo.append(&joueur(), &TeamId("t1".into()), &event, 2)
            .await
            .unwrap();
        let avant = repo.appendus().len();

        let erreur = execute(
            ValidateCustomisationCommand {
                player_id: joueur(),
                author: "Bagouze".into(),
                customisation_ids: ids(1),
                expected_version: 1,
            },
            &repo,
            &panier,
            &FakeCatalog,
            &bus(),
        )
        .await
        .unwrap_err();

        assert!(matches!(
            erreur,
            ValidateCustomisationError::LinesRejected(_)
        ));
        assert_eq!(repo.appendus().len(), avant, "rien ne doit être appliqué");
        assert!(
            panier.load("p1").await.unwrap().is_some(),
            "le panier survit à un refus"
        );
    }

    /// L'annulation est idempotente : deux clics ne produisent pas d'échec.
    #[tokio::test]
    async fn l_annulation_est_idempotente() {
        let panier = FakeBasketRepo::default();

        cancel(
            CancelCustomisationCommand {
                player_id: joueur(),
            },
            &panier,
        )
        .await
        .unwrap();
        cancel(
            CancelCustomisationCommand {
                player_id: joueur(),
            },
            &panier,
        )
        .await
        .unwrap();
    }

    /// L'ordre voulu : le panier est supprimé **avant** l'append. Si l'append
    /// échoue, la saisie est perdue — mais rien n'est appliqué deux fois.
    #[tokio::test]
    async fn le_panier_est_supprime_avant_l_append() {
        let mut repo = FakePlayerRepo::neuf();
        let panier = FakeBasketRepo::default();
        ajouter_stat(&repo, &panier, 1, 0).await.unwrap();
        repo.echoue_a_l_append = true;

        let _ = execute(
            ValidateCustomisationCommand {
                player_id: joueur(),
                author: "Bagouze".into(),
                customisation_ids: ids(1),
                expected_version: 1,
            },
            &repo,
            &panier,
            &FakeCatalog,
            &bus(),
        )
        .await;

        assert_eq!(*panier.suppressions.lock().unwrap(), 1);
        assert!(repo.appendus().is_empty(), "rien ne doit être appliqué");
    }
}

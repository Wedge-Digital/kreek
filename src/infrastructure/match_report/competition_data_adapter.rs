use crate::app::competitions::domain::competition_options::CompetitionOptions;
use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonRepositoryError,
};
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::competitions::use_cases::admin::add_match_use_case::{self, AddMatchError};
use crate::app::match_report::ports::{
    CreationAppariementError, ICompetitionDataPort, InducementSpecDto, RoundContextDto,
    TierRulesDto,
};
use crate::app::references::domain::inducement_pricing::cout_pour_roster;
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::common::services::event_bus::event_bus::EventBus;
use async_trait::async_trait;
use std::sync::Arc;

pub struct CompetitionDataAdapter {
    competition_repo: Arc<dyn ICompetitionRepository>,
    season_repo: Arc<dyn ISeasonRepository>,
    reference_repo: Arc<dyn IReferenceRepository>,
    match_day_repo: Arc<dyn IMatchDayRepository>,
    /// Les deux dépendances de `creer_appariement` (carte 552) : l'adapter est
    /// le **seul** endroit qui connaisse `competitions`, conformément à la règle
    /// des adapters inter-BCs.
    team_info_port: Arc<dyn ITeamInfoPort>,
    event_bus: EventBus,
}

impl CompetitionDataAdapter {
    pub fn new(
        competition_repo: Arc<dyn ICompetitionRepository>,
        season_repo: Arc<dyn ISeasonRepository>,
        reference_repo: Arc<dyn IReferenceRepository>,
        match_day_repo: Arc<dyn IMatchDayRepository>,
        team_info_port: Arc<dyn ITeamInfoPort>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            competition_repo,
            season_repo,
            reference_repo,
            match_day_repo,
            team_info_port,
            event_bus,
        }
    }
}

/// Le refus du use case, traduit dans le vocabulaire du port.
///
/// `TeamAlreadyScheduled` traverse tel quel : c'est l'invariant de la carte 551,
/// et le coach doit lire le même message que le commissaire.
fn traduire(e: AddMatchError) -> CreationAppariementError {
    match e {
        AddMatchError::TeamAlreadyScheduled { equipe, adversaire } => {
            CreationAppariementError::DejaEngagee { equipe, adversaire }
        }
        AddMatchError::TeamsNotEnrolled(noms) => CreationAppariementError::NonEnrolee(noms),
        AddMatchError::RoundNotFound => {
            CreationAppariementError::Indisponible("journée introuvable".to_string())
        }
        AddMatchError::InvalidTeamId => {
            CreationAppariementError::Indisponible("identifiant d'équipe invalide".to_string())
        }
        AddMatchError::Repository(m) => CreationAppariementError::Indisponible(m),
    }
}

/// Ce que vaut une lecture d'options pour la garde de création (carte 550).
///
/// **Trois façons de ne pas savoir, une seule réponse : autoriser.** Saison
/// introuvable, colonne jamais réglée, panne de lecture — dans les trois cas
/// c'est le comportement de toujours qui s'applique. Refuser sur une erreur
/// d'infrastructure priverait un coach de son rapport sans qu'il puisse rien y
/// faire, et sans que l'écran sache le lui dire.
///
/// La garde ne protège donc que ce qui a été **explicitement interdit**, ce qui
/// est exactement son objet.
fn autorise_depuis(lues: Result<Option<CompetitionOptions>, SeasonRepositoryError>) -> bool {
    match lues {
        Ok(Some(options)) => options.autorise_hors_calendrier.0,
        Ok(None) => CompetitionOptions::default().autorise_hors_calendrier.0,
        Err(_) => true,
    }
}

#[async_trait]
impl ICompetitionDataPort for CompetitionDataAdapter {
    async fn autorise_hors_calendrier(&self, season_id: &str) -> bool {
        let Ok(sid) = SeasonId::try_new(season_id) else {
            return true;
        };
        let lues = self.season_repo.find_options(&sid).await;
        if let Err(e) = &lues {
            tracing::error!("competition_data_adapter: options {season_id}: {e}");
        }
        autorise_depuis(lues)
    }

    async fn creer_appariement(
        &self,
        space_id: &str,
        competition_id: &str,
        season_id: &str,
        round_id: &str,
        home_team_id: &str,
        away_team_id: &str,
    ) -> Result<String, CreationAppariementError> {
        // **Le même cœur que l'ajout par un commissaire, une autre annonce.**
        //
        // L'invariant de journée s'applique donc ici sans être écrit deux fois,
        // et `competitions` reste souverain sur ses appariements — le hors
        // calendrier ne contourne pas sa règle, il la consulte.
        //
        // `creer_hors_calendrier` émet `OutOfSchedulePairingCreated` et **non**
        // `PairingCreated` : ce dernier ferait créer un rapport par
        // `pairing_created_listener`, alors que sur ce chemin c'est le
        // contrôleur qui le crée au retour de cet appel. Deux rapports
        // naîtraient pour une rencontre — c'est ce que la carte 555 supprime.
        add_match_use_case::creer_hors_calendrier(
            round_id,
            season_id,
            competition_id,
            space_id,
            home_team_id,
            away_team_id,
            self.match_day_repo.as_ref(),
            self.team_info_port.as_ref(),
            &self.event_bus,
        )
        .await
        .map_err(traduire)
    }

    async fn is_competition_admin(
        &self,
        competition_id: &str,
        coach_id: &str,
    ) -> Result<bool, String> {
        let comp_id =
            crate::app::shared_kernel::bloodbowl::ids::CompetitionId::try_new(competition_id)
                .map_err(|e| e.to_string())?;
        let info = self
            .competition_repo
            .find_base_info(&comp_id)
            .await
            .map_err(|e| e.to_string())?;

        match info {
            Some(base) => Ok(base.admin_ids.iter().any(|id| id.to_string() == coach_id)),
            None => Ok(false),
        }
    }

    async fn find_tier_rules_for_roster(
        &self,
        season_id: &str,
        roster_id: &str,
    ) -> Option<TierRulesDto> {
        let sid = SeasonId::try_new(season_id).ok()?;
        let rules = self.season_repo.find_rules(&sid).await.ok()??;
        let tier = rules
            .tiers
            .iter()
            .find(|t| t.rosters.contains(&roster_id.to_string()))?;
        let allowed_inducements = tier
            .inducements
            .iter()
            .filter_map(|uid| build_inducement_spec(uid, roster_id, &*self.reference_repo))
            .collect();
        let allowed_star_players = tier
            .star_players
            .iter()
            .filter_map(|uid| build_star_player_spec(uid, &*self.reference_repo))
            .collect();
        Some(TierRulesDto {
            allowed_inducements,
            allowed_star_players,
        })
    }

    async fn find_round_context(&self, season_id: &str, round_id: &str) -> Option<RoundContextDto> {
        let sid = SeasonId::try_new(season_id).ok()?;
        let season = self.season_repo.find_full(&sid).await.ok()??;
        let round = self.match_day_repo.find_by_id(round_id).await.ok()??;
        Some(RoundContextDto {
            competition_name: season.competition_name,
            season_name: season.season_name,
            round_name: round.name.to_string(),
        })
    }
}

/// C'est **le prix débité**, celui qui part de la trésorerie du coach.
///
/// Il prend le roster parce qu'un coup de pouce peut coûter moins cher à
/// certaines équipes — le cuistot halfling. `find_tier_rules_for_roster` le
/// connaissait déjà et ne le transmettait pas : le prix réduit du corpus
/// n'était lu nulle part, et l'équipe halfling payait son cuistot 300 kPo
/// (carte 507).
fn build_inducement_spec(
    uid: &str,
    roster_id: &str,
    repo: &dyn IReferenceRepository,
) -> Option<InducementSpecDto> {
    let ind = repo.find_inducement_by_uid(uid)?;
    Some(InducementSpecDto {
        uid: ind.uid.clone(),
        max_qty: ind.max_quantity as u8,
        unit_cost: cout_pour_roster(ind, roster_id),
    })
}

fn build_star_player_spec(uid: &str, repo: &dyn IReferenceRepository) -> Option<InducementSpecDto> {
    let sp = repo.find_star_player_by_uid(uid)?;
    Some(InducementSpecDto {
        uid: sp.uid.clone(),
        max_qty: 1,
        unit_cost: sp.cost,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::competition_options::AutoriseHorsCalendrier;
    use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;

    // ── La garde de création (carte 550) ─────────────────────────────────────

    /// **Le seul cas qui refuse.** Tout le reste autorise, et c'est délibéré.
    #[test]
    fn une_interdiction_explicite_est_la_seule_a_refuser() {
        let interdit = CompetitionOptions {
            autorise_hors_calendrier: AutoriseHorsCalendrier(false),
        };
        assert!(!autorise_depuis(Ok(Some(interdit))));
    }

    /// Les vingt saisons existantes portent `NULL` : si ce cas refusait, la
    /// saisie manuelle disparaîtrait partout au déploiement.
    #[test]
    fn une_saison_jamais_reglee_autorise() {
        assert!(autorise_depuis(Ok(None)));
    }

    /// Une panne de base ne prive pas un coach de son rapport. C'est le choix
    /// inverse de celui qu'on ferait pour une garde d'autorisation — ici on ne
    /// protège pas un droit, on applique un réglage d'organisation.
    #[test]
    fn une_panne_de_lecture_autorise() {
        assert!(autorise_depuis(Err(SeasonRepositoryError::Database(
            "connexion perdue".into()
        ))));
    }

    #[test]
    fn une_autorisation_explicite_autorise() {
        let autorise = CompetitionOptions {
            autorise_hors_calendrier: AutoriseHorsCalendrier(true),
        };
        assert!(autorise_depuis(Ok(Some(autorise))));
    }

    /// **Le prix débité**, celui qui part de la trésorerie — pas celui que le
    /// sélecteur affiche. Un test qui lirait l'écran passerait alors même que
    /// le coach serait prélevé de 300 (carte 507).
    ///
    /// Le dépôt est le **vrai jeu de démonstration**, pas un mock : si le
    /// cuistot en disparaissait, ce test le dirait au lieu de continuer à
    /// prouver quelque chose sur un corpus imaginaire.
    #[test]
    fn le_cuistot_est_debite_a_prix_reduit_pour_les_halflings() {
        let repo = InMemoryReferenceRepository::load_for_tests();
        let spec = build_inducement_spec("HALFLING_MASTER_CHEF", "HALFLING", &repo)
            .expect("le jeu de démonstration porte le cuistot");
        assert_eq!(spec.unit_cost, 100);
    }

    #[test]
    fn le_cuistot_est_debite_plein_tarif_aux_autres() {
        let repo = InMemoryReferenceRepository::load_for_tests();
        for roster in ["DEMO_GRANIT", "DEMO_ZEPHYR", "GNOME"] {
            let spec = build_inducement_spec("HALFLING_MASTER_CHEF", roster, &repo)
                .expect("le jeu de démonstration porte le cuistot");
            assert_eq!(spec.unit_cost, 300, "roster {roster}");
        }
    }

    /// Les autres coups de pouce ne bougent pas — le roster ne les concerne
    /// pas, et la règle ne doit pas déborder.
    #[test]
    fn les_autres_coups_de_pouce_gardent_leur_prix() {
        let repo = InMemoryReferenceRepository::load_for_tests();
        let a = build_inducement_spec("DEMO_MAGE_DES_BRUMES", "HALFLING", &repo).unwrap();
        let b = build_inducement_spec("DEMO_MAGE_DES_BRUMES", "DEMO_GRANIT", &repo).unwrap();
        assert_eq!((a.unit_cost, b.unit_cost), (60, 60));
    }
}

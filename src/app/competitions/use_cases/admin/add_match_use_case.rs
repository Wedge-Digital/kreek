use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::ports::{ITeamInfoPort, TeamInfoDto};
use crate::app::competitions::use_cases::admin::team_enrollment::{
    build_new_pairing_projection, load_enrolled_teams, resolve_team_names,
};
use crate::app::shared_kernel::bloodbowl::ids::PairingId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::EventId;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;
use std::collections::HashMap;

#[derive(Debug)]
pub enum AddMatchError {
    RoundNotFound,
    InvalidTeamId,
    TeamsNotEnrolled(Vec<String>),
    /// Carte 551 — l'une des deux équipes a déjà une rencontre cette journée.
    /// Les deux champs sont des **noms** : `team_display` est déjà chargé ici,
    /// donc la résolution est gratuite, et le refus doit se lire à l'écran.
    TeamAlreadyScheduled {
        equipe: String,
        adversaire: String,
    },
    Repository(String),
}

/// Crée un pairing manuel — refuse (BR : pas de pairing pour une équipe non
/// enrôlée, donc pas d'event `PairingCreated`) si l'une des deux équipes n'est
/// pas `Enrolled` pour la saison, plutôt que d'émettre un event à métadonnées vides.
#[tracing::instrument(skip_all, fields(round_id = ?round_id))]
pub async fn execute(
    round_id: &str,
    season_id: &str,
    competition_id: &str,
    space_id: &str,
    home_team_id: &str,
    away_team_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
) -> Result<String, AddMatchError> {
    let match_day = match_day_repo
        .find_by_id(round_id)
        .await
        .map_err(|e| AddMatchError::Repository(e.to_string()))?
        .ok_or(AddMatchError::RoundNotFound)?;

    let team_display = load_enrolled_teams(season_id, team_port)
        .await
        .map_err(AddMatchError::Repository)?;

    ensure_both_enrolled(home_team_id, away_team_id, &team_display, team_port).await?;

    let home = TeamId::try_new(home_team_id).map_err(|_| AddMatchError::InvalidTeamId)?;
    let away = TeamId::try_new(away_team_id).map_err(|_| AddMatchError::InvalidTeamId)?;
    // **Après** l'enrôlement : une équipe non enrôlée n'a pas d'adversaire à
    // nommer, et l'ordre inverse produirait un message trompeur.
    ensure_journee_libre(&match_day, &home, &away, &team_display)?;

    let pairing = Pairing {
        id: PairingId::new(),
        home_team_id: home,
        away_team_id: away,
    };
    let projection = build_new_pairing_projection(
        home_team_id,
        away_team_id,
        season_id,
        &match_day,
        &team_display,
    );
    match_day_repo
        .save_pairing(round_id, &pairing, &projection)
        .await
        .map_err(|e| AddMatchError::Repository(e.to_string()))?;

    emit_pairing_created(
        home_team_id,
        away_team_id,
        &pairing,
        competition_id,
        season_id,
        space_id,
        &match_day,
        &team_display,
        event_bus,
    );
    // L'identifiant de l'appariement créé : le chemin hors calendrier (carte
    // 552) y rattache son rapport dans la foulée, ce qui est précisément ce qui
    // manquait — l'appariement était fabriqué sans que le rapport l'apprenne.
    Ok(pairing.id.to_string())
}

/// Carte 551 — une équipe joue au plus un match par journée.
///
/// La règle est portée par `MatchDay` : ce use case ne la réimplémente pas, il
/// la consulte et met le refus en mots.
fn ensure_journee_libre(
    match_day: &MatchDay,
    home: &TeamId,
    away: &TeamId,
    team_display: &HashMap<String, TeamInfoDto>,
) -> Result<(), AddMatchError> {
    let Some(bloquant) = match_day.engagement_existant(home, away) else {
        return Ok(());
    };
    let engagee = if bloquant.engage(home) { home } else { away };
    let adversaire = bloquant
        .adversaire_de(engagee)
        .expect("l'appariement bloquant engage cette équipe");

    Err(AddMatchError::TeamAlreadyScheduled {
        equipe: nom_affiche(engagee, team_display),
        adversaire: nom_affiche(adversaire, team_display),
    })
}

/// Le nom de l'équipe, ou son identifiant à défaut — l'adversaire déjà
/// programmé a pu être désenrôlé depuis, et il est alors absent de la map.
fn nom_affiche(id: &TeamId, team_display: &HashMap<String, TeamInfoDto>) -> String {
    let id = id.to_string();
    team_display
        .get(&id)
        .map(|t| t.team_name.clone())
        .unwrap_or(id)
}

async fn ensure_both_enrolled(
    home_team_id: &str,
    away_team_id: &str,
    team_display: &HashMap<String, TeamInfoDto>,
    team_port: &dyn ITeamInfoPort,
) -> Result<(), AddMatchError> {
    let mut missing = Vec::new();
    if !team_display.contains_key(home_team_id) {
        missing.push(home_team_id.to_string());
    }
    if !team_display.contains_key(away_team_id) {
        missing.push(away_team_id.to_string());
    }
    if missing.is_empty() {
        return Ok(());
    }
    let names = resolve_team_names(missing, team_port).await;
    Err(AddMatchError::TeamsNotEnrolled(names))
}

#[allow(clippy::too_many_arguments)]
fn emit_pairing_created(
    home: &str,
    away: &str,
    pairing: &Pairing,
    competition_id: &str,
    season_id: &str,
    space_id: &str,
    match_day: &MatchDay,
    team_display: &HashMap<String, TeamInfoDto>,
    event_bus: &EventBus,
) {
    let home_info = team_display
        .get(home)
        .expect("home team vérifié enrôlé avant émission");
    let away_info = team_display
        .get(away)
        .expect("away team vérifié enrôlé avant émission");

    emettre(
        event_bus,
        CompetitionsDomainEvent::PairingCreated {
            event_id: EventId::new(),
            pairing_id: pairing.id.to_string(),
            competition_id: competition_id.to_string(),
            season_id: season_id.to_string(),
            round_id: match_day.id.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            space_id: space_id.to_string(),
            home_team_name: home_info.team_name.clone(),
            home_roster_name: home_info.roster_name.clone(),
            home_coach_name: home_info.coach_name.clone(),
            home_logo_url: home_info.logo_url.clone(),
            away_team_name: away_info.team_name.clone(),
            away_roster_name: away_info.roster_name.clone(),
            away_coach_name: away_info.coach_name.clone(),
            away_logo_url: away_info.logo_url.clone(),
            round_name: match_day.name.to_string(),
            round_position: match_day.position.into_inner(),
            round_date_start: match_day.date_start.as_ref().map(|d| d.to_string()),
            round_date_end: match_day.date_end.as_ref().map(|d| d.to_string()),
            round_day_type: match_day.day_type.as_str().to_string(),
        }
        .to_enveloppe(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{
        MatchDayName, MatchDayPosition, MatchDayType,
    };
    use crate::app::shared_kernel::bloodbowl::ids::MatchId;
    use crate::common::services::event_bus::event_bus::new_bus;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct FakeMatchDayRepo(MatchDay);
    #[async_trait]
    impl IMatchDayRepository for FakeMatchDayRepo {
        async fn find_by_season(
            &self,
            _: &str,
        ) -> Result<
            Vec<MatchDay>,
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
        async fn find_by_id(
            &self,
            _: &str,
        ) -> Result<
            Option<MatchDay>,
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(Some(self.0.clone()))
        }
        async fn save_match_day(
            &self,
            _: &MatchDay,
        ) -> Result<
            (),
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(())
        }
        async fn delete_match_day(
            &self,
            _: &str,
        ) -> Result<
            (),
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(())
        }
        async fn save_pairing(
            &self,
            _: &str,
            _: &Pairing,
            _: &crate::app::competitions::domain::match_day_repository_port::NewPairingProjection,
        ) -> Result<
            (),
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(())
        }
        async fn find_pairing_id(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<
            Option<String>,
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(None)
        }
        async fn delete_pairing(
            &self,
            _: &str,
        ) -> Result<
            (),
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(())
        }
        async fn ensure_match_days_from_structure(
            &self,
            _: &str,
            _: &[(String, String, String, Option<String>, Option<String>)],
        ) -> Result<
            (),
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(())
        }
        async fn list_resultats(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<
            Vec<crate::app::competitions::domain::match_day_repository_port::PairingDisplayDto>,
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
        async fn list_calendrier(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<
            Vec<crate::app::competitions::domain::match_day_repository_port::PairingDisplayDto>,
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
        async fn list_team_matches(
            &self,
            _: &str,
            _: &str,
        ) -> Result<
            Vec<crate::app::competitions::domain::match_day_repository_port::PairingDisplayDto>,
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
        async fn list_latest_completed_results(
            &self,
            _: &str,
            _: i64,
        ) -> Result<
            Vec<crate::app::competitions::domain::match_day_repository_port::LatestResultDto>,
            crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
    }

    struct FakeTeamInfoPort(Mutex<Vec<TeamInfoDto>>);
    #[async_trait]
    impl ITeamInfoPort for FakeTeamInfoPort {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(self.0.lock().unwrap().clone())
        }
        async fn find_team_names(&self, team_ids: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![TeamInfoDto {
                team_id: team_ids[0].clone(),
                team_name: format!("Équipe {}", team_ids[0]),
                coach_id: String::new(),
                coach_name: String::new(),
                roster_name: String::new(),
                logo_url: None,
            }])
        }
        async fn find_team_enrollment(
            &self,
            _: &str,
        ) -> Result<Option<crate::app::competitions::ports::TeamEnrollmentDto>, String> {
            Ok(None)
        }
    }

    fn dto(id: &str, name: &str) -> TeamInfoDto {
        TeamInfoDto {
            team_id: id.into(),
            team_name: name.into(),
            coach_id: "coach".into(),
            coach_name: "Coach".into(),
            roster_name: "Roster".into(),
            logo_url: None,
        }
    }

    fn sample_match_day() -> MatchDay {
        MatchDay {
            id: MatchId::new(),
            season_id: crate::app::shared_kernel::bloodbowl::ids::SeasonId::new(),
            name: MatchDayName::try_new("Journée 1".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(0).unwrap(),
            pairings: vec![],
        }
    }

    #[tokio::test]
    async fn refuses_when_a_team_is_not_enrolled() {
        let match_day_repo = FakeMatchDayRepo(sample_match_day());
        let team_port = FakeTeamInfoPort(Mutex::new(vec![dto("home", "Home Team")]));
        let event_bus = new_bus();

        let result = execute(
            "r1",
            "s1",
            "c1",
            "sp1",
            "home",
            "away",
            &match_day_repo,
            &team_port,
            &event_bus,
        )
        .await;

        assert!(matches!(result, Err(AddMatchError::TeamsNotEnrolled(_))));
    }

    #[tokio::test]
    async fn succeeds_and_emits_real_names_when_both_enrolled() {
        let home_id = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let away_id = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
        let match_day_repo = FakeMatchDayRepo(sample_match_day());
        let team_port = FakeTeamInfoPort(Mutex::new(vec![
            dto(home_id, "Home Team"),
            dto(away_id, "Away Team"),
        ]));
        let event_bus = new_bus();
        let mut rx = event_bus.subscribe();

        let result = execute(
            "r1",
            "s1",
            "c1",
            "sp1",
            home_id,
            away_id,
            &match_day_repo,
            &team_port,
            &event_bus,
        )
        .await;
        assert!(result.is_ok());

        let envelope = rx
            .try_recv()
            .expect("un event PairingCreated doit être émis");
        let event: CompetitionsDomainEvent = serde_json::from_value(envelope.payload).unwrap();
        let CompetitionsDomainEvent::PairingCreated {
            home_team_name,
            away_team_name,
            round_name,
            ..
        } = event
        else {
            panic!("mauvais type d'event");
        };
        assert_eq!(home_team_name, "Home Team");
        assert_eq!(away_team_name, "Away Team");
        assert_eq!(round_name, "Journée 1");
    }

    // ── L'invariant de journée (carte 551) ───────────────────────────────────

    const ALPHA: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    const BETA: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
    const GAMMA: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
    const DELTA: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAY";

    fn rencontre(domicile: &str, exterieur: &str) -> Pairing {
        Pairing {
            id: PairingId::new(),
            home_team_id: TeamId::try_new(domicile).unwrap(),
            away_team_id: TeamId::try_new(exterieur).unwrap(),
        }
    }

    fn journee_avec(pairings: Vec<Pairing>) -> MatchDay {
        MatchDay {
            pairings,
            ..sample_match_day()
        }
    }

    fn les_quatre_enrolees() -> FakeTeamInfoPort {
        FakeTeamInfoPort(Mutex::new(vec![
            dto(ALPHA, "Alpha"),
            dto(BETA, "Beta"),
            dto(GAMMA, "Gamma"),
            dto(DELTA, "Delta"),
        ]))
    }

    async fn ajouter(
        journee: MatchDay,
        domicile: &str,
        exterieur: &str,
    ) -> Result<String, AddMatchError> {
        execute(
            "r1",
            "s1",
            "c1",
            "sp1",
            domicile,
            exterieur,
            &FakeMatchDayRepo(journee),
            &les_quatre_enrolees(),
            &new_bus(),
        )
        .await
    }

    #[tokio::test]
    async fn refuse_quand_l_equipe_a_domicile_joue_deja_cette_journee() {
        let journee = journee_avec(vec![rencontre(ALPHA, GAMMA)]);

        let refus = ajouter(journee, ALPHA, BETA).await;

        let Err(AddMatchError::TeamAlreadyScheduled { equipe, adversaire }) = refus else {
            panic!("un refus d'engagement était attendu : {refus:?}");
        };
        assert_eq!(equipe, "Alpha");
        assert_eq!(
            adversaire, "Gamma",
            "le refus doit nommer l'adversaire déjà prévu"
        );
    }

    /// **Le test qui compte.**
    ///
    /// L'équipe est engagée **à l'extérieur** d'une autre rencontre. Une
    /// implémentation ne regardant que `home_team_id` passerait le test
    /// précédent et manquerait celui-ci — la moitié des cas.
    #[tokio::test]
    async fn refuse_quand_l_equipe_joue_deja_a_l_exterieur() {
        let journee = journee_avec(vec![rencontre(GAMMA, ALPHA)]);

        let refus = ajouter(journee, ALPHA, BETA).await;

        let Err(AddMatchError::TeamAlreadyScheduled { equipe, adversaire }) = refus else {
            panic!("un refus d'engagement était attendu : {refus:?}");
        };
        assert_eq!(equipe, "Alpha");
        assert_eq!(adversaire, "Gamma");
    }

    /// La seconde équipe compte autant que la première.
    #[tokio::test]
    async fn refuse_quand_c_est_l_adversaire_qui_joue_deja() {
        let journee = journee_avec(vec![rencontre(BETA, DELTA)]);

        let refus = ajouter(journee, ALPHA, BETA).await;

        let Err(AddMatchError::TeamAlreadyScheduled { equipe, adversaire }) = refus else {
            panic!("un refus d'engagement était attendu : {refus:?}");
        };
        assert_eq!(equipe, "Beta");
        assert_eq!(adversaire, "Delta");
    }

    /// Le couple déjà programmé tombe sous la même règle — pas de vérification
    /// séparée, donc pas de second message à maintenir.
    #[tokio::test]
    async fn refuse_le_meme_couple_une_seconde_fois() {
        let journee = journee_avec(vec![rencontre(ALPHA, BETA)]);

        assert!(matches!(
            ajouter(journee, ALPHA, BETA).await,
            Err(AddMatchError::TeamAlreadyScheduled { .. })
        ));
    }

    /// Contre-épreuve : une journée déjà pourvue n'empêche pas d'apparier deux
    /// équipes libres. Sans ce test, un refus systématique passerait les quatre
    /// précédents.
    #[tokio::test]
    async fn accepte_deux_equipes_libres_sur_une_journee_deja_pourvue() {
        let journee = journee_avec(vec![rencontre(GAMMA, DELTA)]);

        assert!(ajouter(journee, ALPHA, BETA).await.is_ok());
    }

    /// Un refus ne doit rien émettre : un `PairingCreated` sur une rencontre
    /// non créée ferait réagir `match_report` et `ranking` dans le vide.
    #[tokio::test]
    async fn un_refus_n_emet_aucun_evenement() {
        let event_bus = new_bus();
        let mut rx = event_bus.subscribe();

        let result = execute(
            "r1",
            "s1",
            "c1",
            "sp1",
            ALPHA,
            BETA,
            &FakeMatchDayRepo(journee_avec(vec![rencontre(ALPHA, GAMMA)])),
            &les_quatre_enrolees(),
            &event_bus,
        )
        .await;

        assert!(result.is_err());
        assert!(rx.try_recv().is_err(), "aucun événement ne doit être émis");
    }
}

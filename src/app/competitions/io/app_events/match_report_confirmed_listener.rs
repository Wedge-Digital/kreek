use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::io::app_events::appariement::{
    trouver_appariement, ContexteAppariement,
};
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportAppEvent;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::Instrument;

pub fn init(
    app_event_bus: &EventBus,
    event_bus: EventBus,
    pool: PgPool,
    match_day_repo: Arc<dyn IMatchDayRepository>,
    team_port: Arc<dyn ITeamInfoPort>,
) {
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) =
                        serde_json::from_value::<MatchReportAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    handle_event(
                        event,
                        &pool,
                        match_day_repo.as_ref(),
                        team_port.as_ref(),
                        &event_bus,
                    )
                    .instrument(span)
                    .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("competitions::match_report_confirmed_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_event(
    event: MatchReportAppEvent,
    pool: &PgPool,
    match_day_repo: &dyn IMatchDayRepository,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
) {
    // Sortie **silencieuse** : le bus porte tous les app events de
    // l'application, et journaliser ceux qui ne nous concernent pas noierait le
    // signal. Toutes les sorties qui suivent, elles, laissent une ligne.
    let MatchReportAppEvent::MatchReportConfirmed {
        match_report_id,
        space_id,
        pairing_id,
        season_id,
        round_id,
        competition_id,
        home_team_id,
        away_team_id,
        ..
    } = event
    else {
        return;
    };

    // Un rapport manuel n'a pas d'appariement : on le lui fabrique, exactement
    // comme le fait la publication. C'est ce qui manquait — l'ancien filtre
    // exigeait `Some(pairing_id)` et abandonnait sans un mot, si bien qu'un
    // match démarré hors calendrier restait invisible jusqu'à sa publication
    // (carte 427).
    let contexte = ContexteAppariement {
        match_report_id: match_report_id.clone(),
        space_id: space_id.clone(),
        competition_id,
        season_id,
        round_id,
        home_team_id,
        away_team_id,
        pairing_id,
    };
    // **On cherche, on ne fabrique plus** (carte 555).
    //
    // L'ancien code créait ici l'appariement manquant d'un rapport manuel. Ce
    // cas n'existe plus : un rapport naît toujours d'un appariement, dont il
    // porte l'identifiant dès son premier événement. Fabriquer encore reviendrait
    // à en créer un second, et c'est ce que la carte 552 a produit.
    let Some(pairing_id) = trouver_appariement(&contexte, match_day_repo).await else {
        tracing::warn!(
            match_report_id = %match_report_id,
            "confirmation ignorée : ce rapport ne désigne aucun appariement"
        );
        return;
    };

    let report_url = AppRoutes::default()
        .match_report
        .edit_match_report(&space_id, &match_report_id);

    let result = sqlx::query!(
        "UPDATE competition_match_display_proj
         SET match_status = 'in_progress',
             match_report_id = $2,
             match_report_url = $3
         WHERE pairing_id = $1",
        pairing_id,
        match_report_id,
        report_url,
    )
    .execute(pool)
    .await;

    match result {
        Err(e) => {
            tracing::error!(
                "competitions::match_report_confirmed_listener: update {pairing_id}: {e}"
            );
        }
        // Zéro ligne touchée était un silence de plus : l'appariement existe,
        // sa ligne d'affichage non, et le match reste absent des résultats.
        Ok(r) if r.rows_affected() == 0 => {
            tracing::warn!(
                pairing_id = %pairing_id,
                match_report_id = %match_report_id,
                "confirmation sans effet : aucune ligne d'affichage pour cet appariement"
            );
        }
        Ok(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{
        MatchDay, MatchDayName, MatchDayPosition, MatchDayType,
    };
    use crate::app::competitions::io::repository::match_day_repository::MatchDayRepository;
    use crate::app::competitions::ports::TeamInfoDto;
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
    use crate::app::shared_kernel::identity::ids::EventId;
    use sqlx::PgPool;

    const HOME: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    const AWAY: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";

    struct FakeTeamInfoPort(Vec<TeamInfoDto>);
    #[async_trait::async_trait]
    impl ITeamInfoPort for FakeTeamInfoPort {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(self.0.clone())
        }
        async fn find_team_names(&self, _: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_enrollment(
            &self,
            _: &str,
        ) -> Result<Option<crate::app::competitions::ports::TeamEnrollmentDto>, String> {
            Ok(None)
        }
    }

    fn equipes() -> FakeTeamInfoPort {
        let dto = |id: &str, n: &str, c: &str| TeamInfoDto {
            team_id: id.into(),
            team_name: n.into(),
            coach_id: c.into(),
            coach_name: c.into(),
            roster_name: "R".into(),
            logo_url: None,
        };
        FakeTeamInfoPort(vec![dto(HOME, "Home", "c1"), dto(AWAY, "Away", "c2")])
    }

    async fn journee(pool: &PgPool) -> MatchDay {
        let jour = MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 7".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(3).unwrap(),
            pairings: vec![],
        };
        MatchDayRepository::new(pool.clone())
            .save_match_day(&jour)
            .await
            .expect("insertion de la journée de test");
        jour
    }

    fn confirmation(round_id: &str, pairing_id: Option<String>) -> MatchReportAppEvent {
        MatchReportAppEvent::MatchReportConfirmed {
            event_id: EventId::new(),
            match_report_id: "01ARZ3NDEKTSV4RRFFQ69G5FAX".to_string(),
            home_team_id: HOME.to_string(),
            away_team_id: AWAY.to_string(),
            space_id: "01ARZ3NDEKTSV4RRFFQ69G5FAY".to_string(),
            pairing_id,
            season_id: SeasonId::new().to_string(),
            round_id: round_id.to_string(),
            competition_id: "01ARZ3NDEKTSV4RRFFQ69G5FAZ".to_string(),
        }
    }

    /// Une rencontre déjà programmée, avec sa ligne d'affichage — l'état
    /// normal depuis la carte 555 : le rapport porte toujours un appariement.
    async fn appariement_existant(pool: &PgPool, jour: &MatchDay) -> String {
        use crate::app::competitions::domain::match_day::Pairing;
        use crate::app::competitions::domain::match_day_repository_port::NewPairingProjection;
        use crate::app::shared_kernel::bloodbowl::ids::PairingId;
        use crate::app::shared_kernel::bloodbowl::team::TeamId;

        let pairing = Pairing {
            id: PairingId::new(),
            home_team_id: TeamId::try_new(HOME).unwrap(),
            away_team_id: TeamId::try_new(AWAY).unwrap(),
        };
        let projection = NewPairingProjection {
            season_id: jour.season_id.to_string(),
            round_name: jour.name.to_string(),
            round_position: jour.position.into_inner(),
            round_date_start: None,
            round_date_end: None,
            round_day_type: jour.day_type.as_str().to_string(),
            home_team_name: "Home".into(),
            home_roster_name: "R".into(),
            home_coach_name: "c1".into(),
            home_logo_url: None,
            away_team_name: "Away".into(),
            away_roster_name: "R".into(),
            away_coach_name: "c2".into(),
            away_logo_url: None,
        };
        MatchDayRepository::new(pool.clone())
            .save_pairing(&jour.id.to_string(), &pairing, &projection)
            .await
            .expect("insertion de l'appariement de test");
        pairing.id.to_string()
    }

    async fn appariements(pool: &PgPool, round_id: &str) -> i64 {
        sqlx::query_scalar!(
            "SELECT count(*) FROM competition_match_day_pairings WHERE match_day_id = $1",
            round_id,
        )
        .fetch_one(pool)
        .await
        .unwrap()
        .unwrap_or(0)
    }

    /// **Une confirmation sans appariement ne fabrique plus rien** (carte 555).
    ///
    /// Le listener créait ici l'appariement manquant d'un rapport manuel — la
    /// carte 427. Ce cas n'existe plus : un rapport naît toujours d'un
    /// appariement dont il porte l'identifiant. Fabriquer encore produirait un
    /// second appariement pour la rencontre, ce que la carte 552 a effectivement
    /// provoqué.
    ///
    /// Ces deux tests remplacent `un_rapport_manuel_confirme_obtient_un_appariement_et_une_ligne`
    /// et `un_rapport_manuel_reconfirme_ne_cree_pas_un_second_appariement`, qui
    /// affirmaient le comportement retiré.
    #[sqlx::test]
    async fn une_confirmation_sans_appariement_est_ignoree(pool: PgPool) {
        let jour = journee(&pool).await;
        let repo = MatchDayRepository::new(pool.clone());
        let bus = crate::common::services::event_bus::event_bus::new_bus();

        handle_event(
            confirmation(&jour.id.to_string(), None),
            &pool,
            &repo,
            &equipes(),
            &bus,
        )
        .await;

        assert_eq!(
            appariements(&pool, &jour.id.to_string()).await,
            0,
            "aucun appariement ne doit être fabriqué"
        );
    }

    /// Le cas nominal : le rapport porte son appariement, et la ligne
    /// d'affichage passe en saisie.
    #[sqlx::test]
    async fn une_confirmation_avec_appariement_passe_la_ligne_en_cours(pool: PgPool) {
        let jour = journee(&pool).await;
        let repo = MatchDayRepository::new(pool.clone());
        let bus = crate::common::services::event_bus::event_bus::new_bus();
        let pairing = appariement_existant(&pool, &jour).await;

        handle_event(
            confirmation(&jour.id.to_string(), Some(pairing.clone())),
            &pool,
            &repo,
            &equipes(),
            &bus,
        )
        .await;

        let statut: Option<String> = sqlx::query_scalar!(
            "SELECT match_status FROM competition_match_display_proj WHERE match_report_id = $1",
            "01ARZ3NDEKTSV4RRFFQ69G5FAX",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert_eq!(statut.as_deref(), Some("in_progress"));
    }

    #[sqlx::test]
    async fn un_rapport_programme_confirme_ne_cree_aucun_appariement(pool: PgPool) {
        let jour = journee(&pool).await;
        let repo = MatchDayRepository::new(pool.clone());
        let bus = crate::common::services::event_bus::event_bus::new_bus();

        handle_event(
            confirmation(
                &jour.id.to_string(),
                Some("un-appariement-existant".to_string()),
            ),
            &pool,
            &repo,
            &equipes(),
            &bus,
        )
        .await;

        assert_eq!(
            appariements(&pool, &jour.id.to_string()).await,
            0,
            "un rapport programmé ne fabrique rien"
        );
    }
}

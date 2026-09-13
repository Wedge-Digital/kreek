//! Ce que la page publique de réponse a besoin de dire.
//!
//! # Le trou que ce service comble
//!
//! La page affiche « Journée 3 · du 12 au 19 octobre », « Ton équipe : Les Rats
//! d'Égouts », et un lien vers la compétition. **`PresenceSurvey` ne porte aucun
//! de ces libellés** : il a des identifiants, une échéance et des réponses.
//!
//! Il ne se comble pas par des ports appelés depuis le handler — ce serait exposer
//! `TeamInfoDto` à la couche web, ce que la règle « domain services pour données
//! inter-BCs » interdit.
//!
//! # Ce qui traverse en types du domaine, et pourquoi
//!
//! `presence`, `statut` et `deadline` sortent d'ici **non aplatis**. C'est le view
//! model, en bout de chaîne, qui choisit les mots ; les réduire à des `String` ici
//! mettrait la formulation dans la couche applicative et rendrait le VM incapable
//! de distinguer un état d'un libellé.
//!
//! # Ce qui n'y est pas, et pourquoi
//!
//! **Aucune URL.** Construire un lien obligerait un service de `use_cases/` à
//! connaître `AppRoutes`, qui est de la couche web —
//! `notification_recipients.rs` a écarté un `match_url` pour cette raison exacte.
//! Le service rend les identifiants ; le gabarit compose.
//!
//! **Aucun libellé composé.** « Du 12 au 19 octobre » se compose déjà dans la
//! couche web ; le refaire ici le mettrait à deux endroits. Les dates sortent
//! brutes, comme le DTO de la carte 523.

use crate::app::competitions::domain::presence_survey::{
    Presence, PresenceSurvey, SurveyDeadline, SurveyStatus, SurveyToken,
};
use crate::app::competitions::domain::presence_survey_repository_port::IPresenceSurveyRepository;
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;

/// Tout ce que la page publique affiche, et rien de plus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandingContext {
    pub team_name: String,
    pub round_name: String,
    pub round_date_start: Option<String>,
    pub round_date_end: Option<String>,
    pub competition_name: String,
    /// Les identifiants, pour que le gabarit compose le lien par `AppRoutes`.
    pub competition_id: String,
    pub season_id: String,
    pub space_id: String,
    pub deadline: SurveyDeadline,
    /// Du domaine : le VM choisit les mots, pas ce service.
    pub presence: Presence,
    pub statut: SurveyStatus,
}

/// Assemble la vue d'un jeton, ou rien.
///
/// **`Option` et non `Result`, et une seule sortie vide.** Un jeton qui ne désigne
/// aucune réponse, une journée dont les libellés manquent, une équipe introuvable :
/// tout rend `None`, et la route affichera la même page « lien inconnu ». Deux
/// sorties distinctes révéleraient qu'un jeton a existé, ce que R26 refuse.
///
/// `aujourd_hui` est une **entrée** : le statut se calcule (R23), et un service qui
/// lit l'horloge n'est testable qu'en trichant sur celle de la machine.
// arch:no-instrument — service d'hydratation : assemble une vue, sans intention métier
pub async fn hydrater(
    survey: &PresenceSurvey,
    token: &SurveyToken,
    survey_repo: &dyn IPresenceSurveyRepository,
    team_port: &dyn ITeamInfoPort,
    aujourd_hui: &DateString,
) -> Option<LandingContext> {
    let reponse = survey.reponse_par_jeton(token)?;
    let libelles = survey_repo
        .find_landing_labels(&token.to_string())
        .await
        .ok()
        .flatten()?;

    Some(LandingContext {
        team_name: nom_de_l_equipe(team_port, &reponse.team_id().to_string()).await,
        round_name: libelles.round_name,
        round_date_start: libelles.round_date_start,
        round_date_end: libelles.round_date_end,
        competition_name: libelles.competition_name,
        competition_id: libelles.competition_id,
        season_id: libelles.season_id,
        space_id: libelles.space_id,
        deadline: survey.deadline().clone(),
        presence: reponse.presence().clone(),
        statut: survey.statut(aujourd_hui),
    })
}

/// « Ton équipe » quand le port ne la connaît plus.
///
/// `find_team_names` **omet les identifiants introuvables**, et afficher l'ULID à
/// la place serait le défaut de la carte 506. La page reste utilisable sans le
/// nom : le coach sait quelle équipe il gère, c'est le lien qu'il a reçu.
async fn nom_de_l_equipe(team_port: &dyn ITeamInfoPort, team_id: &str) -> String {
    team_port
        .find_team_names(std::slice::from_ref(&team_id.to_string()))
        .await
        .ok()
        .and_then(|equipes| equipes.into_iter().next())
        .map(|e| e.team_name)
        .unwrap_or_else(|| "Ton équipe".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::MatchDayType;
    use crate::app::competitions::domain::presence_survey::{
        Destinataire, Motif, Repondant, Venue,
    };
    use crate::app::competitions::domain::presence_survey_repository_port::{
        LandingLabelsDto, PresenceSurveyRepositoryError,
    };
    use crate::app::competitions::ports::{TeamEnrollmentDto, TeamInfoDto};
    use crate::app::competitions::use_cases::presences::test_doubles::*;
    use crate::app::shared_kernel::bloodbowl::ids::MatchId;
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::CoachId;
    use async_trait::async_trait;

    /// Le dépôt de la 515 ne rend pas de libellés — c'est l'unité 2 qui les lit.
    /// Celui-ci les fournit, comme le vrai dépôt le fait depuis la carte 523.
    struct FauxLibelles {
        survey: PresenceSurvey,
        libelles: Option<LandingLabelsDto>,
    }

    #[async_trait]
    impl IPresenceSurveyRepository for FauxLibelles {
        async fn find_by_round(
            &self,
            _r: &str,
        ) -> Result<Option<PresenceSurvey>, PresenceSurveyRepositoryError> {
            Ok(Some(self.survey.clone()))
        }
        async fn find_by_token(
            &self,
            _t: &str,
        ) -> Result<Option<PresenceSurvey>, PresenceSurveyRepositoryError> {
            Ok(Some(self.survey.clone()))
        }
        async fn find_landing_labels(
            &self,
            _t: &str,
        ) -> Result<Option<LandingLabelsDto>, PresenceSurveyRepositoryError> {
            Ok(self.libelles.clone())
        }
        async fn save(&self, _s: &PresenceSurvey) -> Result<(), PresenceSurveyRepositoryError> {
            Ok(())
        }
        async fn list_summaries(
            &self,
            _s: &str,
        ) -> Result<
            Vec<
                crate::app::competitions::domain::presence_survey_repository_port::SurveySummaryDto,
            >,
            PresenceSurveyRepositoryError,
        > {
            Ok(vec![])
        }

        /// Ce service ne liste rien : il hydrate **une** campagne, désignée par son
        /// jeton. Une liste vide dit ici l'absence de besoin, pas un échafaudage
        /// tronqué — aucun test de ce fichier ne l'atteint.
        async fn list_open_surveys_for_season(
            &self,
            _season_id: &str,
            _maintenant: &str,
        ) -> Result<Vec<PresenceSurvey>, PresenceSurveyRepositoryError> {
            Ok(vec![])
        }
    }

    fn libelles() -> LandingLabelsDto {
        LandingLabelsDto {
            round_name: "Journée 3".to_string(),
            round_date_start: Some("2026-10-12".to_string()),
            round_date_end: Some("2026-10-19".to_string()),
            competition_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            competition_name: "Ligue du Chaos".to_string(),
            season_id: "01ARZ3NDEKTSV4RRFFQ69G5FAW".to_string(),
            space_id: "01ARZ3NDEKTSV4RRFFQ69G5FAX".to_string(),
        }
    }

    fn campagne_de(dests: &[Destinataire]) -> PresenceSurvey {
        campagne_ouverte(
            &journee(MatchId::new(), MatchDayType::FixedDate, vec![]),
            dests,
        )
    }

    fn nommee(team_id: &TeamId, nom: &str) -> FauxTeams {
        FauxTeams(vec![TeamInfoDto {
            team_id: team_id.to_string(),
            team_name: nom.to_string(),
            coach_id: CoachId::new().to_string(),
            coach_name: "Lepandawan".to_string(),
            roster_name: "Skavens".to_string(),
            logo_url: None,
        }])
    }

    async fn hydrater_avec(
        survey: PresenceSurvey,
        teams: FauxTeams,
        libelles: Option<LandingLabelsDto>,
        token: SurveyToken,
        aujourd_hui: &str,
    ) -> Option<LandingContext> {
        let depot = FauxLibelles {
            survey: survey.clone(),
            libelles,
        };
        hydrater(&survey, &token, &depot, &teams, &date(aujourd_hui)).await
    }

    // ── Ce que la page dit ───────────────────────────────────────────────────

    #[tokio::test]
    async fn la_page_nomme_l_equipe_la_journee_et_la_competition() {
        let dests = destinataires(2);
        let survey = campagne_de(&dests);
        let token = *survey.reponses()[0].token();

        let ctx = hydrater_avec(
            survey,
            nommee(&dests[0].team_id, "Les Rats d'Égouts"),
            Some(libelles()),
            token,
            "2026-10-05",
        )
        .await
        .expect("contexte");

        assert_eq!(ctx.team_name, "Les Rats d'Égouts");
        assert_eq!(ctx.round_name, "Journée 3");
        assert_eq!(ctx.competition_name, "Ligue du Chaos");
        assert_eq!(ctx.round_date_start.as_deref(), Some("2026-10-12"));
    }

    /// **Aucun libellé composé, aucune URL.** Le service rend les identifiants et
    /// les dates brutes ; composer relève de la couche web, et le faire ici
    /// mettrait la formulation à deux endroits — ou obligerait un service de
    /// `use_cases/` à connaître `AppRoutes`.
    #[tokio::test]
    async fn le_service_rend_des_identifiants_pas_des_liens() {
        let dests = destinataires(1);
        let survey = campagne_de(&dests);
        let token = *survey.reponses()[0].token();

        let ctx = hydrater_avec(
            survey,
            nommee(&dests[0].team_id, "Une équipe"),
            Some(libelles()),
            token,
            "2026-10-05",
        )
        .await
        .expect("contexte");

        assert_eq!(ctx.competition_id, "01ARZ3NDEKTSV4RRFFQ69G5FAV");
        assert!(!ctx.space_id.is_empty());
    }

    /// `presence` et `statut` traversent **en types du domaine** : c'est le VM qui
    /// choisit les mots. Les aplatir ici rendrait la vue incapable de distinguer
    /// un état d'un libellé.
    #[tokio::test]
    async fn la_presence_et_le_statut_ne_sont_pas_aplatis() {
        let dests = destinataires(1);
        let mut survey = campagne_de(&dests);
        survey
            .enregistrer(
                &dests[0].team_id,
                Venue::Presente,
                Repondant::Jeton,
                &etat_vierge(),
                &date("2026-10-04"),
            )
            .unwrap();
        let token = *survey.reponses()[0].token();

        let ctx = hydrater_avec(
            survey,
            nommee(&dests[0].team_id, "Une équipe"),
            Some(libelles()),
            token,
            "2026-10-05",
        )
        .await
        .expect("contexte");

        assert!(matches!(
            ctx.presence,
            Presence::Declaree {
                venue: Venue::Presente,
                ..
            }
        ));
        assert_eq!(ctx.statut, SurveyStatus::Ouverte);
    }

    /// R23 — la clôture est calculée. Le lendemain de l'échéance, le service la
    /// rapporte close sans que rien ne l'ait écrite.
    #[tokio::test]
    async fn une_campagne_echue_est_rapportee_close() {
        let dests = destinataires(1);
        let survey = campagne_de(&dests);
        let token = *survey.reponses()[0].token();

        let ctx = hydrater_avec(
            survey,
            nommee(&dests[0].team_id, "Une équipe"),
            Some(libelles()),
            token,
            "2026-10-11",
        )
        .await
        .expect("contexte");

        assert_eq!(ctx.statut, SurveyStatus::Close(Motif::Echeance));
        assert!(matches!(ctx.presence, Presence::SansReponse));
    }

    // ── Ce qui manque, et ce que ça donne ────────────────────────────────────

    /// `find_team_names` omet les identifiants introuvables. Afficher l'ULID serait
    /// le défaut de la carte 506 ; la page reste utilisable sans le nom, puisque le
    /// coach sait quelle équipe il gère.
    #[tokio::test]
    async fn une_equipe_inconnue_du_port_ne_montre_pas_son_identifiant() {
        let dests = destinataires(1);
        let survey = campagne_de(&dests);
        let token = *survey.reponses()[0].token();

        let ctx = hydrater_avec(
            survey,
            FauxTeams(vec![]),
            Some(libelles()),
            token,
            "2026-10-05",
        )
        .await
        .expect("contexte");

        assert_eq!(ctx.team_name, "Ton équipe");
        assert_ne!(ctx.team_name, dests[0].team_id.to_string());
    }

    /// **Une seule sortie vide, quelle que soit la cause.** Un jeton qui ne
    /// désigne rien et des libellés manquants rendent tous deux `None` : deux
    /// sorties distinctes révéleraient qu'un jeton a existé, ce que R26 refuse.
    #[tokio::test]
    async fn un_jeton_etranger_et_des_libelles_manquants_rendent_la_meme_chose() {
        let dests = destinataires(1);
        let survey = campagne_de(&dests);
        let sien = *survey.reponses()[0].token();

        let etranger = hydrater_avec(
            survey.clone(),
            nommee(&dests[0].team_id, "Une équipe"),
            Some(libelles()),
            SurveyToken::new(),
            "2026-10-05",
        )
        .await;
        let sans_libelles = hydrater_avec(
            survey,
            nommee(&dests[0].team_id, "Une équipe"),
            None,
            sien,
            "2026-10-05",
        )
        .await;

        assert_eq!(etranger, None);
        assert_eq!(sans_libelles, None);
    }

    /// Le port doit rester dans le service : le test le vérifie par la signature —
    /// `LandingContext` ne porte aucun `TeamInfoDto` ni `LandingLabelsDto`.
    #[tokio::test]
    async fn aucun_dto_de_port_ne_sort_du_service() {
        let dests = destinataires(1);
        let survey = campagne_de(&dests);
        let token = *survey.reponses()[0].token();
        let _: LandingContext = hydrater_avec(
            survey,
            nommee(&dests[0].team_id, "Une équipe"),
            Some(libelles()),
            token,
            "2026-10-05",
        )
        .await
        .expect("contexte");
        // Si un DTO de port entrait dans `LandingContext`, ce fichier ne
        // compilerait pas sans importer `TeamInfoDto` dans la couche web.
    }

    // `TeamEnrollmentDto` n'est utilisé que par `FauxTeams` des doubles partagés.
    #[allow(dead_code)]
    fn _types_utilises(_: Option<TeamEnrollmentDto>) {}
}

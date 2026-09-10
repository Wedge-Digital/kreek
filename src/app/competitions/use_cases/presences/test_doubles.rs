//! Les doubles partagés par les tests des use cases de présence.
//!
//! Un module et non quatre copies : `IMatchDayRepository` a treize méthodes, dont
//! deux servent ici. Les recopier dans chaque fichier de test — quatre fois pour
//! les cartes 515 à 518 — ferait de leur signature une dette répartie, qu'un
//! ajout au port obligerait à réparer partout.
//!
//! Les cinq faux qui existent déjà dans le BC ne sont pas repris : ils sont
//! internes à leur module de test, et les rendre partageables serait une refonte
//! sans rapport avec cette carte.

#![cfg(test)]

use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, LatestResultDto, MatchDayRepositoryError, NewPairingProjection,
    PairingDisplayDto,
};
use crate::app::competitions::domain::presence_survey::PresenceSurvey;
use crate::app::competitions::domain::presence_survey_repository_port::{
    IPresenceSurveyRepository, LandingLabelsDto, PresenceSurveyRepositoryError, SurveySummaryDto,
};
use crate::app::competitions::ports::{
    ICompetitionSpaceMemberPort, IMatchReportStatusPort, ITeamInfoPort, SpaceMemberDto,
    TeamEnrollmentDto, TeamInfoDto,
};
use crate::app::competitions::use_cases::presences::survey_mailer::{
    CampagneAAnnoncer, EnvoiPresence, ISurveyMailer, RapportEnvoi,
};
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::shared_kernel::identity::space_definition::SpaceDefinition;
use async_trait::async_trait;
use std::sync::Mutex;

// ── Le dépôt de campagnes ────────────────────────────────────────────────────

/// Il **garde ce qu'on lui écrit** : c'est ce qui permet d'éprouver R20 — la
/// campagne est bien persistée après une expédition en échec.
#[derive(Default)]
pub struct FauxSurveyRepo {
    existante: Option<PresenceSurvey>,
    ecrites: Mutex<Vec<PresenceSurvey>>,
}

impl FauxSurveyRepo {
    pub fn vide() -> Self {
        Self::default()
    }

    pub fn avec(survey: PresenceSurvey) -> Self {
        Self {
            existante: Some(survey),
            ecrites: Mutex::new(vec![]),
        }
    }

    pub fn derniere_ecrite(&self) -> Option<PresenceSurvey> {
        self.ecrites.lock().expect("mutex de test").last().cloned()
    }

    pub fn ecritures(&self) -> usize {
        self.ecrites.lock().expect("mutex de test").len()
    }
}

#[async_trait]
impl IPresenceSurveyRepository for FauxSurveyRepo {
    async fn find_by_round(
        &self,
        _round_id: &str,
    ) -> Result<Option<PresenceSurvey>, PresenceSurveyRepositoryError> {
        Ok(self.existante.clone())
    }

    async fn save(&self, survey: &PresenceSurvey) -> Result<(), PresenceSurveyRepositoryError> {
        self.ecrites
            .lock()
            .expect("mutex de test")
            .push(survey.clone());
        Ok(())
    }

    /// Le jeton retrouve la même campagne que la journée — c'est ce que fait le
    /// vrai dépôt, qui partage son chemin d'hydratation entre les deux.
    async fn find_by_token(
        &self,
        token: &str,
    ) -> Result<Option<PresenceSurvey>, PresenceSurveyRepositoryError> {
        Ok(self
            .existante
            .clone()
            .filter(|s| s.reponses().iter().any(|r| r.token().to_string() == token)))
    }

    /// Les use cases de présence ne titrent aucune page : seule la route publique
    /// de l'unité 2 lira ces libellés, et son propre double les fournira.
    async fn find_landing_labels(
        &self,
        _token: &str,
    ) -> Result<Option<LandingLabelsDto>, PresenceSurveyRepositoryError> {
        Ok(None)
    }

    async fn list_summaries(
        &self,
        _season_id: &str,
    ) -> Result<Vec<SurveySummaryDto>, PresenceSurveyRepositoryError> {
        Ok(vec![])
    }
}

// ── Le dépôt de journées ─────────────────────────────────────────────────────

/// Seules `find_by_id`, `find_by_season` et `save_pairings` répondent vraiment :
/// les dix autres méthodes ne sont pas atteintes par ces use cases, et rendre des
/// listes vides est plus honnête qu'un `unimplemented!()` — un test qui les
/// toucherait constaterait l'absence de donnée, pas une panne de l'échafaudage.
pub struct FauxJournees {
    jour: Option<MatchDay>,
    /// Ce que `find_by_season` rend. Vide par défaut : `jour` seul, comme avant.
    saison: Option<Vec<MatchDay>>,
    ecriture_en_panne: bool,
    ecrits: Mutex<Vec<Pairing>>,
}

impl FauxJournees {
    pub fn avec(jour: MatchDay) -> Self {
        Self {
            jour: Some(jour),
            saison: None,
            ecriture_en_panne: false,
            ecrits: Mutex::new(vec![]),
        }
    }

    pub fn aucune() -> Self {
        Self {
            jour: None,
            saison: None,
            ecriture_en_panne: false,
            ecrits: Mutex::new(vec![]),
        }
    }

    /// Les journées de la saison, pour que `build_historique` ait de quoi
    /// travailler — c'est ce qui permet d'éprouver que l'étiquette d'une rencontre
    /// se recalcule au lieu d'être reprise de la commande.
    pub fn et_la_saison(mut self, jours: Vec<MatchDay>) -> Self {
        self.saison = Some(jours);
        self
    }

    /// Un dépôt qui refuse d'écrire — pour éprouver ce qui se passe **après**
    /// l'échec, et notamment ce qui n'est pas émis.
    pub fn dont_l_ecriture_echoue(mut self) -> Self {
        self.ecriture_en_panne = true;
        self
    }

    pub fn ecrits(&self) -> usize {
        self.ecrits.lock().expect("mutex de test").len()
    }
}

#[async_trait]
impl IMatchDayRepository for FauxJournees {
    async fn find_by_season(&self, _s: &str) -> Result<Vec<MatchDay>, MatchDayRepositoryError> {
        Ok(self
            .saison
            .clone()
            .unwrap_or_else(|| self.jour.clone().into_iter().collect()))
    }
    async fn find_by_id(&self, _id: &str) -> Result<Option<MatchDay>, MatchDayRepositoryError> {
        Ok(self.jour.clone())
    }
    async fn save_match_day(&self, _d: &MatchDay) -> Result<(), MatchDayRepositoryError> {
        Ok(())
    }
    async fn delete_match_day(&self, _id: &str) -> Result<(), MatchDayRepositoryError> {
        Ok(())
    }
    async fn save_pairings(
        &self,
        _id: &str,
        p: &[(Pairing, NewPairingProjection)],
    ) -> Result<(), MatchDayRepositoryError> {
        if self.ecriture_en_panne {
            return Err(MatchDayRepositoryError::PairingsAlreadyExist);
        }
        self.ecrits
            .lock()
            .expect("mutex de test")
            .extend(p.iter().map(|(pairing, _)| pairing.clone()));
        Ok(())
    }
    async fn save_pairing(
        &self,
        _id: &str,
        _p: &Pairing,
        _proj: &NewPairingProjection,
    ) -> Result<(), MatchDayRepositoryError> {
        Ok(())
    }
    async fn find_pairing_id(
        &self,
        _id: &str,
        _home: &str,
        _away: &str,
    ) -> Result<Option<String>, MatchDayRepositoryError> {
        Ok(None)
    }
    async fn delete_pairing(&self, _id: &str) -> Result<(), MatchDayRepositoryError> {
        Ok(())
    }
    async fn ensure_match_days_from_structure(
        &self,
        _s: &str,
        _e: &[(String, String, String, Option<String>, Option<String>)],
    ) -> Result<(), MatchDayRepositoryError> {
        Ok(())
    }
    async fn list_resultats(
        &self,
        _s: &str,
        _c: Option<i32>,
        _l: u32,
    ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
        Ok(vec![])
    }
    async fn list_calendrier(
        &self,
        _s: &str,
        _c: Option<i32>,
        _l: u32,
    ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
        Ok(vec![])
    }
    async fn list_team_matches(
        &self,
        _s: &str,
        _t: &str,
    ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
        Ok(vec![])
    }
    async fn list_latest_completed_results(
        &self,
        _s: &str,
        _l: i64,
    ) -> Result<Vec<LatestResultDto>, MatchDayRepositoryError> {
        Ok(vec![])
    }
}

// ── Les deux ports du roster ─────────────────────────────────────────────────

pub struct FauxTeams(pub Vec<TeamInfoDto>);

#[async_trait]
impl ITeamInfoPort for FauxTeams {
    async fn find_enrolled_teams(&self, _s: &str) -> Result<Vec<TeamInfoDto>, String> {
        Ok(self.0.clone())
    }
    async fn find_team_names(&self, _ids: &[String]) -> Result<Vec<TeamInfoDto>, String> {
        Ok(vec![])
    }
    async fn find_team_enrollment(&self, _t: &str) -> Result<Option<TeamEnrollmentDto>, String> {
        Ok(None)
    }
}

pub struct FauxMembres(pub Vec<SpaceMemberDto>);

#[async_trait]
impl ICompetitionSpaceMemberPort for FauxMembres {
    async fn list_space_members(&self, _s: &SpaceId) -> Vec<SpaceMemberDto> {
        self.0.clone()
    }
    async fn find_member_profile(&self, _c: &CoachId, _s: &SpaceId) -> Option<SpaceProfile> {
        None
    }
    async fn find_all_spaces(&self) -> Vec<SpaceDefinition> {
        vec![]
    }
}

// ── L'état des rapports, et l'expédition ─────────────────────────────────────

/// `figee` posé directement : R13 se raconte mieux par « la journée est figée »
/// que par « voici la liste des appariements dont un rapport est publié ».
pub struct FauxRapports(pub bool);

#[async_trait]
impl IMatchReportStatusPort for FauxRapports {
    async fn find_published_pairings(&self, ids: &[String]) -> Result<Vec<String>, String> {
        Ok(if self.0 { ids.to_vec() } else { vec![] })
    }
}

/// `en_panne` fait échouer tous les envois — c'est l'entrée de R20 : la campagne
/// doit être ouverte et persistée quand même.
pub struct FauxMailer {
    pub en_panne: bool,
    envois: Mutex<Vec<EnvoiPresence>>,
}

impl FauxMailer {
    pub fn qui_marche() -> Self {
        Self {
            en_panne: false,
            envois: Mutex::new(vec![]),
        }
    }

    pub fn en_panne() -> Self {
        Self {
            en_panne: true,
            envois: Mutex::new(vec![]),
        }
    }

    pub fn envois(&self) -> Vec<EnvoiPresence> {
        self.envois.lock().expect("mutex de test").clone()
    }
}

#[async_trait]
impl ISurveyMailer for FauxMailer {
    async fn expedier(
        &self,
        _campagne: &CampagneAAnnoncer,
        envois: &[EnvoiPresence],
    ) -> RapportEnvoi {
        self.envois
            .lock()
            .expect("mutex de test")
            .extend_from_slice(envois);
        if self.en_panne {
            RapportEnvoi {
                envoyes: 0,
                echecs: envois.len(),
            }
        } else {
            RapportEnvoi {
                envoyes: envois.len(),
                echecs: 0,
            }
        }
    }
}

// ── Fabriques ────────────────────────────────────────────────────────────────

use crate::app::competitions::domain::match_day::{MatchDayName, MatchDayPosition, MatchDayType};
use crate::app::competitions::domain::presence_survey::{
    AutoRemind, Destinataire, PresenceSurvey as Campagne, SurveyDeadline, SurveyId,
};
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;

pub fn journee(id: MatchId, day_type: MatchDayType, pairings: Vec<Pairing>) -> MatchDay {
    MatchDay {
        id,
        season_id: SeasonId::new(),
        name: MatchDayName::try_new("Journée 3".to_string()).expect("nom de test"),
        day_type,
        date_start: None,
        date_end: None,
        position: MatchDayPosition::try_new(2).expect("position de test"),
        pairings,
    }
}

pub fn equipe(nom: &str, coach: &CoachId, coach_name: &str) -> TeamInfoDto {
    TeamInfoDto {
        team_id: TeamId::new().to_string(),
        team_name: nom.to_string(),
        coach_id: coach.to_string(),
        coach_name: coach_name.to_string(),
        roster_name: "Humains".to_string(),
        logo_url: None,
    }
}

pub fn membre(coach: &CoachId, nom: &str, email: &str) -> SpaceMemberDto {
    SpaceMemberDto {
        coach_id: coach.to_string(),
        coach_name: nom.to_string(),
        email: email.to_string(),
    }
}

pub fn date(s: &str) -> DateString {
    DateString::try_new(s.to_string()).expect("date de test")
}

pub fn echeance(s: &str) -> SurveyDeadline {
    SurveyDeadline::try_new(s.to_string()).expect("échéance de test")
}

/// Une campagne ouverte le 1er octobre, échéance au 10 — donc ouverte le 5 et
/// close par échéance le 11.
pub fn campagne_ouverte(round: &MatchDay, destinataires: &[Destinataire]) -> Campagne {
    Campagne::ouvrir(
        SurveyId::new(),
        SeasonId::new(),
        round,
        destinataires,
        echeance("2026-10-10"),
        AutoRemind::new(true),
        &date("2026-10-01"),
    )
    .expect("ouverture de test")
}

/// Une journée sans rapport publié et sans appariement — les faits qu'attendent
/// les méthodes de l'agrégat quand rien n'est encore tiré.
pub fn etat_vierge() -> crate::app::competitions::domain::presence_survey::EtatJournee {
    crate::app::competitions::domain::presence_survey::EtatJournee {
        figee: false,
        rencontres: vec![],
    }
}

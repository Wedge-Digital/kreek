//! Relancer ceux qui n'ont pas répondu.
//!
//! # Pourquoi une campagne close refuse la relance
//!
//! Le jeton n'a pas de durée de vie propre : sa validité se lit sur l'état de la
//! campagne (R7). Relancer une campagne close enverrait donc un lien qui ne
//! répond plus — un e-mail qui se contredit lui-même, et un coach qui clique
//! pour rien.
//!
//! Rouvrir d'abord, relancer ensuite : c'est ce que `reopen_survey_use_case`
//! rend possible, et il réarme les anciens liens sans rien réémettre.

use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError,
};
use crate::app::competitions::domain::presence_survey::PresenceSurvey;
use crate::app::competitions::domain::presence_survey_repository_port::{
    IPresenceSurveyRepository, PresenceSurveyRepositoryError,
};
use crate::app::competitions::ports::{ICompetitionSpaceMemberPort, ITeamInfoPort};
use crate::app::competitions::use_cases::presences::survey_mailer::{
    CampagneAAnnoncer, EnvoiPresence, ISurveyMailer,
};
use crate::app::competitions::use_cases::presences::survey_roster_service::{
    self, RosterDeCampagne,
};
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
use crate::app::shared_kernel::identity::ids::SpaceId;

#[derive(Debug)]
pub struct RemindCommand {
    pub round_id: MatchId,
    pub season_id: SeasonId,
    pub space_id: SpaceId,
    pub aujourd_hui: DateString,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RemindOutcome {
    /// Les silencieux au moment de la relance — R5 les tient à part précisément
    /// parce que ce sont les seuls sur lesquels l'organisateur a prise.
    pub relances: usize,
    pub envoyes: usize,
    pub echecs: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RemindError {
    SurveyNotFound,
    RoundNotFound,
    /// R7 — un lien qui ne répond plus ne se relance pas.
    SurveyClosed,
    Database(String),
}

impl From<PresenceSurveyRepositoryError> for RemindError {
    fn from(e: PresenceSurveyRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<MatchDayRepositoryError> for RemindError {
    fn from(e: MatchDayRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

pub struct RemindDeps<'a> {
    pub survey_repo: &'a dyn IPresenceSurveyRepository,
    pub match_day_repo: &'a dyn IMatchDayRepository,
    pub teams: &'a dyn ITeamInfoPort,
    pub members: &'a dyn ICompetitionSpaceMemberPort,
    pub mailer: &'a dyn ISurveyMailer,
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RemindCommand,
    deps: RemindDeps<'_>,
) -> Result<RemindOutcome, RemindError> {
    let survey = deps
        .survey_repo
        .find_by_round(&cmd.round_id.to_string())
        .await?
        .ok_or(RemindError::SurveyNotFound)?;

    if !survey.statut(&cmd.aujourd_hui).est_ouverte() {
        return Err(RemindError::SurveyClosed);
    }

    let round = deps
        .match_day_repo
        .find_by_id(&cmd.round_id.to_string())
        .await?
        .ok_or(RemindError::RoundNotFound)?;
    let roster = survey_roster_service::charger(
        &cmd.season_id.to_string(),
        &cmd.space_id,
        deps.teams,
        deps.members,
    )
    .await
    .map_err(RemindError::Database)?;

    let envois = envois_des_silencieux(&roster, &survey);
    let campagne = CampagneAAnnoncer {
        round_name: round.name.as_ref().to_string(),
        deadline: survey.deadline().clone(),
    };
    let rapport = deps.mailer.expedier(&campagne, &envois).await;

    Ok(RemindOutcome {
        relances: envois.len(),
        envoyes: rapport.envoyes,
        echecs: rapport.echecs,
    })
}

/// Les silencieux **joignables**. Un silencieux sans adresse n'est pas relancé —
/// et ce n'est pas une erreur : R3 l'a compté au lancement, l'organisateur sait
/// qu'il doit le joindre autrement.
fn envois_des_silencieux(roster: &RosterDeCampagne, survey: &PresenceSurvey) -> Vec<EnvoiPresence> {
    survey
        .sans_reponse()
        .iter()
        .filter_map(|r| {
            let equipe = roster.equipe(r.team_id())?;
            Some(EnvoiPresence {
                email: equipe.email.clone()?,
                coach_label: equipe.coach_label.clone(),
                team_name: equipe.team_name.clone(),
                token: *r.token(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::MatchDayType;
    use crate::app::competitions::domain::presence_survey::{
        Destinataire, EtatJournee, Repondant, Venue,
    };
    use crate::app::competitions::use_cases::presences::test_doubles::*;
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::CoachId;

    fn commande(round: &MatchId, aujourd_hui: &str) -> RemindCommand {
        RemindCommand {
            round_id: *round,
            season_id: SeasonId::new(),
            space_id: SpaceId::new(),
            aujourd_hui: date(aujourd_hui),
        }
    }

    /// R7 — le jeton n'a pas de durée de vie propre : sa validité se lit sur
    /// l'état de la campagne. Relancer une campagne close enverrait un lien qui ne
    /// répond plus, donc un e-mail qui se contredit lui-même.
    #[tokio::test]
    async fn une_campagne_close_refuse_la_relance() {
        let round = MatchId::new();
        let jour = journee(round, MatchDayType::FixedDate, vec![]);
        let campagne = campagne_ouverte(&jour, &[]);
        let mailer = FauxMailer::qui_marche();

        // Échéance au 10 : le 11, la campagne est close par R23, sans que rien ne
        // l'ait écrite.
        let refus = execute(
            commande(&round, "2026-10-11"),
            RemindDeps {
                survey_repo: &FauxSurveyRepo::avec(campagne),
                match_day_repo: &FauxJournees(Some(jour)),
                teams: &FauxTeams(vec![]),
                members: &FauxMembres(vec![]),
                mailer: &mailer,
            },
        )
        .await;

        assert_eq!(refus.unwrap_err(), RemindError::SurveyClosed);
        assert!(mailer.envois().is_empty(), "aucun e-mail ne part");
    }

    /// R5 — les silencieux sont les seuls sur lesquels l'organisateur a prise, et
    /// ce sont donc les seuls relancés. Ceux qui ont répondu, dans un sens ou dans
    /// l'autre, ne reçoivent rien.
    #[tokio::test]
    async fn seuls_les_silencieux_joignables_sont_relances() {
        let round = MatchId::new();
        let (a, b, c, d) = (
            CoachId::new(),
            CoachId::new(),
            CoachId::new(),
            CoachId::new(),
        );
        let jour = journee(round, MatchDayType::FixedDate, vec![]);
        let equipes = vec![
            equipe("A répondu présent", &a, "Alpha"),
            equipe("A répondu absent", &b, "Beta"),
            equipe("Silencieux joignable", &c, "Gamma"),
            equipe("Silencieux sans adresse", &d, "Delta"),
        ];
        let ids: Vec<TeamId> = equipes
            .iter()
            .map(|e| TeamId::try_new(&e.team_id).unwrap())
            .collect();
        let coachs: Vec<CoachId> = equipes
            .iter()
            .map(|e| CoachId::try_new(&e.coach_id).unwrap())
            .collect();
        let destinataires: Vec<Destinataire> = ids
            .iter()
            .zip(&coachs)
            .map(|(t, c)| Destinataire {
                team_id: *t,
                coach_id: *c,
            })
            .collect();

        let mut campagne = campagne_ouverte(&jour, &destinataires);
        let vierge = EtatJournee {
            figee: false,
            rencontres: vec![],
        };
        campagne
            .enregistrer(
                &ids[0],
                Venue::Presente,
                Repondant::Jeton,
                &vierge,
                &date("2026-10-04"),
            )
            .unwrap();
        campagne
            .enregistrer(
                &ids[1],
                Venue::Absente,
                Repondant::Jeton,
                &vierge,
                &date("2026-10-04"),
            )
            .unwrap();

        let mailer = FauxMailer::qui_marche();
        let issue = execute(
            commande(&round, "2026-10-05"),
            RemindDeps {
                survey_repo: &FauxSurveyRepo::avec(campagne),
                match_day_repo: &FauxJournees(Some(jour)),
                teams: &FauxTeams(equipes),
                members: &FauxMembres(vec![
                    membre(&a, "Alpha", "a@example.test"),
                    membre(&b, "Beta", "b@example.test"),
                    membre(&c, "Gamma", "c@example.test"),
                ]),
                mailer: &mailer,
            },
        )
        .await
        .expect("relance");

        assert_eq!(
            issue.relances, 1,
            "deux silencieux, mais un seul joignable — R3 a compté l'autre au lancement"
        );
        assert_eq!(issue.envoyes, 1);
        assert_eq!(mailer.envois()[0].team_name, "Silencieux joignable");
    }

    #[tokio::test]
    async fn une_journee_sans_campagne_ne_se_relance_pas() {
        let round = MatchId::new();
        let mailer = FauxMailer::qui_marche();

        let refus = execute(
            commande(&round, "2026-10-05"),
            RemindDeps {
                survey_repo: &FauxSurveyRepo::vide(),
                match_day_repo: &FauxJournees(None),
                teams: &FauxTeams(vec![]),
                members: &FauxMembres(vec![]),
                mailer: &mailer,
            },
        )
        .await;

        assert_eq!(refus.unwrap_err(), RemindError::SurveyNotFound);
    }
}

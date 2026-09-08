//! Ouvrir une campagne de présence sur une journée.
//!
//! # R20 — persister avant d'expédier
//!
//! L'ordre des deux dernières étapes **est** la règle : la campagne existe avant
//! que le premier e-mail ne parte, et un échec d'expédition est journalisé, pas
//! propagé. Un serveur de messagerie indisponible bloquerait sinon une fonction
//! qui reste utilisable sans lui — l'encart du coach connecté, et la saisie
//! manuelle de R6.
//!
//! `ISurveyMailer::expedier` ne rendant pas de `Result`, il n'y a même rien à
//! propager : cf. `survey_mailer.rs`.
//!
//! # La journée de repos est refusée par le domaine, pas ici
//!
//! R2 vit dans `PresenceSurvey::ouvrir` depuis la carte 511. Ce use case relaie
//! son refus au lieu de le rejuger — comme `update_pools_settings` relaie
//! `InvalidPools`. Le garder aux deux étages ferait dire la règle deux fois, et
//! le jour où elle bouge l'un des deux resterait en arrière.

use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError,
};
use crate::app::competitions::domain::presence_survey::{
    AutoRemind, PresenceSurvey, Reponse, SurveyDeadline, SurveyId,
};
use crate::app::competitions::domain::presence_survey_repository_port::{
    IPresenceSurveyRepository, PresenceSurveyRepositoryError,
};
use crate::app::competitions::ports::{ICompetitionSpaceMemberPort, ITeamInfoPort};
use crate::app::competitions::use_cases::presences::survey_mailer::{
    CampagneAAnnoncer, EnvoiPresence, ISurveyMailer, RapportEnvoi,
};
use crate::app::competitions::use_cases::presences::survey_roster_service::{
    self, RosterDeCampagne,
};
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
use crate::app::shared_kernel::identity::ids::SpaceId;

#[derive(Debug)]
pub struct LaunchSurveyCommand {
    pub season_id: SeasonId,
    pub space_id: SpaceId,
    pub round_id: MatchId,
    pub deadline: SurveyDeadline,
    pub auto_remind: AutoRemind,
    /// Une **entrée**, jamais une lecture d'horloge : c'est ce qui rend le use
    /// case testable sans attendre le lendemain. Convention déjà posée par
    /// `send_due_notifications_use_case`.
    pub aujourd_hui: DateString,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LaunchOutcome {
    pub destinataires: usize,
    /// R3 — ce que l'écran annonce : « 2 coachs sans adresse connue ». Un compte,
    /// pas un refus.
    pub sans_adresse: usize,
    pub envoyes: usize,
    pub echecs: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LaunchSurveyError {
    RoundNotFound,
    /// R2 — une campagne vit déjà sur cette journée.
    ///
    /// **`Exists` et non `Open`** : une campagne close occupe la journée tout
    /// autant, et elle se **rouvre** au lieu de se relancer. Nommer l'erreur
    /// d'après l'ouverture ferait croire qu'une campagne close peut être
    /// relancée, ce qui perdrait ses réponses.
    SurveyAlreadyExists,
    /// Le refus vient du domaine — journée de repos (R2), date d'ouverture
    /// invalide. Relayé tel quel : `DomainError` sait s'afficher.
    Refus(DomainError),
    Database(String),
}

impl From<PresenceSurveyRepositoryError> for LaunchSurveyError {
    fn from(e: PresenceSurveyRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<MatchDayRepositoryError> for LaunchSurveyError {
    fn from(e: MatchDayRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

pub struct LaunchDeps<'a> {
    pub survey_repo: &'a dyn IPresenceSurveyRepository,
    pub match_day_repo: &'a dyn IMatchDayRepository,
    pub teams: &'a dyn ITeamInfoPort,
    pub members: &'a dyn ICompetitionSpaceMemberPort,
    pub mailer: &'a dyn ISurveyMailer,
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: LaunchSurveyCommand,
    deps: LaunchDeps<'_>,
) -> Result<LaunchOutcome, LaunchSurveyError> {
    let round = deps
        .match_day_repo
        .find_by_id(&cmd.round_id.to_string())
        .await?
        .ok_or(LaunchSurveyError::RoundNotFound)?;

    if deps
        .survey_repo
        .find_by_round(&cmd.round_id.to_string())
        .await?
        .is_some()
    {
        return Err(LaunchSurveyError::SurveyAlreadyExists);
    }

    let roster = charger_le_roster(&cmd, &deps).await?;
    let survey = PresenceSurvey::ouvrir(
        SurveyId::new(),
        cmd.season_id,
        &round,
        &roster.destinataires(),
        cmd.deadline.clone(),
        cmd.auto_remind,
        &cmd.aujourd_hui,
    )
    .map_err(LaunchSurveyError::Refus)?;

    deps.survey_repo.save(&survey).await?;

    let rapport = expedier(&deps, &survey, &roster, round.name.as_ref()).await;
    Ok(LaunchOutcome {
        destinataires: roster.equipes().len(),
        sans_adresse: roster.sans_adresse(),
        envoyes: rapport.envoyes,
        echecs: rapport.echecs,
    })
}

async fn charger_le_roster(
    cmd: &LaunchSurveyCommand,
    deps: &LaunchDeps<'_>,
) -> Result<RosterDeCampagne, LaunchSurveyError> {
    survey_roster_service::charger(
        &cmd.season_id.to_string(),
        &cmd.space_id,
        deps.teams,
        deps.members,
    )
    .await
    .map_err(LaunchSurveyError::Database)
}

/// **Après la persistance, et sans pouvoir la défaire** — R20.
pub(super) async fn expedier(
    deps: &LaunchDeps<'_>,
    survey: &PresenceSurvey,
    roster: &RosterDeCampagne,
    round_name: &str,
) -> RapportEnvoi {
    let campagne = CampagneAAnnoncer {
        round_name: round_name.to_string(),
        deadline: survey.deadline().clone(),
    };
    let envois = envois_pour(roster, survey.reponses());
    deps.mailer.expedier(&campagne, &envois).await
}

/// Les envois possibles : ceux dont le coach a une adresse connue.
///
/// R3 — les autres ne sont pas une erreur, ils sont **comptés** par
/// `sans_adresse()`. Leur équipe est dans la campagne, elle attend simplement
/// que l'organisateur la joigne autrement.
pub(super) fn envois_pour(roster: &RosterDeCampagne, reponses: &[Reponse]) -> Vec<EnvoiPresence> {
    reponses
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
    use crate::app::competitions::use_cases::presences::test_doubles::*;
    use crate::app::shared_kernel::identity::ids::CoachId;

    fn commande(round: &MatchId, saison: SeasonId) -> LaunchSurveyCommand {
        LaunchSurveyCommand {
            season_id: saison,
            space_id: SpaceId::new(),
            round_id: *round,
            deadline: echeance("2026-10-10"),
            auto_remind: AutoRemind::new(true),
            aujourd_hui: date("2026-10-01"),
        }
    }

    #[tokio::test]
    async fn une_journee_de_repos_est_refusee_par_le_domaine() {
        let round = MatchId::new();
        let coach = CoachId::new();
        let repos = journee(round, MatchDayType::Rest, vec![]);
        let mailer = FauxMailer::qui_marche();
        let depot = FauxSurveyRepo::vide();

        let refus = execute(
            commande(&round, SeasonId::new()),
            LaunchDeps {
                survey_repo: &depot,
                match_day_repo: &FauxJournees(Some(repos)),
                teams: &FauxTeams(vec![equipe("Les Uns", &coach, "Alpha")]),
                members: &FauxMembres(vec![membre(&coach, "Alpha", "a@example.test")]),
                mailer: &mailer,
            },
        )
        .await;

        assert_eq!(
            refus.unwrap_err(),
            LaunchSurveyError::Refus(DomainError::SurveyOnRestDay),
            "R2 vit dans l'agrégat, le use case relaie son refus"
        );
        assert_eq!(depot.ecritures(), 0, "rien n'est persisté");
    }

    /// R2 — une campagne close occupe la journée tout autant : elle se rouvre, elle
    /// ne se relance pas. Relancer perdrait ses réponses.
    #[tokio::test]
    async fn une_seconde_campagne_sur_la_meme_journee_est_refusee() {
        let round = MatchId::new();
        let jour = journee(round, MatchDayType::FixedDate, vec![]);
        let deja = campagne_ouverte(&jour, &[]);
        let mailer = FauxMailer::qui_marche();

        let refus = execute(
            commande(&round, SeasonId::new()),
            LaunchDeps {
                survey_repo: &FauxSurveyRepo::avec(deja),
                match_day_repo: &FauxJournees(Some(jour)),
                teams: &FauxTeams(vec![]),
                members: &FauxMembres(vec![]),
                mailer: &mailer,
            },
        )
        .await;

        assert_eq!(refus.unwrap_err(), LaunchSurveyError::SurveyAlreadyExists);
    }

    // ── R20 — persister avant d'expédier ─────────────────────────────────────

    /// L'entrée de R20 : le serveur de messagerie est en panne, et la campagne
    /// existe quand même. Sans cette règle, l'onglet entier — saisie manuelle,
    /// tirage, validation — dépendrait d'un service dont il n'a pas besoin.
    #[tokio::test]
    async fn un_mailer_en_panne_n_empeche_pas_l_ouverture() {
        let round = MatchId::new();
        let coach = CoachId::new();
        let jour = journee(round, MatchDayType::FixedDate, vec![]);
        let depot = FauxSurveyRepo::vide();
        let mailer = FauxMailer::en_panne();

        let issue = execute(
            commande(&round, SeasonId::new()),
            LaunchDeps {
                survey_repo: &depot,
                match_day_repo: &FauxJournees(Some(jour)),
                teams: &FauxTeams(vec![equipe("Les Uns", &coach, "Alpha")]),
                members: &FauxMembres(vec![membre(&coach, "Alpha", "a@example.test")]),
                mailer: &mailer,
            },
        )
        .await
        .expect("R20 — l'échec d'envoi est journalisé, pas propagé");

        assert_eq!(issue.envoyes, 0);
        assert_eq!(issue.echecs, 1, "l'échec est dit, pas tu");
        let ecrite = depot.derniere_ecrite().expect("la campagne est persistée");
        assert_eq!(ecrite.reponses().len(), 1);
    }

    // ── R3 — un coach sans adresse n'empêche pas le lancement ────────────────

    #[tokio::test]
    async fn les_sans_adresse_entrent_dans_la_campagne_et_sont_comptes() {
        let round = MatchId::new();
        let (joignable, muet) = (CoachId::new(), CoachId::new());
        let jour = journee(round, MatchDayType::FixedDate, vec![]);
        let depot = FauxSurveyRepo::vide();
        let mailer = FauxMailer::qui_marche();

        let issue = execute(
            commande(&round, SeasonId::new()),
            LaunchDeps {
                survey_repo: &depot,
                match_day_repo: &FauxJournees(Some(jour)),
                teams: &FauxTeams(vec![
                    equipe("Les Joignables", &joignable, "Alpha"),
                    equipe("Les Muets", &muet, "Beta"),
                ]),
                members: &FauxMembres(vec![membre(&joignable, "Alpha", "a@example.test")]),
                mailer: &mailer,
            },
        )
        .await
        .expect("R3 — la fiche incomplète d'un seul ne bloque pas la campagne");

        assert_eq!(issue.destinataires, 2);
        assert_eq!(issue.sans_adresse, 1);
        assert_eq!(
            issue.envoyes, 1,
            "un seul e-mail part : l'autre coach n'a pas d'adresse"
        );
        assert_eq!(mailer.envois().len(), 1);
        assert_eq!(mailer.envois()[0].team_name, "Les Joignables");
        assert_eq!(
            depot.derniere_ecrite().expect("persistée").reponses().len(),
            2,
            "les deux équipes sont dans la campagne, seul l'e-mail manque"
        );
    }

    /// Le jeton envoyé est celui que l'agrégat a engendré, et pas un autre : c'est
    /// lui qui fera l'autorisation (R7), donc un jeton refabriqué au moment de
    /// l'envoi ne répondrait à rien.
    #[tokio::test]
    async fn le_jeton_expedie_est_celui_de_la_reponse() {
        let round = MatchId::new();
        let coach = CoachId::new();
        let jour = journee(round, MatchDayType::FixedDate, vec![]);
        let depot = FauxSurveyRepo::vide();
        let mailer = FauxMailer::qui_marche();

        execute(
            commande(&round, SeasonId::new()),
            LaunchDeps {
                survey_repo: &depot,
                match_day_repo: &FauxJournees(Some(jour)),
                teams: &FauxTeams(vec![equipe("Les Uns", &coach, "Alpha")]),
                members: &FauxMembres(vec![membre(&coach, "Alpha", "a@example.test")]),
                mailer: &mailer,
            },
        )
        .await
        .expect("lancement");

        let persistee = depot.derniere_ecrite().expect("persistée");
        assert_eq!(
            mailer.envois()[0].token,
            *persistee.reponses()[0].token(),
            "le jeton voyage depuis l'agrégat, il ne se refabrique pas"
        );
    }
}

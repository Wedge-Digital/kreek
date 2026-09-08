//! Poser une présence — **le point d'écriture des trois chemins**.
//!
//! # Un seul use case, et non un par chemin d'entrée
//!
//! L'organisateur depuis les boutons de la carte, le coach depuis son jeton,
//! le coach connecté depuis l'encart : trois entrées, une écriture. R1 porte sur
//! l'équipe et jamais sur le coach, donc rien ne distingue ces chemins une fois
//! l'autorisation faite — et un use case par chemin aurait produit trois endroits
//! où tenir R6 et R13.
//!
//! # R28 — l'appelant dit qui répond, l'agrégat vérifie
//!
//! `par` est un **champ de la commande**, jamais une déduction du handler. Si
//! l'auteur se déduisait de la route, la page publique et l'encart devraient
//! chacun le reconstituer, et R6 tiendrait à trois endroits au lieu d'un.
//!
//! Le use case ne contrôle rien : il transmet, et l'agrégat refuse un `Coach(id)`
//! qui ne correspond pas au `coach_id` de la réponse. C'est « le use case fournit
//! les faits, le domaine décide », appliqué à l'identité.
//!
//! # R13 par le port, jamais par la projection locale
//!
//! `competitions` porte pourtant `home_score` et `away_score` dans ses propres
//! DTO de journée, et la tentation est de lire ce qu'on a sous la main. Mais
//! cette projection est alimentée par un app event, donc **en retard d'un
//! battement**, et R13 est un garde-fou bloquant : la fraîcheur y est critique.
//! C'est le critère du `CLAUDE.md` — consultation bloquante, port synchrone,
//! jamais cache local.
//!
//! # Il ne répare rien
//!
//! Quand la journée est déjà appariée, il enregistre et **signale** qu'une
//! rencontre est à refaire. C'est la carte 518 qui répare, sur décision de
//! l'organisateur : la maquette montre une proposition qu'on valide, jamais un
//! fait accompli.

use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError,
};
use crate::app::competitions::domain::presence_survey::{EffetReponse, Repondant, Venue};
use crate::app::competitions::domain::presence_survey_repository_port::{
    IPresenceSurveyRepository, PresenceSurveyRepositoryError,
};
use crate::app::competitions::ports::IMatchReportStatusPort;
use crate::app::competitions::use_cases::presences::etat_journee::etat_de_la_journee;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;

#[derive(Debug)]
pub struct RecordAnswerCommand {
    pub round_id: MatchId,
    pub team_id: TeamId,
    pub venue: Venue,
    /// Ce qui autorise la réponse (R28). L'organisateur passe
    /// `Organisateur(id du connecté)`, la route publique `Jeton`, l'encart
    /// `Coach(id du connecté)`.
    pub par: Repondant,
    pub aujourd_hui: DateString,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AnswerOutcome {
    /// R12 — la rencontre que la défection rend caduque, s'il y en a une.
    ///
    /// **L'identifiant et non un booléen** : `EffetReponse` le porte déjà, et le
    /// réduire obligerait la carte 518 à redécouvrir *quelle* rencontre refaire.
    ///
    /// Toujours `None` sur une arrivée tardive (R16) : un arrivant n'apparaît dans
    /// aucune rencontre, donc aucune n'est à refaire. C'est `desaccord` qui le
    /// fait voir, et l'écran le recalcule à chaque affichage.
    pub rencontre_a_refaire: Option<PairingId>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RecordAnswerError {
    SurveyNotFound,
    RoundNotFound,
    /// R19, R13, R21 et R28 — relayés du domaine, jamais rejugés ici.
    /// `DomainError` sait s'afficher : son message sert de corps de réponse.
    Refus(DomainError),
    Database(String),
}

impl From<PresenceSurveyRepositoryError> for RecordAnswerError {
    fn from(e: PresenceSurveyRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<MatchDayRepositoryError> for RecordAnswerError {
    fn from(e: MatchDayRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RecordAnswerCommand,
    survey_repo: &dyn IPresenceSurveyRepository,
    match_day_repo: &dyn IMatchDayRepository,
    match_report_port: &dyn IMatchReportStatusPort,
) -> Result<AnswerOutcome, RecordAnswerError> {
    let mut survey = survey_repo
        .find_by_round(&cmd.round_id.to_string())
        .await?
        .ok_or(RecordAnswerError::SurveyNotFound)?;

    let round = match_day_repo
        .find_by_id(&cmd.round_id.to_string())
        .await?
        .ok_or(RecordAnswerError::RoundNotFound)?;
    let journee = etat_de_la_journee(&round, match_report_port)
        .await
        .map_err(RecordAnswerError::Database)?;

    let effet = survey
        .enregistrer(&cmd.team_id, cmd.venue, cmd.par, &journee, &cmd.aujourd_hui)
        .map_err(RecordAnswerError::Refus)?;

    survey_repo.save(&survey).await?;
    Ok(AnswerOutcome {
        rencontre_a_refaire: match effet {
            EffetReponse::Enregistree => None,
            EffetReponse::EnregistreeRencontreARefaire { pairing } => Some(pairing),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{MatchDay, MatchDayType, Pairing};
    use crate::app::competitions::domain::presence_survey::{
        Destinataire, Presence, PresenceSurvey,
    };
    use crate::app::competitions::use_cases::presences::test_doubles::*;
    use crate::app::shared_kernel::bloodbowl::ids::PairingId;
    use crate::app::shared_kernel::identity::ids::CoachId;

    struct Campagne {
        round: MatchDay,
        survey: PresenceSurvey,
        equipes: Vec<TeamId>,
        coachs: Vec<CoachId>,
    }

    /// Une campagne de `n` équipes sur une journée vide.
    fn campagne(n: usize) -> Campagne {
        let round = journee(MatchId::new(), MatchDayType::FixedDate, vec![]);
        batir(round, n)
    }

    /// La même, mais la journée porte les rencontres données — R24 : elles
    /// appartiennent à la journée, la campagne ne les possède pas.
    fn campagne_appariee(n: usize, paires: &[(usize, usize)]) -> Campagne {
        let vide = campagne(n);
        let pairings: Vec<Pairing> = paires
            .iter()
            .map(|&(a, b)| Pairing {
                id: PairingId::new(),
                home_team_id: vide.equipes[a],
                away_team_id: vide.equipes[b],
            })
            .collect();
        let round = MatchDay {
            pairings,
            ..vide.round.clone()
        };
        Campagne { round, ..vide }
    }

    fn batir(round: MatchDay, n: usize) -> Campagne {
        let equipes: Vec<TeamId> = (0..n).map(|_| TeamId::new()).collect();
        let coachs: Vec<CoachId> = (0..n).map(|_| CoachId::new()).collect();
        let destinataires: Vec<Destinataire> = equipes
            .iter()
            .zip(&coachs)
            .map(|(t, c)| Destinataire {
                team_id: *t,
                coach_id: *c,
            })
            .collect();
        let survey = campagne_ouverte(&round, &destinataires);
        Campagne {
            round,
            survey,
            equipes,
            coachs,
        }
    }

    fn commande(
        c: &Campagne,
        rang: usize,
        venue: Venue,
        par: Repondant,
        le: &str,
    ) -> RecordAnswerCommand {
        RecordAnswerCommand {
            round_id: c.round.id,
            team_id: c.equipes[rang],
            venue,
            par,
            aujourd_hui: date(le),
        }
    }

    async fn poser(
        c: Campagne,
        cmd: RecordAnswerCommand,
        figee: bool,
    ) -> (Result<AnswerOutcome, RecordAnswerError>, FauxSurveyRepo) {
        let depot = FauxSurveyRepo::avec(c.survey);
        let issue = execute(
            cmd,
            &depot,
            &FauxJournees(Some(c.round)),
            &FauxRapports(figee),
        )
        .await;
        (issue, depot)
    }

    // ── R6 — l'organisateur pose une réponse, et cela se voit ────────────────

    #[tokio::test]
    async fn l_organisateur_pose_une_reponse_et_son_identifiant_est_garde() {
        let c = campagne(3);
        let admin = CoachId::new();
        let equipe = c.equipes[1];
        let cmd = commande(
            &c,
            1,
            Venue::Presente,
            Repondant::Organisateur(admin),
            "2026-10-05",
        );

        let (issue, depot) = poser(c, cmd, false).await;

        assert_eq!(issue.unwrap().rencontre_a_refaire, None);
        let posee = depot.derniere_ecrite().expect("persistée");
        let reponse = posee.reponse_de(&equipe).expect("réponse");
        assert!(
            matches!(reponse.presence(), Presence::Declaree { par: Repondant::Organisateur(id), .. } if id == &admin),
            "R6 — « qui a dit qu'il venait » se pose après le tirage, pas avant"
        );
    }

    // ── R12 — la sortie signale la rencontre à refaire ───────────────────────

    #[tokio::test]
    async fn un_desistement_apres_le_tirage_rend_l_identifiant_de_sa_rencontre() {
        let c = campagne_appariee(4, &[(0, 1), (2, 3)]);
        let touchee = c.round.pairings[1].id;
        // Les quatre sont présentes, puis la dernière se désiste.
        let mut survey = c.survey.clone();
        for rang in 0..4 {
            survey
                .enregistrer(
                    &c.equipes[rang],
                    Venue::Presente,
                    Repondant::Jeton,
                    &etat_vierge(),
                    &date("2026-10-04"),
                )
                .unwrap();
        }
        let c = Campagne { survey, ..c };
        let cmd = commande(&c, 3, Venue::Absente, Repondant::Jeton, "2026-10-05");

        let (issue, _) = poser(c, cmd, false).await;

        assert_eq!(
            issue.unwrap().rencontre_a_refaire,
            Some(touchee),
            "R12 — elle signale, et la carte 518 réparera sur décision"
        );
    }

    /// R16 — une arrivée tardive n'a aucune rencontre à refaire : elle rejoint le
    /// vivier des orphelins, que `desaccord` fait voir à l'affichage.
    #[tokio::test]
    async fn une_arrivee_tardive_ne_signale_aucune_rencontre() {
        let c = campagne_appariee(3, &[(0, 1)]);
        let cmd = commande(&c, 2, Venue::Presente, Repondant::Jeton, "2026-10-05");

        let (issue, depot) = poser(c, cmd, false).await;

        assert_eq!(issue.unwrap().rencontre_a_refaire, None);
        assert_eq!(depot.ecritures(), 1, "elle est bien enregistrée");
    }

    // ── R13 — une journée figée par un rapport ne bouge plus ─────────────────

    #[tokio::test]
    async fn une_journee_figee_est_refusee_et_rien_n_est_persiste() {
        let c = campagne_appariee(2, &[(0, 1)]);
        let cmd = commande(&c, 0, Venue::Absente, Repondant::Jeton, "2026-10-05");

        let (issue, depot) = poser(c, cmd, true).await;

        assert_eq!(
            issue.unwrap_err(),
            RecordAnswerError::Refus(DomainError::RoundFrozenByReport)
        );
        assert_eq!(depot.ecritures(), 0);
    }

    // ── R19 — une équipe hors campagne ───────────────────────────────────────

    #[tokio::test]
    async fn une_equipe_hors_campagne_est_refusee() {
        let c = campagne(3);
        let etrangere = TeamId::new();
        let cmd = RecordAnswerCommand {
            round_id: c.round.id,
            team_id: etrangere,
            venue: Venue::Presente,
            par: Repondant::Organisateur(CoachId::new()),
            aujourd_hui: date("2026-10-05"),
        };

        let (issue, depot) = poser(c, cmd, false).await;

        assert_eq!(
            issue.unwrap_err(),
            RecordAnswerError::Refus(DomainError::TeamNotInSurvey {
                team: etrangere.to_string()
            })
        );
        assert_eq!(depot.ecritures(), 0);
    }

    // ── R21 — la clôture arrête le coach, jamais l'organisateur ──────────────

    #[tokio::test]
    async fn sur_une_campagne_close_le_coach_est_refuse_et_l_organisateur_passe() {
        // Échéance au 10 : le 11, la campagne est close par R23.
        let c = campagne(3);
        let sien = c.coachs[0];
        let cmd = commande(&c, 0, Venue::Presente, Repondant::Coach(sien), "2026-10-11");
        let (refus, depot) = poser(c, cmd, false).await;
        assert_eq!(
            refus.unwrap_err(),
            RecordAnswerError::Refus(DomainError::SurveyClosedForCoach)
        );
        assert_eq!(depot.ecritures(), 0);

        let c = campagne(3);
        let cmd = commande(
            &c,
            0,
            Venue::Presente,
            Repondant::Organisateur(CoachId::new()),
            "2026-10-11",
        );
        let (rattrapage, depot) = poser(c, cmd, false).await;
        assert!(
            rattrapage.is_ok(),
            "R6 — c'est l'organisateur qui rattrape le coup de fil reçu après l'échéance"
        );
        assert_eq!(depot.ecritures(), 1);
    }

    // ── R7 / R28 — le jeton passe par ce même use case ───────────────────────

    /// Le chemin de l'unité 2 : le jeton *est* l'autorisation, et le use case ne
    /// lui oppose aucun propriétaire — ce serait comparer la réponse à elle-même.
    #[tokio::test]
    async fn le_jeton_n_est_pas_confronte_a_un_proprietaire() {
        let c = campagne(3);
        let cmd = commande(&c, 2, Venue::Absente, Repondant::Jeton, "2026-10-05");

        let (issue, depot) = poser(c, cmd, false).await;

        assert!(issue.is_ok());
        assert_eq!(depot.derniere_ecrite().unwrap().compte_absents(), 1);
    }

    #[tokio::test]
    async fn un_coach_ne_repond_pas_pour_l_equipe_d_un_autre() {
        let c = campagne(3);
        let equipe = c.equipes[0];
        let voisin = c.coachs[1];
        let cmd = commande(
            &c,
            0,
            Venue::Presente,
            Repondant::Coach(voisin),
            "2026-10-05",
        );

        let (issue, depot) = poser(c, cmd, false).await;

        assert_eq!(
            issue.unwrap_err(),
            RecordAnswerError::Refus(DomainError::TeamNotOwnedByCoach {
                team: equipe.to_string()
            })
        );
        assert_eq!(depot.ecritures(), 0);
    }

    #[tokio::test]
    async fn une_journee_sans_campagne_ne_recoit_aucune_reponse() {
        let c = campagne(3);
        let cmd = commande(&c, 0, Venue::Presente, Repondant::Jeton, "2026-10-05");

        let issue = execute(
            cmd,
            &FauxSurveyRepo::vide(),
            &FauxJournees(Some(c.round)),
            &FauxRapports(false),
        )
        .await;

        assert_eq!(issue.unwrap_err(), RecordAnswerError::SurveyNotFound);
    }
}

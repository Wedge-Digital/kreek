//! Écrire au calendrier le tirage que l'organisateur a validé.
//!
//! # Le client renvoie les paires, jamais les étiquettes
//!
//! `ProposedPairing` porte un `Historique` — « 1re rencontre » ou « 2e rencontre ·
//! J1 » — qui est **dérivé** de la saison. Le laisser entrer par la commande
//! permettrait à une proposition falsifiée d'afficher « inédite » sur une
//! revanche : le libellé viendrait du navigateur au lieu des journées.
//!
//! La commande ne porte donc que les couples d'équipes et l'exemptée, et le use
//! case recalcule l'historique depuis les journées avant de soumettre le tout à
//! l'agrégat.
//!
//! # L'émission vient après le commit, jamais dedans
//!
//! Un listener qui réagit à un `PairingCreated` dont la transaction est ensuite
//! annulée aurait travaillé sur un fait qui n'a pas eu lieu, et rien ne le lui
//! dirait. L'ordre inverse paraît plus naturel — « tout dans la même unité » — et
//! c'est le piège.
//!
//! # Deux transactions, et c'est assumé
//!
//! `save_pairings` écrit la journée, `save` écrit la campagne : deux agrégats,
//! deux dépôts, deux transactions. Une panne entre les deux laisse la journée
//! appariée et la campagne se croyant vierge.
//!
//! **Ce n'est pas la règle « projection dans la même transaction »** — il ne
//! s'agit pas d'un événement et de sa projection, mais de deux agrégats. Et la
//! conséquence est bénigne : `desaccord` lit l'appariement **sur la journée**
//! (R24), donc l'écran reste juste. Seule l'exemptée apparaîtrait à tort comme
//! orpheline, et re-valider ou réparer le corrige.
//!
//! Bâtir un dépôt qui écrit les deux tables coûterait un objet qui connaît deux
//! agrégats, pour un défaut qui se voit et se répare.

use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError, NewPairingProjection,
};
use crate::app::competitions::domain::presence_survey::PresenceSurvey;
use crate::app::competitions::domain::presence_survey_repository_port::{
    IPresenceSurveyRepository, PresenceSurveyRepositoryError,
};
use crate::app::competitions::domain::tirage::{DrawProposal, ProposedPairing, RencontresJouees};
use crate::app::competitions::ports::{ITeamInfoPort, TeamInfoDto};
use crate::app::competitions::use_cases::admin::team_enrollment::{
    build_new_pairing_projection, load_enrolled_teams,
};
use crate::app::competitions::use_cases::appariement_ecrit::{
    emettre_pairing_created, OuEstEcrite,
};
use crate::app::competitions::use_cases::entree_du_tirage::{build_historique, build_interdites};
use crate::app::competitions::use_cases::presences::draw_pairings_use_case::ids_inscrits;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::common::services::event_bus::event_bus::EventBus;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ConfirmDrawCommand {
    pub round_id: MatchId,
    pub season_id: SeasonId,
    pub competition_id: String,
    pub space_id: SpaceId,
    /// Les couples validés par l'organisateur — **sans leur étiquette**, qui se
    /// recalcule.
    pub rencontres: Vec<(TeamId, TeamId)>,
    pub exemptee: Option<TeamId>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConfirmOutcome {
    pub ecrites: usize,
    pub exemptee: Option<TeamId>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfirmDrawError {
    SurveyNotFound,
    RoundNotFound,
    /// R11, constatée avant le calcul **et** sous le verrou de `save_pairings`.
    PairingsAlreadyExist,
    /// R5, R10, R18 et R22 — le refus vient de l'agrégat, relayé tel quel.
    Refus(DomainError),
    Database(String),
}

impl From<PresenceSurveyRepositoryError> for ConfirmDrawError {
    fn from(e: PresenceSurveyRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<MatchDayRepositoryError> for ConfirmDrawError {
    fn from(e: MatchDayRepositoryError) -> Self {
        match e {
            MatchDayRepositoryError::PairingsAlreadyExist => Self::PairingsAlreadyExist,
            autre => Self::Database(autre.to_string()),
        }
    }
}

pub struct ConfirmDeps<'a> {
    pub survey_repo: &'a dyn IPresenceSurveyRepository,
    pub match_day_repo: &'a dyn IMatchDayRepository,
    pub teams: &'a dyn ITeamInfoPort,
    pub event_bus: &'a EventBus,
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ConfirmDrawCommand,
    deps: ConfirmDeps<'_>,
) -> Result<ConfirmOutcome, ConfirmDrawError> {
    let mut survey = deps
        .survey_repo
        .find_by_round(&cmd.round_id.to_string())
        .await?
        .ok_or(ConfirmDrawError::SurveyNotFound)?;
    let round = deps
        .match_day_repo
        .find_by_id(&cmd.round_id.to_string())
        .await?
        .ok_or(ConfirmDrawError::RoundNotFound)?;
    if !round.pairings.is_empty() {
        return Err(ConfirmDrawError::PairingsAlreadyExist);
    }

    let team_display = load_enrolled_teams(&cmd.season_id.to_string(), deps.teams)
        .await
        .map_err(ConfirmDrawError::Database)?;
    let proposition = reconstituer(&cmd, deps.match_day_repo).await?;

    let equipes: Vec<TeamId> = cmd.rencontres.iter().flat_map(|&(a, b)| [a, b]).collect();
    survey
        .valider_proposition(
            &proposition,
            &ids_inscrits(&team_display),
            &build_interdites(&equipes, &team_display),
        )
        .map_err(ConfirmDrawError::Refus)?;

    let a_ecrire = batir(&cmd, &round, &team_display);
    deps.match_day_repo
        .save_pairings(&cmd.round_id.to_string(), &a_ecrire)
        .await?;

    annoncer(&cmd, &deps, &a_ecrire);
    survey.marquer_appariee(cmd.exemptee);
    deps.survey_repo.save(&survey).await?;

    Ok(ConfirmOutcome {
        ecrites: a_ecrire.len(),
        exemptee: cmd.exemptee,
    })
}

/// L'étiquette de chaque couple, **recalculée depuis les journées** : c'est la
/// seule façon qu'elle dise la vérité, la commande ne la portant pas.
async fn reconstituer(
    cmd: &ConfirmDrawCommand,
    match_day_repo: &dyn IMatchDayRepository,
) -> Result<DrawProposal, ConfirmDrawError> {
    let jours = match_day_repo
        .find_by_season(&cmd.season_id.to_string())
        .await?;
    let historique: RencontresJouees = build_historique(&jours, &cmd.round_id.to_string());

    Ok(DrawProposal {
        rencontres: cmd
            .rencontres
            .iter()
            .map(|&(home, away)| ProposedPairing {
                home,
                away,
                historique: historique.historique(&home, &away),
            })
            .collect(),
        exemptee: cmd.exemptee,
        ..Default::default()
    })
}

fn batir(
    cmd: &ConfirmDrawCommand,
    round: &MatchDay,
    team_display: &HashMap<String, TeamInfoDto>,
) -> Vec<(Pairing, NewPairingProjection)> {
    cmd.rencontres
        .iter()
        .map(|&(home, away)| {
            let projection = build_new_pairing_projection(
                &home.to_string(),
                &away.to_string(),
                &cmd.season_id.to_string(),
                round,
                team_display,
            );
            let pairing = Pairing {
                id: PairingId::new(),
                home_team_id: home,
                away_team_id: away,
            };
            (pairing, projection)
        })
        .collect()
}

/// **Après le commit.** L'appel est volontairement après `save_pairings`, et un
/// test le vérifie : une écriture en échec n'annonce rien.
fn annoncer(
    cmd: &ConfirmDrawCommand,
    deps: &ConfirmDeps<'_>,
    a_ecrire: &[(Pairing, NewPairingProjection)],
) {
    for (pairing, projection) in a_ecrire {
        emettre_pairing_created(
            deps.event_bus,
            OuEstEcrite {
                competition_id: &cmd.competition_id,
                space_id: &cmd.space_id.to_string(),
                round_id: &cmd.round_id.to_string(),
            },
            pairing,
            projection,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::MatchDayType;
    use crate::app::competitions::domain::presence_survey::{
        Appariement, Destinataire, Repondant, Venue,
    };
    use crate::app::competitions::domain::tirage::Historique;
    use crate::app::competitions::use_cases::presences::test_doubles::*;
    use crate::app::shared_kernel::identity::ids::CoachId;
    use crate::common::services::event_bus::event_bus::new_bus;

    struct Plateau {
        round: MatchDay,
        survey: PresenceSurvey,
        equipes: Vec<TeamInfoDto>,
        ids: Vec<TeamId>,
    }

    fn plateau(n: usize) -> Plateau {
        let round = journee(MatchId::new(), MatchDayType::FixedDate, vec![]);
        let equipes: Vec<TeamInfoDto> = (0..n)
            .map(|i| {
                equipe(
                    &format!("Équipe {i}"),
                    &CoachId::new(),
                    &format!("Coach {i}"),
                )
            })
            .collect();
        let ids: Vec<TeamId> = equipes
            .iter()
            .map(|e| TeamId::try_new(&e.team_id).unwrap())
            .collect();
        let destinataires: Vec<Destinataire> = equipes
            .iter()
            .zip(&ids)
            .map(|(e, t)| Destinataire {
                team_id: *t,
                coach_id: CoachId::try_new(&e.coach_id).unwrap(),
            })
            .collect();
        let mut survey = campagne_ouverte(&round, &destinataires);
        for t in &ids {
            survey
                .enregistrer(
                    t,
                    Venue::Presente,
                    Repondant::Jeton,
                    &etat_vierge(),
                    &date("2026-10-04"),
                )
                .unwrap();
        }
        Plateau {
            round,
            survey,
            equipes,
            ids,
        }
    }

    fn commande(
        p: &Plateau,
        rencontres: Vec<(TeamId, TeamId)>,
        exemptee: Option<TeamId>,
    ) -> ConfirmDrawCommand {
        ConfirmDrawCommand {
            round_id: p.round.id,
            season_id: SeasonId::new(),
            competition_id: "c1".to_string(),
            space_id: SpaceId::new(),
            rencontres,
            exemptee,
        }
    }

    #[tokio::test]
    async fn une_proposition_juste_s_ecrit_et_s_annonce() {
        let p = plateau(4);
        let depot = FauxSurveyRepo::avec(p.survey.clone());
        let journees = FauxJournees::avec(p.round.clone());
        let bus = new_bus();
        let mut abonne = bus.subscribe();

        let issue = execute(
            commande(&p, vec![(p.ids[0], p.ids[1]), (p.ids[2], p.ids[3])], None),
            ConfirmDeps {
                survey_repo: &depot,
                match_day_repo: &journees,
                teams: &FauxTeams(p.equipes.clone()),
                event_bus: &bus,
            },
        )
        .await
        .expect("validation");

        assert_eq!(issue.ecrites, 2);
        assert_eq!(journees.ecrits(), 2);
        assert!(abonne.try_recv().is_ok(), "un PairingCreated par rencontre");
        assert!(abonne.try_recv().is_ok());
        assert_eq!(
            depot.derniere_ecrite().expect("campagne").appariement(),
            &Appariement::Fait { exemptee: None }
        );
    }

    // ── R11 — la journée déjà appariée, aux deux étages ──────────────────────

    #[tokio::test]
    async fn une_journee_deja_appariee_est_refusee_avant_tout_calcul() {
        let p = plateau(4);
        let deja = journee(
            p.round.id,
            MatchDayType::FixedDate,
            vec![Pairing {
                id: PairingId::new(),
                home_team_id: p.ids[0],
                away_team_id: p.ids[1],
            }],
        );
        let bus = new_bus();

        let refus = execute(
            commande(&p, vec![(p.ids[0], p.ids[1])], None),
            ConfirmDeps {
                survey_repo: &FauxSurveyRepo::avec(p.survey.clone()),
                match_day_repo: &FauxJournees::avec(deja),
                teams: &FauxTeams(p.equipes.clone()),
                event_bus: &bus,
            },
        )
        .await;

        assert_eq!(refus.unwrap_err(), ConfirmDrawError::PairingsAlreadyExist);
    }

    /// Le verrou de `save_pairings` refuse la seconde de deux validations
    /// concurrentes — celle qui a franchi la garde avant que l'autre n'écrive.
    #[tokio::test]
    async fn le_refus_sous_le_verrou_remonte_comme_r11() {
        let p = plateau(4);
        let bus = new_bus();
        let mut abonne = bus.subscribe();
        let journees = FauxJournees::avec(p.round.clone()).dont_l_ecriture_echoue();

        let refus = execute(
            commande(&p, vec![(p.ids[0], p.ids[1])], None),
            ConfirmDeps {
                survey_repo: &FauxSurveyRepo::avec(p.survey.clone()),
                match_day_repo: &journees,
                teams: &FauxTeams(p.equipes.clone()),
                event_bus: &bus,
            },
        )
        .await;

        assert_eq!(refus.unwrap_err(), ConfirmDrawError::PairingsAlreadyExist);
        assert_eq!(journees.ecrits(), 0);
        assert!(
            abonne.try_recv().is_err(),
            "aucun PairingCreated quand l'écriture échoue — l'émission vient après le commit"
        );
    }

    // ── R22 — la validation revérifie, elle n'écrit pas sur parole ───────────

    /// Une proposition falsifiée : une équipe absente y est appariée. R5 la refuse
    /// à la validation, alors que l'aperçu ne l'avait jamais proposée.
    #[tokio::test]
    async fn une_proposition_falsifiee_est_refusee() {
        let round = journee(MatchId::new(), MatchDayType::FixedDate, vec![]);
        let equipes = vec![
            equipe("Présente", &CoachId::new(), "Alpha"),
            equipe("Silencieuse", &CoachId::new(), "Beta"),
        ];
        let ids: Vec<TeamId> = equipes
            .iter()
            .map(|e| TeamId::try_new(&e.team_id).unwrap())
            .collect();
        let destinataires: Vec<Destinataire> = equipes
            .iter()
            .zip(&ids)
            .map(|(e, t)| Destinataire {
                team_id: *t,
                coach_id: CoachId::try_new(&e.coach_id).unwrap(),
            })
            .collect();
        let mut survey = campagne_ouverte(&round, &destinataires);
        survey
            .enregistrer(
                &ids[0],
                Venue::Presente,
                Repondant::Jeton,
                &etat_vierge(),
                &date("2026-10-04"),
            )
            .unwrap();
        let journees = FauxJournees::avec(round.clone());
        let bus = new_bus();

        let refus = execute(
            ConfirmDrawCommand {
                round_id: round.id,
                season_id: SeasonId::new(),
                competition_id: "c1".to_string(),
                space_id: SpaceId::new(),
                rencontres: vec![(ids[0], ids[1])],
                exemptee: None,
            },
            ConfirmDeps {
                survey_repo: &FauxSurveyRepo::avec(survey),
                match_day_repo: &journees,
                teams: &FauxTeams(equipes),
                event_bus: &bus,
            },
        )
        .await;

        assert_eq!(
            refus.unwrap_err(),
            ConfirmDrawError::Refus(DomainError::TeamNotPresent {
                team: ids[1].to_string()
            })
        );
        assert_eq!(journees.ecrits(), 0);
    }

    // ── R18 — une désengagée est refusée à la validation ─────────────────────

    #[tokio::test]
    async fn une_desengagee_est_refusee_a_la_validation() {
        let p = plateau(3);
        let encore_inscrites: Vec<TeamInfoDto> = p.equipes[..2].to_vec();
        let bus = new_bus();

        let refus = execute(
            commande(&p, vec![(p.ids[0], p.ids[2])], None),
            ConfirmDeps {
                survey_repo: &FauxSurveyRepo::avec(p.survey.clone()),
                match_day_repo: &FauxJournees::avec(p.round.clone()),
                teams: &FauxTeams(encore_inscrites),
                event_bus: &bus,
            },
        )
        .await;

        assert_eq!(
            refus.unwrap_err(),
            ConfirmDrawError::Refus(DomainError::TeamNoLongerEnrolled {
                team: p.ids[2].to_string()
            })
        );
    }

    // ── L'étiquette se recalcule, elle ne se reçoit pas ──────────────────────

    /// La commande ne porte pas d'`Historique` : le laisser entrer permettrait à
    /// une proposition falsifiée d'afficher « inédite » sur une revanche. Ce test
    /// vérifie que le use case le retrouve depuis les journées de la saison.
    #[tokio::test]
    async fn l_historique_d_une_rencontre_se_recalcule_depuis_les_journees() {
        let p = plateau(4);
        let passee = journee(
            MatchId::new(),
            MatchDayType::FixedDate,
            vec![Pairing {
                id: PairingId::new(),
                home_team_id: p.ids[0],
                away_team_id: p.ids[1],
            }],
        );
        let journees =
            FauxJournees::avec(p.round.clone()).et_la_saison(vec![passee, p.round.clone()]);

        let prop = reconstituer(&commande(&p, vec![(p.ids[0], p.ids[1])], None), &journees)
            .await
            .expect("reconstitution");

        assert!(
            matches!(prop.rencontres[0].historique, Historique::Revanche { .. }),
            "la paire a déjà été programmée : l'étiquette doit le dire, quoi que \
             le navigateur ait renvoyé"
        );
    }

    #[tokio::test]
    async fn une_journee_sans_campagne_ne_se_valide_pas() {
        let p = plateau(4);
        let bus = new_bus();

        let refus = execute(
            commande(&p, vec![(p.ids[0], p.ids[1])], None),
            ConfirmDeps {
                survey_repo: &FauxSurveyRepo::vide(),
                match_day_repo: &FauxJournees::avec(p.round.clone()),
                teams: &FauxTeams(p.equipes.clone()),
                event_bus: &bus,
            },
        )
        .await;

        assert_eq!(refus.unwrap_err(), ConfirmDrawError::SurveyNotFound);
    }
}

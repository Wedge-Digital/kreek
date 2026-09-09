//! Écrire la réparation que l'organisateur a validée.
//!
//! # Les mêmes gardes que la validation initiale
//!
//! La proposition de réparation voyage par le client comme la proposition
//! initiale, donc elle revient modifiable : R22 la revérifie intégralement. Ce
//! n'est pas de la défiance — une réponse a pu changer entre l'affichage du
//! panneau et le clic.
//!
//! # `valider_proposition` reçoit **toute la journée**, pas la seule réparation
//!
//! Sa vérification de parité regarde les présents que rien n'apparie : lui donner
//! la seule rencontre réparée la ferait crier sur les quatre autres matchs, qui
//! sont légitimes et qu'on ne touche pas. Le use case reconstitue donc la journée
//! telle qu'elle sera **après** réparation.
//!
//! # `save_pairing` et non `save_pairings`
//!
//! La méthode plurielle refuse une journée qui porte déjà des appariements — R11,
//! constatée sous son verrou — et c'est exactement le cas ici : les autres matchs
//! ne bougent pas. La singulière « ne juge pas de l'état de la journée », ce que
//! son propre commentaire annonce.
//!
//! # L'exemptée enregistrée est celle qui a **finalement** eu lieu
//!
//! R9 croisée à R12 : quand l'exemptée reprend du service, elle n'a pas été
//! exemptée, et la compter comme telle la ferait passer devant à la journée
//! suivante pour une exemption qu'elle n'a pas subie. À l'inverse, l'adversaire
//! qui se retrouve sans match subit une exemption que R9 n'a pas choisie — on
//! l'enregistre parce qu'elle a eu lieu, pas parce qu'une règle l'a désignée.

use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError,
};
use crate::app::competitions::domain::presence_survey_repository_port::{
    IPresenceSurveyRepository, PresenceSurveyRepositoryError,
};
use crate::app::competitions::domain::tirage::{DrawProposal, ProposedPairing};
use crate::app::competitions::ports::{IMatchReportStatusPort, ITeamInfoPort, TeamInfoDto};
use crate::app::competitions::use_cases::admin::delete_pairing_use_case;
use crate::app::competitions::use_cases::admin::team_enrollment::{
    build_new_pairing_projection, load_enrolled_teams,
};
use crate::app::competitions::use_cases::appariement_ecrit::{
    emettre_pairing_created, OuEstEcrite,
};
use crate::app::competitions::use_cases::entree_du_tirage::{build_historique, build_interdites};
use crate::app::competitions::use_cases::presences::draw_pairings_use_case::ids_inscrits;
use crate::app::competitions::use_cases::presences::etat_journee::etat_de_la_journee;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::common::services::event_bus::event_bus::EventBus;
use std::collections::HashMap;

#[derive(Debug)]
pub struct RepairCommand {
    pub round_id: MatchId,
    pub season_id: SeasonId,
    pub competition_id: String,
    pub space_id: SpaceId,
    /// Les rencontres que la défection rend caduques. **Un `Vec`** : plusieurs
    /// défections peuvent s'accumuler avant que l'organisateur ne regarde.
    pub a_defaire: Vec<PairingId>,
    /// Ce que l'organisateur a validé — sans étiquette, qui se recalcule (517).
    pub rencontres: Vec<(TeamId, TeamId)>,
    pub exemptee: Option<TeamId>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RepairOutcome {
    pub defaites: usize,
    pub ecrites: usize,
    pub exemptee: Option<TeamId>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RepairError {
    SurveyNotFound,
    RoundNotFound,
    /// R13 — une journée dont un rapport est publié ne se répare pas.
    /// R5, R10, R18 et R22 — le refus vient de l'agrégat, relayé tel quel.
    Refus(DomainError),
    Database(String),
}

impl From<PresenceSurveyRepositoryError> for RepairError {
    fn from(e: PresenceSurveyRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<MatchDayRepositoryError> for RepairError {
    fn from(e: MatchDayRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

pub struct RepairDeps<'a> {
    pub survey_repo: &'a dyn IPresenceSurveyRepository,
    pub match_day_repo: &'a dyn IMatchDayRepository,
    pub report_status: &'a dyn IMatchReportStatusPort,
    pub teams: &'a dyn ITeamInfoPort,
    pub event_bus: &'a EventBus,
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RepairCommand,
    deps: RepairDeps<'_>,
) -> Result<RepairOutcome, RepairError> {
    let mut survey = deps
        .survey_repo
        .find_by_round(&cmd.round_id.to_string())
        .await?
        .ok_or(RepairError::SurveyNotFound)?;
    let round = deps
        .match_day_repo
        .find_by_id(&cmd.round_id.to_string())
        .await?
        .ok_or(RepairError::RoundNotFound)?;

    let journee = etat_de_la_journee(&round, deps.report_status)
        .await
        .map_err(RepairError::Database)?;
    if journee.figee {
        return Err(RepairError::Refus(DomainError::RoundFrozenByReport));
    }

    let team_display = load_enrolled_teams(&cmd.season_id.to_string(), deps.teams)
        .await
        .map_err(RepairError::Database)?;
    let apres = journee_apres_reparation(&cmd, &round, deps.match_day_repo).await?;
    let equipes: Vec<TeamId> = apres
        .rencontres
        .iter()
        .flat_map(|r| [r.home, r.away])
        .collect();

    survey
        .valider_proposition(
            &apres,
            &ids_inscrits(&team_display),
            &build_interdites(&equipes, &team_display),
        )
        .map_err(RepairError::Refus)?;

    defaire(&cmd, &deps).await?;
    ecrire(&cmd, &deps, &round, &team_display).await?;

    survey.marquer_appariee(cmd.exemptee);
    deps.survey_repo.save(&survey).await?;

    Ok(RepairOutcome {
        defaites: cmd.a_defaire.len(),
        ecrites: cmd.rencontres.len(),
        exemptee: cmd.exemptee,
    })
}

/// La journée telle qu'elle sera : ce qu'on garde, plus ce qu'on écrit.
///
/// C'est cette vue-là que `valider_proposition` doit juger — R22 regarde la
/// parité sur l'ensemble, pas sur un fragment.
async fn journee_apres_reparation(
    cmd: &RepairCommand,
    round: &MatchDay,
    match_day_repo: &dyn IMatchDayRepository,
) -> Result<DrawProposal, RepairError> {
    let jours = match_day_repo
        .find_by_season(&cmd.season_id.to_string())
        .await?;
    let historique = build_historique(&jours, &cmd.round_id.to_string());

    let gardees = round
        .pairings
        .iter()
        .filter(|p| !cmd.a_defaire.contains(&p.id))
        .map(|p| (p.home_team_id, p.away_team_id));

    Ok(DrawProposal {
        rencontres: gardees
            .chain(cmd.rencontres.iter().copied())
            .map(|(home, away)| ProposedPairing {
                home,
                away,
                historique: historique.historique(&home, &away),
            })
            .collect(),
        exemptee: cmd.exemptee,
        ..Default::default()
    })
}

/// Chaque suppression repasse par `delete_pairing_use_case`, qui revérifie qu'un
/// rapport n'est pas publié sur **cette** rencontre et émet son `PairingDeleted`.
/// La garde de journée l'a déjà dit ; celle-ci est celle du chemin d'écriture,
/// et la doubler ne coûte rien.
async fn defaire(cmd: &RepairCommand, deps: &RepairDeps<'_>) -> Result<(), RepairError> {
    for pairing in &cmd.a_defaire {
        delete_pairing_use_case::execute(
            &pairing.to_string(),
            deps.match_day_repo,
            deps.report_status,
            deps.event_bus,
        )
        .await
        .map_err(|e| RepairError::Database(format!("{e:?}")))?;
    }
    Ok(())
}

async fn ecrire(
    cmd: &RepairCommand,
    deps: &RepairDeps<'_>,
    round: &MatchDay,
    team_display: &HashMap<String, TeamInfoDto>,
) -> Result<(), RepairError> {
    for &(home, away) in &cmd.rencontres {
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
        deps.match_day_repo
            .save_pairing(&cmd.round_id.to_string(), &pairing, &projection)
            .await?;
        emettre_pairing_created(
            deps.event_bus,
            OuEstEcrite {
                competition_id: &cmd.competition_id,
                space_id: &cmd.space_id.to_string(),
                round_id: &cmd.round_id.to_string(),
            },
            &pairing,
            &projection,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::MatchDayType;
    use crate::app::competitions::domain::presence_survey::{
        Appariement, Destinataire, PresenceSurvey, Repondant, Venue,
    };
    use crate::app::competitions::use_cases::presences::test_doubles::*;
    use crate::app::shared_kernel::identity::ids::CoachId;
    use crate::common::services::event_bus::event_bus::new_bus;

    struct Plateau {
        round: MatchDay,
        survey: PresenceSurvey,
        equipes: Vec<TeamInfoDto>,
        ids: Vec<TeamId>,
    }

    fn plateau(n: usize, paires: &[(usize, usize)], exemptee: Option<usize>) -> Plateau {
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
        let pairings: Vec<Pairing> = paires
            .iter()
            .map(|&(a, b)| Pairing {
                id: PairingId::new(),
                home_team_id: ids[a],
                away_team_id: ids[b],
            })
            .collect();
        let round = journee(MatchId::new(), MatchDayType::FixedDate, pairings);
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
        survey.marquer_appariee(exemptee.map(|i| ids[i]));
        Plateau {
            round,
            survey,
            equipes,
            ids,
        }
    }

    fn se_decommande(p: &mut Plateau, rang: usize) {
        p.survey
            .enregistrer(
                &p.ids[rang],
                Venue::Absente,
                Repondant::Jeton,
                &etat_vierge(),
                &date("2026-10-05"),
            )
            .unwrap();
    }

    fn commande(
        p: &Plateau,
        a_defaire: Vec<PairingId>,
        rencontres: Vec<(TeamId, TeamId)>,
        exemptee: Option<TeamId>,
    ) -> RepairCommand {
        RepairCommand {
            round_id: p.round.id,
            season_id: SeasonId::new(),
            competition_id: "c1".to_string(),
            space_id: SpaceId::new(),
            a_defaire,
            rencontres,
            exemptee,
        }
    }

    // ── R12 — l'exemptée reprend du service, et cesse d'être exemptée ────────

    /// Le point de R9 croisée à R12 : compter comme exemptée une équipe qui a
    /// finalement joué la ferait passer devant à la journée suivante pour une
    /// exemption qu'elle n'a pas subie.
    #[tokio::test]
    async fn l_exemptee_qui_reprend_du_service_n_est_plus_comptee_exemptee() {
        let mut p = plateau(5, &[(0, 1), (2, 3)], Some(4));
        let touchee = p.round.pairings[1].id;
        se_decommande(&mut p, 3);
        let depot = FauxSurveyRepo::avec(p.survey.clone());
        let journees = FauxJournees::avec(p.round.clone());
        let bus = new_bus();

        let issue = execute(
            commande(&p, vec![touchee], vec![(p.ids[2], p.ids[4])], None),
            RepairDeps {
                survey_repo: &depot,
                match_day_repo: &journees,
                report_status: &FauxRapports(false),
                teams: &FauxTeams(p.equipes.clone()),
                event_bus: &bus,
            },
        )
        .await
        .expect("réparation");

        assert_eq!(issue.defaites, 1);
        assert_eq!(issue.ecrites, 1);
        assert_eq!(
            depot.derniere_ecrite().expect("campagne").appariement(),
            &Appariement::Fait { exemptee: None },
            "elle a joué : elle n'a pas été exemptée"
        );
    }

    // ── Sans orphelin : l'adversaire subit l'exemption ──────────────────────

    #[tokio::test]
    async fn sans_orphelin_l_adversaire_devient_l_exemptee_enregistree() {
        let mut p = plateau(4, &[(0, 1), (2, 3)], None);
        let touchee = p.round.pairings[1].id;
        se_decommande(&mut p, 3);
        let depot = FauxSurveyRepo::avec(p.survey.clone());
        let bus = new_bus();

        let issue = execute(
            commande(&p, vec![touchee], vec![], Some(p.ids[2])),
            RepairDeps {
                survey_repo: &depot,
                match_day_repo: &FauxJournees::avec(p.round.clone()),
                report_status: &FauxRapports(false),
                teams: &FauxTeams(p.equipes.clone()),
                event_bus: &bus,
            },
        )
        .await
        .expect("réparation");

        assert_eq!(issue.ecrites, 0);
        assert_eq!(
            depot.derniere_ecrite().expect("campagne").appariement(),
            &Appariement::Fait {
                exemptee: Some(p.ids[2])
            },
            "exemption subie, que R9 n'a pas choisie — enregistrée parce qu'elle a lieu"
        );
    }

    // ── Les autres matchs ne bougent pas ────────────────────────────────────

    /// Refaire le tirage entier pour une défection ferait trois mécontents pour
    /// en soulager un : leurs coachs ont déjà noté leur adversaire.
    #[tokio::test]
    async fn la_rencontre_intacte_n_est_ni_defaite_ni_reecrite() {
        let mut p = plateau(5, &[(0, 1), (2, 3)], Some(4));
        let intacte = p.round.pairings[0].id;
        let touchee = p.round.pairings[1].id;
        se_decommande(&mut p, 3);
        let bus = new_bus();
        let mut abonne = bus.subscribe();

        execute(
            commande(&p, vec![touchee], vec![(p.ids[2], p.ids[4])], None),
            RepairDeps {
                survey_repo: &FauxSurveyRepo::avec(p.survey.clone()),
                match_day_repo: &FauxJournees::avec(p.round.clone()),
                report_status: &FauxRapports(false),
                teams: &FauxTeams(p.equipes.clone()),
                event_bus: &bus,
            },
        )
        .await
        .expect("réparation");

        let mut types = vec![];
        while let Ok(e) = abonne.try_recv() {
            types.push(e.event_type.clone());
        }
        assert_eq!(
            types,
            vec!["PairingDeleted", "PairingCreated"],
            "une suppression, une création — la rencontre intacte n'est pas touchée"
        );
        assert_ne!(intacte, touchee);
    }

    // ── R13 — une journée figée ne se répare pas ────────────────────────────

    #[tokio::test]
    async fn une_journee_figee_refuse_la_reparation() {
        let mut p = plateau(4, &[(0, 1), (2, 3)], None);
        let touchee = p.round.pairings[1].id;
        se_decommande(&mut p, 3);
        let depot = FauxSurveyRepo::avec(p.survey.clone());
        let bus = new_bus();
        let mut abonne = bus.subscribe();

        let refus = execute(
            commande(&p, vec![touchee], vec![], Some(p.ids[2])),
            RepairDeps {
                survey_repo: &depot,
                match_day_repo: &FauxJournees::avec(p.round.clone()),
                report_status: &FauxRapports(true),
                teams: &FauxTeams(p.equipes.clone()),
                event_bus: &bus,
            },
        )
        .await;

        assert_eq!(
            refus.unwrap_err(),
            RepairError::Refus(DomainError::RoundFrozenByReport)
        );
        assert_eq!(depot.ecritures(), 0);
        assert!(abonne.try_recv().is_err(), "rien n'est ni défait ni écrit");
    }

    // ── R22 — la réparation est revérifiée comme la validation initiale ─────

    /// Une réparation qui apparierait le désistant est refusée par R5 : la
    /// proposition voyage par le client, donc elle revient modifiable.
    #[tokio::test]
    async fn une_reparation_qui_apparie_le_desistant_est_refusee() {
        let mut p = plateau(5, &[(0, 1), (2, 3)], Some(4));
        let touchee = p.round.pairings[1].id;
        se_decommande(&mut p, 3);
        let depot = FauxSurveyRepo::avec(p.survey.clone());
        let bus = new_bus();

        let refus = execute(
            commande(&p, vec![touchee], vec![(p.ids[3], p.ids[4])], None),
            RepairDeps {
                survey_repo: &depot,
                match_day_repo: &FauxJournees::avec(p.round.clone()),
                report_status: &FauxRapports(false),
                teams: &FauxTeams(p.equipes.clone()),
                event_bus: &bus,
            },
        )
        .await;

        assert_eq!(
            refus.unwrap_err(),
            RepairError::Refus(DomainError::TeamNotPresent {
                team: p.ids[3].to_string()
            })
        );
        assert_eq!(depot.ecritures(), 0);
    }
}

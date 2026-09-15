//! Ce qu'on propose quand une journée appariée ne concorde plus.
//!
//! # La réparation **est** un tirage sur le vivier
//!
//! La conception décrivait un algorithme propre à la réparation : l'exemptée
//! d'abord considérée, sinon l'adversaire devient exempté. C'est exactement ce
//! que `tirer` fait déjà d'un vivier — R8.1 apparie le plus possible, R8.2 évite
//! les revanches, R10 refuse les paires interdites, et ce qui reste est exempté.
//!
//! « L'exemptée est le premier remplaçant considéré » n'est donc pas une règle à
//! écrire : **elle est dans le vivier**, et R8.1 la fait servir parce qu'un
//! appariement de plus vaut mieux qu'une exemption. « Sinon l'adversaire devient
//! exempté » est le cas où le vivier ne contient qu'eux deux et que la paire est
//! impossible — ou qu'il est seul.
//!
//! Un second algorithme aurait divergé du premier au premier changement de règle.
//! Celui-ci ne peut pas : c'est le même.
//!
//! # R16 est le même mécanisme en sens inverse
//!
//! Un arrivant tardif rejoint les orphelins et s'apparie avec l'exemptée s'il y
//! en a une. Rien à écrire de plus : `desaccord` le range déjà dans `orphelins`,
//! et le vivier les traite tous pareil.
//!
//! # Les autres matchs ne bougent pas
//!
//! Seules les rencontres que `desaccord` désigne sont défaites. Refaire le tirage
//! entier pour une défection ferait trois mécontents pour en soulager un : leurs
//! coachs ont déjà noté leur adversaire.

use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError,
};
use crate::app::competitions::domain::presence_survey::{
    Appariement, Desaccord, EtatJournee, PresenceSurvey,
};
use crate::app::competitions::domain::presence_survey_repository_port::{
    IPresenceSurveyRepository, PresenceSurveyRepositoryError,
};
use crate::app::competitions::domain::tirage::{tirer, DrawInput, DrawProposal};
use crate::app::competitions::ports::{IMatchReportStatusPort, ITeamInfoPort};
use crate::app::competitions::use_cases::admin::team_enrollment::load_enrolled_teams;
use crate::app::competitions::use_cases::entree_du_tirage::{
    build_historique, build_interdites, build_matchs_joues,
};
use crate::app::competitions::use_cases::presences::draw_pairings_use_case::ids_inscrits;
use crate::app::competitions::use_cases::presences::etat_journee::etat_de_la_journee;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::collections::HashSet;

#[derive(Debug)]
pub struct ProposeRepairCommand {
    pub round_id: MatchId,
    pub season_id: SeasonId,
}

/// Ce que le panneau de défection affiche, et que l'organisateur valide.
#[derive(Debug, PartialEq, Eq)]
pub struct RepairProposal {
    /// Les rencontres à défaire — celles dont un camp n'est plus présent.
    pub a_defaire: Vec<PairingId>,
    /// Ce que le tirage propose sur le vivier.
    pub proposition: DrawProposal,
    /// Le vivier soumis au tirage, pour que l'écran dise **qui** on réapparie.
    pub vivier: Vec<TeamId>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProposeRepairError {
    SurveyNotFound,
    RoundNotFound,
    /// Rien à réparer : les présences et les appariements concordent.
    AucunDesaccord,
    Database(String),
}

impl From<PresenceSurveyRepositoryError> for ProposeRepairError {
    fn from(e: PresenceSurveyRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<MatchDayRepositoryError> for ProposeRepairError {
    fn from(e: MatchDayRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

pub struct ProposeDeps<'a> {
    pub survey_repo: &'a dyn IPresenceSurveyRepository,
    pub match_day_repo: &'a dyn IMatchDayRepository,
    pub report_status: &'a dyn IMatchReportStatusPort,
    pub teams: &'a dyn ITeamInfoPort,
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ProposeRepairCommand,
    deps: ProposeDeps<'_>,
) -> Result<RepairProposal, ProposeRepairError> {
    let survey = deps
        .survey_repo
        .find_by_round(&cmd.round_id.to_string())
        .await?
        .ok_or(ProposeRepairError::SurveyNotFound)?;
    let round = deps
        .match_day_repo
        .find_by_id(&cmd.round_id.to_string())
        .await?
        .ok_or(ProposeRepairError::RoundNotFound)?;

    let journee = etat_de_la_journee(&round, deps.report_status)
        .await
        .map_err(ProposeRepairError::Database)?;
    let desaccord = survey
        .desaccord(&journee)
        .ok_or(ProposeRepairError::AucunDesaccord)?;

    let team_display = load_enrolled_teams(&cmd.season_id.to_string(), deps.teams)
        .await
        .map_err(ProposeRepairError::Database)?;
    let inscrites = ids_inscrits(&team_display);
    let vivier = vivier(&survey, &journee, &desaccord, &inscrites);

    let jours = deps
        .match_day_repo
        .find_by_season(&cmd.season_id.to_string())
        .await?;
    let exclue = cmd.round_id.to_string();

    let input = DrawInput {
        interdites: build_interdites(&vivier, &team_display),
        equipes: vivier.clone(),
        historique: build_historique(&jours, &exclue),
        matchs_joues: build_matchs_joues(&jours, &exclue),
    };
    Ok(RepairProposal {
        a_defaire: desaccord.rencontres_a_refaire.clone(),
        proposition: tirer(&input, &mut StdRng::from_os_rng()),
        vivier,
    })
}

/// Qui doit être réapparié : les rescapés des rencontres cassées, les orphelins,
/// et **l'exemptée**.
///
/// L'exemptée y entre parce qu'elle est disponible, pas parce qu'une règle la
/// désigne : c'est R8.1 qui la fera jouer, un appariement de plus valant mieux
/// qu'une exemption. Sur le cas de la maquette, ça garde quatre matchs là où
/// « l'adversaire devient exempt » en aurait perdu deux — l'orphelin *et*
/// l'exemptée restant sur le banc.
///
/// R18 filtre les désinscrites : les réapparier proposerait une rencontre que la
/// validation refuserait.
fn vivier(
    survey: &PresenceSurvey,
    journee: &EtatJournee,
    desaccord: &Desaccord,
    inscrites: &HashSet<TeamId>,
) -> Vec<TeamId> {
    let mut vivier: Vec<TeamId> = rescapes(survey, journee, desaccord);
    vivier.extend(desaccord.orphelins.iter().copied());
    if let Appariement::Fait {
        exemptee: Some(e), ..
    } = survey.appariement()
    {
        vivier.push(*e);
    }
    vivier.retain(|t| inscrites.contains(t));
    vivier
}

/// Les camps encore présents des rencontres à défaire. Le désistant, lui, n'y est
/// pas — c'est bien pour ça que sa rencontre tombe.
fn rescapes(survey: &PresenceSurvey, journee: &EtatJournee, desaccord: &Desaccord) -> Vec<TeamId> {
    journee
        .rencontres
        .iter()
        .filter(|r| desaccord.rencontres_a_refaire.contains(&r.pairing))
        .flat_map(|r| [r.home, r.away])
        .filter(|t| {
            survey
                .reponse_de(t)
                .is_some_and(|rep| rep.presence().compte_pour_le_tirage())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{MatchDay, MatchDayType, Pairing};
    use crate::app::competitions::domain::presence_survey::{Destinataire, Repondant, Venue};
    use crate::app::competitions::ports::TeamInfoDto;
    use crate::app::competitions::use_cases::presences::test_doubles::*;
    use crate::app::shared_kernel::identity::ids::CoachId;

    struct Plateau {
        round: MatchDay,
        survey: PresenceSurvey,
        equipes: Vec<TeamInfoDto>,
        ids: Vec<TeamId>,
    }

    /// `n` équipes présentes, chacune son coach, et la journée appariée selon
    /// `paires` avec `exemptee` mise de côté.
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

    /// Fait se décommander l'équipe de rang `rang`.
    fn se_decommande(p: &mut Plateau, rang: usize) {
        p.survey
            .enregistrer(
                &p.ids[rang],
                Venue::Absente,
                Repondant::Jeton,
                &EtatJournee {
                    figee: false,
                    rencontres: vec![],
                },
                &date("2026-10-05"),
            )
            .unwrap();
    }

    async fn proposer(p: &Plateau) -> Result<RepairProposal, ProposeRepairError> {
        execute(
            ProposeRepairCommand {
                round_id: p.round.id,
                season_id: SeasonId::new(),
            },
            ProposeDeps {
                survey_repo: &FauxSurveyRepo::avec(p.survey.clone()),
                match_day_repo: &FauxJournees::avec(p.round.clone()),
                report_status: &FauxRapports(false),
                teams: &FauxTeams(p.equipes.clone()),
            },
        )
        .await
    }

    // ── R12 — l'exemptée reprend du service ─────────────────────────────────

    /// Cinq présents, deux rencontres, une exemptée. Le 3 se décommande : sa
    /// rencontre tombe, le 2 devient orphelin, et l'exemptée est **dans le
    /// vivier**. R8.1 les apparie — quatre matchs conservés là où « l'adversaire
    /// devient exempt » en aurait perdu deux.
    #[tokio::test]
    async fn une_defection_fait_reprendre_du_service_a_l_exemptee() {
        let mut p = plateau(5, &[(0, 1), (2, 3)], Some(4));
        let touchee = p.round.pairings[1].id;
        se_decommande(&mut p, 3);

        let prop = proposer(&p).await.expect("proposition");

        assert_eq!(prop.a_defaire, vec![touchee]);
        assert_eq!(
            prop.vivier.len(),
            2,
            "le rescapé et l'exemptée, le désistant n'y est pas"
        );
        assert!(prop.vivier.contains(&p.ids[2]) && prop.vivier.contains(&p.ids[4]));
        assert_eq!(prop.proposition.rencontres.len(), 1);
        assert_eq!(
            prop.proposition.exemptee, None,
            "les deux jouent : plus personne sur le banc"
        );
    }

    // ── Défection sans orphelin : l'adversaire devient exempté ──────────────

    /// Quatre présents, deux rencontres, aucune exemptée. Le 3 se décommande : le
    /// 2 se retrouve seul au vivier. C'est une exemption **subie**, que R9 n'a pas
    /// choisie — on l'enregistre parce qu'elle a lieu.
    #[tokio::test]
    async fn sans_orphelin_l_adversaire_devient_exempte() {
        let mut p = plateau(4, &[(0, 1), (2, 3)], None);
        se_decommande(&mut p, 3);

        let prop = proposer(&p).await.expect("proposition");

        assert_eq!(prop.vivier, vec![p.ids[2]]);
        assert!(prop.proposition.rencontres.is_empty());
        assert_eq!(prop.proposition.exemptee, Some(p.ids[2]));
    }

    // ── R16 — l'arrivée tardive, le même mécanisme en sens inverse ──────────

    /// Trois présents dont un arrivé tard, une rencontre, une exemptée. L'arrivant
    /// est rangé dans les orphelins par `desaccord`, et le vivier les traite tous
    /// pareil : il s'apparie avec l'exemptée.
    #[tokio::test]
    async fn une_arrivee_tardive_s_apparie_avec_l_exemptee() {
        let equipes: Vec<TeamInfoDto> = (0..4)
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
        let round = journee(
            MatchId::new(),
            MatchDayType::FixedDate,
            vec![Pairing {
                id: PairingId::new(),
                home_team_id: ids[0],
                away_team_id: ids[1],
            }],
        );
        let destinataires: Vec<Destinataire> = equipes
            .iter()
            .zip(&ids)
            .map(|(e, t)| Destinataire {
                team_id: *t,
                coach_id: CoachId::try_new(&e.coach_id).unwrap(),
            })
            .collect();
        let mut survey = campagne_ouverte(&round, &destinataires);
        // 0, 1 et 2 présents au tirage ; 2 exemptée. Le 3 arrive après.
        for rang in [0, 1, 2] {
            survey
                .enregistrer(
                    &ids[rang],
                    Venue::Presente,
                    Repondant::Jeton,
                    &etat_vierge(),
                    &date("2026-10-04"),
                )
                .unwrap();
        }
        survey.marquer_appariee(Some(ids[2]));
        survey
            .enregistrer(
                &ids[3],
                Venue::Presente,
                Repondant::Jeton,
                &etat_vierge(),
                &date("2026-10-06"),
            )
            .unwrap();

        let prop = execute(
            ProposeRepairCommand {
                round_id: round.id,
                season_id: SeasonId::new(),
            },
            ProposeDeps {
                survey_repo: &FauxSurveyRepo::avec(survey),
                match_day_repo: &FauxJournees::avec(round),
                report_status: &FauxRapports(false),
                teams: &FauxTeams(equipes),
            },
        )
        .await
        .expect("proposition");

        assert!(
            prop.a_defaire.is_empty(),
            "aucune rencontre à casser : personne ne s'est décommandé"
        );
        assert_eq!(prop.vivier.len(), 2, "l'arrivant et l'exemptée");
        assert_eq!(prop.proposition.rencontres.len(), 1);
        assert_eq!(prop.proposition.exemptee, None);
    }

    // ── R18 — une désinscrite ne se réapparie pas ───────────────────────────

    #[tokio::test]
    async fn une_desinscrite_est_ecartee_du_vivier() {
        let mut p = plateau(5, &[(0, 1), (2, 3)], Some(4));
        se_decommande(&mut p, 3);
        // Le rescapé a quitté la saison : le port ne le rend plus.
        let encore: Vec<TeamInfoDto> = p
            .equipes
            .iter()
            .filter(|e| e.team_id != p.ids[2].to_string())
            .cloned()
            .collect();

        let prop = execute(
            ProposeRepairCommand {
                round_id: p.round.id,
                season_id: SeasonId::new(),
            },
            ProposeDeps {
                survey_repo: &FauxSurveyRepo::avec(p.survey.clone()),
                match_day_repo: &FauxJournees::avec(p.round.clone()),
                report_status: &FauxRapports(false),
                teams: &FauxTeams(encore),
            },
        )
        .await
        .expect("proposition");

        assert_eq!(prop.vivier, vec![p.ids[4]], "seule l'exemptée reste");
        assert_eq!(prop.proposition.exemptee, Some(p.ids[4]));
    }

    // ── Rien à réparer ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn une_journee_qui_concorde_n_a_rien_a_reparer() {
        let p = plateau(4, &[(0, 1), (2, 3)], None);

        let refus = proposer(&p).await;

        assert_eq!(refus.unwrap_err(), ProposeRepairError::AucunDesaccord);
    }
}

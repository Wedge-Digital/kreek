//! Tirer au sort les présents — **un POST qui n'écrit rien**.
//!
//! L'aperçu ne persiste pas : la proposition part au client et en revient
//! modifiable, ce que `confirm_draw` revérifie intégralement (R22). C'est ce qui
//! permet à l'organisateur de regarder avant de trancher.
//!
//! # Il filtre les désengagées, comme la validation le fera
//!
//! R18 — sans ce filtre, l'aperçu proposerait une rencontre que la validation
//! refuserait, et l'organisateur verrait son tirage changer sans avoir rien fait.
//! Les deux étages filtrent donc, sur la même source.
//!
//! # Instrumenté, bien qu'il n'écrive rien
//!
//! C'est une action de l'organisateur, et le journal doit la porter : « pourquoi
//! ce tirage-là » se pose après coup. Le marqueur `arch:no-instrument` est
//! réservé aux services d'hydratation, qui n'ont pas d'intention métier.

use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError,
};
use crate::app::competitions::domain::presence_survey::PresenceSurvey;
use crate::app::competitions::domain::presence_survey_repository_port::{
    IPresenceSurveyRepository, PresenceSurveyRepositoryError,
};
use crate::app::competitions::domain::tirage::{tirer, DrawInput, DrawProposal};
use crate::app::competitions::ports::{ITeamInfoPort, TeamInfoDto};
use crate::app::competitions::use_cases::admin::team_enrollment::load_enrolled_teams;
use crate::app::competitions::use_cases::entree_du_tirage::{
    build_historique, build_interdites, build_matchs_joues,
};
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct DrawCommand {
    pub round_id: MatchId,
    pub season_id: SeasonId,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DrawError {
    SurveyNotFound,
    RoundNotFound,
    /// R11 — la journée porte déjà des appariements. L'organisateur la vide
    /// depuis le Calendrier s'il veut retirer.
    PairingsAlreadyExist,
    /// R15, par `peut_tirer` : moins de deux **appariables**, présentes et
    /// toujours inscrites.
    Refus(DomainError),
    Database(String),
}

impl From<PresenceSurveyRepositoryError> for DrawError {
    fn from(e: PresenceSurveyRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<MatchDayRepositoryError> for DrawError {
    fn from(e: MatchDayRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

pub struct DrawDeps<'a> {
    pub survey_repo: &'a dyn IPresenceSurveyRepository,
    pub match_day_repo: &'a dyn IMatchDayRepository,
    pub teams: &'a dyn ITeamInfoPort,
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(cmd: DrawCommand, deps: DrawDeps<'_>) -> Result<DrawProposal, DrawError> {
    let (survey, inscrites, team_display) = charger(&cmd, &deps).await?;
    let equipes = presents_inscrits(&survey, &inscrites);

    survey.peut_tirer(&inscrites).map_err(DrawError::Refus)?;

    let jours = deps
        .match_day_repo
        .find_by_season(&cmd.season_id.to_string())
        .await?;
    let exclue = cmd.round_id.to_string();

    let input = DrawInput {
        interdites: build_interdites(&equipes, &team_display),
        equipes,
        historique: build_historique(&jours, &exclue),
        matchs_joues: build_matchs_joues(&jours, &exclue),
    };
    Ok(tirer(&input, &mut StdRng::from_os_rng()))
}

/// La campagne, les équipes encore inscrites, et de quoi les nommer.
///
/// R11 est vérifiée ici, avant tout calcul : proposer un tirage sur une journée
/// déjà appariée ferait espérer une écriture que `save_pairings` refuserait sous
/// son verrou.
async fn charger(
    cmd: &DrawCommand,
    deps: &DrawDeps<'_>,
) -> Result<
    (
        PresenceSurvey,
        HashSet<TeamId>,
        HashMap<String, TeamInfoDto>,
    ),
    DrawError,
> {
    let survey = deps
        .survey_repo
        .find_by_round(&cmd.round_id.to_string())
        .await?
        .ok_or(DrawError::SurveyNotFound)?;

    let round = deps
        .match_day_repo
        .find_by_id(&cmd.round_id.to_string())
        .await?
        .ok_or(DrawError::RoundNotFound)?;
    if !round.pairings.is_empty() {
        return Err(DrawError::PairingsAlreadyExist);
    }

    let team_display = load_enrolled_teams(&cmd.season_id.to_string(), deps.teams)
        .await
        .map_err(DrawError::Database)?;
    Ok((survey, ids_inscrits(&team_display), team_display))
}

/// R18 — les équipes encore inscrites à la saison. `load_enrolled_teams` ne rend
/// qu'elles, donc l'ensemble de ses clés **est** la liste des inscrites.
pub fn ids_inscrits(team_display: &HashMap<String, TeamInfoDto>) -> HashSet<TeamId> {
    team_display
        .keys()
        .filter_map(|id| TeamId::try_new(id).ok())
        .collect()
}

/// R5 croisée à R18 : présentes **et** toujours inscrites. C'est ce que le tirage
/// reçoit, et ce que `peut_tirer` compte.
pub fn presents_inscrits(survey: &PresenceSurvey, inscrites: &HashSet<TeamId>) -> Vec<TeamId> {
    survey
        .presents()
        .iter()
        .map(|r| *r.team_id())
        .filter(|t| inscrites.contains(t))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{MatchDay, MatchDayType, Pairing};
    use crate::app::competitions::domain::presence_survey::{Destinataire, Repondant, Venue};
    use crate::app::competitions::use_cases::presences::test_doubles::*;
    use crate::app::shared_kernel::bloodbowl::ids::PairingId;
    use crate::app::shared_kernel::identity::ids::CoachId;

    struct Plateau {
        round: MatchDay,
        survey: PresenceSurvey,
        equipes: Vec<TeamInfoDto>,
        ids: Vec<TeamId>,
    }

    /// `n` équipes, chacune son coach, toutes présentes et toutes inscrites.
    fn plateau(n: usize, pairings: Vec<Pairing>) -> Plateau {
        let round = journee(MatchId::new(), MatchDayType::FixedDate, pairings);
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

    async fn tirer_sur(p: &Plateau) -> Result<DrawProposal, DrawError> {
        execute(
            DrawCommand {
                round_id: p.round.id,
                season_id: SeasonId::new(),
            },
            DrawDeps {
                survey_repo: &FauxSurveyRepo::avec(p.survey.clone()),
                match_day_repo: &FauxJournees::avec(p.round.clone()),
                teams: &FauxTeams(p.equipes.clone()),
            },
        )
        .await
    }

    #[tokio::test]
    async fn quatre_presents_donnent_deux_rencontres_et_rien_n_est_ecrit() {
        let p = plateau(4, vec![]);

        let prop = tirer_sur(&p).await.expect("aperçu");

        assert_eq!(prop.rencontres.len(), 2);
        assert_eq!(prop.exemptee, None);
    }

    // ── R11 — une journée déjà appariée ne se retire pas ─────────────────────

    #[tokio::test]
    async fn une_journee_deja_appariee_refuse_l_apercu() {
        let deja = vec![Pairing {
            id: PairingId::new(),
            home_team_id: TeamId::new(),
            away_team_id: TeamId::new(),
        }];
        let p = plateau(4, deja);

        let refus = tirer_sur(&p).await;

        assert_eq!(refus.unwrap_err(), DrawError::PairingsAlreadyExist);
    }

    // ── R15 — le tirage refuse en dessous de deux appariables ────────────────

    #[tokio::test]
    async fn un_seul_present_ne_suffit_pas() {
        let round = journee(MatchId::new(), MatchDayType::FixedDate, vec![]);
        let equipes = vec![
            equipe("Seule présente", &CoachId::new(), "Alpha"),
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

        let refus = execute(
            DrawCommand {
                round_id: round.id,
                season_id: SeasonId::new(),
            },
            DrawDeps {
                survey_repo: &FauxSurveyRepo::avec(survey),
                match_day_repo: &FauxJournees::avec(round),
                teams: &FauxTeams(equipes),
            },
        )
        .await;

        assert_eq!(
            refus.unwrap_err(),
            DrawError::Refus(DomainError::NotEnoughPresent { presents: 1 })
        );
    }

    // ── R18 — l'aperçu filtre les désengagées, comme la validation ───────────

    /// Sans ce filtre, l'aperçu proposerait une rencontre que la validation
    /// refuserait, et l'organisateur verrait son tirage changer sans avoir rien
    /// fait.
    #[tokio::test]
    async fn une_desengagee_est_ecartee_de_l_apercu() {
        let p = plateau(3, vec![]);
        // La troisième a quitté la saison : le port ne la rend plus.
        let encore_inscrites: Vec<TeamInfoDto> = p.equipes[..2].to_vec();

        let prop = execute(
            DrawCommand {
                round_id: p.round.id,
                season_id: SeasonId::new(),
            },
            DrawDeps {
                survey_repo: &FauxSurveyRepo::avec(p.survey.clone()),
                match_day_repo: &FauxJournees::avec(p.round.clone()),
                teams: &FauxTeams(encore_inscrites),
            },
        )
        .await
        .expect("aperçu");

        assert_eq!(prop.rencontres.len(), 1);
        assert_eq!(prop.exemptee, None, "deux inscrites, donc aucune exemptée");
        let appariees: Vec<TeamId> = prop
            .rencontres
            .iter()
            .flat_map(|r| [r.home, r.away])
            .collect();
        assert!(
            !appariees.contains(&p.ids[2]),
            "la désengagée n'apparaît pas, alors qu'elle est bien présente"
        );
    }

    // ── R10 — la matière est inter-BC, la règle est métier ───────────────────

    /// Deux équipes d'un même coach ne se rencontrent pas, et `build_interdites`
    /// le déduit de `team_display` — le domaine ne sait pas qu'un coach existe.
    #[tokio::test]
    async fn deux_equipes_d_un_meme_coach_ne_sont_pas_appariees() {
        let round = journee(MatchId::new(), MatchDayType::FixedDate, vec![]);
        let solo = CoachId::new();
        let equipes = vec![
            equipe("Ses deux — A", &solo, "Lepandawan"),
            equipe("Ses deux — B", &solo, "Lepandawan"),
            equipe("Une autre", &CoachId::new(), "Ghorak"),
            equipe("Encore une", &CoachId::new(), "Skreek"),
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

        let prop = execute(
            DrawCommand {
                round_id: round.id,
                season_id: SeasonId::new(),
            },
            DrawDeps {
                survey_repo: &FauxSurveyRepo::avec(survey),
                match_day_repo: &FauxJournees::avec(round),
                teams: &FauxTeams(equipes),
            },
        )
        .await
        .expect("aperçu");

        let interdite_appariee = prop.rencontres.iter().any(|r| {
            (r.home == ids[0] && r.away == ids[1]) || (r.home == ids[1] && r.away == ids[0])
        });
        assert!(
            !interdite_appariee,
            "R10 — un coach ne joue pas contre lui-même"
        );
        assert_eq!(prop.rencontres.len(), 2);
    }

    #[tokio::test]
    async fn une_journee_sans_campagne_ne_se_tire_pas() {
        let round = journee(MatchId::new(), MatchDayType::FixedDate, vec![]);

        let refus = execute(
            DrawCommand {
                round_id: round.id,
                season_id: SeasonId::new(),
            },
            DrawDeps {
                survey_repo: &FauxSurveyRepo::vide(),
                match_day_repo: &FauxJournees::avec(round),
                teams: &FauxTeams(vec![]),
            },
        )
        .await;

        assert_eq!(refus.unwrap_err(), DrawError::SurveyNotFound);
    }
}

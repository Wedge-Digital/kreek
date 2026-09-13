//! Ce que l'encart du coach connecté a besoin de savoir.
//!
//! # Un service d'hydratation, pas un use case
//!
//! Il n'a aucune intention métier : il croise trois lectures — les campagnes
//! ouvertes de la saison, les journées qui les portent, les équipes engagées du
//! coach — et rend de quoi afficher. Rien n'est décidé ici, et rien n'est écrit.
//!
//! # `TeamInfoDto` s'arrête à ce fichier
//!
//! Ni le handler ni le gabarit ne le voient. C'est la règle des domain services
//! pour données inter-BC : le DTO du port se transforme ici, une fois, et ce qui
//! en sort porte les types du domaine.
//!
//! # `confirmes` vient du domaine, jamais d'un compte local
//!
//! `compte_presents()` compte **toute** la campagne. Un `filter().count()` sur
//! `mes_equipes` ne verrait que les équipes du coach et afficherait « 1 équipe a
//! déjà confirmé » là où il y en a neuf — le défaut de la carte 492/495, où la vue
//! recomptait ce que le domaine savait compter. Ici il ne se verrait même pas sur
//! un coach à une seule équipe, les deux nombres coïncidant.
//!
//! # R29 — combien, jamais qui
//!
//! Le compte sort d'ici ; la liste des équipes confirmées, non. Une réponse est
//! donnée à l'organisateur, qui apparie ; elle n'est pas publiée aux autres
//! coachs.

use crate::app::competitions::domain::match_day::MatchDay;
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::domain::presence_survey::{Presence, PresenceSurvey, SurveyDeadline};
use crate::app::competitions::domain::presence_survey_repository_port::IPresenceSurveyRepository;
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::MatchId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::CoachId;

/// Une campagne ouverte, vue par un coach donné.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampagneOuverte {
    pub round_id: MatchId,
    pub round_name: String,
    /// Les bornes brutes de la journée, non composées : « du 12 au 19 » se met en
    /// forme dans la vue, comme pour la page publique (carte 523).
    pub date_start: Option<String>,
    pub date_end: Option<String>,
    pub deadline: SurveyDeadline,
    /// `compte_presents()` — le compte de la campagne entière.
    pub confirmes: usize,
    pub mes_equipes: Vec<MonEquipe>,
}

/// Une équipe du coach dans cette campagne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonEquipe {
    pub team_id: TeamId,
    pub team_name: String,
    /// **Le type du domaine, pas une chaîne.** C'est le VM, en bout de chaîne, qui
    /// choisit les mots ; l'aplatir ici mettrait la formulation dans la couche
    /// applicative.
    pub presence: Presence,
}

/// Les campagnes ouvertes qui **concernent ce coach**, par échéance croissante.
///
/// Une campagne où il n'engage aucune équipe est écartée : l'encart lui poserait
/// une question sans réponse possible.
///
/// **Une carte par campagne, jamais une seule.** R2 garantit une campagne par
/// *journée*, pas par compétition : deux journées peuvent être sondées en même
/// temps, et taire l'une parce que l'autre existe ferait manquer une échéance.
///
/// Les erreurs de lecture rendent une liste vide plutôt que de remonter : l'encart
/// est secondaire, et une page de détail qui refuserait de s'afficher parce qu'un
/// sondage est illisible coûterait plus que l'encart ne rapporte. Le journal, lui,
/// garde la trace.
// arch:no-instrument — service d'hydratation : assemble une vue, sans intention métier
pub async fn hydrater(
    season_id: &str,
    coach_id: &CoachId,
    survey_repo: &dyn IPresenceSurveyRepository,
    match_day_repo: &dyn IMatchDayRepository,
    team_port: &dyn ITeamInfoPort,
    maintenant: &DateString,
) -> Vec<CampagneOuverte> {
    let campagnes = match survey_repo
        .list_open_surveys_for_season(season_id, maintenant.as_ref())
        .await
    {
        Ok(v) if !v.is_empty() => v,
        Ok(_) => return vec![],
        Err(e) => {
            tracing::error!("encart de présence : campagnes illisibles — {e}");
            return vec![];
        }
    };

    let mes_equipes = equipes_du_coach(season_id, coach_id, team_port).await;
    if mes_equipes.is_empty() {
        return vec![];
    }
    let journees = journees_de_la_saison(season_id, match_day_repo).await;

    let mut ouvertes: Vec<CampagneOuverte> = campagnes
        .iter()
        .filter_map(|c| campagne_pour(c, &journees, &mes_equipes))
        .collect();
    // L'échéance la plus proche en premier : c'est celle qui presse.
    ouvertes.sort_by(|a, b| a.deadline.as_ref().cmp(b.deadline.as_ref()));
    ouvertes
}

/// Les équipes engagées du coach, `(id, nom)`.
///
/// Le port rend **toutes** les équipes de la saison ; le filtre sur le coach est
/// ici parce que c'est la seule question que l'encart pose.
async fn equipes_du_coach(
    season_id: &str,
    coach_id: &CoachId,
    team_port: &dyn ITeamInfoPort,
) -> Vec<(TeamId, String)> {
    let engagees = match team_port.find_enrolled_teams(season_id).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("encart de présence : équipes illisibles — {e}");
            return vec![];
        }
    };
    engagees
        .iter()
        .filter(|t| t.coach_id == coach_id.to_string())
        .filter_map(|t| Some((TeamId::try_new(&t.team_id).ok()?, t.team_name.clone())))
        .collect()
}

async fn journees_de_la_saison(
    season_id: &str,
    match_day_repo: &dyn IMatchDayRepository,
) -> Vec<MatchDay> {
    match match_day_repo.find_by_season(season_id).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("encart de présence : journées illisibles — {e}");
            vec![]
        }
    }
}

/// Une campagne, si elle concerne ce coach **et** si sa journée est connue.
///
/// Une journée introuvable écarte la campagne au lieu de rendre un titre vide :
/// « Seras-tu là pour  ? » est pire que rien, et c'est le genre de valeur inventée
/// que le `CLAUDE.md` interdit aux gabarits.
fn campagne_pour(
    survey: &PresenceSurvey,
    journees: &[MatchDay],
    mes_equipes: &[(TeamId, String)],
) -> Option<CampagneOuverte> {
    let journee = journees.iter().find(|j| &j.id == survey.round_id())?;
    let miennes = lignes_du_coach(survey, mes_equipes);
    if miennes.is_empty() {
        return None;
    }
    Some(CampagneOuverte {
        round_id: journee.id,
        round_name: journee.name.as_ref().to_string(),
        date_start: journee.date_start.as_ref().map(|d| d.as_ref().to_string()),
        date_end: journee.date_end.as_ref().map(|d| d.as_ref().to_string()),
        deadline: survey.deadline().clone(),
        confirmes: survey.compte_presents(),
        mes_equipes: miennes,
    })
}

/// Les réponses de la campagne qui portent sur une équipe du coach.
///
/// On part des **réponses**, pas des équipes engagées : une équipe inscrite après
/// l'ouverture de la campagne n'y a pas de réponse, donc aucun jeton et aucune
/// ligne à afficher. Partir des équipes aurait produit une ligne sans réponse
/// possible.
fn lignes_du_coach(survey: &PresenceSurvey, mes_equipes: &[(TeamId, String)]) -> Vec<MonEquipe> {
    survey
        .reponses()
        .iter()
        .filter_map(|r| {
            let (_, nom) = mes_equipes.iter().find(|(id, _)| id == r.team_id())?;
            Some(MonEquipe {
                team_id: *r.team_id(),
                team_name: nom.clone(),
                presence: r.presence().clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::MatchDayType;
    use crate::app::competitions::domain::presence_survey::{
        Destinataire, PresenceSurvey as Campagne, Repondant, Venue,
    };
    use crate::app::competitions::ports::TeamInfoDto;
    use crate::app::competitions::use_cases::presences::test_doubles::*;
    use crate::app::shared_kernel::identity::ids::CoachId;

    /// Une campagne dont les destinataires **sont** les équipes données : c'est ce
    /// que fait `ouvrir` en vrai, à partir du roster de la saison.
    fn campagne_sur(round: &MatchDay, equipes: &[(&TeamInfoDto, &CoachId)]) -> Campagne {
        let dests: Vec<Destinataire> = equipes
            .iter()
            .map(|(t, c)| Destinataire {
                team_id: TeamId::try_new(&t.team_id).unwrap(),
                coach_id: **c,
            })
            .collect();
        campagne_ouverte(round, &dests)
    }

    fn hydrater_avec(
        depot: &FauxSurveyRepo,
        journees: &FauxJournees,
        teams: &FauxTeams,
        coach: &CoachId,
    ) -> Vec<CampagneOuverte> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(hydrater(
                "saison",
                coach,
                depot,
                journees,
                teams,
                &date("2026-10-05"),
            ))
    }

    // ── Ce que l'encart montre, et à qui ─────────────────────────────────────

    #[test]
    fn sans_campagne_ouverte_l_encart_n_a_rien_a_dire() {
        let coach = CoachId::new();
        let jour = journee(MatchId::new(), MatchDayType::TimeFrame, vec![]);

        let vues = hydrater_avec(
            &FauxSurveyRepo::vide(),
            &FauxJournees::avec(jour.clone()).et_la_saison(vec![jour]),
            &FauxTeams(vec![equipe("Les Miens", &coach, "Alice")]),
            &coach,
        );

        assert!(vues.is_empty());
    }

    /// R28 vu de l'affichage : l'encart ne montre que les équipes du coach. Celles
    /// des autres sont dans la campagne, pas dans sa carte.
    #[test]
    fn seules_les_equipes_du_coach_apparaissent() {
        let (moi, autre) = (CoachId::new(), CoachId::new());
        let jour = journee(MatchId::new(), MatchDayType::TimeFrame, vec![]);
        let mienne = equipe("Les Miens", &moi, "Alice");
        let sienne = equipe("Les Siens", &autre, "Bob");
        let survey = campagne_sur(&jour, &[(&mienne, &moi), (&sienne, &autre)]);

        let vues = hydrater_avec(
            &FauxSurveyRepo::avec(survey),
            &FauxJournees::avec(jour.clone()).et_la_saison(vec![jour]),
            &FauxTeams(vec![mienne.clone(), sienne]),
            &moi,
        );

        assert_eq!(vues.len(), 1);
        assert_eq!(vues[0].mes_equipes.len(), 1);
        assert_eq!(vues[0].mes_equipes[0].team_name, "Les Miens");
    }

    /// **Le test qui compte.** `confirmes` doit valoir le compte de *toute* la
    /// campagne, pas des équipes du coach — c'est le défaut de la carte 495, où la
    /// vue recomptait ce que le domaine savait compter.
    ///
    /// Neuf équipes confirment, une seule est au coach : un `filter().count()` sur
    /// `mes_equipes` afficherait « 1 équipe a déjà confirmé ». Sur un coach à une
    /// équipe qui serait la seule présente, les deux nombres coïncideraient et le
    /// défaut resterait invisible — d'où les dix équipes.
    #[test]
    fn confirmes_compte_toute_la_campagne_et_non_les_equipes_du_coach() {
        let moi = CoachId::new();
        let jour = journee(MatchId::new(), MatchDayType::TimeFrame, vec![]);
        let mienne = equipe("Les Miens", &moi, "Alice");
        let autres: Vec<(TeamInfoDto, CoachId)> = (0..9)
            .map(|i| {
                let c = CoachId::new();
                (equipe(&format!("Équipe {i}"), &c, "Bob"), c)
            })
            .collect();

        let mut couples: Vec<(&TeamInfoDto, &CoachId)> = vec![(&mienne, &moi)];
        couples.extend(autres.iter().map(|(t, c)| (t, c)));
        let mut survey = campagne_sur(&jour, &couples);

        // Les neuf autres confirment ; le coach, lui, n'a pas répondu.
        for (t, c) in &autres {
            survey
                .enregistrer(
                    &TeamId::try_new(&t.team_id).unwrap(),
                    Venue::Presente,
                    Repondant::Coach(*c),
                    &etat_vierge(),
                    &date("2026-10-04"),
                )
                .expect("réponse acceptée");
        }

        let mut toutes = vec![mienne.clone()];
        toutes.extend(autres.iter().map(|(t, _)| t.clone()));
        let vues = hydrater_avec(
            &FauxSurveyRepo::avec(survey),
            &FauxJournees::avec(jour.clone()).et_la_saison(vec![jour]),
            &FauxTeams(toutes),
            &moi,
        );

        assert_eq!(
            vues[0].mes_equipes.len(),
            1,
            "une seule équipe est la sienne"
        );
        assert_eq!(
            vues[0].confirmes, 9,
            "le compte est celui de la campagne, pas de ce que le coach voit"
        );
    }

    /// R2 porte sur la **journée**, pas sur la compétition : deux journées peuvent
    /// être sondées en même temps. Taire l'une parce que l'autre existe ferait
    /// manquer une échéance.
    ///
    /// L'ordre est celui des échéances, la plus proche d'abord — c'est celle qui
    /// presse.
    #[test]
    fn deux_campagnes_ouvertes_donnent_deux_cartes_par_echeance_croissante() {
        let moi = CoachId::new();
        let (j1, j2) = (
            journee(MatchId::new(), MatchDayType::TimeFrame, vec![]),
            journee(MatchId::new(), MatchDayType::TimeFrame, vec![]),
        );
        let mienne = equipe("Les Miens", &moi, "Alice");
        let dests = vec![Destinataire {
            team_id: TeamId::try_new(&mienne.team_id).unwrap(),
            coach_id: moi,
        }];
        // La seconde échoit **plus tôt** que la première : si l'ordre était celui
        // de la lecture, ce test passerait sans rien prouver.
        let tardive = campagne_ouverte(&j1, &dests);
        let mut pressee = campagne_ouverte(&j2, &dests);
        pressee
            .rouvrir(echeance("2026-10-07"), &etat_vierge(), &date("2026-10-05"))
            .expect("échéance rapprochée");

        let vues = hydrater_avec(
            &FauxSurveyRepo::avec(tardive).et_aussi(pressee),
            &FauxJournees::avec(j1.clone()).et_la_saison(vec![j1, j2.clone()]),
            &FauxTeams(vec![mienne]),
            &moi,
        );

        assert_eq!(vues.len(), 2);
        assert_eq!(vues[0].round_id, j2.id, "l'échéance la plus proche d'abord");
        assert_eq!(vues[0].deadline.as_ref(), "2026-10-07");
    }

    /// Un coach qui n'engage aucune équipe dans la campagne ne voit rien — l'encart
    /// lui poserait une question sans réponse possible.
    #[test]
    fn un_coach_etranger_a_la_campagne_ne_voit_pas_d_encart() {
        let (moi, autre) = (CoachId::new(), CoachId::new());
        let jour = journee(MatchId::new(), MatchDayType::TimeFrame, vec![]);
        let sienne = equipe("Les Siens", &autre, "Bob");
        let survey = campagne_sur(&jour, &[(&sienne, &autre)]);

        let vues = hydrater_avec(
            &FauxSurveyRepo::avec(survey),
            &FauxJournees::avec(jour.clone()).et_la_saison(vec![jour]),
            &FauxTeams(vec![sienne]),
            &moi,
        );

        assert!(vues.is_empty());
    }
}

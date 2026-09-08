//! La persistance de la campagne, sur une vraie base.
//!
//! Ce qui est éprouvé ici ne repose sur aucune ligne de Rust : ce sont les
//! **contraintes** — trois index uniques et un `CHECK` — qui portent R1, R2 et
//! la cohérence de `Presence`. Un test qui mockerait sqlx ne vérifierait donc
//! rien du tout, d'où `#[sqlx::test]`, qui monte une base migrée par test.

use crate::app::competitions::domain::match_day::{
    MatchDay, MatchDayName, MatchDayPosition, MatchDayType,
};
use crate::app::competitions::domain::presence_survey::{
    AutoRemind, Destinataire, EtatJournee, Presence, PresenceSurvey, Repondant, ReponduLe,
    SurveyDeadline, SurveyId, Venue,
};
use crate::app::competitions::domain::presence_survey_repository_port::IPresenceSurveyRepository;
use crate::app::competitions::io::repository::presence_survey_repository::PresenceSurveyRepository;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::CoachId;

const SAISON: &str = "01KZVCKDG19DXZHJA295WSJGMV";

async fn poser_la_journee(pool: &sqlx::PgPool, id: &str, position: i32, day_type: &str) {
    sqlx::query(
        "INSERT INTO competition_match_days (id, season_id, name, day_type, position)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(SAISON)
    .bind(format!("Journée {}", position + 1))
    .bind(day_type)
    .bind(position)
    .execute(pool)
    .await
    .expect("journée de test");
}

fn journee(id: MatchId) -> MatchDay {
    MatchDay {
        id,
        season_id: SeasonId::try_new(SAISON).unwrap(),
        name: MatchDayName::try_new("Journée 1".to_string()).unwrap(),
        day_type: MatchDayType::FixedDate,
        date_start: None,
        date_end: None,
        position: MatchDayPosition::try_new(0).unwrap(),
        pairings: vec![],
    }
}

/// Une journée sans rapport publié et sans appariement — les faits qu'`enregistrer`
/// attend quand rien n'est encore tiré.
fn journee_vierge() -> EtatJournee {
    EtatJournee {
        figee: false,
        rencontres: vec![],
    }
}

/// Passe par le seul chemin d'écriture d'une `Presence` (carte 512) : ces tests
/// éprouvent la persistance, pas les règles, mais ils n'ont pas à les contourner.
fn declarer(
    survey: &mut PresenceSurvey,
    dests: &[Destinataire],
    rang: usize,
    venue: Venue,
    par: Repondant,
) {
    survey
        .enregistrer(
            &dests[rang].team_id,
            venue,
            par,
            &journee_vierge(),
            &DateString::try_new("2026-10-05".to_string()).unwrap(),
        )
        .expect("réponse acceptée");
}

fn campagne(round: &MatchDay, destinataires: &[Destinataire]) -> PresenceSurvey {
    PresenceSurvey::ouvrir(
        SurveyId::new(),
        SeasonId::try_new(SAISON).unwrap(),
        round,
        destinataires,
        SurveyDeadline::try_new("2026-10-10".to_string()).unwrap(),
        AutoRemind::new(true),
        &DateString::try_new("2026-10-01".to_string()).unwrap(),
    )
    .expect("ouverture")
}

fn destinataires(n: usize) -> Vec<Destinataire> {
    (0..n)
        .map(|_| Destinataire {
            team_id: TeamId::new(),
            coach_id: CoachId::new(),
        })
        .collect()
}

// ── R2 — un seul sondage vivant par journée ──────────────────────────────────

/// **La règle est tenue par la base, pas par le code applicatif.** Le use case
/// vérifiera aussi, mais il lit hors transaction : deux lancements simultanés
/// franchiraient tous deux sa garde. L'index, lui, ne se laisse pas doubler.
#[sqlx::test]
async fn une_seconde_campagne_sur_la_meme_journee_est_refusee(pool: sqlx::PgPool) {
    let round_id = MatchId::new();
    poser_la_journee(&pool, &round_id.to_string(), 0, "fixed_date").await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    let round = journee(round_id);

    depot
        .save(&campagne(&round, &destinataires(4)))
        .await
        .expect("la première s'écrit");

    let refus = depot.save(&campagne(&round, &destinataires(4))).await;

    assert!(refus.is_err(), "la seconde doit être refusée par l'index");
}

// ── Le CHECK, seul endroit où l'incohérence est interdite côté base ──────────

#[sqlx::test]
async fn une_reponse_declaree_sans_horodatage_est_refusee(pool: sqlx::PgPool) {
    let round_id = MatchId::new();
    poser_la_journee(&pool, &round_id.to_string(), 0, "fixed_date").await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    let survey = campagne(&journee(round_id), &destinataires(2));
    depot.save(&survey).await.expect("écriture");

    // Écriture directe : aucun chemin du domaine ne peut produire cette ligne,
    // c'est justement ce que le CHECK doit garantir.
    let refus = sqlx::query(
        "UPDATE competition_presence_answers SET presence = 'presente' WHERE survey_id = $1",
    )
    .bind(survey.id().to_string())
    .execute(&pool)
    .await;

    assert!(refus.is_err(), "présente sans repondu_le doit être refusée");
}

// ── L'aller-retour des trois états de `Presence` ─────────────────────────────

/// R6 — l'identifiant de l'organisateur survit à la persistance : c'est lui qui
/// distingue une réponse posée d'une réponse reçue, et la question « qui a dit
/// qu'il venait » se pose après le tirage, pas avant.
#[sqlx::test]
async fn les_trois_presences_reviennent_intactes(pool: sqlx::PgPool) {
    let round_id = MatchId::new();
    poser_la_journee(&pool, &round_id.to_string(), 0, "fixed_date").await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    let dests = destinataires(3);
    let mut survey = campagne(&journee(round_id), &dests);

    let admin = CoachId::new();
    declarer(
        &mut survey,
        &dests,
        0,
        Venue::Presente,
        Repondant::Organisateur(admin),
    );
    declarer(&mut survey, &dests, 1, Venue::Absente, Repondant::Jeton);

    depot.save(&survey).await.expect("écriture");
    let relue = depot
        .find_by_round(&round_id.to_string())
        .await
        .expect("lecture")
        .expect("campagne");

    assert_eq!(relue.compte_presents(), 1);
    assert_eq!(relue.compte_absents(), 1);
    assert_eq!(relue.compte_sans_reponse(), 1);

    let posee = relue
        .reponses()
        .iter()
        .find(|r| r.presence().compte_pour_le_tirage())
        .expect("la présente");
    assert!(
        matches!(posee.presence(), Presence::Declaree { par: Repondant::Organisateur(id), .. } if id == &admin),
        "l'identifiant de l'organisateur doit survivre à l'aller-retour"
    );
}

/// R28 — le canal ne se persiste pas : `Jeton` relu redevient
/// `Coach(coach_id de la réponse)`, ce qui est exactement vrai. La question
/// qu'on se pose après coup n'a que deux réponses possibles, pas trois.
#[sqlx::test]
async fn une_reponse_par_jeton_se_relit_comme_venant_du_coach(pool: sqlx::PgPool) {
    let round_id = MatchId::new();
    poser_la_journee(&pool, &round_id.to_string(), 0, "fixed_date").await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    let dests = destinataires(1);
    let coach = dests[0].coach_id;
    let mut survey = campagne(&journee(round_id), &dests);
    declarer(&mut survey, &dests, 0, Venue::Presente, Repondant::Jeton);

    depot.save(&survey).await.expect("écriture");
    let relue = depot
        .find_by_round(&round_id.to_string())
        .await
        .unwrap()
        .unwrap();

    assert!(matches!(
        relue.reponses()[0].presence(),
        Presence::Declaree { par: Repondant::Coach(id), .. } if id == &coach
    ));
}

// ── L'idempotence de `save` ──────────────────────────────────────────────────

#[sqlx::test]
async fn ecrire_deux_fois_laisse_le_meme_etat(pool: sqlx::PgPool) {
    let round_id = MatchId::new();
    poser_la_journee(&pool, &round_id.to_string(), 0, "fixed_date").await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    let survey = campagne(&journee(round_id), &destinataires(5));

    depot.save(&survey).await.expect("première écriture");
    depot.save(&survey).await.expect("seconde écriture");

    let lignes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM competition_presence_answers WHERE survey_id = $1",
    )
    .bind(survey.id().to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(lignes, 5, "cinq équipes, cinq lignes, pas dix");
}

/// Le jeton n'est **pas** réattribué à la réécriture : le réattribuer
/// invaliderait un lien déjà parti par e-mail, alors que R7 veut qu'il réponde
/// tant que la campagne vit.
#[sqlx::test]
async fn reecrire_une_campagne_ne_change_pas_les_jetons(pool: sqlx::PgPool) {
    let round_id = MatchId::new();
    poser_la_journee(&pool, &round_id.to_string(), 0, "fixed_date").await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    let survey = campagne(&journee(round_id), &destinataires(3));
    depot.save(&survey).await.unwrap();

    let avant: Vec<String> =
        sqlx::query_scalar("SELECT token FROM competition_presence_answers ORDER BY team_id")
            .fetch_all(&pool)
            .await
            .unwrap();
    depot.save(&survey).await.unwrap();
    let apres: Vec<String> =
        sqlx::query_scalar("SELECT token FROM competition_presence_answers ORDER BY team_id")
            .fetch_all(&pool)
            .await
            .unwrap();

    assert_eq!(avant, apres);
}

// ── La barre latérale ────────────────────────────────────────────────────────

/// Une journée sans campagne **figure quand même** dans la liste : la faire
/// disparaître laisserait croire à un trou dans le calendrier.
#[sqlx::test]
async fn les_journees_sans_campagne_figurent_dans_le_resume(pool: sqlx::PgPool) {
    let sondee = MatchId::new();
    let vierge = MatchId::new();
    let repos = MatchId::new();
    poser_la_journee(&pool, &sondee.to_string(), 0, "fixed_date").await;
    poser_la_journee(&pool, &vierge.to_string(), 1, "fixed_date").await;
    poser_la_journee(&pool, &repos.to_string(), 2, "rest").await;

    let depot = PresenceSurveyRepository::new(pool.clone());
    let dests = destinataires(4);
    let mut survey = campagne(&journee(sondee), &dests);
    declarer(&mut survey, &dests, 0, Venue::Presente, Repondant::Jeton);
    declarer(&mut survey, &dests, 1, Venue::Absente, Repondant::Jeton);
    depot.save(&survey).await.unwrap();

    let resume = depot.list_summaries(SAISON).await.expect("résumé");

    assert_eq!(resume.len(), 3, "les trois journées, sondées ou non");
    assert_eq!(resume[0].attendues, 4);
    assert_eq!(resume[0].reponses, 2);
    assert_eq!(resume[0].presents, 1);
    assert!(resume[0].deadline.is_some());

    assert_eq!(resume[1].attendues, 0, "aucune campagne, aucune attendue");
    assert!(resume[1].deadline.is_none());
    assert!(!resume[1].is_rest);
    assert!(
        resume[2].is_rest,
        "la journée de repos est signalée comme telle"
    );
}

/// Le résumé ne calcule aucun statut : il rend `deadline` et `close_le` bruts,
/// et `statut_de` tranche. Le produire en SQL mettrait R23 à deux endroits.
#[sqlx::test]
async fn le_resume_rend_les_dates_brutes_et_pas_un_statut(pool: sqlx::PgPool) {
    let round_id = MatchId::new();
    poser_la_journee(&pool, &round_id.to_string(), 0, "fixed_date").await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    depot
        .save(&campagne(&journee(round_id), &destinataires(2)))
        .await
        .unwrap();

    let resume = depot.list_summaries(SAISON).await.unwrap();

    assert_eq!(resume[0].deadline.as_deref(), Some("2026-10-10"));
    assert_eq!(resume[0].close_le, None, "aucune clôture décidée");
    assert_eq!(resume[0].appariee, Some(false));
}

// ── Une réponse hors de sa campagne n'existe pas ─────────────────────────────

#[sqlx::test]
async fn une_journee_sans_campagne_ne_rend_rien(pool: sqlx::PgPool) {
    let round_id = MatchId::new();
    poser_la_journee(&pool, &round_id.to_string(), 0, "fixed_date").await;
    let depot = PresenceSurveyRepository::new(pool);

    let rien = depot
        .find_by_round(&round_id.to_string())
        .await
        .expect("lecture");

    assert!(
        rien.is_none(),
        "None, pas une erreur : c'est ce qui distingue une journée non sondée d'une panne"
    );
}

/// Le `ReponduLe` sert au test seul — il documente que l'horodatage traverse.
#[allow(dead_code)]
fn _horodatage() -> ReponduLe {
    ReponduLe::try_new("2026-10-05".to_string()).unwrap()
}

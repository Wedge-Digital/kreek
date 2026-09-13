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
    SurveyDeadline, SurveyId, SurveyToken, Venue,
};
use crate::app::competitions::domain::presence_survey_repository_port::IPresenceSurveyRepository;
use crate::app::competitions::io::repository::presence_survey_repository::PresenceSurveyRepository;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::CoachId;

const SAISON: &str = "01KZVCKDG19DXZHJA295WSJGMV";

/// La compétition et la saison auxquelles la journée appartient.
///
/// `poser_la_journee` suffisait tant qu'on ne lisait que la campagne ; les
/// libellés de la page publique joignent trois tables de `competitions`, et elles
/// doivent exister.
async fn poser_la_competition(pool: &sqlx::PgPool) {
    let competition_id = "01KZVCKDG19DXZHJA295WSJGMW";
    sqlx::query(
        "INSERT INTO competitions (id, space_id, name, logo)
         VALUES ($1, $2, 'Compétition de test', '')
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(competition_id)
    .bind("01KZVCKDG19DXZHJA295WSJGMX")
    .execute(pool)
    .await
    .expect("compétition de test");

    sqlx::query(
        "INSERT INTO competition_seasons (id, competition_id, name, status)
         VALUES ($1, $2, 'Saison de test', 'draft')
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(SAISON)
    .bind(competition_id)
    .execute(pool)
    .await
    .expect("saison de test");
}

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

/// La même campagne, échéance choisie. `campagne` fige la sienne au 10 octobre,
/// ce qui suffit aux lectures unitaires ; la liste des campagnes ouvertes, elle,
/// se juge précisément sur cette date.
fn campagne_echeant(
    round: &MatchDay,
    destinataires: &[Destinataire],
    deadline: &str,
) -> PresenceSurvey {
    PresenceSurvey::ouvrir(
        SurveyId::new(),
        SeasonId::try_new(SAISON).unwrap(),
        round,
        destinataires,
        SurveyDeadline::try_new(deadline.to_string()).unwrap(),
        AutoRemind::new(true),
        &DateString::try_new("2026-10-01".to_string()).unwrap(),
    )
    .expect("ouverture")
}

/// Pose une journée en base et rend l'agrégat correspondant — les deux vont
/// toujours ensemble, et les séparer a déjà produit des campagnes orphelines.
async fn journee_posee(pool: &sqlx::PgPool, position: i32) -> MatchDay {
    let id = MatchId::new();
    poser_la_journee(pool, &id.to_string(), position, "time_frame").await;
    journee(id)
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

// ── Le jeton, chemin de la route publique (carte 523) ────────────────────────

/// La campagne d'un jeton vient avec **toutes** ses réponses, pas seulement celle
/// que le jeton désigne : l'agrégat n'existe pas à moitié, et `enregistrer`
/// vérifie R19 sur l'ensemble des destinataires.
#[sqlx::test]
async fn un_jeton_rend_la_campagne_avec_toutes_ses_reponses(pool: sqlx::PgPool) {
    let round_id = MatchId::new();
    poser_la_journee(&pool, &round_id.to_string(), 0, "fixed_date").await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    let dests = destinataires(3);
    let survey = campagne(&journee(round_id), &dests);
    let jeton = survey.reponses()[1].token().to_string();
    depot.save(&survey).await.expect("écriture");

    let relue = depot
        .find_by_token(&jeton)
        .await
        .expect("lecture")
        .expect("campagne");

    assert_eq!(relue.id(), survey.id());
    assert_eq!(
        relue.reponses().len(),
        3,
        "les trois réponses, pas seulement celle du jeton"
    );
    assert_eq!(
        relue.reponse_par_jeton(survey.reponses()[1].token()),
        relue.reponse_de(&dests[1].team_id),
        "le jeton désigne bien la réponse de son équipe"
    );
}

/// **Un jeton inconnu rend `Ok(None)`, jamais une erreur.**
///
/// C'est cette distinction qui produit la page « lien inconnu » (R26) plutôt qu'un
/// `500` — et un jeton tronqué par un client mail est un cas courant, pas une
/// panne.
///
/// Le jeton mal formé est éprouvé avec : R26 veut que la page publique ne révèle
/// **jamais** si un jeton a existé, et deux issues distinctes ici donneraient deux
/// réponses distinctes là-bas.
#[sqlx::test]
async fn un_jeton_inconnu_ou_mal_forme_rend_none_et_non_une_erreur(pool: sqlx::PgPool) {
    let round_id = MatchId::new();
    poser_la_journee(&pool, &round_id.to_string(), 0, "fixed_date").await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    depot
        .save(&campagne(&journee(round_id), &destinataires(2)))
        .await
        .expect("écriture");

    for jeton in [
        SurveyToken::new().to_string(),
        "pas-un-jeton".to_string(),
        String::new(),
    ] {
        let issue = depot.find_by_token(&jeton).await;
        assert!(
            matches!(issue, Ok(None)),
            "« {jeton} » doit rendre Ok(None), et non une erreur"
        );
    }
}

/// Les libellés de la page publique, et **rien du BC `teams`** : le nom de
/// l'équipe vient d'`ITeamInfoPort`, une jointure vers ses tables étant l'exacte
/// violation que la souveraineté des données nomme.
#[sqlx::test]
async fn les_libelles_de_la_page_publique_viennent_des_tables_de_competitions(pool: sqlx::PgPool) {
    let round_id = MatchId::new();
    poser_la_competition(&pool).await;
    poser_la_journee(&pool, &round_id.to_string(), 2, "fixed_date").await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    let survey = campagne(&journee(round_id), &destinataires(1));
    let jeton = survey.reponses()[0].token().to_string();
    depot.save(&survey).await.expect("écriture");

    let libelles = depot
        .find_landing_labels(&jeton)
        .await
        .expect("lecture")
        .expect("libellés");

    assert_eq!(libelles.round_name, "Journée 3");
    assert_eq!(libelles.competition_name, "Compétition de test");
    assert_eq!(libelles.season_id, SAISON);
    assert!(!libelles.space_id.is_empty());
}

/// **Rien n'est inventé** : une journée sans date rend `None` sur les deux bornes,
/// et la vue n'affichera pas la ligne. Un repli — « date à définir » — se
/// confondrait avec une date saisie.
#[sqlx::test]
async fn une_journee_sans_date_rend_deux_bornes_vides(pool: sqlx::PgPool) {
    let round_id = MatchId::new();
    poser_la_competition(&pool).await;
    poser_la_journee(&pool, &round_id.to_string(), 2, "time_frame").await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    let survey = campagne(&journee(round_id), &destinataires(1));
    let jeton = survey.reponses()[0].token().to_string();
    depot.save(&survey).await.expect("écriture");

    let libelles = depot
        .find_landing_labels(&jeton)
        .await
        .expect("lecture")
        .expect("libellés");

    assert_eq!(libelles.round_date_start, None);
    assert_eq!(libelles.round_date_end, None);
}

#[sqlx::test]
async fn un_jeton_inconnu_n_a_pas_de_libelles(pool: sqlx::PgPool) {
    let depot = PresenceSurveyRepository::new(pool.clone());

    let issue = depot
        .find_landing_labels(&SurveyToken::new().to_string())
        .await;

    assert!(matches!(issue, Ok(None)));
}

// ── Les campagnes ouvertes d'une saison — carte 529 ──────────────────────────

/// L'élagage SQL et `statut()` disent la même chose : une campagne échue hier ne
/// remonte pas.
#[sqlx::test]
async fn une_campagne_echue_de_la_veille_n_est_pas_rendue(pool: sqlx::PgPool) {
    poser_la_competition(&pool).await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    let jour = journee_posee(&pool, 1).await;
    depot
        .save(&campagne_echeant(&jour, &destinataires(2), "2026-10-10"))
        .await
        .expect("écriture");

    let ouvertes = depot
        .list_open_surveys_for_season(SAISON, "2026-10-11")
        .await
        .expect("lecture");

    assert!(ouvertes.is_empty());
}

/// **Le test du `>=`, et le seul qui le voit.**
///
/// `statut_de` ferme sur `aujourd_hui > deadline` : une campagne échéant le 10
/// répond encore le 10. Un `>` dans le SQL l'écarterait — et ce serait le seul
/// sens dangereux de ce compromis, celui où le domaine n'a plus rien à rattraper
/// parce que la ligne n'est jamais arrivée.
#[sqlx::test]
async fn une_campagne_echeant_aujourd_hui_repond_encore(pool: sqlx::PgPool) {
    poser_la_competition(&pool).await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    let jour = journee_posee(&pool, 1).await;
    depot
        .save(&campagne_echeant(&jour, &destinataires(2), "2026-10-10"))
        .await
        .expect("écriture");

    let ouvertes = depot
        .list_open_surveys_for_season(SAISON, "2026-10-10")
        .await
        .expect("lecture");

    assert_eq!(
        ouvertes.len(),
        1,
        "le jour de l'échéance, la campagne répond"
    );
    assert!(ouvertes[0]
        .statut(&DateString::try_new("2026-10-10".to_string()).unwrap())
        .est_ouverte());
}

/// R23 — la décision ferme, quelle que soit l'échéance. L'échéance est ici **à
/// venir** : une requête qui ne filtrerait que sur la date la laisserait passer.
#[sqlx::test]
async fn une_campagne_close_par_decision_n_est_pas_rendue(pool: sqlx::PgPool) {
    poser_la_competition(&pool).await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    let jour = journee_posee(&pool, 1).await;
    let mut survey = campagne_echeant(&jour, &destinataires(2), "2026-12-31");
    survey
        .clore(&DateString::try_new("2026-10-05".to_string()).unwrap())
        .expect("clôture");
    depot.save(&survey).await.expect("écriture");

    let ouvertes = depot
        .list_open_surveys_for_season(SAISON, "2026-10-06")
        .await
        .expect("lecture");

    assert!(ouvertes.is_empty(), "close par décision, échéance à venir");
}

/// Elle rend des **agrégats** : l'encart a besoin de la réponse de chaque équipe
/// et de son horodatage, pas d'un compte.
#[sqlx::test]
async fn une_campagne_ouverte_revient_avec_toutes_ses_reponses(pool: sqlx::PgPool) {
    poser_la_competition(&pool).await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    let jour = journee_posee(&pool, 1).await;
    let dests = destinataires(3);
    let mut survey = campagne_echeant(&jour, &dests, "2026-12-31");
    declarer(&mut survey, &dests, 0, Venue::Presente, Repondant::Jeton);
    depot.save(&survey).await.expect("écriture");

    let ouvertes = depot
        .list_open_surveys_for_season(SAISON, "2026-10-06")
        .await
        .expect("lecture");

    assert_eq!(ouvertes.len(), 1);
    assert_eq!(
        ouvertes[0].reponses().len(),
        3,
        "les trois, pas la seule posée"
    );
    assert_eq!(ouvertes[0].presents().len(), 1);
    // L'horodatage traverse, et c'est lui que l'encart affiche : « répondu le … ».
    assert!(matches!(
        ouvertes[0].presents()[0].presence(),
        Presence::Declaree { .. }
    ));
}

#[sqlx::test]
async fn deux_campagnes_ouvertes_reviennent_toutes_les_deux(pool: sqlx::PgPool) {
    poser_la_competition(&pool).await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    for (rang, jour) in [journee_posee(&pool, 1).await, journee_posee(&pool, 2).await]
        .iter()
        .enumerate()
    {
        depot
            .save(&campagne_echeant(
                jour,
                &destinataires(rang + 1),
                "2026-12-31",
            ))
            .await
            .expect("écriture");
    }

    let ouvertes = depot
        .list_open_surveys_for_season(SAISON, "2026-10-06")
        .await
        .expect("lecture");

    assert_eq!(ouvertes.len(), 2);
}

/// **Le test que le regroupement rend nécessaire.**
///
/// Les réponses des deux campagnes arrivent dans une seule requête, et c'est leur
/// `survey_id` qui les répartit. Un regroupement fautif donnerait à l'une les
/// réponses de l'autre — ou les deux à chacune — et aucun test sur une campagne
/// unique ne le verrait.
#[sqlx::test]
async fn les_reponses_ne_se_melangent_pas_entre_campagnes(pool: sqlx::PgPool) {
    poser_la_competition(&pool).await;
    let depot = PresenceSurveyRepository::new(pool.clone());
    let (une, deux) = (journee_posee(&pool, 1).await, journee_posee(&pool, 2).await);
    let dests_une = destinataires(1);
    let dests_deux = destinataires(3);
    depot
        .save(&campagne_echeant(&une, &dests_une, "2026-12-31"))
        .await
        .expect("écriture");
    depot
        .save(&campagne_echeant(&deux, &dests_deux, "2026-12-31"))
        .await
        .expect("écriture");

    let ouvertes = depot
        .list_open_surveys_for_season(SAISON, "2026-10-06")
        .await
        .expect("lecture");

    // Chaque campagne retrouve **ses** équipes, et non le total des deux.
    for campagne in &ouvertes {
        let attendues: &[Destinataire] = if campagne.round_id() == &une.id {
            &dests_une
        } else {
            &dests_deux
        };
        assert_eq!(campagne.reponses().len(), attendues.len());
        for reponse in campagne.reponses() {
            assert!(
                attendues.iter().any(|d| &d.team_id == reponse.team_id()),
                "une réponse d'une autre campagne s'est glissée ici"
            );
        }
    }
}

/// Une saison sans campagne ouverte ne fait **pas** d'aller-retour pour ses
/// réponses : c'est le cas courant sur la page de détail, et le court-circuit sur
/// la liste vide est là pour lui.
#[sqlx::test]
async fn une_saison_sans_campagne_ouverte_rend_une_liste_vide(pool: sqlx::PgPool) {
    poser_la_competition(&pool).await;
    let depot = PresenceSurveyRepository::new(pool.clone());

    let ouvertes = depot
        .list_open_surveys_for_season(SAISON, "2026-10-06")
        .await
        .expect("lecture");

    assert!(ouvertes.is_empty());
}

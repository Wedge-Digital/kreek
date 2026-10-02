//! L'ajustement de trésorerie, de la garde au grand livre (carte 559).
//!
//! # Ce que ces tests prouvent, et qu'aucun test unitaire ne voyait
//!
//! Que la règle est **branchée**. `is_space_admin` a ses tests, le domaine a
//! les siens, mais rien ne disait jusqu'ici que le bouton disparaît pour un
//! membre simple ni que son POST est refusé. C'est le routeur de production qui
//! est monté ici, avec sa garde et son extracteur.
//!
//! # Les deux moitiés
//!
//! Un test qui n'assène qu'un `403` passerait aussi bien si la route n'existait
//! pas. Chaque garde est donc éprouvée des deux côtés : le membre simple est
//! refusé, et l'administrateur **traverse et écrit vraiment**.

use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RosterId, SeasonId};
use crate::app::shared_kernel::bloodbowl::staff_counts::{
    ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::shared_kernel::identity::sulid::SUlid;
use crate::app::teams::domain::team::TeamDomainEvent;
use crate::app::teams::domain::value_objects::{DedicatedFans, Kpo, RosterName, TeamName};
use crate::app::teams::ports::ITeamRepository;
use crate::app::teams::routes::Routes;
use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

const MEMBRE_SIMPLE: &str = crate::cli::seed_e2e::SIMPLE_COACH_NAME;
const COMMISSAIRE: &str = crate::cli::seed_e2e::DEV_COACH_NAME;

/// Une équipe à 1000 kPo dans l'espace E2E, semée **par le dépôt** : une équipe
/// posée en projection seule rendrait `404`, et les refus se liraient alors
/// pour une raison étrangère.
async fn equipe(pool: &sqlx::PgPool) -> (String, String) {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");

    let (space_id,): (String,) =
        sqlx::query_as("SELECT id FROM spaces WHERE space_name = 'Espace E2E'")
            .fetch_one(pool)
            .await
            .expect("espace E2E semé");
    let (proprietaire,): (String,) =
        sqlx::query_as("SELECT id FROM auth__users WHERE coach_name = 'E2E Coach 02'")
            .fetch_one(pool)
            .await
            .expect("coach propriétaire semé");

    let team_id = SUlid::new().to_string();
    let repo = crate::app::teams::io::repository::team_repository::TeamRepository::new(
        pool.clone(),
        crate::common::services::event_bus::event_bus::new_bus(),
    );
    repo.append(
        &team_id,
        &TeamDomainEvent::TeamCreated {
            team_id: TeamId::try_new(&team_id).unwrap(),
            space_id: SpaceId::try_new(&space_id).unwrap(),
            competition_id: CompetitionId::try_new(&SUlid::new().to_string()).unwrap(),
            competition_name: "Ligue de Condate".to_string(),
            season_id: SeasonId::try_new(&SUlid::new().to_string()).unwrap(),
            season_name: "Saison 2025".to_string(),
            name: TeamName::try_new("Les Korrigans FC".to_string()).unwrap(),
            logo_url: None,
            roster_id: RosterId::try_new(&SUlid::new().to_string()).unwrap(),
            roster_name: RosterName::try_new("Elfes Sylvestres".to_string()).unwrap(),
            coach_id: CoachId::try_new(&proprietaire).unwrap(),
            coach_name: "E2E Coach 02".to_string(),
            treasury: Kpo(1000),
            dedicated_fans: DedicatedFans::try_new(2).unwrap(),
            rerolls: RerollCount(3),
            apothecaries: ApothecaryCount(1),
            assistants: AssistantCount(2),
            cheerleaders: CheerleaderCount(3),
        },
        0,
    )
    .await
    .expect("équipe semée par le dépôt");

    (space_id, team_id)
}

fn formulaire(sens: &str, montant: u32, motif: &str) -> String {
    format!(
        "direction={sens}&amount_kpo={montant}&note={}",
        urlencoding::encode(motif)
    )
}

/// **La carte 500 retire les boutons, pas la page.** Le relevé reste lisible ;
/// c'est le panneau qui disparaît.
#[sqlx::test]
async fn un_membre_simple_lit_le_releve_sans_son_panneau(pool: sqlx::PgPool) {
    let (space, team) = equipe(&pool).await;
    let membre = Harnais::connecte_en_tant_que(pool, MEMBRE_SIMPLE).await;

    let r = membre.get(&Routes.team_treasury(&space, &team)).await;

    assert_eq!(r.statut, StatusCode::OK, "le relevé reste lisible");
    assert!(
        r.corps.contains("Relevé des mouvements") || r.corps.contains("Aucun mouvement"),
        "le relevé doit bien être rendu"
    );
    assert!(
        !r.corps.contains("tr-btn-adjust"),
        "un membre simple ne doit pas voir le bouton"
    );
    assert!(
        !r.corps.contains("ajustementTresorerie"),
        "ni l'Alpine du panneau"
    );
}

/// La contre-épreuve : sans elle, le test ci-dessus passerait aussi bien si le
/// panneau n'était rendu pour personne.
#[sqlx::test]
async fn un_commissaire_voit_le_panneau_et_sa_route(pool: sqlx::PgPool) {
    let (space, team) = equipe(&pool).await;
    let admin = Harnais::connecte_en_tant_que(pool, COMMISSAIRE).await;

    let r = admin.get(&Routes.team_treasury(&space, &team)).await;

    assert_eq!(r.statut, StatusCode::OK);
    assert!(r.corps.contains("tr-btn-adjust"), "le bouton doit être là");
    assert!(
        r.corps
            .contains(&Routes.team_treasury_adjust(&space, &team)),
        "le formulaire doit porter sa route"
    );
}

#[sqlx::test]
async fn un_membre_simple_ne_peut_pas_ajuster(pool: sqlx::PgPool) {
    let (space, team) = equipe(&pool).await;
    let membre = Harnais::connecte_en_tant_que(pool, MEMBRE_SIMPLE).await;

    let r = membre
        .post_htmx(
            &Routes.team_treasury_adjust(&space, &team),
            &formulaire("Credit", 120, "Compensation"),
        )
        .await;

    assert_eq!(r.statut, StatusCode::FORBIDDEN);
}

/// **200 et non 4xx**, et c'est le point du test : htmx n'échange pas une
/// réponse non-2xx par défaut. Un 422 n'afficherait rien, et le bouton
/// paraîtrait sans effet.
#[sqlx::test]
async fn un_montant_invalide_rend_un_message_et_non_une_erreur_http(pool: sqlx::PgPool) {
    let (space, team) = equipe(&pool).await;
    let admin = Harnais::connecte_en_tant_que(pool, COMMISSAIRE).await;

    let r = admin
        .post_htmx(
            &Routes.team_treasury_adjust(&space, &team),
            &formulaire("Credit", 123, "Compensation"),
        )
        .await;

    assert_eq!(r.statut, StatusCode::OK, "sinon htmx n'échange rien");
    assert_eq!(r.entete("HX-Retarget"), Some("#adm-msg"));
    assert_eq!(
        r.entete("HX-Reselect"),
        Some("#adm-msg"),
        "sans lui, le hx-select du formulaire filtrerait la réponse et rien ne s'afficherait"
    );
    assert!(r.corps.contains("multiple de 5"), "corps : {}", r.corps);
}

/// Le motif vide est refusé **par le serveur**, pas seulement par Alpine.
#[sqlx::test]
async fn un_motif_vide_est_refuse_par_le_serveur(pool: sqlx::PgPool) {
    let (space, team) = equipe(&pool).await;
    let admin = Harnais::connecte_en_tant_que(pool, COMMISSAIRE).await;

    let r = admin
        .post_htmx(
            &Routes.team_treasury_adjust(&space, &team),
            &formulaire("Credit", 120, "   "),
        )
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    assert!(
        r.corps.contains("motif est obligatoire"),
        "corps : {}",
        r.corps
    );
}

/// Un retrait que la caisse ne couvre pas : le domaine refuse, et le message
/// arrive au panneau plutôt qu'en code HTTP.
#[sqlx::test]
async fn un_retrait_non_couvert_rend_le_refus_du_domaine(pool: sqlx::PgPool) {
    let (space, team) = equipe(&pool).await;
    let admin = Harnais::connecte_en_tant_que(pool, COMMISSAIRE).await;

    let r = admin
        .post_htmx(
            &Routes.team_treasury_adjust(&space, &team),
            // L'équipe a 1000 kPo ; le plafond de 500 interdit d'en demander
            // plus, donc on vide d'abord la caisse par deux retraits.
            &formulaire("Debit", 500, "Sanction"),
        )
        .await;
    assert_eq!(r.statut, StatusCode::OK);

    let r = admin
        .post_htmx(
            &Routes.team_treasury_adjust(&space, &team),
            &formulaire("Debit", 500, "Sanction"),
        )
        .await;
    assert_eq!(r.statut, StatusCode::OK);

    let r = admin
        .post_htmx(
            &Routes.team_treasury_adjust(&space, &team),
            &formulaire("Debit", 5, "Sanction de trop"),
        )
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    assert_eq!(r.entete("HX-Retarget"), Some("#adm-msg"));
    assert!(
        r.corps.contains("ne couvre pas"),
        "le refus du domaine doit arriver au panneau — corps : {}",
        r.corps
    );
}

/// **Le test de bout en bout.** Il frappe le routeur de production, et relit la
/// page rendue : le solde doit avoir bougé **dans le bandeau et dans l'en-tête
/// de fiche**, qui vit hors de la zone d'onglets. C'est pour lui que la réponse
/// vise `#app-content` et non `#team-tab-zone`.
#[sqlx::test]
async fn un_credit_monte_le_solde_aux_deux_endroits_qui_l_affichent(pool: sqlx::PgPool) {
    let (space, team) = equipe(&pool).await;
    let admin = Harnais::connecte_en_tant_que(pool, COMMISSAIRE).await;

    let r = admin
        .post_htmx(
            &Routes.team_treasury_adjust(&space, &team),
            &formulaire("Credit", 120, "Forfait des Griffons — journée 3"),
        )
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    assert!(
        r.entete("HX-Retarget").is_none(),
        "un succès ne retarge rien : il rend la page entière"
    );
    assert_eq!(
        r.corps.matches("1120 kPo").count(),
        3,
        "le solde du bandeau, le terme de l'équation, et l'en-tête de fiche — corps : {}",
        r.corps
    );
    assert!(
        r.corps.contains("Ajustement de trésorerie"),
        "la ligne doit apparaître au relevé"
    );
    assert!(
        r.corps.contains("Forfait des Griffons — journée 3"),
        "avec son motif"
    );
}

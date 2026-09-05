//! Les actions d'équipe sont réservées à qui de droit (carte 501).
//!
//! # Ce que ce test prouve, et qu'aucun test unitaire ne pouvait prouver
//!
//! La règle — « propriétaire, ou admin d'espace, ou admin de compétition » —
//! a ses six tests unitaires depuis la carte 389, et ils passaient déjà quand
//! le défaut était là. Ce qui manquait, c'est que la règle soit **branchée**
//! sur les dix-huit routes qui agissent sur une équipe : elle ne l'était que
//! sur une, `costly-mistakes/roll`.
//!
//! Ce test monte donc le **routeur de production** et frappe les dix-huit. Un
//! oubli de câblage — une route restée dans `routes_ouvertes()` — se voit
//! immédiatement ; un test unitaire du middleware ne l'aurait jamais vu.
//!
//! # La contre-épreuve fait la moitié du travail
//!
//! Un test qui n'assène qu'un `403` passe aussi bien quand la route n'existe
//! pas, quand le `space_id` est mal formé, ou quand la phase ne s'y prête pas —
//! les trois se sont produits dans ce dépôt. Chaque route est donc frappée
//! deux fois : par un tiers, qui doit recevoir `403`, et par un ayant droit,
//! qui doit recevoir **n'importe quoi d'autre**.
//!
//! Ce « n'importe quoi d'autre » est le plus souvent un `422` de phase ou de
//! formulaire absent, et c'est exactement ce qu'on veut : la requête a
//! traversé la garde et atteint le handler. Vérifier un `200` demanderait de
//! semer chaque phase et chaque corps de formulaire, pour prouver quelque
//! chose que ce test ne cherche pas.

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

/// Le tiers de l'histoire : membre simple de l'espace, propriétaire de rien.
const TIERS: &str = crate::cli::seed_e2e::SIMPLE_COACH_NAME;
/// L'ayant droit : administrateur de l'espace, sans être propriétaire de
/// l'équipe. C'est la branche intéressante — la propriété court-circuite les
/// deux autres questions, et l'exercer ne prouverait rien du câblage des ports.
const AYANT_DROIT: &str = crate::cli::seed_e2e::DEV_COACH_NAME;

/// Une équipe de l'espace E2E, appartenant à un coach qui n'est ni l'un ni
/// l'autre des deux protagonistes.
///
/// Semée **par le dépôt**, et non par un `INSERT` : `find_by_id` hydrate
/// l'agrégat depuis l'event store, et une équipe posée en projection seule
/// rendrait `404` — l'assertion du tiers serait alors `404 != 403`, rouge pour
/// une raison étrangère, et celle de l'ayant droit verte sans rien prouver.
async fn equipe_d_un_autre(pool: &sqlx::PgPool) -> (String, String) {
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

/// Les dix-huit routes du groupe gardé, construites par `Routes` — et non
/// écrites à la main. Un chemin renommé sans que le test suive ferait frapper
/// une route inexistante, dont le `404` se lirait comme un refus.
fn routes_gardees(
    space: &str,
    team: &str,
) -> (Vec<(&'static str, String)>, Vec<(&'static str, String)>) {
    let r = Routes;
    let lectures = vec![
        ("recruitment_page", r.recruitment_page(space, team)),
        (
            "recruitment_catalog",
            r.recruitment_catalog_widget(space, team),
        ),
        ("recruitment_cart", r.recruitment_cart_widget(space, team)),
        ("dismissals_page", r.dismissals_page(space, team)),
        ("dismissals_roster", r.dismissals_roster_widget(space, team)),
        ("dismissals_cart", r.dismissals_cart_widget(space, team)),
        ("costly_mistakes_page", r.costly_mistakes_page(space, team)),
    ];
    let ecritures = vec![
        (
            "validate_improvement",
            r.validate_improvement_phase(space, team),
        ),
        (
            "validate_recruitment",
            r.validate_recruitment_phase(space, team),
        ),
        (
            "validate_dismissals",
            r.validate_dismissals_phase(space, team),
        ),
        ("costly_mistakes_roll", r.costly_mistakes_roll(space, team)),
        ("add_player", r.recruitment_add_player(space, team)),
        ("remove_player", r.recruitment_remove_player(space, team)),
        ("add_staff", r.recruitment_add_staff(space, team)),
        ("remove_staff", r.recruitment_remove_staff(space, team)),
        ("mark_player", r.dismissals_mark_player(space, team)),
        ("unmark_player", r.dismissals_unmark_player(space, team)),
        ("mark_staff", r.dismissals_mark_staff(space, team)),
        ("unmark_staff", r.dismissals_unmark_staff(space, team)),
    ];
    (lectures, ecritures)
}

#[sqlx::test]
async fn un_coach_tiers_ne_peut_agir_sur_aucune_route_d_action(pool: sqlx::PgPool) {
    let (space, team) = equipe_d_un_autre(&pool).await;
    let (lectures, ecritures) = routes_gardees(&space, &team);
    let tiers = Harnais::connecte_en_tant_que(pool, TIERS).await;

    for (nom, url) in &lectures {
        assert_eq!(
            tiers.get(url).await.statut,
            StatusCode::FORBIDDEN,
            "GET {nom} : un membre simple ne doit pas atteindre cet écran"
        );
    }
    for (nom, url) in &ecritures {
        assert_eq!(
            tiers.post_htmx(url, "").await.statut,
            StatusCode::FORBIDDEN,
            "POST {nom} : un membre simple ne doit pas pouvoir agir"
        );
    }
}

/// La contre-épreuve. Sans elle, le test ci-dessus passerait aussi bien si les
/// dix-huit routes avaient été fermées à tout le monde.
#[sqlx::test]
async fn un_administrateur_de_l_espace_traverse_la_garde(pool: sqlx::PgPool) {
    let (space, team) = equipe_d_un_autre(&pool).await;
    let (lectures, ecritures) = routes_gardees(&space, &team);
    let admin = Harnais::connecte_en_tant_que(pool, AYANT_DROIT).await;

    for (nom, url) in lectures.iter().chain(ecritures.iter()) {
        let statut = admin.get(url).await.statut;
        assert_ne!(
            statut,
            StatusCode::FORBIDDEN,
            "{nom} : un admin d'espace doit traverser la garde (reçu {statut})"
        );
        assert_ne!(
            statut,
            StatusCode::UNAUTHORIZED,
            "{nom} : la session doit être reconnue (reçu {statut})"
        );
    }
}

/// Ce qui reste ouvert doit le rester : la carte 500 retire les boutons de la
/// fiche, elle n'en interdit pas la lecture.
#[sqlx::test]
async fn la_fiche_d_equipe_reste_lisible_par_un_tiers(pool: sqlx::PgPool) {
    let (space, team) = equipe_d_un_autre(&pool).await;
    let r = Routes;
    let tiers = Harnais::connecte_en_tant_que(pool, TIERS).await;

    for (nom, url) in [
        ("team_detail", r.team_detail(&space, &team)),
        ("team_treasury", r.team_treasury(&space, &team)),
        ("team_matches", r.team_matches(&space, &team)),
    ] {
        assert_eq!(
            tiers.get(&url).await.statut,
            StatusCode::OK,
            "{nom} : la lecture de la fiche n'est pas gardée"
        );
    }
}

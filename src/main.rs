extern crate core;

mod app;
mod cli;
#[allow(special_module_name)]
pub mod common;
mod config;
mod infrastructure;
mod state;
pub mod web;

use config::{AppConfig, EmailConfig, EmailProvider};
use state::AppState;
use std::path::Path;
use std::time::Duration;

use crate::app::auth::auth_backend::AuthBackend;
use crate::app::auth::context::AuthContext;
use crate::app::auth::routes::path;
use crate::app::competitions::context::CompetitionsContext;
use crate::app::news::context::NewsContext;
use crate::app::players::context::PlayersContext;
use crate::app::ranking::context::RankingContext;
use crate::app::references::context::ReferencesContext;
use crate::app::spaces::context::SpacesContext;
use crate::app::team_creation::context::TeamCreationContext;
use crate::app::teams::context::TeamsContext;
use crate::app::{
    auth, competitions, match_report, players, ranking, spaces, team_creation, teams,
};
use crate::common::event_listener::event_log_feeder;
use crate::common::services::email::{ConsoleEmailService, IEmailService, ResendMailService};
use crate::common::services::event_bus::event_bus::new_bus;
use crate::common::services::observability::use_case_journal::UseCaseJournal;
use crate::common::session_store::DashMapStore;
use crate::infrastructure::spaces::host_layout_adapter::KreekSpacesLayout;
use crate::infrastructure::team_creation::competition_rules_adapter::CompetitionRulesAdapter;
use crate::infrastructure::team_creation::reference_data_adapter::ReferenceDataAdapter;
use crate::web::middleware::bypass_auth::bypass_auth_middleware;
use crate::web::middleware::panic_response::JournalDePanic;
use crate::web::middleware::request_log::request_log;
use crate::web::middleware::require_auth::require_auth;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::{response::Redirect, routing::get, Router};
use axum_login::AuthManagerLayerBuilder;
use clap::Parser;
use std::sync::Arc;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::services::ServeDir;
use tower_livereload::LiveReloadLayer;
use tower_sessions::SessionManagerLayer;

#[derive(Parser)]
#[command(name = "kreek", about = "kreek — Blood Bowl league manager")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Lance le serveur HTTP
    Serve,
    /// Seed les comptes utilisateurs depuis un fichier JSON
    SeedAccounts {
        #[arg(long, default_value = "scripts/seed_accounts.json")]
        input: String,
    },
    /// Seed synthétique minimal pour la suite e2e (space + coachs + adhésions)
    SeedE2e,
    /// Expédie les notifications dues aujourd'hui (cron quotidien)
    SendNotifications {
        /// Vise une autre date que le jour même. Réservé à l'exploitation :
        /// R9 interdit au cron de regarder en arrière, pas à un humain.
        #[arg(long)]
        date: Option<String>,
        /// Compte ce qui partirait, sans rien réserver ni envoyer.
        #[arg(long)]
        dry_run: bool,
    },
}

/// Le pool, et les deux garde-fous contre les transactions fantômes (carte 317).
///
/// **`idle_in_transaction_session_timeout`**, posé ici et non par un
/// `ALTER ROLE` : le réglage voyage alors avec le dépôt et vaut pour dev, test
/// et production, au lieu de vivre dans une base et pas dans les autres.
///
/// Il répond à un mode de panne précis : une requête annulée en vol — un client
/// qui abandonne — fait *dropper* le future du handler entre le `BEGIN` et le
/// `COMMIT`. `sqlx::Transaction::drop` ne peut pas `await` son `ROLLBACK`, la
/// connexion retourne au pool encore dans sa transaction, et ses verrous
/// bloquent tout le monde jusqu'au redémarrage. Observé : trois minutes de
/// blocage sur trois connexions.
///
/// **`max_lifetime`** n'empêche pas la fuite, il en borne la durée de vie : une
/// connexion empoisonnée finit par être recyclée au lieu de rester dans le pool
/// jusqu'au prochain déploiement.
///
/// `test_before_acquire` n'a **pas** été ajouté : il vaut déjà `true` par
/// défaut, et son ping réussit parfaitement sur une connexion oisive dans une
/// transaction — il détecte une connexion morte, pas une connexion empoisonnée.
/// L'inscrire aurait donné l'illusion d'un second garde-fou.
async fn init_pool(cfg: &AppConfig) -> sqlx::PgPool {
    let idle_in_tx = cfg.database.idle_in_transaction_timeout_seconds;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(cfg.database.max_connections)
        .min_connections(cfg.database.min_connections)
        .acquire_timeout(Duration::from_secs(cfg.database.acquire_timeout_seconds))
        .idle_timeout(Duration::from_secs(cfg.database.idle_timeout_seconds))
        .max_lifetime(Duration::from_secs(30 * 60))
        .after_connect(move |conn, _meta| {
            Box::pin(async move {
                sqlx::query(&format!(
                    "SET idle_in_transaction_session_timeout = '{}s'",
                    idle_in_tx
                ))
                .execute(conn)
                .await?;
                Ok(())
            })
        })
        .connect(&cfg.database.url)
        .await
        .expect("Impossible de se connecter à la base de données")
}

// Embarque les fichiers SQL de `migrations/` dans le binaire à la compilation
// (plus besoin du dossier sur la machine cible).
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

async fn run_migrations(pool: &sqlx::PgPool) {
    if let Err(e) = MIGRATOR.run(pool).await {
        tracing::error!("échec des migrations au démarrage : {e}");
        std::process::exit(1);
    }
}

/// Le repli quand le niveau configuré est illisible, et le socle des
/// directives : `sqlx` reste à `warn` quel que soit le niveau de l'application.
/// En `debug` il déverserait chaque requête, alors que `warn` donne exactement
/// ce qui manquait — les échecs et les requêtes lentes.
const JOURNAL_REPLI: &str = "kreek=info,sqlx=warn";

/// Construit le filtre, et rend le niveau refusé s'il y en a un — la mise en
/// garde ne peut pas être émise ici, le souscripteur n'existant pas encore.
fn filtre_de_journalisation(niveau: &str) -> (tracing_subscriber::EnvFilter, Option<String>) {
    // `RUST_LOG` prime : c'est l'échappatoire d'investigation, qui ouvre un BC
    // le temps d'un incident sans toucher à la configuration déployée.
    if let Ok(filtre) = tracing_subscriber::EnvFilter::try_from_default_env() {
        return (filtre, None);
    }
    filtre_depuis_config(niveau)
}

/// Séparée de la lecture de `RUST_LOG` pour être testable : manipuler une
/// variable d'environnement dans un test la partagerait avec tous les autres,
/// qui tournent en parallèle.
pub(crate) fn filtre_depuis_config(
    niveau: &str,
) -> (tracing_subscriber::EnvFilter, Option<String>) {
    // Une valeur vide est un `.env` recopié du modèle sans être renseigné :
    // c'est une absence de choix, pas une erreur de saisie.
    if niveau.trim().is_empty() {
        return (tracing_subscriber::EnvFilter::new(JOURNAL_REPLI), None);
    }
    match tracing_subscriber::EnvFilter::try_new(format!("kreek={niveau},sqlx=warn")) {
        Ok(filtre) => (filtre, None),
        // Un niveau mal orthographié ne doit pas empêcher un serveur de
        // démarrer — mais il ne doit pas non plus passer inaperçu.
        Err(_) => (
            tracing_subscriber::EnvFilter::new(JOURNAL_REPLI),
            Some(niveau.to_string()),
        ),
    }
}

/// Composition de couches et non `fmt().init()` : la ligne du chemin nominal
/// est produite par `UseCaseJournal`, qui a besoin d'un `registry` sous lui
/// pour ranger l'état d'un span entre son ouverture et sa fermeture. Le filtre
/// est posé en premier, donc la couche est muette dès que `kreek` l'est.
fn init_journal(cfg: &AppConfig) {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let (filtre, refuse) = filtre_de_journalisation(&cfg.log.level);
    tracing_subscriber::registry()
        .with(filtre)
        .with(tracing_subscriber::fmt::layer())
        .with(UseCaseJournal)
        .init();
    if let Some(niveau) = refuse {
        tracing::warn!(
            niveau,
            "niveau de journalisation illisible — repli sur « info »"
        );
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // La configuration précède le journal, puisqu'elle en porte le niveau.
    // Rien ne journalise entre les deux : un échec de chargement passe par le
    // gestionnaire de panique, pas par `tracing`.
    let cfg =
        AppConfig::load().expect("Configuration invalide — vérifiez vos variables d'environnement");
    init_journal(&cfg);

    let pool = init_pool(&cfg).await;
    run_migrations(&pool).await;

    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => run_server(cfg, pool).await,
        Command::SeedAccounts { input } => {
            if let Err(e) = cli::seed_accounts::execute(&pool, &input).await {
                tracing::error!("seed-accounts failed: {e}");
                std::process::exit(1);
            }
        }
        Command::SeedE2e => {
            if let Err(e) = cli::seed_e2e::execute(&pool).await {
                tracing::error!("seed-e2e failed: {e}");
                std::process::exit(1);
            }
        }
        Command::SendNotifications { date, dry_run } => {
            let email = build_email_service(cfg.email.clone());
            match cli::send_notifications::execute(&cfg, &pool, date, dry_run, email).await {
                // `exit(1)` dès qu'un envoi a échoué : c'est ce qui rend R1
                // observable. Une exécution parfaite et une exécution ayant
                // perdu douze e-mails ne doivent pas se ressembler.
                Ok(r) if r.failed > 0 => {
                    tracing::error!(failed = r.failed, "des notifications ont échoué");
                    std::process::exit(1);
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("send-notifications failed: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}

/// Charge les données de référence au démarrage. Échec fatal : sans elles,
/// l'application ne peut servir aucune page.
fn load_references(cfg: &AppConfig) -> ReferencesContext {
    ReferencesContext::new(Path::new(&cfg.references.dir)).unwrap_or_else(|e| {
        panic!(
            "{e} — vérifiez REFERENCES__DIR (valeur courante : « {} »)",
            cfg.references.dir
        )
    })
}

/// Choisit l'expéditeur d'emails déclaré par la configuration. La clé d'API
/// est vérifiée ici plutôt qu'au premier envoi : un déploiement mal configuré
/// doit refuser de démarrer, pas avaler les réinitialisations de mot de passe.
fn build_email_service(cfg: EmailConfig) -> Arc<dyn IEmailService> {
    match cfg.provider {
        EmailProvider::Console => {
            tracing::warn!(
                "EMAIL__PROVIDER=console — les emails sont écrits sur la sortie \
                 standard, aucun envoi réel"
            );
            Arc::new(ConsoleEmailService)
        }
        EmailProvider::Resend => {
            assert!(
                !cfg.api_key.is_empty(),
                "EMAIL__PROVIDER=resend exige EMAIL__API_KEY"
            );
            Arc::new(ResendMailService::new(cfg.api_key, cfg.from, cfg.from_name))
        }
    }
}

/// Assemble l'application : bus, listeners, adapters, les dix contextes.
///
/// Extraite de `run_server` pour que les tests de handler construisent **le**
/// câblage de production, et non une réplique. Un constructeur `for_tests`
/// distinct donnerait un harnais vert sur un montage que la production n'a
/// pas — c'est la seule chose qui rende ce niveau de test digne de confiance.
pub async fn compose(cfg: AppConfig, pool: sqlx::PgPool) -> AppState {
    let event_bus = new_bus();
    let app_event_bus = new_bus();

    event_log_feeder::init(&event_bus, pool.clone());
    auth::context::init_app_event_publisher(&event_bus, app_event_bus.clone());

    spaces::context::init_app_event_listeners(&app_event_bus, pool.clone());
    spaces::context::init_app_event_publisher(&event_bus, app_event_bus.clone());

    let competitions_team_info_port = Arc::new(
        crate::infrastructure::competitions::team_info_adapter::TeamInfoAdapter::new(Arc::new(
            crate::app::teams::io::repository::team_repository::TeamRepository::new(
                pool.clone(),
                event_bus.clone(),
            ),
        )),
    );
    let competitions_match_day_repository = Arc::new(
        crate::app::competitions::io::repository::match_day_repository::MatchDayRepository::new(
            pool.clone(),
        ),
    );
    competitions::context::init_listeners(
        &event_bus,
        app_event_bus.clone(),
        pool.clone(),
        competitions_match_day_repository.clone(),
        competitions_team_info_port.clone(),
    );
    team_creation::context::init_app_event_publisher(&event_bus, app_event_bus.clone());
    let references = load_references(&cfg);
    let teams_journeyman_type = Arc::new(
        crate::infrastructure::teams::journeyman_type_adapter::JourneymanTypeAdapter::new(
            references.repository.clone(),
        ),
    );
    let teams_roster_catalog = Arc::new(
        crate::infrastructure::teams::roster_catalog_adapter::RosterCatalogAdapter::new(
            references.repository.clone(),
        ),
    );
    let teams_squad = Arc::new(crate::infrastructure::teams::squad_adapter::SquadAdapter::new(
        Arc::new(crate::app::players::io::repository::projection_repository::PgPlayerProjectionRepository::new(pool.clone())),
    ));
    teams::context::init_listeners(
        &app_event_bus,
        &event_bus,
        pool.clone(),
        teams_squad.clone(),
        teams_roster_catalog.clone(),
        teams_journeyman_type.clone(),
    );
    let players_skill_catalog: Arc<dyn crate::app::players::ports::ISkillCatalogPort> = Arc::new(
        crate::infrastructure::players::skill_catalog_adapter::SkillCatalogAdapter::new(
            references.repository.clone(),
        ),
    );
    let match_report_comp_data = Arc::new(
        crate::infrastructure::match_report::competition_data_adapter::CompetitionDataAdapter::new(
            Arc::new(crate::app::competitions::io::repository::competition_repository::CompetitionRepository::new(pool.clone())),
            Arc::new(crate::app::competitions::io::repository::season_repository::SeasonRepository::new(pool.clone())),
            references.repository.clone(),
            Arc::new(crate::app::competitions::io::repository::match_day_repository::MatchDayRepository::new(pool.clone())),
        ),
    );
    let match_report_team_data = Arc::new(
        crate::infrastructure::match_report::ref_team_data_adapter::RefTeamDataAdapter::new(
            Arc::new(
                crate::app::teams::io::repository::team_repository::TeamRepository::new(
                    pool.clone(),
                    event_bus.clone(),
                ),
            ),
            references.repository.clone(),
        ),
    );
    match_report::context::init_listeners(
        &event_bus,
        &app_event_bus,
        pool.clone(),
        match_report_comp_data.clone(),
        match_report_team_data.clone(),
    );
    players::context::init_listeners(
        &event_bus,
        &app_event_bus,
        pool.clone(),
        players_skill_catalog.clone(),
    );

    let ranking_competition_port = Arc::new(
        crate::infrastructure::ranking::competition_info_adapter::RankingCompetitionAdapter::new(
            Arc::new(
                crate::app::competitions::io::repository::season_repository::SeasonRepository::new(
                    pool.clone(),
                ),
            ),
            competitions_team_info_port.clone(),
            Arc::new(
                crate::app::competitions::io::repository::group_repository::GroupRepository::new(
                    pool.clone(),
                ),
            ),
        ),
    );
    ranking::context::init_listeners(
        &app_event_bus,
        pool.clone(),
        ranking_competition_port.clone(),
    );

    // Une seule construction, normalisée : `host_domain` peut porter son
    // schéma ou non, et cinq endroits le lui recollaient à la main.
    let app_url = cfg.app_url();
    let email_service = build_email_service(cfg.email);

    // Le second déclencheur de R11 : l'ouverture des inscriptions part sur un
    // **fait** — la saison s'ouvre — et non sur une date à comparer au jour.
    // Attendre le cron du lendemain ferait arriver l'annonce un jour trop tard.
    competitions::context::init_registration_open_listener(
        &event_bus,
        Arc::new(
            crate::app::competitions::io::app_events::competition_ready_listener::RegistrationOpenDeps {
                pool: pool.clone(),
                teams: competitions_team_info_port.clone(),
                members: Arc::new(crate::infrastructure::competitions::space_member_adapter::SpaceMemberAdapter::new(
                    Arc::new(crate::app::spaces::io::repository::space_repository::SpaceRepository::new(pool.clone())),
                    Arc::new(crate::app::spaces::io::repository::user_cache_repository::SpaceUserCacheRepository::new(pool.clone())),
                )),
                email: email_service.clone(),
                app_url: app_url.clone(),
            },
        ),
    );

    AppState {
        auth: AuthContext::new(
            &pool,
            event_bus.clone(),
            email_service.clone(),
            app_url.clone(),
            crate::web::routes::path::APP_LAYOUT.to_string(),
        ),
        spaces: SpacesContext::new(
            &pool,
            event_bus.clone(),
            Arc::new(KreekSpacesLayout {
                app_url: app_url.clone(),
            }),
            email_service.clone(),
        ),
        competitions: CompetitionsContext::new(
            &pool,
            event_bus.clone(),
            competitions_team_info_port,
            Arc::new(crate::infrastructure::competitions::reference_name_adapter::ReferenceNameAdapter::new(
                references.repository.clone(),
            )),
            Arc::new(crate::infrastructure::competitions::space_member_adapter::SpaceMemberAdapter::new(
                Arc::new(crate::app::spaces::io::repository::space_repository::SpaceRepository::new(pool.clone())),
                Arc::new(crate::app::spaces::io::repository::user_cache_repository::SpaceUserCacheRepository::new(pool.clone())),
            )),
            Arc::new(crate::infrastructure::competitions::tiebreak_catalog_adapter::TiebreakCatalogAdapter::new()),
            Arc::new(crate::infrastructure::competitions::match_report_status_adapter::MatchReportStatusAdapter::new(
                Arc::new(crate::app::match_report::io::repository::match_report_repository::MatchReportRepository::new(pool.clone())),
            )),
            // `competitions` demande à `ranking` de se rejouer. Pas de cycle :
            // les deux adaptateurs se ramènent aux dépôts, et
            // `ranking_competition_port` est construit plus haut.
            Arc::new(crate::infrastructure::competitions::ranking_recompute_adapter::RankingRecomputeAdapter::new(
                Arc::new(crate::app::ranking::io::repository::ranking_repository::PgRankingRepository::new(pool.clone())),
                ranking_competition_port.clone(),
            )),
            email_service.clone(),
            app_url.clone(),
        ),
        news: NewsContext::new(&pool),
        references: references.clone(),
        team_creation: TeamCreationContext::new(
            &pool,
            event_bus.clone(),
            Arc::new(ReferenceDataAdapter::new(references.repository.clone())),
            Arc::new(CompetitionRulesAdapter::new(Arc::new(
                crate::app::competitions::io::repository::season_repository::SeasonRepository::new(pool.clone()),
            ))),
            Arc::new(crate::infrastructure::team_creation::competition_display_adapter::CompetitionDisplayAdapter::new(
                Arc::new(crate::app::competitions::io::repository::competition_repository::CompetitionRepository::new(pool.clone())),
                Arc::new(crate::app::competitions::io::repository::season_repository::SeasonRepository::new(pool.clone())),
            )),
        ),
        match_report: {
            let comp_data = match_report_comp_data.clone();
            let team_data = match_report_team_data.clone();
            let player_data = Arc::new(
                crate::infrastructure::match_report::player_data_adapter::PlayerDataAdapter::new(
                    Arc::new(crate::app::players::io::repository::projection_repository::PgPlayerProjectionRepository::new(pool.clone())),
                    Arc::new(crate::app::players::io::repository::player_repository::PgPlayerRepository::new(pool.clone())),
                ),
            );
            let coach_data = Arc::new(
                crate::infrastructure::match_report::coach_data_adapter::CoachDataAdapter::new(
                    Arc::new(crate::app::spaces::io::repository::user_cache_repository::SpaceUserCacheRepository::new(pool.clone())),
                ),
            );
            let space_admin = Arc::new(
                crate::infrastructure::match_report::space_admin_adapter::SpaceAdminAdapter::new(
                    Arc::new(crate::app::spaces::io::repository::space_repository::SpaceRepository::new(pool.clone())),
                ),
            );
            let spp_calculator = Arc::new(
                crate::infrastructure::match_report::spp_calculator_adapter::SppCalculatorAdapter::new(
                    references.repository.clone(),
                ),
            );
            let keyword_catalog = Arc::new(
                crate::infrastructure::match_report::keyword_catalog_adapter::KeywordCatalogAdapter::new(
                    references.repository.clone(),
                ),
            );
            match_report::context::MatchReportContext::new(&pool, comp_data, team_data, player_data, coach_data, space_admin, spp_calculator, keyword_catalog, event_bus.clone())
        },
        teams: {
            TeamsContext::new(
                &pool,
                event_bus.clone(),
                teams_journeyman_type,
                teams_roster_catalog,
                teams_squad,
                Arc::new(crate::infrastructure::teams::access_adapter::TeamAccessAdapter::new(
                    Arc::new(crate::app::spaces::io::repository::space_repository::SpaceRepository::new(pool.clone())),
                    Arc::new(crate::app::competitions::io::repository::competition_repository::CompetitionRepository::new(pool.clone())),
                )),
                Arc::new(crate::infrastructure::teams::dice_adapter::DiceAdapter),
                Arc::new(crate::infrastructure::teams::match_context_adapter::MatchContextAdapter::new(pool.clone())),
            )
        },
        players: PlayersContext::new(
            &pool,
            players_skill_catalog.clone(),
            Arc::new(crate::infrastructure::players::team_roster_adapter::TeamRosterAdapter::new(
                Arc::new(crate::app::teams::io::repository::team_repository::TeamRepository::new(pool.clone(), event_bus.clone())),
            )),
            Arc::new(crate::infrastructure::players::competition_admin_adapter::CompetitionAdminAdapter::new(
                Arc::new(crate::app::competitions::io::repository::competition_repository::CompetitionRepository::new(pool.clone())),
            )),
            Arc::new(crate::infrastructure::players::space_member_adapter::SpaceMemberAdapter::new(
                Arc::new(crate::app::spaces::io::repository::space_repository::SpaceRepository::new(pool.clone())),
            )),
            event_bus.clone(),
        ),
        ranking: RankingContext::new(
            &pool,
            ranking_competition_port,
            Arc::new(crate::infrastructure::ranking::admin_adapter::RankingAdminAdapter::new(
                Arc::new(crate::app::competitions::io::repository::competition_repository::CompetitionRepository::new(pool.clone())),
                Arc::new(crate::app::spaces::io::repository::space_repository::SpaceRepository::new(pool.clone())),
            )),
        ),
        // Un résolveur par ressource identifiable dans un chemin. Les six
        // autres BCs arrivent avec les cartes 318 à 322 ; un paramètre sans
        // résolveur passe, faute de quoi la migration devrait être atomique.
        space_ownership: Arc::new(vec![
            Arc::new(
                crate::infrastructure::players::space_ownership::PlayerSpaceOwnership::new(Arc::new(
                    crate::app::players::io::repository::projection_repository::PgPlayerProjectionRepository::new(
                        pool.clone(),
                    ),
                )),
            ),
            Arc::new(
                crate::infrastructure::competitions::space_ownership::CompetitionSpaceOwnership::new(
                    Arc::new(crate::app::competitions::io::repository::competition_repository::CompetitionRepository::new(pool.clone())),
                ),
            ),
            Arc::new(
                crate::infrastructure::news::space_ownership::ArticleSpaceOwnership::new(Arc::new(
                    crate::app::news::io::repository::article_repository::ArticleRepository::new(
                        pool.clone(),
                    ),
                )),
            ),
            Arc::new(
                crate::infrastructure::teams::space_ownership::TeamSpaceOwnership::new(
                    Arc::new(
                        crate::app::teams::io::repository::team_repository::TeamRepository::new(
                            pool.clone(),
                            event_bus.clone(),
                        ),
                    ),
                    Arc::new(
                        crate::app::team_creation::io::team_creation_repository::TeamDraftRepository::new(pool.clone()),
                    ),
                ),
            ),
            Arc::new(
                crate::infrastructure::match_report::space_ownership::MatchReportSpaceOwnership::new(
                    Arc::new(crate::app::match_report::io::repository::match_report_repository::MatchReportRepository::new(pool.clone())),
                ),
            ),
            Arc::new(
                crate::infrastructure::competitions::space_ownership::SeasonSpaceOwnership::new(
                    Arc::new(crate::app::competitions::io::repository::season_repository::SeasonRepository::new(pool.clone())),
                ),
            ),
        ]),
        bypass_auth: cfg.bypass_auth,
        event_bus: event_bus.clone(),
        app_event_bus: app_event_bus.clone(),
    }
}

/// Le routeur complet, sans la socket : c'est ce que `oneshot` prend en test.
///
/// `bypass_auth` est posé en `route_layer` — le harnais peut donc poser sa
/// propre couche d'identité à la place, sans que le code de production ait à
/// connaître les besoins des tests.
pub fn build_router(state: AppState) -> Router {
    // Un doublon de paramètre est une erreur de câblage : mieux vaut un
    // démarrage qui échoue qu'un résolveur silencieusement ignoré.
    crate::web::middleware::space_scope::verifier_unicite_des_parametres(&state.space_ownership);

    let session_layer = SessionManagerLayer::new(DashMapStore::new());
    let auth_layer = AuthManagerLayerBuilder::new(
        AuthBackend::new(state.auth.user_repository.clone()),
        session_layer,
    )
    .build();

    let protected = Router::new()
        .merge(app::news::router::router())
        .merge(app::references::router::router())
        .merge(app::team_creation::router::router())
        .merge(app::players::router::router())
        .merge(app::ranking::router::router())
        .merge(app::teams::router::router())
        .merge(app::match_report::router::router())
        .merge(app::competitions::router::router())
        .merge(app::spaces::router::router())
        .merge(web::router::router())
        .route_layer(from_fn(require_auth))
        .route_layer(from_fn_with_state(state.clone(), bypass_auth_middleware))
        // Posé au même endroit que `bypass_auth` — à l'intérieur
        // d'`AuthManagerLayer`. Le contrôle ne dépend pas de l'identité, mais
        // l'ordre se choisit consciemment : la carte 311 a appris qu'une couche
        // posée par-dessus le routeur s'exécute avant l'authentification.
        .route_layer(from_fn_with_state(
            state.clone(),
            crate::web::middleware::space_scope::space_scope_middleware,
        ));

    let auth_app = Router::new()
        .route("/", get(|| async { Redirect::to(path::AUTH_LAYOUT) }))
        .merge(app::auth::router::router())
        .merge(protected)
        // **Sous le journal**, donc à l'intérieur du span de requête : la ligne
        // `ERROR` qu'émet la couche porte alors le `rid` et le chemin. Posée
        // au-dessus, elle sortirait orpheline — l'incident qu'on cherche
        // justement à documenter, documenté à moitié. Bénéfice second :
        // `request_log` voit passer le `500` et journalise sa ligne de fin, là
        // où un panic ne produisait aucune réponse du tout.
        //
        // `custom` et non `new` : le gestionnaire par défaut journalise sur la
        // cible `tower_http::catch_panic`, que le filtre `kreek=…` n'active
        // pas — la ligne n'existerait tout simplement pas. Voir
        // `web::middleware::panic_response`.
        .layer(CatchPanicLayer::custom(JournalDePanic))
        // Sous `auth_layer` et non par-dessus : le journal nomme le coach, or
        // `AuthSession` n'existe qu'une fois la session chargée. Rien de perdu
        // pour autant — `AuthManagerLayer` ne rejette personne, et les refus de
        // `require_auth` et `space_scope`, posés en `route_layer` plus profond,
        // restent enveloppés. Posée ici plutôt que sur le routeur externe, la
        // couche ignore aussi `/static`, dont chaque fichier produirait une
        // ligne sans valeur de diagnostic.
        .layer(from_fn(request_log))
        .layer(auth_layer)
        .with_state(state);

    let app = Router::new()
        // Avant `/static` : le bundle n'existe qu'en mémoire, `ServeDir` ne
        // saurait pas le trouver sur le disque.
        .route("/css/{fichier}", get(web::css_bundle::servir))
        .nest_service("/static", ServeDir::new("assets/static"))
        .merge(auth_app);

    #[cfg(debug_assertions)]
    let app = {
        #[derive(Clone, Copy)]
        struct NotHtmxRequest;
        impl tower_livereload::predicate::Predicate<axum::http::Request<axum::body::Body>>
            for NotHtmxRequest
        {
            fn check(&mut self, req: &axum::http::Request<axum::body::Body>) -> bool {
                !req.headers().contains_key("hx-request")
            }
        }
        Router::new()
            .nest_service("/ui", ServeDir::new("assets/templates"))
            .merge(app)
            .layer(LiveReloadLayer::new().request_predicate(NotHtmxRequest))
    };

    app
}

async fn run_server(cfg: AppConfig, pool: sqlx::PgPool) {
    // Avant tout le reste : un échec ici est fatal, et vaut mieux qu'un serveur
    // qui démarre sans styles.
    let debut = std::time::Instant::now();
    web::css_bundle::construire();
    tracing::info!(
        duree_ms = debut.elapsed().as_millis(),
        app = %web::css_bundle::bundle("app").chemin,
        "bundles CSS construits"
    );

    let server_address = cfg.server_addr();
    let state = compose(cfg, pool.clone()).await;

    // Entre `compose` et `build_router` : tous les adapters sont construits,
    // le corpus de règles est chargé, et le serveur n'écoute pas encore. Un
    // échec ici refuse le démarrage — cf. `infrastructure::data_migrations`.
    infrastructure::data_migrations::executer(&state, &pool).await;

    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(&server_address)
        .await
        .unwrap();

    // Sans cette ligne, un démarrage réussi n'affiche rien du tout : « serveur
    // muet » et « serveur planté » se ressemblent. Deux instances ont ainsi pu
    // écouter le même port sur deux piles différentes — l'une en IPv4, l'autre
    // en IPv6 — sans que rien ne le signale.
    tracing::info!(adresse = %server_address, "serveur démarré");

    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le harnais de la carte 311 passe par le vrai `build_router()`, donc il
    /// ne peut atteindre que des routes réelles — et aucune ne panique. Ce
    /// mini-routeur vérifie donc ce qui est vérifiable sans ajouter de code de
    /// production : qu'un panic devient une réponse plutôt qu'une connexion
    /// coupée. Le **placement** de la couche sous le span, lui, se constate à
    /// la lecture du journal.
    /// Nommée plutôt qu'anonyme : une fermeture `async { panic!(…) }` a pour
    /// type de retour `!`, que le compilateur refuse désormais de résoudre en
    /// `()` pour satisfaire `IntoResponse`. Un type de retour explicite lève
    /// l'ambiguïté.
    async fn route_qui_panique() -> String {
        panic!("boum — panic volontaire de test")
    }

    #[tokio::test]
    async fn un_panic_devient_une_reponse_500() {
        use tower::ServiceExt;

        let app = Router::new()
            .route("/panique", get(route_qui_panique))
            .layer(CatchPanicLayer::custom(JournalDePanic));

        let reponse = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/panique")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            reponse.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn un_niveau_valide_est_repris_tel_quel() {
        let (filtre, refuse) = filtre_depuis_config("debug");
        assert!(refuse.is_none());
        assert!(filtre.to_string().contains("kreek=debug"));
    }

    #[test]
    fn une_directive_complete_n_est_pas_un_niveau() {
        // `LOG__LEVEL` porte un niveau, qui devient celui de la cible `kreek`.
        // Y mettre une directive entière donne `kreek=kreek::app::…=debug`,
        // que le filtre refuse — et c'est tant mieux : le repli signalé vaut
        // mieux qu'une directive silencieusement inopérante. Le ciblage fin
        // passe par `RUST_LOG`, qui prime.
        let (filtre, refuse) = filtre_depuis_config("kreek::app::players=debug");
        assert!(refuse.is_some());
        assert_eq!(filtre.to_string(), JOURNAL_REPLI);
    }

    #[test]
    fn sqlx_est_epingle_a_warn_quel_que_soit_le_niveau() {
        // En `debug`, `sqlx` déverserait chaque requête. C'est le seul réglage
        // du filtre qui ne suive pas le niveau de l'application.
        let (filtre, _) = filtre_depuis_config("debug");
        assert!(filtre.to_string().contains("sqlx=warn"));
    }

    #[test]
    fn un_niveau_illisible_se_replie_en_le_signalant() {
        let (filtre, refuse) = filtre_depuis_config("verbeux");
        assert_eq!(refuse.as_deref(), Some("verbeux"));
        assert_eq!(filtre.to_string(), JOURNAL_REPLI);
    }

    #[test]
    fn un_niveau_vide_se_replie_sans_rien_signaler() {
        // Un `.env` recopié du modèle sans être renseigné : une absence de
        // choix, pas une faute de frappe — donc pas d'avertissement.
        let (filtre, refuse) = filtre_depuis_config("   ");
        assert!(refuse.is_none());
        assert_eq!(filtre.to_string(), JOURNAL_REPLI);
    }

    fn email_cfg(provider: EmailProvider, api_key: &str) -> EmailConfig {
        EmailConfig {
            provider,
            api_key: api_key.to_string(),
            from: "mailer@example.test".into(),
            from_name: "Kreek".into(),
        }
    }

    #[test]
    fn console_ne_reclame_aucune_cle_d_api() {
        let _ = build_email_service(email_cfg(EmailProvider::Console, ""));
    }

    #[test]
    fn resend_avec_sa_cle_est_accepte() {
        let _ = build_email_service(email_cfg(EmailProvider::Resend, "re_xxx"));
    }

    /// Le démarrage échoue plutôt que de découvrir la clé manquante au premier
    /// envoi — c'est-à-dire au moment où un coach attend son mot de passe.
    #[test]
    #[should_panic(expected = "EMAIL__API_KEY")]
    fn resend_sans_cle_refuse_de_demarrer() {
        let _ = build_email_service(email_cfg(EmailProvider::Resend, ""));
    }
}

//! Le retrait d'une poule **désaffecte réellement ses équipes** (carte 423).
//!
//! Rien de tout cela ne repose sur du Rust : c'est la cascade `ON DELETE
//! CASCADE` de `competition_group_teams` qui défait les affectations, et
//! l'atomicité vient d'une transaction sqlx. Un test à doublure ne vérifierait
//! donc **rien** — d'où `#[sqlx::test]`, qui monte une base migrée par test.
//!
//! Ce que ces tests attrapent, et que rien d'autre ne verrait : la table des
//! poules est alimentée par un `INSERT … ON CONFLICT DO UPDATE` qui **ne
//! supprime jamais**. Sans la suppression explicite, retirer une poule du JSONB
//! la laisserait en base avec ses équipes — le retrait serait cosmétique, et
//! l'écran montrerait pourtant qu'il a eu lieu.

use crate::app::competitions::domain::competition_structure::{
    CompetitionStructure, DispatchType, RankingGroup, RankingGroupConfig, RankingGroupName,
    ScheduleConfig, UseRankingGroups,
};
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonRepositoryError,
};
use crate::app::competitions::io::repository::season_repository::SeasonRepository;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::app::shared_kernel::bloodbowl::ranking_group_id::RankingGroupId;
use sqlx::PgPool;

fn groupe(id: &str, nom: &str) -> RankingGroup {
    RankingGroup {
        id: RankingGroupId::try_new(id.to_string()).unwrap(),
        name: RankingGroupName::try_new(nom.to_string()).unwrap(),
    }
}

fn structure(groupes: Vec<RankingGroup>) -> CompetitionStructure {
    CompetitionStructure {
        ranking_group: RankingGroupConfig::try_new(
            UseRankingGroups(!groupes.is_empty()),
            DispatchType::Automatic,
            groupes,
        )
        .unwrap(),
        schedule: calendrier(),
    }
}

/// Le décor : une saison, deux poules matérialisées, deux équipes affectées à la
/// première et une à la seconde.
async fn decor(pool: &PgPool) -> SeasonId {
    let season = SeasonId::new();
    sqlx::query(
        "INSERT INTO competitions (id, space_id, name, logo, created_at)
         VALUES ($1, $2, 'Compétition de test', $3, now())",
    )
    .bind(season.to_string())
    .bind(SeasonId::new().to_string())
    .bind("https://res.cloudinary.com/demo/image/upload/v1/x.jpg")
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO competition_seasons (id, competition_id, name, status)
         VALUES ($1, $1, 'Saison 1', 'ready')",
    )
    .bind(season.to_string())
    .execute(pool)
    .await
    .unwrap();

    for (i, (id, nom)) in [("ga", "Poule A"), ("gb", "Poule B")].iter().enumerate() {
        sqlx::query(
            "INSERT INTO competition_groups (id, season_id, name, position) VALUES ($1,$2,$3,$4)",
        )
        .bind(id)
        .bind(season.to_string())
        .bind(nom)
        .bind(i as i32)
        .execute(pool)
        .await
        .unwrap();
    }
    for (groupe, equipe) in [("ga", "t1"), ("ga", "t2"), ("gb", "t3")] {
        sqlx::query("INSERT INTO competition_group_teams (group_id, team_id) VALUES ($1,$2)")
            .bind(groupe)
            .bind(equipe)
            .execute(pool)
            .await
            .unwrap();
    }
    season
}

async fn compter(pool: &PgPool, table: &str, season: &SeasonId) -> i64 {
    let sql = match table {
        "groupes" => "SELECT count(*) FROM competition_groups WHERE season_id = $1".to_string(),
        _ => "SELECT count(*) FROM competition_group_teams t
              JOIN competition_groups g ON g.id = t.group_id
              WHERE g.season_id = $1"
            .to_string(),
    };
    sqlx::query_scalar(&sql)
        .bind(season.to_string())
        .fetch_one(pool)
        .await
        .unwrap()
}

/// **Retirer une poule emporte ses équipes**, et le compte rendu est exact.
#[sqlx::test]
async fn retirer_une_poule_desaffecte_ses_equipes(pool: PgPool) {
    let depot = SeasonRepository::new(pool.clone());
    let season = decor(&pool).await;
    assert_eq!(compter(&pool, "affectations", &season).await, 3);

    let defaites = depot
        .save_structure_and_prune_groups(
            &season,
            &structure(vec![groupe("gb", "Poule B")]),
            &["gb".to_string()],
        )
        .await
        .unwrap();

    assert_eq!(defaites, 2, "les deux équipes de la poule A");
    assert_eq!(compter(&pool, "groupes", &season).await, 1);
    assert_eq!(
        compter(&pool, "affectations", &season).await,
        1,
        "seule l'équipe de la poule B reste affectée"
    );
}

/// **Retirer toutes les poules n'est pas un cas particulier.** `kept_ids` vide,
/// tout part — et c'est le signe que la forme est juste : aucune branche.
#[sqlx::test]
async fn retirer_toutes_les_poules_est_autorise(pool: PgPool) {
    let depot = SeasonRepository::new(pool.clone());
    let season = decor(&pool).await;

    let defaites = depot
        .save_structure_and_prune_groups(&season, &structure(vec![]), &[])
        .await
        .unwrap();

    assert_eq!(defaites, 3);
    assert_eq!(compter(&pool, "groupes", &season).await, 0);
    assert_eq!(compter(&pool, "affectations", &season).await, 0);
}

/// **Le statut ne bouge pas.** `save_structure` pose
/// `status = 'structure_selected'`, ce qui sert le magicien de création. Sur une
/// saison en cours, ce serait la faire régresser sous `ready` — et la carte 407
/// interdit la création d'équipe sur une saison qui ne l'est pas. Modifier une
/// poule aurait cassé l'inscription de la compétition entière, sans un mot.
#[sqlx::test]
async fn le_statut_de_la_saison_n_est_pas_touche(pool: PgPool) {
    let depot = SeasonRepository::new(pool.clone());
    let season = decor(&pool).await;

    depot
        .save_structure_and_prune_groups(
            &season,
            &structure(vec![groupe("ga", "Renommée")]),
            &["ga".to_string()],
        )
        .await
        .unwrap();

    let statut: String = sqlx::query_scalar("SELECT status FROM competition_seasons WHERE id = $1")
        .bind(season.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(statut, "ready", "la saison a régressé sous « prête »");
}

/// **Le renommage atteint la table, pas seulement la déclaration.**
///
/// La table n'était rafraîchie qu'à l'ouverture de l'onglet Poules
/// (`ensure_groups_from_structure`). Sans écriture ici, la déclaration aurait dit
/// « Renommée » et la table « Poule A » — deux écrans en contradiction, sans que
/// rien ne le signale. Constaté à l'écran.
#[sqlx::test]
async fn renommer_une_poule_atteint_la_table(pool: PgPool) {
    let depot = SeasonRepository::new(pool.clone());
    let season = decor(&pool).await;

    depot
        .save_structure_and_prune_groups(
            &season,
            &structure(vec![groupe("ga", "Renommée"), groupe("gb", "Poule B")]),
            &["ga".to_string(), "gb".to_string()],
        )
        .await
        .unwrap();

    let nom: String = sqlx::query_scalar("SELECT name FROM competition_groups WHERE id = 'ga'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(nom, "Renommée", "la table porte encore l'ancien nom");
}

/// **Une poule neuve est matérialisée aussitôt**, sans attendre qu'on ouvre
/// l'onglet Poules — sans quoi elle n'existerait qu'en déclaration, et rien ne
/// pourrait lui affecter d'équipe.
#[sqlx::test]
async fn une_poule_neuve_est_materialisee(pool: PgPool) {
    let depot = SeasonRepository::new(pool.clone());
    let season = decor(&pool).await;

    depot
        .save_structure_and_prune_groups(
            &season,
            &structure(vec![groupe("ga", "Poule A"), groupe("gneuve", "Poule C")]),
            &["ga".to_string(), "gneuve".to_string()],
        )
        .await
        .unwrap();

    let noms: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM competition_groups WHERE season_id = $1 ORDER BY position",
    )
    .bind(season.to_string())
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(noms, vec!["Poule A".to_string(), "Poule C".to_string()]);
}

/// Renommer sans retirer ne défait aucune affectation.
#[sqlx::test]
async fn renommer_une_poule_ne_defait_rien(pool: PgPool) {
    let depot = SeasonRepository::new(pool.clone());
    let season = decor(&pool).await;

    let defaites = depot
        .save_structure_and_prune_groups(
            &season,
            &structure(vec![groupe("ga", "Renommée"), groupe("gb", "Poule B")]),
            &["ga".to_string(), "gb".to_string()],
        )
        .await
        .unwrap();

    assert_eq!(defaites, 0);
    assert_eq!(compter(&pool, "affectations", &season).await, 3);
}

#[sqlx::test]
async fn une_saison_inconnue_est_refusee(pool: PgPool) {
    let depot = SeasonRepository::new(pool.clone());

    let issue = depot
        .save_structure_and_prune_groups(&SeasonId::new(), &structure(vec![]), &[])
        .await;

    assert!(matches!(issue, Err(SeasonRepositoryError::SeasonNotFound)));
}

// ── Le reste de la structure, qui doit survivre ─────────────────────────────

/// **Le calendrier fait partie de ce que le retrait des poules doit épargner.**
/// Il porte une date de début distincte du défaut, précisément pour qu'un test
/// qui le perdrait s'en aperçoive.
fn calendrier() -> ScheduleConfig {
    use crate::app::competitions::domain::competition_structure::{ScheduleType, UseSchedule};
    use crate::app::shared_kernel::bloodbowl::date_string::DateString;
    ScheduleConfig {
        use_schedule: UseSchedule(true),
        schedule_type: ScheduleType::default(),
        schedule_start_date: DateString::try_new("2026-09-01".to_string()).unwrap(),
        schedule_end_date: DateString::default(),
        scheduled_dates: vec![],
    }
}

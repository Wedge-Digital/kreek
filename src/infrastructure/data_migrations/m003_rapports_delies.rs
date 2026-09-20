//! Les rapports qui ignorent leur appariement (carte 552).
//!
//! # Ce qu'elle répare
//!
//! Un rapport « hors calendrier » naissait sans appariement ; `competitions`
//! lui en fabriquait un après coup et **ne le lui disait jamais**. Son
//! `MatchReportCreated` porte donc `pairing_id: null` à vie, et toutes les
//! recherches inverses pairing → rapport sont aveugles : l'appariement
//! supprimé n'annule pas le rapport, et un rapport publié ne protège pas son
//! appariement.
//!
//! Relevé sur la base de production du 15 septembre 2026 : **16 rapports
//! vivants déliés**, dont 14 avaient déjà leur appariement — le lien manquait,
//! pas l'objet.
//!
//! # Pourquoi elle réécrit `MatchReportCreated`
//!
//! C'est une entorse assumée à l'immuabilité de l'event store, et elle mérite
//! d'être dite.
//!
//! Corriger la seule projection ne tiendrait pas : `match_report_proj.pairing_id`
//! n'est écrit qu'à l'INSERT, depuis cet événement. Le premier rebuild
//! effacerait la correction, et le défaut reviendrait sans prévenir.
//!
//! L'alternative propre — un événement `PairingAttached` — demande de le gérer
//! dans les quatre états de `rehydrate`, pour des rapports qui sont justement
//! répartis sur les quatre. À l'échelle de 16 agrégats, le coût dépasse le
//! gain.
//!
//! Ce que la réécriture corrige n'est d'ailleurs pas un fait : l'événement
//! affirmait « créé sans appariement », ce qui était **faux** dès l'instant où
//! l'appariement avait été fabriqué. On ne réécrit pas l'histoire, on répare
//! une donnée qui n'a jamais été vraie.
//!
//! # Les cas qu'elle ne peut pas lier
//!
//! Un rapport dont les deux équipes jouent **déjà** cette journée-là contre
//! d'autres adversaires ne peut pas retrouver d'appariement sans violer
//! l'invariant de la carte 551. Il est annulé, et ses deux équipes sont
//! libérées de leur phase de saisie — sans quoi elles resteraient verrouillées
//! sur un rapport qui n'existe plus.
//!
//! Un rapport dont la **journée n'existe pas** suit le même chemin (carte 556).
//! Le premier jet ne le prévoyait pas : une journée absente n'a aucun
//! appariement, elle passait donc pour libre, et l'insertion violait la clé
//! étrangère — ce qui refusait le démarrage. Huit brouillons de la base de
//! développement portaient l'ULID d'exemple de la spécification en guise de
//! journée, résidus d'un test e2e retiré depuis.
//!
//! # Ce qu'elle journalise, et quand
//!
//! Rien en `info` avant le commit. La transaction est annulée entière au
//! premier échec, et une ligne « rattaché » écrite pour un rapport traité avant
//! ce point décrirait une écriture qui n'a jamais eu lieu. Les décisions
//! s'écrivent en `debug` ; c'est le registre qui annonce le nombre d'agrégats
//! touchés, une fois la transaction validée.

use crate::app::shared_kernel::bloodbowl::ids::PairingId;
use crate::infrastructure::data_migrations::DataMigration;
use crate::state::AppState;
use async_trait::async_trait;
use sqlx::{Postgres, Row, Transaction};

pub struct RapportsDelies;

/// Un rapport vivant dont l'agrégat ne porte pas son appariement.
struct Delie {
    match_report_id: String,
    season_id: String,
    round_id: String,
    home_team_id: String,
    away_team_id: String,
}

#[async_trait]
impl DataMigration for RapportsDelies {
    fn nom(&self) -> &'static str {
        "552-rapports-delies"
    }

    async fn executer(
        &self,
        _state: &AppState,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<usize, String> {
        let mut touches = 0;
        for d in delies(tx).await? {
            traiter(tx, &d).await?;
            touches += 1;
        }
        Ok(touches)
    }
}

/// Lier si l'appariement existe ou peut naître, annuler sinon.
async fn traiter(tx: &mut Transaction<'_, Postgres>, d: &Delie) -> Result<(), String> {
    if let Some(pairing_id) = appariement_existant(tx, d).await? {
        return lier(tx, d, &pairing_id).await;
    }
    if !journee_existe(tx, d).await? {
        tracing::warn!(
            match_report_id = %d.match_report_id,
            round_id = %d.round_id,
            "rapport délié annulé : sa journée n'existe pas"
        );
        return annuler(tx, d).await;
    }
    if journee_occupee(tx, d).await? {
        tracing::warn!(
            match_report_id = %d.match_report_id,
            "rapport délié annulé : ses deux équipes jouent déjà cette journée"
        );
        return annuler(tx, d).await;
    }
    let pairing_id = creer_appariement(tx, d).await?;
    lier(tx, d, &pairing_id).await
}

async fn delies(tx: &mut Transaction<'_, Postgres>) -> Result<Vec<Delie>, String> {
    let rows = sqlx::query(
        "SELECT match_report_id, season_id, round_id, home_team_id, away_team_id
         FROM match_report_proj
         WHERE phase <> 'Cancelled' AND pairing_id IS NULL",
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| format!("lecture des rapports déliés : {e}"))?;

    Ok(rows
        .into_iter()
        .map(|r| Delie {
            match_report_id: r.get("match_report_id"),
            season_id: r.get("season_id"),
            round_id: r.get("round_id"),
            home_team_id: r.get("home_team_id"),
            away_team_id: r.get("away_team_id"),
        })
        .collect())
}

/// L'appariement du rapport, s'il existe déjà.
///
/// Deux sources, dans cet ordre : la ligne d'affichage, qui porte le lien que
/// le rapport ignore, puis le rapprochement par journée et équipes.
///
/// **Les deux camps sont testés** : rien ne garantit que le rapport et
/// l'appariement aient retenu le même domicile, et ne comparer que
/// `home_team_id` manquerait la moitié des cas.
async fn appariement_existant(
    tx: &mut Transaction<'_, Postgres>,
    d: &Delie,
) -> Result<Option<String>, String> {
    let par_affichage: Option<String> = sqlx::query_scalar(
        "SELECT pairing_id FROM competition_match_display_proj WHERE match_report_id = $1",
    )
    .bind(&d.match_report_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| format!("lecture de la ligne d'affichage : {e}"))?;
    if par_affichage.is_some() {
        return Ok(par_affichage);
    }

    sqlx::query_scalar(
        "SELECT id FROM competition_match_day_pairings
         WHERE match_day_id = $1
           AND ((home_team_id = $2 AND away_team_id = $3)
             OR (home_team_id = $3 AND away_team_id = $2))
         LIMIT 1",
    )
    .bind(&d.round_id)
    .bind(&d.home_team_id)
    .bind(&d.away_team_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| format!("rapprochement par journée et équipes : {e}"))
}

/// La journée du rapport existe-t-elle encore ? Sans elle, aucun appariement ne
/// peut naître : `match_day_id` est une clé étrangère.
async fn journee_existe(tx: &mut Transaction<'_, Postgres>, d: &Delie) -> Result<bool, String> {
    sqlx::query_scalar::<_, bool>(
        "SELECT exists(SELECT 1 FROM competition_match_days WHERE id = $1)",
    )
    .bind(&d.round_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| format!("recherche de la journée {} : {e}", d.round_id))
}

/// L'une des deux équipes joue-t-elle déjà cette journée ? C'est l'invariant de
/// la carte 551, appliqué aux données existantes.
async fn journee_occupee(tx: &mut Transaction<'_, Postgres>, d: &Delie) -> Result<bool, String> {
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM competition_match_day_pairings
         WHERE match_day_id = $1
           AND (home_team_id IN ($2, $3) OR away_team_id IN ($2, $3))",
    )
    .bind(&d.round_id)
    .bind(&d.home_team_id)
    .bind(&d.away_team_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| format!("recherche d'un engagement concurrent : {e}"))?;
    Ok(n > 0)
}

/// Pose le lien des deux côtés : l'événement d'abord, la projection ensuite.
///
/// L'événement est la source ; corriger la projection seule serait effacé au
/// premier rebuild, et c'est précisément le piège que cette migration évite.
async fn lier(
    tx: &mut Transaction<'_, Postgres>,
    d: &Delie,
    pairing_id: &str,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE match_report_event_store
         SET payload = jsonb_set(payload, '{pairing_id}', to_jsonb($2::text))
         WHERE match_report_id = $1 AND event_type = 'MatchReportCreated'",
    )
    .bind(&d.match_report_id)
    .bind(pairing_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("réécriture de MatchReportCreated : {e}"))?;

    sqlx::query("UPDATE match_report_proj SET pairing_id = $2 WHERE match_report_id = $1")
        .bind(&d.match_report_id)
        .bind(pairing_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("mise à jour de la projection : {e}"))?;

    tracing::debug!(
        match_report_id = %d.match_report_id,
        pairing_id = %pairing_id,
        "rapport délié rattaché à son appariement"
    );
    Ok(())
}

/// Crée l'appariement manquant, et sa ligne d'affichage.
///
/// Les données d'affichage viennent de `team_proj` et de la journée. Les
/// initiales sont approchées par les deux premières lettres : c'est une donnée
/// de repli, jamais lue quand un logo existe, et la reproduire exactement
/// demanderait de rejouer `initials()` en SQL.
async fn creer_appariement(
    tx: &mut Transaction<'_, Postgres>,
    d: &Delie,
) -> Result<String, String> {
    let pairing_id = PairingId::new().to_string();

    sqlx::query(
        "INSERT INTO competition_match_day_pairings (id, match_day_id, home_team_id, away_team_id)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(&pairing_id)
    .bind(&d.round_id)
    .bind(&d.home_team_id)
    .bind(&d.away_team_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        format!(
            "création de l'appariement du rapport {} sur la journée {} : {e}",
            d.match_report_id, d.round_id
        )
    })?;

    inserer_ligne_affichage(tx, d, &pairing_id).await?;
    Ok(pairing_id)
}

async fn inserer_ligne_affichage(
    tx: &mut Transaction<'_, Postgres>,
    d: &Delie,
    pairing_id: &str,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO competition_match_display_proj (
            pairing_id, season_id, round_id, round_name, round_position, round_day_type,
            home_team_id, home_team_name, home_roster_name, home_coach_name, home_logo_url,
            home_initials,
            away_team_id, away_team_name, away_roster_name, away_coach_name, away_logo_url,
            away_initials,
            match_status, match_report_id)
         SELECT $1, $2, md.id, md.name, md.position, md.day_type,
                h.team_id, h.team_name, h.roster_name, h.coach_name, h.logo_url,
                upper(substring(h.team_name, 1, 2)),
                a.team_id, a.team_name, a.roster_name, a.coach_name, a.logo_url,
                upper(substring(a.team_name, 1, 2)),
                'in_progress', $5
         FROM competition_match_days md, team_proj h, team_proj a
         WHERE md.id = $3 AND h.team_id = $4 AND a.team_id = $6",
    )
    .bind(pairing_id)
    .bind(&d.season_id)
    .bind(&d.round_id)
    .bind(&d.home_team_id)
    .bind(&d.match_report_id)
    .bind(&d.away_team_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        format!(
            "insertion de la ligne d'affichage du rapport {} sur la journée {} : {e}",
            d.match_report_id, d.round_id
        )
    })?;
    Ok(())
}

/// Annule le rapport **et libère ses deux équipes**.
///
/// Les deux vont ensemble : un rapport annulé dont les équipes restent en phase
/// de saisie les verrouille sur un match qui n'existe plus — c'est exactement
/// l'état dans lequel l'incident du 15 septembre les avait laissées.
async fn annuler(tx: &mut Transaction<'_, Postgres>, d: &Delie) -> Result<(), String> {
    let version: i64 = sqlx::query_scalar(
        "SELECT coalesce(max(version), 0) + 1 FROM match_report_event_store
         WHERE match_report_id = $1",
    )
    .bind(&d.match_report_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| format!("lecture de la version : {e}"))?;

    sqlx::query(
        "INSERT INTO match_report_event_store (match_report_id, event_type, payload, version)
         VALUES ($1, 'MatchReportCancelled', $2, $3)",
    )
    .bind(&d.match_report_id)
    .bind(serde_json::json!({
        "type": "MatchReportCancelled",
        "reason": "Rapport sans appariement possible (carte 552)",
        "journeymen": [],
        "pairing_id": null,
        "home_team_id": d.home_team_id,
        "away_team_id": d.away_team_id,
    }))
    .bind(version)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("append de l'annulation : {e}"))?;

    sqlx::query(
        "UPDATE match_report_proj SET phase = 'Cancelled', version = $2
         WHERE match_report_id = $1",
    )
    .bind(&d.match_report_id)
    .bind(version)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("annulation dans la projection : {e}"))?;

    liberer_les_equipes(tx, d).await
}

async fn liberer_les_equipes(tx: &mut Transaction<'_, Postgres>, d: &Delie) -> Result<(), String> {
    for team_id in [&d.home_team_id, &d.away_team_id] {
        let version: i64 = sqlx::query_scalar(
            "SELECT coalesce(max(version), 0) + 1 FROM team_event_store WHERE team_id = $1",
        )
        .bind(team_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| format!("lecture de la version d'équipe : {e}"))?;

        sqlx::query(
            "INSERT INTO team_event_store (team_id, event_type, payload, version)
             VALUES ($1, 'MatchReportingCancelled', $2, $3)",
        )
        .bind(team_id)
        .bind(serde_json::json!({
            "type": "MatchReportingCancelled",
            "match_report_id": d.match_report_id,
        }))
        .bind(version)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("append de la libération d'équipe : {e}"))?;

        sqlx::query("UPDATE team_proj SET game_phase = 'ReadyToPlay' WHERE team_id = $1")
            .bind(team_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("libération de l'équipe {team_id} : {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::data_migrations::appliquer;

    /// L'ULID d'exemple de la spécification — celui que portaient les huit
    /// brouillons de la base de développement.
    const JOURNEE_INCONNUE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    const RAPPORT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB0";
    const DOMICILE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB1";
    const VISITEUR: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB2";

    async fn etat(pool: sqlx::PgPool) -> AppState {
        crate::compose(crate::config::AppConfig::for_tests(), pool).await
    }

    /// Un rapport en saisie sur une journée qui n'existe pas, et ses deux
    /// équipes verrouillées dessus — l'état que la migration doit trouver.
    async fn semer(pool: &sqlx::PgPool) {
        let mut tx = pool.begin().await.unwrap();
        for (team_id, nom) in [(DOMICILE, "Domicile"), (VISITEUR, "Visiteur")] {
            sqlx::query(
                "INSERT INTO team_proj (team_id, space_id, team_name, coach_name, roster_name, game_phase)
                 VALUES ($1, 'espace', $2, 'coach', 'roster', 'MatchReporting')",
            )
            .bind(team_id)
            .bind(nom)
            .execute(&mut *tx)
            .await
            .unwrap();
        }
        sqlx::query(
            "INSERT INTO match_report_event_store (match_report_id, event_type, payload, version)
             VALUES ($1, 'MatchReportCreated', $2, 1)",
        )
        .bind(RAPPORT)
        .bind(serde_json::json!({
            "type": "MatchReportCreated",
            "round_id": JOURNEE_INCONNUE,
            "pairing_id": null,
        }))
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO match_report_proj (match_report_id, space_id, competition_id, season_id,
                round_id, home_team_id, away_team_id, created_by, origin, phase, version)
             VALUES ($1, 'espace', 'competition', 'saison', $2, $3, $4, 'coach', 'Manual',
                'ReadyToPublish', 1)",
        )
        .bind(RAPPORT)
        .bind(JOURNEE_INCONNUE)
        .bind(DOMICILE)
        .bind(VISITEUR)
        .execute(&mut *tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    async fn phase_du_rapport(pool: &sqlx::PgPool) -> String {
        sqlx::query_scalar("SELECT phase FROM match_report_proj WHERE match_report_id = $1")
            .bind(RAPPORT)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn phase_de_l_equipe(pool: &sqlx::PgPool, team_id: &str) -> String {
        sqlx::query_scalar("SELECT game_phase FROM team_proj WHERE team_id = $1")
            .bind(team_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    /// Le cas qui refusait le démarrage : la journée absente passait pour libre,
    /// et l'insertion de l'appariement violait la clé étrangère.
    #[sqlx::test]
    async fn un_rapport_sur_une_journee_inconnue_est_annule_et_libere_ses_equipes(
        pool: sqlx::PgPool,
    ) {
        let state = etat(pool.clone()).await;
        semer(&pool).await;

        appliquer(&state, &pool, vec![Box::new(RapportsDelies)])
            .await
            .expect("la migration passe malgré la journée inconnue");

        assert_eq!(phase_du_rapport(&pool).await, "Cancelled");
        assert_eq!(phase_de_l_equipe(&pool, DOMICILE).await, "ReadyToPlay");
        assert_eq!(phase_de_l_equipe(&pool, VISITEUR).await, "ReadyToPlay");
        let appariements: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM competition_match_day_pairings WHERE match_day_id = $1",
        )
        .bind(JOURNEE_INCONNUE)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            appariements, 0,
            "aucun appariement ne naît sur une journée absente"
        );
    }
}

//! La persistance de la campagne de présence.
//!
//! # Le dépôt reconstruit l'enum, il ne le devine pas
//!
//! `presence`, `repondu_le` et `saisi_par_admin` sont la projection à plat de
//! `Presence`. À la lecture, elles redeviennent l'enum — et une ligne
//! incohérente, qu'aucun chemin d'écriture ne produit et que le `CHECK` refuse,
//! est traitée comme une **erreur de dépôt**.
//!
//! **Aucun `.ok()?` ici.** `roster_service` en porte deux, et le `CLAUDE.md` les
//! cite comme le mécanisme qui a fait disparaître un roster sans une ligne de
//! journal. Une réponse escamotée à la lecture, c'est une équipe qui manque au
//! tirage sans que personne sache pourquoi.

use crate::app::competitions::domain::presence_survey::{
    Appariement, AutoRemind, FermeeLe, Fermeture, OpenedAt, Presence, PresenceSurvey, Repondant,
    ReponduLe, Reponse, SurveyDeadline, SurveyId, SurveyToken, Venue,
};
use crate::app::competitions::domain::presence_survey_repository_port::{
    IPresenceSurveyRepository, LandingLabelsDto, PresenceSurveyRepositoryError, SurveySummaryDto,
};
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::CoachId;
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::collections::HashMap;

#[derive(Clone)]
pub struct PresenceSurveyRepository {
    pool: PgPool,
}

impl PresenceSurveyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn db_err(e: impl std::fmt::Display) -> PresenceSurveyRepositoryError {
    PresenceSurveyRepositoryError::Database(e.to_string())
}

/// Une ligne incohérente en base n'est pas une donnée à ignorer : c'est un
/// défaut à signaler. Elle ne peut venir ni d'un chemin d'écriture ni du
/// `CHECK`, donc si elle existe, quelque chose d'autre a écrit.
fn incoherente(quoi: &str, id: &str) -> PresenceSurveyRepositoryError {
    PresenceSurveyRepositoryError::Database(format!(
        "ligne de présence incohérente ({quoi}) sur {id} — le CHECK aurait dû la refuser"
    ))
}

#[async_trait]
impl IPresenceSurveyRepository for PresenceSurveyRepository {
    async fn find_by_round(
        &self,
        round_id: &str,
    ) -> Result<Option<PresenceSurvey>, PresenceSurveyRepositoryError> {
        let Some(row) = sqlx::query(include_str!("sql/presences/find_survey_by_round.sql"))
            .bind(round_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
        else {
            return Ok(None);
        };

        let id: String = row.try_get("id").map_err(db_err)?;
        let reponses = self.lire_les_reponses(&id).await?;
        Ok(Some(rehydrater(&row, reponses)?))
    }

    /// **Le même chemin d'hydratation que `find_by_round`.** Les deux réutilisent
    /// `lire_les_reponses` et `rehydrater` : deux constructions séparées auraient
    /// pu diverger, et l'agrégat rendu par l'une n'aurait plus été celui de
    /// l'autre.
    async fn find_by_token(
        &self,
        token: &str,
    ) -> Result<Option<PresenceSurvey>, PresenceSurveyRepositoryError> {
        let Some(row) = sqlx::query(include_str!("sql/presences/find_survey_by_token.sql"))
            .bind(token)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
        else {
            return Ok(None);
        };

        let id: String = row.try_get("id").map_err(db_err)?;
        let reponses = self.lire_les_reponses(&id).await?;
        Ok(Some(rehydrater(&row, reponses)?))
    }

    async fn find_landing_labels(
        &self,
        token: &str,
    ) -> Result<Option<LandingLabelsDto>, PresenceSurveyRepositoryError> {
        sqlx::query_as::<_, LandingLabelsDto>(include_str!("sql/presences/find_landing_labels.sql"))
            .bind(token)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)
    }

    async fn save(&self, survey: &PresenceSurvey) -> Result<(), PresenceSurveyRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;

        sqlx::query(include_str!("sql/presences/upsert_survey.sql"))
            .bind(survey.id().to_string())
            .bind(survey.season_id().to_string())
            .bind(survey.round_id().to_string())
            .bind(survey.deadline().to_string())
            .bind(survey.auto_remind().into_inner())
            .bind(survey.opened_at().to_string())
            .bind(ferme_le(survey.fermeture()))
            .bind(exemptee(survey.appariement()))
            .bind(matches!(survey.appariement(), Appariement::Fait { .. }))
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;

        for reponse in survey.reponses() {
            ecrire_la_reponse(&mut tx, survey.id(), reponse).await?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    async fn list_summaries(
        &self,
        season_id: &str,
    ) -> Result<Vec<SurveySummaryDto>, PresenceSurveyRepositoryError> {
        sqlx::query_as::<_, SurveySummaryDto>(include_str!(
            "sql/presences/list_survey_summaries.sql"
        ))
        .bind(season_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)
    }

    /// **Le même chemin d'hydratation que les deux lectures unitaires** —
    /// `rehydrater`, sur des réponses lues à part. Construire l'agrégat autrement
    /// ici aurait donné un troisième assemblage, libre de diverger des deux
    /// premiers sans que rien ne le dise.
    async fn list_open_surveys_for_season(
        &self,
        season_id: &str,
        maintenant: &str,
    ) -> Result<Vec<PresenceSurvey>, PresenceSurveyRepositoryError> {
        let rows = sqlx::query(include_str!(
            "sql/presences/list_open_surveys_for_season.sql"
        ))
        .bind(season_id)
        .bind(maintenant)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        let mut par_campagne = self
            .lire_les_reponses_groupees(&identifiants(&rows)?)
            .await?;
        rows.iter()
            .map(|row| {
                let id: String = row.try_get("id").map_err(db_err)?;
                // Une campagne sans réponse est possible — une saison sans équipe
                // engagée en produit une. Elle se relit telle qu'elle a été
                // écrite : vide, et non absente.
                rehydrater(row, par_campagne.remove(&id).unwrap_or_default())
            })
            .collect()
    }
}

impl PresenceSurveyRepository {
    async fn lire_les_reponses(
        &self,
        survey_id: &str,
    ) -> Result<Vec<Reponse>, PresenceSurveyRepositoryError> {
        let rows = sqlx::query(include_str!("sql/presences/find_answers_by_survey.sql"))
            .bind(survey_id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

        rows.iter().map(reponse_depuis).collect()
    }

    /// Les réponses de plusieurs campagnes, indexées par campagne.
    ///
    /// **Une requête, pas une par campagne.** C'est tout l'intérêt de la méthode
    /// qui l'appelle : réutiliser `lire_les_reponses` dans une boucle aurait fait
    /// `1 + N` allers-retours, déplaçant d'un cran le coût que la requête de liste
    /// venait de supprimer.
    ///
    /// Le court-circuit sur la liste vide n'est pas une optimisation : `= ANY` sur
    /// un tableau vide rendrait zéro ligne, mais l'aller-retour aurait lieu — et
    /// « aucune campagne ouverte » est le cas **courant** sur cette saison.
    async fn lire_les_reponses_groupees(
        &self,
        survey_ids: &[String],
    ) -> Result<HashMap<String, Vec<Reponse>>, PresenceSurveyRepositoryError> {
        if survey_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query(include_str!("sql/presences/find_answers_by_surveys.sql"))
            .bind(survey_ids)
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

        let mut par_campagne: HashMap<String, Vec<Reponse>> = HashMap::new();
        for row in &rows {
            let survey_id: String = row.try_get("survey_id").map_err(db_err)?;
            par_campagne
                .entry(survey_id)
                .or_default()
                .push(reponse_depuis(row)?);
        }
        Ok(par_campagne)
    }
}

/// Les identifiants des campagnes rendues, dans l'ordre de la requête.
fn identifiants(
    rows: &[sqlx::postgres::PgRow],
) -> Result<Vec<String>, PresenceSurveyRepositoryError> {
    rows.iter()
        .map(|r| r.try_get("id").map_err(db_err))
        .collect()
}

// ── Lecture ──────────────────────────────────────────────────────────────────

fn rehydrater(
    row: &sqlx::postgres::PgRow,
    reponses: Vec<Reponse>,
) -> Result<PresenceSurvey, PresenceSurveyRepositoryError> {
    let id: String = row.try_get("id").map_err(db_err)?;
    let close_le: Option<String> = row.try_get("close_le").map_err(db_err)?;
    let exemptee: Option<String> = row.try_get("exemptee").map_err(db_err)?;
    let appariee: bool = row.try_get("appariee").map_err(db_err)?;

    Ok(PresenceSurvey::rehydrater(
        SurveyId::try_new(&id).map_err(db_err)?,
        SeasonId::try_new(&texte(row, "season_id")?).map_err(db_err)?,
        MatchId::try_new(&texte(row, "round_id")?).map_err(db_err)?,
        SurveyDeadline::try_new(texte(row, "deadline")?).map_err(db_err)?,
        AutoRemind::new(row.try_get::<bool, _>("auto_remind").map_err(db_err)?),
        OpenedAt::try_new(texte(row, "opened_at")?).map_err(db_err)?,
        fermeture_depuis(close_le, &id)?,
        reponses,
        appariement_depuis(appariee, exemptee)?,
    ))
}

fn fermeture_depuis(
    close_le: Option<String>,
    id: &str,
) -> Result<Fermeture, PresenceSurveyRepositoryError> {
    match close_le {
        None => Ok(Fermeture::Aucune),
        Some(d) => FermeeLe::try_new(d)
            .map(|le| Fermeture::Decidee { le })
            .map_err(|_| incoherente("close_le", id)),
    }
}

/// `appariee` et `exemptee` se lisent ensemble : une exemptée sans appariement
/// n'a pas de sens, et c'est précisément ce que l'enum du domaine rend
/// inexprimable.
fn appariement_depuis(
    appariee: bool,
    exemptee: Option<String>,
) -> Result<Appariement, PresenceSurveyRepositoryError> {
    if !appariee {
        return Ok(Appariement::Aucun);
    }
    let exemptee = match exemptee {
        None => None,
        Some(t) => Some(TeamId::try_new(&t).map_err(db_err)?),
    };
    Ok(Appariement::Fait { exemptee })
}

fn reponse_depuis(row: &sqlx::postgres::PgRow) -> Result<Reponse, PresenceSurveyRepositoryError> {
    let id: String = row.try_get("id").map_err(db_err)?;
    Ok(Reponse::rehydrater(
        TeamId::try_new(&texte(row, "team_id")?).map_err(db_err)?,
        CoachId::try_new(&texte(row, "coach_id")?).map_err(db_err)?,
        SurveyToken::try_new(&texte(row, "token")?).map_err(db_err)?,
        presence_depuis(row, &id)?,
    ))
}

/// Trois colonnes redeviennent un enum. Les combinaisons absurdes ne peuvent pas
/// arriver — le `CHECK` les refuse — mais si elles arrivaient, elles seraient
/// dites plutôt que tues.
fn presence_depuis(
    row: &sqlx::postgres::PgRow,
    id: &str,
) -> Result<Presence, PresenceSurveyRepositoryError> {
    let presence = texte(row, "presence")?;
    let repondu_le: Option<String> = row.try_get("repondu_le").map_err(db_err)?;
    let saisi_par: Option<String> = row.try_get("saisi_par_admin").map_err(db_err)?;

    let venue = match presence.as_str() {
        "sans_reponse" => return Ok(Presence::SansReponse),
        "presente" => Venue::Presente,
        "absente" => Venue::Absente,
        autre => return Err(incoherente(&format!("presence='{autre}'"), id)),
    };

    let le = repondu_le
        .ok_or_else(|| incoherente("réponse déclarée sans horodatage", id))
        .and_then(|d| ReponduLe::try_new(d).map_err(|_| incoherente("repondu_le", id)))?;

    // Le canal ne se persiste pas (R28) : `NULL` couvre le jeton comme l'encart,
    // et relire `Coach(coach_id de la réponse)` est exactement vrai — le coach a
    // répondu. La question qu'on se pose après coup — « qui a dit qu'il
    // venait » — n'a que deux réponses possibles, pas trois.
    let par = match saisi_par {
        Some(admin) => Repondant::Organisateur(CoachId::try_new(&admin).map_err(db_err)?),
        None => Repondant::Coach(CoachId::try_new(&texte(row, "coach_id")?).map_err(db_err)?),
    };

    Ok(Presence::Declaree { venue, le, par })
}

fn texte(row: &sqlx::postgres::PgRow, col: &str) -> Result<String, PresenceSurveyRepositoryError> {
    row.try_get::<String, _>(col).map_err(db_err)
}

// ── Écriture ─────────────────────────────────────────────────────────────────

async fn ecrire_la_reponse(
    tx: &mut sqlx::PgConnection,
    survey_id: &SurveyId,
    reponse: &Reponse,
) -> Result<(), PresenceSurveyRepositoryError> {
    let (presence, repondu_le, saisi_par) = a_plat(reponse.presence());

    sqlx::query(include_str!("sql/presences/upsert_answer.sql"))
        .bind(SurveyId::new().to_string())
        .bind(survey_id.to_string())
        .bind(reponse.team_id().to_string())
        .bind(reponse.coach_id().to_string())
        .bind(reponse.token().to_string())
        .bind(presence)
        .bind(repondu_le)
        .bind(saisi_par)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    Ok(())
}

/// L'enum vers ses trois colonnes. Le `CHECK` de la migration refuse tout ce que
/// cette fonction ne peut pas produire.
fn a_plat(presence: &Presence) -> (&'static str, Option<String>, Option<String>) {
    match presence {
        Presence::SansReponse => ("sans_reponse", None, None),
        Presence::Declaree { venue, le, par } => {
            let venue = match venue {
                Venue::Presente => "presente",
                Venue::Absente => "absente",
            };
            let saisi_par = match par {
                Repondant::Organisateur(id) => Some(id.to_string()),
                Repondant::Coach(_) | Repondant::Jeton => None,
            };
            (venue, Some(le.to_string()), saisi_par)
        }
    }
}

fn ferme_le(fermeture: &Fermeture) -> Option<String> {
    match fermeture {
        Fermeture::Aucune => None,
        Fermeture::Decidee { le } => Some(le.to_string()),
    }
}

fn exemptee(appariement: &Appariement) -> Option<String> {
    match appariement {
        Appariement::Aucun => None,
        Appariement::Fait { exemptee } => exemptee.map(|t| t.to_string()),
    }
}

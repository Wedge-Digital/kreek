//! Carte 387 — les joueurs qui portaient déjà des compétences Élite.
//!
//! # Ce qu'elle ne fait pas
//!
//! Réécrire l'histoire. `players` est event-sourcé : les `PlayerSkillPurchased`
//! et `InitialSkillEarned` déjà écrits gardent le `value_delta` calculé le jour
//! de leur émission. C'est un événement **neuf** qui porte l'écart.
//!
//! # Un seul événement par joueur
//!
//! Et non un par compétence : c'est une correction unique, appliquée d'un bloc.
//! Un joueur sans compétence Élite n'en reçoit aucun — une migration ne doit
//! pas laisser de trace là où elle n'a rien changé.
//!
//! # Elle écrit dans la transaction du registre
//!
//! Donc pas via `IPlayerRepository::append`, qui ouvre la sienne. Elle appelle
//! les deux fonctions transactionnelles que le dépôt expose déjà —
//! `insert_player_event` et `upsert_player_projection` — de sorte que le SQL
//! d'écriture ne soit pas dupliqué ici : la migration écrit exactement comme
//! l'application.

use crate::app::players::domain::events::{PlayerDomainEvent, RecalibrationReason};
use crate::app::players::domain::player::{AcquisitionMode, PlayerId, TeamId};
use crate::app::players::domain::value_objects::KpoDelta;
use crate::app::players::io::repository::player_repository::{
    insert_player_event, upsert_player_projection,
};
use crate::infrastructure::data_migrations::DataMigration;
use crate::state::AppState;
use async_trait::async_trait;
use sqlx::{Postgres, Row, Transaction};

/// Ce que la carte ajoute au barème, dans les deux accès.
const BONUS_ELITE: i32 = 10;

pub struct BonusElite;

#[async_trait]
impl DataMigration for BonusElite {
    fn nom(&self) -> &'static str {
        "387-bonus-elite"
    }

    async fn executer(
        &self,
        state: &AppState,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<usize, String> {
        let joueurs: Vec<(String, String, i32)> =
            sqlx::query("SELECT player_id, team_id, version FROM players_proj ORDER BY player_id")
                .fetch_all(&mut **tx)
                .await
                .map_err(|e| format!("lecture des joueurs : {e}"))?
                .into_iter()
                .map(|r| (r.get("player_id"), r.get("team_id"), r.get("version")))
                .collect();

        let mut touches = 0usize;
        for (player_id, team_id, version) in joueurs {
            let pid = PlayerId(player_id.clone());

            // L'agrégat, et non la projection : c'est lui qui porte les
            // compétences acquises et leur mode d'acquisition.
            let joueur = state
                .players
                .repository
                .find_by_id(&pid)
                .await
                .map_err(|e| format!("chargement de {player_id} : {e}"))?;
            let Some(joueur) = joueur else { continue };

            let elites = joueur
                .acquired_skills
                .iter()
                // Une compétence donnée par un commissaire a compté **zéro**
                // dans la valeur du joueur — c'est la règle du mode
                // customisation, écrite dans `Player::apply`. Lui appliquer le
                // rattrapage ajouterait dix kPo au titre d'un barème qui ne lui
                // a jamais servi, et rendrait le joueur plus cher que la règle
                // ne le prévoit.
                .filter(|s| s.mode != AcquisitionMode::Customised)
                .filter(|s| {
                    state
                        .players
                        .skill_catalog
                        .find_skill(s.skill_id.as_ref())
                        .map(|c| c.is_elite)
                        .unwrap_or(false)
                })
                .count();

            if elites == 0 {
                continue;
            }

            let delta = KpoDelta::try_new(BONUS_ELITE * elites as i32)
                .map_err(|e| format!("delta pour {player_id} : {e:?}"))?;
            let event = PlayerDomainEvent::PlayerValueRecalibrated {
                player_id: pid,
                team_id: TeamId(team_id),
                delta,
                reason: RecalibrationReason::BonusElite,
            };

            insert_player_event(tx, &event, version + 1)
                .await
                .map_err(|e| format!("append pour {player_id} : {e}"))?;
            upsert_player_projection(tx, &event)
                .await
                .map_err(|e| format!("projection pour {player_id} : {e}"))?;
            touches += 1;
        }
        Ok(touches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::player::{Spp, ValueKpo};
    use crate::app::players::domain::value_objects::{
        CustomisationId, PositionNameVo, RosterLineId, SkillId, SkillName, SppCost,
    };
    use crate::app::shared_kernel::identity::ids::SpaceId;
    use crate::infrastructure::data_migrations::appliquer;

    /// Deux compétences du corpus d'exemple : l'une Élite, l'autre non.
    const ELITE: &str = "SECOND_SOUFFLE";
    const STANDARD: &str = "APPUI_FERME";

    async fn etat(pool: sqlx::PgPool) -> AppState {
        crate::compose(crate::config::AppConfig::for_tests(), pool).await
    }

    /// Sème un joueur puis lui pose les compétences données, comme le ferait
    /// un achat en SPP — c'est l'état que la migration doit trouver.
    async fn semer(
        state: &AppState,
        pool: &sqlx::PgPool,
        player_id: &str,
        competences: &[&str],
    ) -> (PlayerId, TeamId) {
        let pid = PlayerId(player_id.to_string());
        let tid = TeamId(format!("team-{player_id}"));

        let cree = PlayerDomainEvent::PlayerCreated {
            player_id: pid.clone(),
            team_id: tid.clone(),
            space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Piétaille".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("DEMO_GRANIT__PIETAILLE".to_string()).unwrap(),
            jersey: None,
            base_skills: vec![],
            starting_spp: Spp(0),
            starting_value: ValueKpo(100),
        };
        let mut tx = pool.begin().await.unwrap();
        insert_player_event(&mut tx, &cree, 1).await.unwrap();
        upsert_player_projection(&mut tx, &cree).await.unwrap();

        for (i, skill) in competences.iter().enumerate() {
            let acquise = PlayerDomainEvent::PlayerSkillPurchased {
                player_id: pid.clone(),
                team_id: tid.clone(),
                skill_id: SkillId::try_new(skill.to_string()).unwrap(),
                skill_name: SkillName::try_new(skill.to_string()).unwrap(),
                mode: AcquisitionMode::Chosen,
                spp_cost: SppCost::try_new(6).unwrap(),
                value_delta: ValueKpo(20),
                category_css: String::new(),
            };
            insert_player_event(&mut tx, &acquise, 2 + i as i32)
                .await
                .unwrap();
            upsert_player_projection(&mut tx, &acquise).await.unwrap();
        }
        tx.commit().await.unwrap();
        let _ = state;
        (pid, tid)
    }

    async fn valeur(pool: &sqlx::PgPool, player_id: &PlayerId) -> i32 {
        sqlx::query_scalar("SELECT value_kpo FROM players_proj WHERE player_id = $1")
            .bind(&player_id.0)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn recalibrations(pool: &sqlx::PgPool, player_id: &PlayerId) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM players_events
             WHERE player_id = $1 AND event_type = 'PlayerValueRecalibrated'",
        )
        .bind(&player_id.0)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn deux_elites_valent_vingt_de_plus_en_un_seul_evenement(pool: sqlx::PgPool) {
        let state = etat(pool.clone()).await;
        let (pid, _) = semer(&state, &pool, "deux-elites", &[ELITE, "MASSE_INEBRANLABLE"]).await;
        let avant = valeur(&pool, &pid).await;

        appliquer(&state, &pool, vec![Box::new(BonusElite)])
            .await
            .unwrap();

        assert_eq!(valeur(&pool, &pid).await, avant + 20);
        assert_eq!(
            recalibrations(&pool, &pid).await,
            1,
            "une correction unique, pas une par compétence"
        );
    }

    /// Le cas que le premier jet manquait.
    ///
    /// Une compétence donnée par un commissaire a compté **zéro** dans la
    /// valeur du joueur — `Player::apply` l'écrit en dur. Lui appliquer le
    /// rattrapage ajouterait dix kPo au titre d'un barème qui ne lui a jamais
    /// servi. Le filtre ne regardait que l'élitisme, jamais l'origine.
    #[sqlx::test]
    async fn une_elite_donnee_par_un_commissaire_ne_rapporte_rien(pool: sqlx::PgPool) {
        let state = etat(pool.clone()).await;
        let pid = PlayerId("elite-offerte".to_string());
        let tid = TeamId("team-elite-offerte".to_string());

        let cree = PlayerDomainEvent::PlayerCreated {
            player_id: pid.clone(),
            team_id: tid.clone(),
            space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Piétaille".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("DEMO_GRANIT__PIETAILLE".to_string()).unwrap(),
            jersey: None,
            base_skills: vec![],
            starting_spp: Spp(0),
            starting_value: ValueKpo(100),
        };
        let offerte = PlayerDomainEvent::PlayerSkillCustomised {
            player_id: pid.clone(),
            team_id: tid.clone(),
            customisation_id: CustomisationId::try_new("c-migration".to_string()).unwrap(),
            skill_id: SkillId::try_new(ELITE.to_string()).unwrap(),
            skill_name: SkillName::try_new(ELITE.to_string()).unwrap(),
            author: "Commissaire".to_string(),
        };
        let mut tx = pool.begin().await.unwrap();
        insert_player_event(&mut tx, &cree, 1).await.unwrap();
        upsert_player_projection(&mut tx, &cree).await.unwrap();
        insert_player_event(&mut tx, &offerte, 2).await.unwrap();
        upsert_player_projection(&mut tx, &offerte).await.unwrap();
        tx.commit().await.unwrap();

        let avant = valeur(&pool, &pid).await;
        appliquer(&state, &pool, vec![Box::new(BonusElite)])
            .await
            .unwrap();

        assert_eq!(
            valeur(&pool, &pid).await,
            avant,
            "une Élite offerte n'a jamais valorisé le joueur : rien à rattraper"
        );
        assert_eq!(recalibrations(&pool, &pid).await, 0);
    }

    /// Une migration ne doit pas laisser de trace là où elle n'a rien changé.
    #[sqlx::test]
    async fn un_joueur_sans_elite_ne_recoit_aucun_evenement(pool: sqlx::PgPool) {
        let state = etat(pool.clone()).await;
        let (pid, _) = semer(&state, &pool, "sans-elite", &[STANDARD]).await;
        let avant = valeur(&pool, &pid).await;

        appliquer(&state, &pool, vec![Box::new(BonusElite)])
            .await
            .unwrap();

        assert_eq!(valeur(&pool, &pid).await, avant);
        assert_eq!(recalibrations(&pool, &pid).await, 0);
    }

    /// La table de garde tient : un second démarrage ne recorrige rien.
    #[sqlx::test]
    async fn le_rejeu_n_ecrit_rien(pool: sqlx::PgPool) {
        let state = etat(pool.clone()).await;
        let (pid, _) = semer(&state, &pool, "rejeu", &[ELITE]).await;

        appliquer(&state, &pool, vec![Box::new(BonusElite)])
            .await
            .unwrap();
        let apres_une = valeur(&pool, &pid).await;
        appliquer(&state, &pool, vec![Box::new(BonusElite)])
            .await
            .unwrap();

        assert_eq!(valeur(&pool, &pid).await, apres_une);
        assert_eq!(recalibrations(&pool, &pid).await, 1);
    }
}

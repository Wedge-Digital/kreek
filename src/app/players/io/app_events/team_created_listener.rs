use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::{AcquisitionMode, PlayerId, Spp, TeamId, ValueKpo};
use crate::app::players::domain::value_objects::{
    JerseyVo, PositionNameVo, RosterLineId, SkillId, SkillName, SppCost,
};
use crate::app::players::io::repository::player_repository::{
    insert_player_event, upsert_player_projection,
};
use crate::app::players::ports::{ISkillCatalogPort, RepositoryError};
use crate::app::shared_kernel::app_events::team_creation_app_events::{
    PlayerPayload, TeamCreationAppEvent,
};
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::fmt;
use std::sync::Arc;

#[derive(Debug)]
pub enum ListenerError {
    AlreadyProcessed,
    Repository(RepositoryError),
    Database(sqlx::Error),
}

impl fmt::Display for ListenerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyProcessed => write!(f, "joueur déjà créé (idempotence)"),
            Self::Repository(e) => write!(f, "repository : {e}"),
            Self::Database(e) => write!(f, "base de données : {e}"),
        }
    }
}

impl From<RepositoryError> for ListenerError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::ConcurrentWrite => Self::AlreadyProcessed,
            other => Self::Repository(other),
        }
    }
}

// ── Résolution depuis le référentiel ─────────────────────────────────────────

fn resolve_base_skills(roster_line_id: &str, catalog: &dyn ISkillCatalogPort) -> Vec<SkillId> {
    catalog
        .find_position(roster_line_id)
        .map(|pos| {
            pos.base_skills
                .iter()
                .filter_map(|uid| SkillId::try_new(uid.clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}

fn base_position_kpo(roster_line_id: &str, catalog: &dyn ISkillCatalogPort) -> u32 {
    catalog
        .find_position(roster_line_id)
        .map(|pos| pos.cost)
        .unwrap_or(0)
}

fn skill_category_css(category: &str) -> &'static str {
    match category {
        "GENERAL" => "type-general",
        "STRENGTH" => "type-strength",
        "AGILITY" => "type-agility",
        "PASSING" => "type-passing",
        "MUTATION" => "type-mutation",
        _ => "type-general",
    }
}

fn skill_value_delta(is_primary: bool, is_elite: bool) -> u32 {
    let base = if is_primary { 20 } else { 40 };
    let bonus = if is_elite { 10 } else { 0 };
    base + bonus
}

fn parse_mode(mode: &str) -> AcquisitionMode {
    if mode == "Random" {
        AcquisitionMode::Random
    } else {
        AcquisitionMode::Chosen
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

async fn handle_player(
    team_id: &str,
    space_id: &str,
    payload: &PlayerPayload,
    pool: &PgPool,
    catalog: &dyn ISkillCatalogPort,
) -> Result<(), ListenerError> {
    let player_id = PlayerId(payload.instance_id.clone());
    let team_id_vo = TeamId(team_id.to_string());

    let base_skills = resolve_base_skills(&payload.roster_line_id, catalog);
    let starting_value = ValueKpo(base_position_kpo(&payload.roster_line_id, catalog));

    // ── Version 1 : création du joueur (sans compétences acquises) ────────────
    let space_id_vo = SpaceId::try_new(space_id).unwrap_or_else(|_| SpaceId::new());
    let position_name = PositionNameVo::try_new(payload.position_name.clone())
        .unwrap_or_else(|_| PositionNameVo::try_new("Joueur".to_string()).unwrap());
    let roster_line_id = RosterLineId::try_new(payload.roster_line_id.clone())
        .unwrap_or_else(|_| RosterLineId::try_new("unknown".to_string()).unwrap());
    let jersey_vo = payload
        .jersey
        .and_then(|j| JerseyVo::try_new(j as u16).ok());

    let created = PlayerDomainEvent::PlayerCreated {
        player_id: player_id.clone(),
        team_id: team_id_vo.clone(),
        space_id: space_id_vo,
        position_name,
        roster_line_id,
        jersey: jersey_vo,
        base_skills,
        starting_spp: Spp(0),
        starting_value,
    };

    let mut tx = pool.begin().await.map_err(ListenerError::Database)?;
    insert_player_event(&mut tx, &created, 1).await?;
    upsert_player_projection(&mut tx, &created).await?;
    tx.commit().await.map_err(ListenerError::Database)?;

    // ── Versions 2..n : une compétence acquise par event ─────────────────────
    let position = catalog.find_position(&payload.roster_line_id);

    for (idx, skill) in payload.acquired_skills.iter().enumerate() {
        let version = (2 + idx) as i32;
        let skill_ref = catalog.find_skill(&skill.skill_id);
        let skill_name = skill_ref
            .as_ref()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| skill.skill_id.clone());
        let is_primary = position
            .as_ref()
            .map(|pos| {
                pos.primary_categories.iter().any(|cat| {
                    skill_ref
                        .as_ref()
                        .map(|s| s.category == *cat)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        let is_elite = skill_ref.as_ref().map(|s| s.is_elite).unwrap_or(false);
        let category = skill_ref
            .as_ref()
            .map(|s| s.category.as_str())
            .unwrap_or("");
        let category_css = skill_category_css(category).to_string();
        let value_delta = ValueKpo(skill_value_delta(is_primary, is_elite));

        let skill_id_vo = SkillId::try_new(skill.skill_id.clone())
            .unwrap_or_else(|_| SkillId::try_new("unknown".to_string()).unwrap());
        let skill_name_vo = SkillName::try_new(skill_name.clone())
            .unwrap_or_else(|_| SkillName::try_new("Unknown".to_string()).unwrap());
        let spp_cost_vo =
            SppCost::try_new(skill.spp_cost).unwrap_or_else(|_| SppCost::try_new(0).unwrap());

        let earned = PlayerDomainEvent::InitialSkillEarned {
            player_id: player_id.clone(),
            team_id: team_id_vo.clone(),
            skill_id: skill_id_vo,
            skill_name: skill_name_vo,
            category_css,
            mode: parse_mode(&skill.mode),
            spp_cost: spp_cost_vo,
            is_primary,
            is_elite,
            value_delta,
        };

        let mut tx = pool.begin().await.map_err(ListenerError::Database)?;
        insert_player_event(&mut tx, &earned, version).await?;
        upsert_player_projection(&mut tx, &earned).await?;
        tx.commit().await.map_err(ListenerError::Database)?;
    }

    Ok(())
}

async fn handle_team_created(
    team_id: &str,
    space_id: &str,
    players: &[PlayerPayload],
    pool: &PgPool,
    catalog: &dyn ISkillCatalogPort,
) -> Result<(), ListenerError> {
    for payload in players {
        handle_player(team_id, space_id, payload, pool, catalog).await?;
    }
    Ok(())
}

// ── Abonnement ────────────────────────────────────────────────────────────────

pub fn init(app_event_bus: &EventBus, pool: PgPool, skill_catalog: Arc<dyn ISkillCatalogPort>) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(app_event) =
                        serde_json::from_value::<TeamCreationAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    let TeamCreationAppEvent::TeamCreated {
                        team_id,
                        space_id,
                        players,
                        ..
                    } = app_event;
                    if let Err(e) = handle_team_created(
                        &team_id,
                        &space_id,
                        &players,
                        &pool,
                        skill_catalog.as_ref(),
                    )
                    .await
                    {
                        match e {
                            ListenerError::AlreadyProcessed => tracing::warn!(
                                "players team_created_listener: joueurs déjà créés pour {team_id}"
                            ),
                            other => tracing::error!(
                                "players team_created_listener: échec pour {team_id}: {other}"
                            ),
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("players team_created_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::{AcquisitionMode, PlayerId, TeamId, ValueKpo};
use crate::app::players::domain::value_objects::{SkillId, SkillName, SppCost};
use crate::app::players::io::app_events::player_creation::{creer_joueur, ListenerError};
use crate::app::players::io::repository::player_repository::{
    insert_player_event, upsert_player_projection,
};
use crate::app::players::ports::ISkillCatalogPort;
use crate::app::shared_kernel::app_events::team_creation_app_events::{
    PlayerPayload, TeamCreationAppEvent,
};
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::Instrument;

/// Valeur ajoutée par une compétence obtenue **à la création** de l'équipe.
///
/// Même table que l'achat en SPP (`improvement_cost_service::resolve_skill_cost`) :
/// une compétence vaut le même prix quelle que soit son origine. Ce n'était pas
/// le cas avant la carte 249, où ce chemin appliquait un barème codé en dur,
/// assorti d'un bonus élite que l'autre chemin ignorait.
///
/// L'élitisme entre dans le calcul depuis la carte 387. La parité posée par la
/// 249 n'en est pas rompue : c'est le barème **commun** qui a gagné deux cases,
/// et les deux origines le lisent toujours au même endroit.
pub fn initial_skill_value_delta(
    catalog: &dyn ISkillCatalogPort,
    is_primary: bool,
    is_elite: bool,
) -> ValueKpo {
    ValueKpo(catalog.skill_value_delta(!is_primary, is_elite))
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

    creer_joueur(
        team_id,
        space_id,
        &payload.instance_id,
        &payload.roster_line_id,
        &payload.position_name,
        payload.jersey.map(|j| j as u16),
        pool,
        catalog,
    )
    .await?;

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
        let value_delta = initial_skill_value_delta(catalog, is_primary, is_elite);

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
    event_bus: &EventBus,
) -> Result<(), ListenerError> {
    for payload in players {
        handle_player(team_id, space_id, payload, pool, catalog).await?;
    }

    // Émis **après** la boucle : c'est ce qui garantit qu'un recalcul de TV
    // déclenché par cet événement voit un roster complet. `teams` et `players`
    // s'abonnent au même `TeamCreated` dans deux tâches indépendantes — sans ce
    // signal, `teams` peut atteindre `ReadyToPlay` avant qu'aucun joueur
    // n'existe et figer une TV à zéro.
    //
    // L'émetteur de l'enveloppe est le `team_id`, pas un joueur : c'est un fait
    // d'équipe, et c'est lui que le listener de `teams` lira pour savoir quelle
    // équipe recalculer.
    let completed = PlayerDomainEvent::InitialRosterCompleted {
        team_id: TeamId(team_id.to_string()),
        player_count: players.len() as u32,
    };
    emettre(event_bus, completed.to_enveloppe(team_id));

    Ok(())
}

// ── Abonnement ────────────────────────────────────────────────────────────────

pub fn init(
    app_event_bus: &EventBus,
    event_bus: EventBus,
    pool: PgPool,
    skill_catalog: Arc<dyn ISkillCatalogPort>,
) {
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
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
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    if let Err(e) = handle_team_created(
                        &team_id,
                        &space_id,
                        &players,
                        &pool,
                        skill_catalog.as_ref(),
                        &event_bus,
                    )
                    .instrument(span)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::app_events::players_app_events::PlayersAppEvent;
    use crate::common::services::event_bus::event_bus::new_bus;

    fn roster_completed(team_id: &str, count: u32) -> PlayerDomainEvent {
        PlayerDomainEvent::InitialRosterCompleted {
            team_id: TeamId(team_id.to_string()),
            player_count: count,
        }
    }

    /// L'enveloppe porte le `team_id` en émetteur, et non un joueur : c'est un
    /// fait d'équipe, et c'est cet émetteur que le listener de `teams` lit pour
    /// savoir quelle équipe recalculer.
    #[test]
    fn l_evenement_de_roster_complet_a_l_equipe_pour_emetteur() {
        let enveloppe = roster_completed("t-42", 11).to_enveloppe("t-42");

        assert_eq!(enveloppe.emitter, "t-42");
        assert_eq!(enveloppe.event_type, "InitialRosterCompleted");
    }

    /// Le domain event doit franchir la frontière — sans ce mapping, le
    /// publisher ne produirait rien et `teams` ne recalculerait jamais.
    #[test]
    fn le_roster_complet_se_convertit_en_app_event() {
        let app_event = roster_completed("t-42", 11)
            .to_app_event()
            .expect("InitialRosterCompleted doit franchir la frontière vers teams");

        let PlayersAppEvent::InitialRosterCompleted {
            team_id,
            player_count,
        } = app_event
        else {
            panic!("variante inattendue");
        };
        assert_eq!(team_id, "t-42");
        assert_eq!(player_count, 11);
    }

    /// Les événements internes à `players` ne doivent pas fuir vers `teams`.
    #[test]
    fn les_autres_evenements_ne_franchissent_pas_la_frontiere() {
        let touchdown = PlayerDomainEvent::MatchImpactReverted {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t-42".into()),
            match_report_id: crate::app::players::domain::match_impact::MatchReportId("mr".into()),
        };
        assert!(touchdown.to_app_event().is_none());
    }

    #[tokio::test]
    async fn le_bus_interne_recoit_l_evenement_en_fin_de_boucle() {
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let _ = bus.send(roster_completed("t-7", 12).to_enveloppe("t-7"));

        let enveloppe = rx.recv().await.expect("l'événement doit être publié");
        assert_eq!(enveloppe.emitter, "t-7");
    }
}

use crate::app::players::domain::events::PlayerDomainEvent;
use serde::{Deserialize, Serialize};

// ── Value objects ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spp(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueKpo(pub u32);

// ── Compétences acquises ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquiredSkill {
    pub skill_id:   String,
    pub skill_name: String,
    pub mode:       AcquisitionMode,
    pub spp_cost:   u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcquisitionMode {
    Chosen,
    Random,
}

// ── Agrégat Player ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Player {
    pub id:              PlayerId,
    pub team_id:         TeamId,
    pub space_id:        String,
    pub position_name:   String,
    pub roster_line_id:  String,
    pub personal_name:   String,
    pub jersey:          Option<u8>,
    pub base_skills:     Vec<String>,
    pub acquired_skills: Vec<AcquiredSkill>,
    pub spp:             Spp,
    pub value:           ValueKpo,
}

impl Player {
    /// Reconstruit l'état de l'agrégat en rejouant une séquence d'events.
    /// Retourne `None` si la séquence est vide.
    pub fn from_events(events: &[PlayerDomainEvent]) -> Option<Self> {
        let mut state: Option<Self> = None;
        for event in events {
            state = Self::apply(state, event);
        }
        state
    }

    fn apply(current: Option<Self>, event: &PlayerDomainEvent) -> Option<Self> {
        match event {
            PlayerDomainEvent::PlayerCreated {
                player_id, team_id, space_id, position_name, roster_line_id,
                personal_name, jersey, base_skills, starting_spp, starting_value,
            } => {
                if current.is_some() {
                    return current;
                }
                Some(Self {
                    id:              player_id.clone(),
                    team_id:         team_id.clone(),
                    space_id:        space_id.clone(),
                    position_name:   position_name.clone(),
                    roster_line_id:  roster_line_id.clone(),
                    personal_name:   personal_name.clone(),
                    jersey:          *jersey,
                    base_skills:     base_skills.clone(),
                    acquired_skills: vec![],
                    spp:             *starting_spp,
                    value:           *starting_value,
                })
            }
            PlayerDomainEvent::InitialSkillEarned {
                skill_id, skill_name, mode, spp_cost, value_delta, ..
            } => {
                let mut player = current?;
                player.acquired_skills.push(AcquiredSkill {
                    skill_id:   skill_id.clone(),
                    skill_name: skill_name.clone(),
                    mode:       *mode,
                    spp_cost:   *spp_cost,
                });
                player.value = ValueKpo(player.value.0 + value_delta.0);
                Some(player)
            }
        }
    }
}

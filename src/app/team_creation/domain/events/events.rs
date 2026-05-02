use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum GameEvent {
    DraftTeamCreated(DraftTeamCreatedEvent),
    RulesetSelected(RulesetSelectedEvent),
    RosterSelected(RosterSelectedEvent),
}
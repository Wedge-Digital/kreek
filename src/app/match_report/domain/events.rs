use crate::app::match_report::domain::value_objects::{
    ActionId, ActionPlayer, D3Roll, DedicatedFans, FanFactorMod, InducementPurchase,
    MatchActionType, MatchGain, MatchReportOrigin, TeamSide, TeamValue, TempPlayer,
};
use crate::app::shared_kernel::bloodbowl::inducement_definition::InducementId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::match_report::domain::value_objects::TurnNumber;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MatchReportDomainEvent {
    MatchReportCreated {
        match_report_id: MatchReportId,
        space_id: SpaceId,
        competition_id: CompetitionId,
        season_id: SeasonId,
        round_id: RoundId,
        home_team_id: TeamId,
        away_team_id: TeamId,
        created_by: CoachId,
        origin: MatchReportOrigin,
        #[serde(default)]
        pairing_id: Option<String>,
    },
    SelectionUpdated {
        home_team_id: TeamId,
        away_team_id: TeamId,
        updated_by: CoachId,
    },
    SelectionConfirmed {
        confirmed_by: CoachId,
    },
    FanFactorRecorded {
        home_fan_roll: D3Roll,
        away_fan_roll: D3Roll,
        #[serde(default)]
        home_dedicated_fans: DedicatedFans,
        #[serde(default)]
        away_dedicated_fans: DedicatedFans,
        recorded_by: CoachId,
    },
    MatchReportCancelled {
        reason: String,
        /// Équipes du rapport annulé. Le BC `teams` en a besoin pour libérer le
        /// verrou de saisie posé à la confirmation de la sélection — et l'état
        /// `Cancelled` ne retient qu'un id et une raison, donc l'information ne
        /// serait plus lisible après coup.
        ///
        /// `None` sur les événements persistés avant l'ajout de ces champs :
        /// seuls des brouillons étaient annulables à l'époque, sans verrou à
        /// défaire.
        #[serde(default)]
        home_team_id: Option<TeamId>,
        #[serde(default)]
        away_team_id: Option<TeamId>,
    },
    TeamValuesRecorded {
        home_team_value: TeamValue,
        away_team_value: TeamValue,
        recorded_by: CoachId,
    },
    InducementsRecorded {
        team_id: TeamId,
        purchases: Vec<InducementPurchase>,
        recorded_by: CoachId,
    },
    StarPlayerEngaged {
        team_id: TeamId,
        star_player_uid: InducementId,
        recorded_by: CoachId,
    },
    TempPlayersInitialized {
        team_id: TeamId,
        players: Vec<TempPlayer>,
    },
    TempPlayersReset {
        team_id: TeamId,
    },
    ActionRecorded {
        action_id: ActionId,
        team_side: TeamSide,
        turn: TurnNumber,
        player: ActionPlayer,
        action: MatchActionType,
        player_display_name: String,
        #[serde(default)]
        player_position: String,
        recorded_by: CoachId,
    },
    ActionDeleted {
        action_id: ActionId,
        team_side: TeamSide,
        deleted_by: CoachId,
    },
    PostMatchRecorded {
        home_gain: MatchGain,
        away_gain: MatchGain,
        home_fan_mod: FanFactorMod,
        away_fan_mod: FanFactorMod,
        summary_title: Option<String>,
        summary_body: Option<String>,
        recorded_by: CoachId,
    },
    MatchReportPublished {
        published_by: CoachId,
        published_at: DateTime<Utc>,
    },
    /// Ramène un rapport publié en état corrigeable. Sans motif : la correction
    /// n'en exige pas.
    MatchReportUnpublished {
        unpublished_by: CoachId,
        unpublished_at: DateTime<Utc>,
    },
}

impl MatchReportDomainEvent {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::MatchReportCreated { .. } => "MatchReportCreated",
            Self::SelectionUpdated { .. } => "SelectionUpdated",
            Self::SelectionConfirmed { .. } => "SelectionConfirmed",
            Self::FanFactorRecorded { .. } => "FanFactorRecorded",
            Self::MatchReportCancelled { .. } => "MatchReportCancelled",
            Self::TeamValuesRecorded { .. } => "TeamValuesRecorded",
            Self::InducementsRecorded { .. } => "InducementsRecorded",
            Self::StarPlayerEngaged { .. } => "StarPlayerEngaged",
            Self::TempPlayersInitialized { .. } => "TempPlayersInitialized",
            Self::TempPlayersReset { .. } => "TempPlayersReset",
            Self::ActionRecorded { .. } => "ActionRecorded",
            Self::ActionDeleted { .. } => "ActionDeleted",
            Self::PostMatchRecorded { .. } => "PostMatchRecorded",
            Self::MatchReportPublished { .. } => "MatchReportPublished",
            Self::MatchReportUnpublished { .. } => "MatchReportUnpublished",
        }
    }

    pub fn schema_version(&self) -> &'static str {
        "1.0"
    }

    /// Enveloppe pour publication sur le bus interne du BC. `match_report_id` est fourni
    /// par l'appelant (déjà connu du use case) plutôt que dupliqué dans chaque variant.
    pub fn to_enveloppe(&self, match_report_id: &str) -> crate::common::event_envelope::EventEnvelope {
        crate::common::event_envelope::EventEnvelope {
            event_id: crate::app::shared_kernel::identity::ids::EventId::new().to_string(),
            emitter: match_report_id.to_string(),
            event_type: self.type_name().to_string(),
            tags: serde_json::json!([]),
            payload: serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}

use crate::app::players::domain::match_impact::{
    InjuryType, MatchContext, MatchReportId, SppEarned, StatKind,
};
use crate::app::players::domain::player::{AcquisitionMode, PlayerId, Spp, TeamId, ValueKpo};
use crate::app::players::domain::value_objects::{
    DisplayOrder, JerseyVo, PersonalName, PositionNameVo, RosterLineId, SkillId, SkillName, SppCost,
};
use crate::app::shared_kernel::identity::ids::SpaceId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerDomainEvent {
    /// Le roster initial d'une équipe est au complet — tous ses joueurs ont été
    /// créés et leurs compétences de départ enregistrées.
    ///
    /// **Jamais persisté** : aucun agrégat de `players` ne porte le roster, la
    /// création initiale étant une boucle sur N joueurs dans un listener. Cet
    /// événement n'existe que pour être publié, et c'est la seule concession de
    /// la série — préférable à laisser `teams` valoriser lui-même le payload,
    /// ce qui dupliquerait la règle de valorisation de `players` (la duplication
    /// même qui avait produit les deux tables divergentes de la carte 249).
    InitialRosterCompleted { team_id: TeamId, player_count: u32 },
    PlayerCreated {
        player_id: PlayerId,
        team_id: TeamId,
        space_id: SpaceId,
        position_name: PositionNameVo,
        roster_line_id: RosterLineId,
        jersey: Option<JerseyVo>,
        base_skills: Vec<SkillId>,
        starting_spp: Spp,
        starting_value: ValueKpo,
    },
    InitialSkillEarned {
        player_id: PlayerId,
        team_id: TeamId,
        skill_id: SkillId,
        skill_name: SkillName,
        category_css: String,
        mode: AcquisitionMode,
        spp_cost: SppCost,
        is_primary: bool,
        is_elite: bool,
        value_delta: ValueKpo,
    },

    // ── Dépense de SPP post-match (phase PlayerImprovement) ────────────────────
    PlayerSkillPurchased {
        player_id: PlayerId,
        team_id: TeamId,
        skill_id: SkillId,
        skill_name: SkillName,
        category_css: String,
        mode: AcquisitionMode,
        spp_cost: SppCost,
        value_delta: ValueKpo,
    },
    PlayerStatIncreased {
        player_id: PlayerId,
        team_id: TeamId,
        stat: StatKind,
        spp_cost: SppCost,
        value_delta: ValueKpo,
    },

    // ── Impact des rapports de match ───────────────────────────────────────────
    // player_id/team_id sont redondants avec l'agrégat (déjà identifié par son
    // propre flux d'events) mais nécessaires à la couche persistance, qui route
    // l'append par (player_id, team_id) — même besoin que PlayerCreated/InitialSkillEarned.
    TouchdownScored {
        player_id: PlayerId,
        team_id: TeamId,
        context: MatchContext,
        spp_earned: SppEarned,
    },
    PassCompleted {
        player_id: PlayerId,
        team_id: TeamId,
        context: MatchContext,
        spp_earned: SppEarned,
    },
    InterceptionMade {
        player_id: PlayerId,
        team_id: TeamId,
        context: MatchContext,
        spp_earned: SppEarned,
    },
    CasualtyInflicted {
        player_id: PlayerId,
        team_id: TeamId,
        context: MatchContext,
        spp_earned: SppEarned,
    },
    MatchMvpNamed {
        player_id: PlayerId,
        team_id: TeamId,
        context: MatchContext,
        spp_earned: SppEarned,
    },
    FoulCommitted {
        player_id: PlayerId,
        team_id: TeamId,
        context: MatchContext,
    },
    InjurySustained {
        player_id: PlayerId,
        team_id: TeamId,
        context: MatchContext,
        injury_type: InjuryType,
    },
    PlayerAvailabilityRestored {
        player_id: PlayerId,
        team_id: TeamId,
        match_report_id: MatchReportId,
    },
    MatchConcluded {
        player_id: PlayerId,
        team_id: TeamId,
        context: MatchContext,
        team_score: u8,
        opponent_score: u8,
    },

    /// L'impact de ce match sur ce joueur a été annulé — le rapport a été
    /// dépublié pour correction.
    ///
    /// Événement **mince** à dessein : il énonce un fait, pas les montants à
    /// retrancher. Ceux-ci vivent dans l'instantané `last_match` de l'agrégat,
    /// lui-même reconstruit par les événements qui précèdent. Au rejeu, `apply`
    /// dispose donc exactement des mêmes valeurs qu'au moment de l'émission.
    MatchImpactReverted {
        player_id: PlayerId,
        team_id: TeamId,
        match_report_id: MatchReportId,
    },
    /// Le coach a renvoyé ce joueur. Il cesse d'appartenir à l'effectif ; il
    /// n'est pas effacé — `players` est event-sourcé, et le joueur garde ses
    /// SPP, ses compétences et son historique.
    ///
    /// Homonyme de l'événement domaine de `teams` et de l'app event qui les
    /// relie : nommer le même fait pareil des deux côtés n'est pas nommer un
    /// événement d'après son origine externe, que le CLAUDE.md interdit.
    PlayerDismissed {
        player_id: PlayerId,
        team_id: TeamId,
    },

    // ── Édition de l'effectif par le coach ─────────────────────────────────────
    // Trois événements distincts plutôt qu'un `PlayerEdited` fourre-tout : ce
    // sont trois gestes différents, et le use case n'émet que ceux dont le champ
    // a réellement changé. Le `Option::None` est signifiant — il efface la valeur.
    PlayerRenamed {
        player_id: PlayerId,
        team_id: TeamId,
        personal_name: Option<PersonalName>,
    },
    PlayerJerseyChanged {
        player_id: PlayerId,
        team_id: TeamId,
        jersey: Option<JerseyVo>,
    },
    PlayerReordered {
        player_id: PlayerId,
        team_id: TeamId,
        display_order: DisplayOrder,
    },
}

impl PlayerDomainEvent {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::PlayerCreated { .. } => "PlayerCreated",
            Self::InitialSkillEarned { .. } => "InitialSkillEarned",
            Self::PlayerSkillPurchased { .. } => "PlayerSkillPurchased",
            Self::PlayerStatIncreased { .. } => "PlayerStatIncreased",
            Self::TouchdownScored { .. } => "TouchdownScored",
            Self::PassCompleted { .. } => "PassCompleted",
            Self::InterceptionMade { .. } => "InterceptionMade",
            Self::CasualtyInflicted { .. } => "CasualtyInflicted",
            Self::MatchMvpNamed { .. } => "MatchMvpNamed",
            Self::FoulCommitted { .. } => "FoulCommitted",
            Self::InjurySustained { .. } => "InjurySustained",
            Self::PlayerAvailabilityRestored { .. } => "PlayerAvailabilityRestored",
            Self::MatchConcluded { .. } => "MatchConcluded",
            Self::MatchImpactReverted { .. } => "MatchImpactReverted",
            Self::PlayerDismissed { .. } => "PlayerDismissed",
            Self::InitialRosterCompleted { .. } => "InitialRosterCompleted",
            Self::PlayerRenamed { .. } => "PlayerRenamed",
            Self::PlayerJerseyChanged { .. } => "PlayerJerseyChanged",
            Self::PlayerReordered { .. } => "PlayerReordered",
        }
    }

    /// Conversion vers l'app event franchissant la frontière vers `teams` —
    /// `None` pour tout ce qui reste interne à `players`, c'est-à-dire presque
    /// tout. Seul le publisher (couche IO) appelle cette méthode : un listener
    /// n'émet jamais d'app event directement.
    pub fn to_app_event(
        &self,
    ) -> Option<crate::app::shared_kernel::app_events::players_app_events::PlayersAppEvent> {
        use crate::app::shared_kernel::app_events::players_app_events::PlayersAppEvent;
        match self {
            Self::InitialRosterCompleted {
                team_id,
                player_count,
            } => Some(PlayersAppEvent::InitialRosterCompleted {
                team_id: team_id.0.clone(),
                player_count: *player_count,
            }),
            Self::PlayerDismissed { player_id, team_id } => {
                Some(PlayersAppEvent::PlayerDismissed {
                    team_id: team_id.0.clone(),
                    player_id: player_id.0.clone(),
                })
            }
            // Joker : le compilateur ne signalera pas un événement qu'on
            // oublierait de faire sortir du BC. Ajouter un bras est délibéré.
            _ => None,
        }
    }

    /// Publication sur le bus interne du BC (pas l'event store — cf.
    /// `IPlayerRepository::append` pour la persistance). Seuls les use cases
    /// qui doivent notifier un autre BC (ex. dépense de SPP → `teams`)
    /// publient sur ce bus.
    pub fn to_enveloppe(&self, player_id: &str) -> crate::common::event_envelope::EventEnvelope {
        crate::common::event_envelope::EventEnvelope {
            event_id: crate::app::shared_kernel::identity::ids::EventId::new().to_string(),
            emitter: player_id.to_string(),
            event_type: self.type_name().to_string(),
            tags: serde_json::json!([]),
            payload: serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}

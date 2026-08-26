use crate::app::players::domain::match_impact::{
    InjuryType, MatchContext, MatchReportId, SppEarned, StatKind,
};
use crate::app::players::domain::player::{AcquisitionMode, PlayerId, Spp, TeamId, ValueKpo};
use crate::app::players::domain::value_objects::{
    CustomisationId, DisplayOrder, JerseyVo, KpoDelta, PersonalName, PositionNameVo, RosterLineId,
    SkillId, SkillName, SppAmount, SppCost,
};
use crate::app::shared_kernel::identity::ids::SpaceId;
use serde::{Deserialize, Serialize};

/// Ce qui a motivé une recalibration de valeur.
///
/// Un enum et non un texte libre : ces événements se relisent des années plus
/// tard, et « pourquoi cette valeur a-t-elle bougé sans que personne n'y
/// touche » est la première question qu'on leur posera. Une chaîne libre y
/// répondrait mal et ne se filtrerait pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecalibrationReason {
    /// Carte 387 — les compétences Élite valent dix kPo de plus, et les
    /// joueurs qui en portaient avant la règle ont été corrigés d'un bloc.
    BonusElite,
}

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
    /// Le joueur se met à haïr une espèce après avoir encaissé un coup.
    ///
    /// **Aucun champ de valeur**, contrairement à l'état projeté qui portera des
    /// zéros : un trait gagné en encaissant un coup ne se paie pas et ne
    /// renchérit pas le joueur — le champ n'existe pas, il ne vaut pas zéro.
    /// C'est la distinction que le projet tient depuis la customisation.
    ///
    /// `context` porte le match, ce qui rend le gain défaisable à la
    /// dépublication sans qu'aucun compteur ait à être tenu à jour.
    PlayerHatredGained {
        player_id: PlayerId,
        team_id: TeamId,
        context: MatchContext,
        skill_id: SkillId,
        skill_name: SkillName,
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

    // ── Customisation par un commissaire ──────────────────────────────────────
    // Quatre événements distincts, un par famille, délibérément séparés des
    // événements d'évolution normale : c'est ce qui permet à l'historique du
    // joueur de distinguer ce qu'il a gagné de ce qu'on lui a donné.
    //
    // `author` est le nom du commissaire qui valide. Le panier étant propre au
    // joueur et non à son auteur, c'est le validateur qui endosse tout un lot —
    // écart connu à la traçabilité, la concurrence entre commissaires ayant été
    // jugée improbable au niveau métier.
    //
    // Ni la compétence ni la caractéristique ne portent de `value_delta` : seul
    // le prix déplace la valeur d'équipe. Il n'existe pas, il ne vaut pas zéro —
    // un champ à zéro inviterait quelqu'un à le remplir.
    /// `skill_name` est porté par l'événement comme il l'est par
    /// `PlayerSkillPurchased` : le domaine ne sait pas résoudre un nom depuis un
    /// identifiant, le catalogue appartenant à `references`.
    PlayerSkillCustomised {
        player_id: PlayerId,
        team_id: TeamId,
        customisation_id: CustomisationId,
        skill_id: SkillId,
        skill_name: SkillName,
        author: String, // arch:ok nom d'affichage, aucun invariant à protéger
    },
    /// `offset` est **brut**, pas en crans : le domaine a traduit, et
    /// l'événement enregistre ce qui a été réellement appliqué. Un rejeu ne
    /// doit dépendre d'aucune convention externe.
    PlayerStatCustomised {
        player_id: PlayerId,
        team_id: TeamId,
        customisation_id: CustomisationId,
        stat: StatKind,
        offset: i8,     // arch:ok offset brut déjà validé par les bornes du domaine
        author: String, // arch:ok
    },
    PlayerValueCustomised {
        player_id: PlayerId,
        team_id: TeamId,
        customisation_id: CustomisationId,
        delta: KpoDelta,
        author: String, // arch:ok
    },
    /// Une correction de valeur appliquée par une **migration de données**,
    /// jamais par un geste d'utilisateur.
    ///
    /// `PlayerValueCustomised` n'est pas réutilisé : il dit « un commissaire a
    /// posé cette valeur hors barème ». Le relire dans un an sur un joueur que
    /// personne n'a touché induirait en erreur — et il déclenche en plus un
    /// recalcul de valeur d'équipe dont une migration n'a pas besoin, la
    /// suivante les recalculant toutes.
    ///
    /// On ne réécrit pas l'histoire : les `PlayerSkillPurchased` et
    /// `InitialSkillEarned` déjà écrits gardent le `value_delta` calculé le
    /// jour de leur émission. C'est cet événement-ci qui porte l'écart.
    PlayerValueRecalibrated {
        player_id: PlayerId,
        team_id: TeamId,
        delta: KpoDelta,
        reason: RecalibrationReason,
    },
    PlayerSppCustomised {
        player_id: PlayerId,
        team_id: TeamId,
        customisation_id: CustomisationId,
        amount: SppAmount,
        author: String, // arch:ok
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
            Self::PlayerHatredGained { .. } => "PlayerHatredGained",
            Self::PlayerAvailabilityRestored { .. } => "PlayerAvailabilityRestored",
            Self::MatchConcluded { .. } => "MatchConcluded",
            Self::MatchImpactReverted { .. } => "MatchImpactReverted",
            Self::PlayerDismissed { .. } => "PlayerDismissed",
            Self::InitialRosterCompleted { .. } => "InitialRosterCompleted",
            Self::PlayerRenamed { .. } => "PlayerRenamed",
            Self::PlayerJerseyChanged { .. } => "PlayerJerseyChanged",
            Self::PlayerReordered { .. } => "PlayerReordered",
            Self::PlayerSkillCustomised { .. } => "PlayerSkillCustomised",
            Self::PlayerStatCustomised { .. } => "PlayerStatCustomised",
            Self::PlayerValueCustomised { .. } => "PlayerValueCustomised",
            Self::PlayerValueRecalibrated { .. } => "PlayerValueRecalibrated",
            Self::PlayerSppCustomised { .. } => "PlayerSppCustomised",
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
            // Seule la customisation de **prix** franchit la frontière : c'est
            // la seule qui déplace la valeur d'équipe. Compétence,
            // caractéristique et SPP customisés restent dans le BC.
            Self::PlayerValueCustomised {
                player_id, team_id, ..
            } => Some(PlayersAppEvent::PlayerValueCustomised {
                team_id: team_id.0.clone(),
                player_id: player_id.0.clone(),
            }),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::value_objects::SkillName;
    use crate::app::shared_kernel::app_events::players_app_events::PlayersAppEvent;

    fn ids() -> (PlayerId, TeamId, CustomisationId) {
        (
            PlayerId("p1".into()),
            TeamId("t1".into()),
            CustomisationId::try_new("c1".to_string()).unwrap(),
        )
    }

    /// La règle la plus contre-intuitive de la fonctionnalité, et la seule
    /// qu'un lecteur de bonne foi prendrait pour un bug : **seul le prix
    /// franchit la frontière du BC**.
    ///
    /// Dans la progression normale, une compétence achetée en SPP augmente la
    /// valeur du joueur. La customisation, elle, **pose** une valeur au lieu de
    /// la dériver d'un barème — une compétence offerte par un commissaire ne
    /// déplace donc pas la valeur d'équipe.
    ///
    /// Ce test est ce qui tient l'asymétrie : le code, lui, se contenterait
    /// d'un bras de plus.
    #[test]
    fn seule_la_customisation_de_prix_sort_du_bc() {
        let (player_id, team_id, customisation_id) = ids();

        let prix = PlayerDomainEvent::PlayerValueCustomised {
            player_id: player_id.clone(),
            team_id: team_id.clone(),
            customisation_id: customisation_id.clone(),
            delta: KpoDelta::try_new(-15).unwrap(),
            author: "Bagouze".into(),
        };
        assert!(matches!(
            prix.to_app_event(),
            Some(PlayersAppEvent::PlayerValueCustomised { .. })
        ));

        let competence = PlayerDomainEvent::PlayerSkillCustomised {
            player_id: player_id.clone(),
            team_id: team_id.clone(),
            customisation_id: customisation_id.clone(),
            skill_id: SkillId::try_new("BLOCK".to_string()).unwrap(),
            skill_name: SkillName::try_new("Bloc".to_string()).unwrap(),
            author: "Bagouze".into(),
        };
        assert!(
            competence.to_app_event().is_none(),
            "une compétence customisée ne déplace pas la valeur d'équipe"
        );

        let caracteristique = PlayerDomainEvent::PlayerStatCustomised {
            player_id: player_id.clone(),
            team_id: team_id.clone(),
            customisation_id: customisation_id.clone(),
            stat: StatKind::Ag,
            offset: -1,
            author: "Bagouze".into(),
        };
        assert!(
            caracteristique.to_app_event().is_none(),
            "une caractéristique customisée ne déplace pas la valeur d'équipe"
        );

        let spp = PlayerDomainEvent::PlayerSppCustomised {
            player_id,
            team_id,
            customisation_id,
            amount: SppAmount::try_new(5).unwrap(),
            author: "Bagouze".into(),
        };
        assert!(spp.to_app_event().is_none());
    }

    /// L'émetteur est l'**équipe**, pas le joueur : c'est elle que le listener
    /// de `teams` recalculera.
    #[test]
    fn l_app_event_de_prix_est_emis_au_nom_de_l_equipe() {
        let (player_id, team_id, customisation_id) = ids();
        let app_event = PlayerDomainEvent::PlayerValueCustomised {
            player_id,
            team_id,
            customisation_id,
            delta: KpoDelta::try_new(10).unwrap(),
            author: "Bagouze".into(),
        }
        .to_app_event()
        .unwrap();

        let enveloppe = app_event.to_enveloppe();
        assert_eq!(enveloppe.emitter, "t1");
        assert_eq!(enveloppe.event_type, "PlayersPlayerValueCustomised");
    }
}

use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish;
use crate::app::match_report::domain::value_objects::{
    CorrectionEligibility, D3Roll, DedicatedFans, FanFactorMod, InducementPurchase, MatchAction,
    MatchGain, MatchReportOrigin, TempPlayer,
};
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::bloodbowl::inducement_definition::InducementId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct MatchReportPublished {
    pub id: MatchReportId,
    pub space_id: SpaceId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub created_by: CoachId,
    pub origin: MatchReportOrigin,
    pub pairing_id: Option<String>,
    pub home_fan_roll: Option<D3Roll>,
    pub away_fan_roll: Option<D3Roll>,
    pub home_dedicated_fans: DedicatedFans,
    pub away_dedicated_fans: DedicatedFans,
    pub home_inducements: Option<Vec<InducementPurchase>>,
    pub away_inducements: Option<Vec<InducementPurchase>>,
    pub star_engagements: Vec<(TeamId, InducementId)>,
    pub home_temp_players: Vec<TempPlayer>,
    pub away_temp_players: Vec<TempPlayer>,
    pub home_actions: Vec<MatchAction>,
    pub away_actions: Vec<MatchAction>,
    pub version: u64,
    pub home_gain: MatchGain,
    pub away_gain: MatchGain,
    pub home_fan_mod: FanFactorMod,
    pub away_fan_mod: FanFactorMod,
    pub summary_title: Option<String>,
    pub summary_body: Option<String>,
    pub published_by: CoachId,
    pub published_at: DateTime<Utc>,
}

impl MatchReportPublished {
    /// Ramène le rapport en état corrigeable.
    ///
    /// L'éligibilité est calculée hors du domaine — elle dépend de l'état
    /// d'autres BCs — mais la décision d'autoriser ou non appartient ici.
    pub fn unpublish(
        &self,
        unpublished_by: CoachId,
        eligibility: CorrectionEligibility,
    ) -> Result<(MatchReportReadyToPublish, MatchReportDomainEvent), DomainError> {
        if let CorrectionEligibility::Blocked(blocker) = eligibility {
            return Err(DomainError::CorrectionNotAllowed(blocker));
        }
        let event = MatchReportDomainEvent::MatchReportUnpublished {
            unpublished_by,
            unpublished_at: Utc::now(),
        };
        Ok((self.to_ready_to_publish(), event))
    }

    /// Reconstruit l'état corrigeable à partir du rapport publié. Symétrique de
    /// `from_ready_to_publish`, drapeau de correction en plus.
    fn to_ready_to_publish(&self) -> MatchReportReadyToPublish {
        MatchReportReadyToPublish {
            id: self.id.clone(),
            space_id: self.space_id.clone(),
            competition_id: self.competition_id.clone(),
            season_id: self.season_id.clone(),
            round_id: self.round_id.clone(),
            home_team_id: self.home_team_id.clone(),
            away_team_id: self.away_team_id.clone(),
            created_by: self.created_by,
            origin: self.origin,
            pairing_id: self.pairing_id.clone(),
            home_fan_roll: self.home_fan_roll,
            away_fan_roll: self.away_fan_roll,
            home_dedicated_fans: self.home_dedicated_fans,
            away_dedicated_fans: self.away_dedicated_fans,
            home_inducements: self.home_inducements.clone(),
            away_inducements: self.away_inducements.clone(),
            star_engagements: self.star_engagements.clone(),
            home_temp_players: self.home_temp_players.clone(),
            away_temp_players: self.away_temp_players.clone(),
            home_actions: self.home_actions.clone(),
            away_actions: self.away_actions.clone(),
            version: self.version + 1,
            home_gain: self.home_gain,
            away_gain: self.away_gain,
            home_fan_mod: self.home_fan_mod,
            away_fan_mod: self.away_fan_mod,
            summary_title: self.summary_title.clone(),
            summary_body: self.summary_body.clone(),
            was_published_before: true,
        }
    }

    pub fn from_ready_to_publish(
        rtp: &MatchReportReadyToPublish,
        published_by: CoachId,
        published_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: rtp.id.clone(),
            space_id: rtp.space_id.clone(),
            competition_id: rtp.competition_id.clone(),
            season_id: rtp.season_id.clone(),
            round_id: rtp.round_id.clone(),
            home_team_id: rtp.home_team_id.clone(),
            away_team_id: rtp.away_team_id.clone(),
            created_by: rtp.created_by,
            origin: rtp.origin,
            pairing_id: rtp.pairing_id.clone(),
            home_fan_roll: rtp.home_fan_roll,
            away_fan_roll: rtp.away_fan_roll,
            home_dedicated_fans: rtp.home_dedicated_fans,
            away_dedicated_fans: rtp.away_dedicated_fans,
            home_inducements: rtp.home_inducements.clone(),
            away_inducements: rtp.away_inducements.clone(),
            star_engagements: rtp.star_engagements.clone(),
            home_temp_players: rtp.home_temp_players.clone(),
            away_temp_players: rtp.away_temp_players.clone(),
            home_actions: rtp.home_actions.clone(),
            away_actions: rtp.away_actions.clone(),
            version: rtp.version + 1,
            home_gain: rtp.home_gain,
            away_gain: rtp.away_gain,
            home_fan_mod: rtp.home_fan_mod,
            away_fan_mod: rtp.away_fan_mod,
            summary_title: rtp.summary_title.clone(),
            summary_body: rtp.summary_body.clone(),
            published_by,
            published_at,
        }
    }
}

#[cfg(test)]
mod unpublish_tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::{CorrectionBlocker, TeamSide};
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RoundId, SeasonId};
    use crate::app::shared_kernel::identity::ids::SpaceId;

    fn published() -> MatchReportPublished {
        MatchReportPublished {
            id: MatchReportId::new(),
            space_id: SpaceId::new(),
            competition_id: CompetitionId::new(),
            season_id: SeasonId::new(),
            round_id: RoundId::new(),
            home_team_id: TeamId::new(),
            away_team_id: TeamId::new(),
            created_by: CoachId::new(),
            origin: MatchReportOrigin::Manual,
            pairing_id: None,
            home_fan_roll: None,
            away_fan_roll: None,
            home_dedicated_fans: DedicatedFans::default(),
            away_dedicated_fans: DedicatedFans::default(),
            home_inducements: None,
            away_inducements: None,
            star_engagements: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
            home_actions: vec![],
            away_actions: vec![],
            version: 7,
            home_gain: MatchGain::try_new(10_000).unwrap(),
            away_gain: MatchGain::try_new(5_000).unwrap(),
            home_fan_mod: FanFactorMod::try_new(1).unwrap(),
            away_fan_mod: FanFactorMod::try_new(-1).unwrap(),
            summary_title: Some("Titre".to_string()),
            summary_body: Some("Corps".to_string()),
            published_by: CoachId::new(),
            published_at: Utc::now(),
        }
    }

    #[test]
    fn depublication_refusee_si_des_spp_ont_ete_depenses() {
        let blocker = CorrectionBlocker::SppAlreadySpent {
            side: TeamSide::Away,
        };
        let err = published()
            .unpublish(CoachId::new(), CorrectionEligibility::Blocked(blocker))
            .unwrap_err();
        assert_eq!(err, DomainError::CorrectionNotAllowed(blocker));
    }

    #[test]
    fn depublication_refusee_si_une_equipe_a_avance_de_phase() {
        let blocker = CorrectionBlocker::PhaseAdvanced {
            side: TeamSide::Home,
        };
        let err = published()
            .unpublish(CoachId::new(), CorrectionEligibility::Blocked(blocker))
            .unwrap_err();
        assert_eq!(err, DomainError::CorrectionNotAllowed(blocker));
    }

    /// Échouer fermé : une éligibilité indéterminée bloque comme un vrai motif.
    #[test]
    fn depublication_refusee_si_l_eligibilite_est_indeterminee() {
        let err = published()
            .unpublish(
                CoachId::new(),
                CorrectionEligibility::Blocked(CorrectionBlocker::EligibilityUnknown),
            )
            .unwrap_err();
        assert_eq!(
            err,
            DomainError::CorrectionNotAllowed(CorrectionBlocker::EligibilityUnknown)
        );
    }

    #[test]
    fn depublication_produit_un_etat_corrigeable_portant_le_drapeau() {
        let p = published();
        let (rtp, _) = p
            .unpublish(CoachId::new(), CorrectionEligibility::Eligible)
            .unwrap();

        assert!(rtp.was_published_before);
        assert_eq!(rtp.id, p.id);
        assert_eq!(rtp.home_gain, p.home_gain);
        assert_eq!(rtp.summary_title, p.summary_title);
        assert_eq!(rtp.version, p.version + 1);
    }

    #[test]
    fn depublication_produit_l_evenement_avec_son_auteur() {
        let unpublished_by = CoachId::new();
        let (_, event) = published()
            .unpublish(unpublished_by, CorrectionEligibility::Eligible)
            .unwrap();

        match event {
            MatchReportDomainEvent::MatchReportUnpublished {
                unpublished_by: by, ..
            } => {
                assert_eq!(by, unpublished_by);
            }
            other => panic!("événement inattendu : {other:?}"),
        }
    }
}

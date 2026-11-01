use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::value_objects::{EntryMode, MatchReportOrigin};
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};

#[derive(Debug, Clone)]
pub struct MatchReportDraft {
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
    pub version: u64,
}

impl MatchReportDraft {
    pub fn create(
        id: MatchReportId,
        space_id: SpaceId,
        competition_id: CompetitionId,
        season_id: SeasonId,
        round_id: RoundId,
        home_team_id: TeamId,
        away_team_id: TeamId,
        created_by: CoachId,
        origin: MatchReportOrigin,
        pairing_id: Option<String>,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        if home_team_id == away_team_id {
            return Err(DomainError::SameTeam);
        }
        let event = MatchReportDomainEvent::MatchReportCreated {
            match_report_id: id,
            space_id,
            competition_id,
            season_id,
            round_id,
            home_team_id,
            away_team_id,
            created_by,
            origin,
            pairing_id,
        };
        let draft = Self::from_created_event(&event);
        Ok((draft, event))
    }

    /// Un match du calendrier **garde ses équipes** (règle R4 de l'épic E21,
    /// carte 633) : seul le déplacement admin le change, jamais la sélection.
    /// Un brouillon né hors calendrier — il n'en reste plus depuis la carte
    /// 552, qui le confirme à la création — garde, lui, sa sélection.
    pub fn keeps_its_teams(&self) -> bool {
        self.origin == MatchReportOrigin::Pairing
    }

    pub fn update_selection(
        &self,
        home_team_id: TeamId,
        away_team_id: TeamId,
        updated_by: CoachId,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        if home_team_id == away_team_id {
            return Err(DomainError::SameTeam);
        }
        let event = MatchReportDomainEvent::SelectionUpdated {
            home_team_id,
            away_team_id,
            updated_by,
        };
        let updated = self.apply_selection_updated(&event);
        Ok((updated, event))
    }

    /// **Aucun journalier à retirer** : ils naissent à l'initialisation des
    /// joueurs temporaires, qui n'a lieu qu'en phase d'avant-match. Un
    /// brouillon n'en a jamais fait exister.
    pub fn cancel(self, reason: String) -> MatchReportDomainEvent {
        MatchReportDomainEvent::MatchReportCancelled {
            reason,
            journeymen: vec![],
            home_team_id: Some(self.home_team_id),
            away_team_id: Some(self.away_team_id),
            pairing_id: self.pairing_id,
        }
    }

    /// Démarre le compte rendu : l'avant-match, et **deux** événements à
    /// appender ensemble — `SelectionConfirmed` puis `EntryModeChosen`
    /// (carte 631). Celui qui confirme est celui qui démarre.
    pub fn confirm_selection(
        self,
        confirmed_by: CoachId,
        mode: EntryMode,
    ) -> (MatchReportPreMatch, Vec<MatchReportDomainEvent>) {
        let pre_match = MatchReportPreMatch::from_draft(self, confirmed_by)
            .choose_entry_mode(mode, confirmed_by);
        let events = vec![
            MatchReportDomainEvent::SelectionConfirmed { confirmed_by },
            MatchReportDomainEvent::EntryModeChosen {
                mode,
                started_by: confirmed_by,
            },
        ];
        (pre_match, events)
    }

    pub(crate) fn from_created_event(event: &MatchReportDomainEvent) -> Self {
        match event {
            MatchReportDomainEvent::MatchReportCreated {
                match_report_id,
                space_id,
                competition_id,
                season_id,
                round_id,
                home_team_id,
                away_team_id,
                created_by,
                origin,
                pairing_id,
            } => Self {
                id: *match_report_id,
                space_id: *space_id,
                competition_id: *competition_id,
                season_id: *season_id,
                round_id: *round_id,
                home_team_id: *home_team_id,
                away_team_id: *away_team_id,
                created_by: *created_by,
                origin: *origin,
                pairing_id: pairing_id.clone(),
                version: 1,
            },
            _ => unreachable!("from_created_event appelé avec un mauvais event"),
        }
    }

    pub(crate) fn apply_selection_updated(&self, event: &MatchReportDomainEvent) -> Self {
        match event {
            MatchReportDomainEvent::SelectionUpdated {
                home_team_id,
                away_team_id,
                ..
            } => Self {
                home_team_id: *home_team_id,
                away_team_id: *away_team_id,
                version: self.version + 1,
                ..self.clone()
            },
            _ => unreachable!("apply_selection_updated appelé avec un mauvais event"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(origin: MatchReportOrigin) -> MatchReportDraft {
        MatchReportDraft::create(
            MatchReportId::new(),
            SpaceId::new(),
            CompetitionId::new(),
            SeasonId::new(),
            RoundId::new(),
            TeamId::new(),
            TeamId::new(),
            CoachId::new(),
            origin,
            None,
        )
        .expect("deux équipes distinctes")
        .0
    }

    /// R4 : un match du calendrier ne change plus d'équipes par la sélection.
    #[test]
    fn a_scheduled_draft_keeps_its_teams() {
        assert!(draft(MatchReportOrigin::Pairing).keeps_its_teams());
    }

    /// Un brouillon hors calendrier d'avant la carte 552 garde sa sélection.
    #[test]
    fn a_manual_draft_does_not_keep_its_teams() {
        assert!(!draft(MatchReportOrigin::Manual).keeps_its_teams());
    }
}

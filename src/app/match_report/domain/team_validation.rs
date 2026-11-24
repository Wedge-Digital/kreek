//! La validation d'un camp, en « Chacun son équipe » (carte 659-A, R9 et
//! § 12.4 à 12.6 de E22).
//!
//! Chaque coach valide son équipe dès que **son** camp a son MVP et son
//! après-match, sans attendre l'autre : la validation se rejoue en `PreMatch`
//! comme en `ReadyToPublish`. La seconde validation publie — elle a toujours
//! lieu en `ReadyToPublish`, puisque le second camp n'a son après-match qu'avec
//! `PostMatchClosed` — et `TeamSideValidated` part avec `MatchReportPublished`
//! dans le même `append_many`, `published_by` étant le second validateur.
//!
//! **Un camp validé est figé** (§ 12.5) : toute écriture dans ce camp est
//! refusée par `SideValidated`, que la couche web rend en 409. L'état des
//! validations traverse `into_pre_match` / `from_pre_match`, par où passent les
//! corrections d'un rapport prêt à publier : sans quoi le gel se contournerait.
//!
//! **Le retrait** (carte 659-B, § 12.5) : un camp validé retire sa validation
//! tant que le rapport n'est pas publié, et redevient modifiable. Il n'y a
//! rien à retirer d'un rapport publié : la seconde validation publie aussitôt.

use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_published::MatchReportPublished;
use crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish;
use crate::app::match_report::domain::value_objects::{
    EntryMode, MatchAction, MatchActionType, SideAuthors, TeamSide,
};
use crate::app::shared_kernel::identity::ids::CoachId;

/// L'issue d'une validation dans un rapport prêt à publier.
#[derive(Debug)]
pub enum SideValidationOutcome {
    /// L'autre camp n'a pas encore validé.
    Validated(MatchReportReadyToPublish),
    /// La seconde validation : le rapport est publié.
    Published(MatchReportPublished),
}

/// Le gel d'un camp validé (§ 12.5) : toute écriture y est refusée.
pub(crate) fn ensure_not_validated(
    validated_by: &SideAuthors,
    side: TeamSide,
) -> Result<(), DomainError> {
    match validated_by.of(side) {
        Some(_) => Err(DomainError::SideValidated(side)),
        None => Ok(()),
    }
}

/// Le retrait d'une validation : le mode B, un camp validé (carte 659-B).
fn check_withdrawable(
    entry_mode: EntryMode,
    validated_by: &SideAuthors,
    side: TeamSide,
) -> Result<(), DomainError> {
    if entry_mode != EntryMode::TeamEntry {
        return Err(DomainError::NotTeamEntry);
    }
    match validated_by.of(side) {
        Some(_) => Ok(()),
        None => Err(DomainError::SideNotValidated(side)),
    }
}

fn withdrawn_event(side: TeamSide, withdrawn_by: CoachId) -> MatchReportDomainEvent {
    MatchReportDomainEvent::TeamSideValidationWithdrawn {
        team_side: side,
        withdrawn_by,
    }
}

/// Ce que la validation d'un camp regarde — **de ce camp seul**, jamais de
/// l'autre (§ 12.4).
struct SideReadiness {
    entry_mode: EntryMode,
    validated_by: SideAuthors,
    has_mvp: bool,
    has_post_match: bool,
}

impl SideReadiness {
    /// Le mode B, un camp pas encore validé, son MVP, son après-match.
    fn check(&self, side: TeamSide) -> Result<(), DomainError> {
        if self.entry_mode != EntryMode::TeamEntry {
            return Err(DomainError::NotTeamEntry);
        }
        ensure_not_validated(&self.validated_by, side)?;
        if !self.has_mvp {
            return Err(DomainError::MvpMissing(side));
        }
        if !self.has_post_match {
            return Err(DomainError::PostMatchMissing(side));
        }
        Ok(())
    }
}

fn has_mvp(actions: &[MatchAction]) -> bool {
    actions
        .iter()
        .any(|a| matches!(a.action, MatchActionType::Mvp))
}

fn validated_event(side: TeamSide, validated_by: CoachId) -> MatchReportDomainEvent {
    MatchReportDomainEvent::TeamSideValidated {
        team_side: side,
        validated_by,
    }
}

impl MatchReportPreMatch {
    /// La validation d'un camp qui attend l'après-match de l'autre.
    ///
    /// La seconde validation n'arrive jamais ici : l'autre camp n'aurait pu
    /// valider qu'avec son après-match, et les deux après-matchs font passer
    /// en `ReadyToPublish`. Si elle y arrivait quand même, elle ne publierait
    /// pas : la publication part d'un rapport prêt à publier, et de lui seul.
    pub fn validate_side(
        &self,
        side: TeamSide,
        validated_by: CoachId,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        self.readiness(side).check(side)?;
        let validated = self.clone().apply_side_validated(side, validated_by);
        Ok((validated, validated_event(side, validated_by)))
    }

    /// Le gel : refuse toute écriture dans un camp validé (§ 12.5).
    pub fn check_side_open(&self, side: TeamSide) -> Result<(), DomainError> {
        ensure_not_validated(&self.validated_by, side)
    }

    fn readiness(&self, side: TeamSide) -> SideReadiness {
        SideReadiness {
            entry_mode: self.entry_mode,
            validated_by: self.validated_by,
            has_mvp: has_mvp(self.actions_for(side)),
            has_post_match: self.post_match_of(side).is_some(),
        }
    }

    /// Le rejeu d'un `TeamSideValidated` dans l'avant-match.
    pub(crate) fn apply_side_validated(mut self, side: TeamSide, validated_by: CoachId) -> Self {
        self.validated_by = self.validated_by.with(side, validated_by);
        self.version += 1;
        self
    }

    /// Le retrait de la validation d'un camp qui attend l'autre (carte 659-B) :
    /// le camp redevient modifiable.
    pub fn withdraw_side_validation(
        &self,
        side: TeamSide,
        withdrawn_by: CoachId,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        check_withdrawable(self.entry_mode, &self.validated_by, side)?;
        let withdrawn = self.clone().apply_side_validation_withdrawn(side);
        Ok((withdrawn, withdrawn_event(side, withdrawn_by)))
    }

    /// Le rejeu d'un `TeamSideValidationWithdrawn` dans l'avant-match.
    pub(crate) fn apply_side_validation_withdrawn(mut self, side: TeamSide) -> Self {
        self.validated_by = self.validated_by.without(side);
        self.version += 1;
        self
    }
}

impl MatchReportReadyToPublish {
    /// La validation d'un camp. Si l'autre a déjà validé, elle publie :
    /// `TeamSideValidated` puis `MatchReportPublished`, à appender ensemble,
    /// `published_by` étant ce second validateur (R9).
    pub fn validate_side(
        &self,
        side: TeamSide,
        validated_by: CoachId,
    ) -> Result<(SideValidationOutcome, Vec<MatchReportDomainEvent>), DomainError> {
        self.readiness(side).check(side)?;
        let validated = self.clone().apply_side_validated(side, validated_by);
        let event = validated_event(side, validated_by);
        if validated.validated_by.of(side.opposite()).is_none() {
            return Ok((SideValidationOutcome::Validated(validated), vec![event]));
        }
        let (published, published_event) = validated.publish(validated_by);
        let outcome = SideValidationOutcome::Published(published);
        Ok((outcome, vec![event, published_event]))
    }

    /// Le gel : refuse toute écriture dans un camp validé (§ 12.5).
    pub fn check_side_open(&self, side: TeamSide) -> Result<(), DomainError> {
        ensure_not_validated(&self.validated_by, side)
    }

    fn readiness(&self, side: TeamSide) -> SideReadiness {
        let actions = match side {
            TeamSide::Home => &self.home_actions,
            TeamSide::Away => &self.away_actions,
        };
        SideReadiness {
            entry_mode: self.entry_mode,
            validated_by: self.validated_by,
            has_mvp: has_mvp(actions),
            has_post_match: self.post_match_of(side).is_some(),
        }
    }

    /// Le rejeu d'un `TeamSideValidated` dans un rapport prêt à publier.
    pub(crate) fn apply_side_validated(mut self, side: TeamSide, validated_by: CoachId) -> Self {
        self.validated_by = self.validated_by.with(side, validated_by);
        self.version += 1;
        self
    }

    /// Le retrait de la validation d'un camp, l'autre n'ayant pas validé
    /// (carte 659-B) : le camp redevient modifiable.
    pub fn withdraw_side_validation(
        &self,
        side: TeamSide,
        withdrawn_by: CoachId,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        check_withdrawable(self.entry_mode, &self.validated_by, side)?;
        let withdrawn = self.clone().apply_side_validation_withdrawn(side);
        Ok((withdrawn, withdrawn_event(side, withdrawn_by)))
    }

    /// Le rejeu d'un `TeamSideValidationWithdrawn` dans un rapport prêt à
    /// publier.
    pub(crate) fn apply_side_validation_withdrawn(mut self, side: TeamSide) -> Self {
        self.validated_by = self.validated_by.without(side);
        self.version += 1;
        self
    }
}

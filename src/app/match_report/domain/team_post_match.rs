//! L'après-match d'un camp, en « Chacun son équipe » (carte 658, R7 et R8 de
//! E22).
//!
//! Chaque coach enregistre les gains, le bonus, la modification de fans et le
//! paragraphe de son équipe ; le domicile y ajoute le titre. Le premier camp
//! attend l'autre dans l'avant-match ; le second ferme l'après-match
//! (`PostMatchClosed`) et le rapport passe en `ReadyToPublish`. En correction,
//! un camp remplace ses seules valeurs.
//!
//! Les valeurs d'un camp remplissent, une fois l'après-match fermé, **les
//! champs du mode A** (`home_gain`, `home_fan_mod`…) : le payload publié est
//! le même dans les deux modes.

use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish;
use crate::app::match_report::domain::value_objects::{
    EntryMode, FanFactorMod, MatchAction, MatchActionType, MatchGain, NoStallingBonus,
    NoStallingBonuses, TeamSide,
};
use crate::app::shared_kernel::identity::ids::CoachId;

/// Ce qu'un camp a enregistré de son après-match.
#[derive(Debug, Clone, PartialEq)]
pub struct SidePostMatch {
    pub gain: MatchGain,
    pub no_stalling_bonus: NoStallingBonus,
    pub fan_mod: FanFactorMod,
    /// Texte libre, sans validation domaine (exception de `CLAUDE.md`).
    pub paragraph: Option<String>,
    pub recorded_by: CoachId,
}

/// La commande d'un camp : son après-match, et le titre s'il est le domicile.
#[derive(Debug, Clone, PartialEq)]
pub struct TeamPostMatch {
    pub side: TeamSide,
    pub post_match: SidePostMatch,
    pub title: Option<String>,
}

impl TeamPostMatch {
    pub fn to_event(&self) -> MatchReportDomainEvent {
        MatchReportDomainEvent::TeamPostMatchRecorded {
            team_side: self.side,
            gain: self.post_match.gain,
            no_stalling_bonus: self.post_match.no_stalling_bonus,
            fan_mod: self.post_match.fan_mod,
            summary_title: self.title.clone(),
            summary_paragraph: self.post_match.paragraph.clone(),
            recorded_by: self.post_match.recorded_by,
        }
    }

    /// Le domaine refuse avant d'écrire : mode, titre, MVP du camp (R7).
    fn check(&self, entry_mode: EntryMode, actions: &[MatchAction]) -> Result<(), DomainError> {
        if entry_mode != EntryMode::TeamEntry {
            return Err(DomainError::NotTeamEntry);
        }
        if self.side == TeamSide::Away && self.title.is_some() {
            return Err(DomainError::TitleIsHomeOnly);
        }
        if !actions
            .iter()
            .any(|a| matches!(a.action, MatchActionType::Mvp))
        {
            return Err(DomainError::MvpMissing(self.side));
        }
        Ok(())
    }
}

/// Les après-matchs des deux camps tant que l'avant-match les attend, et le
/// titre du domicile. Vide en « Je saisis tout ».
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TeamPostMatches {
    pub home: Option<SidePostMatch>,
    pub away: Option<SidePostMatch>,
    pub title: Option<String>,
}

impl TeamPostMatches {
    pub fn of(&self, side: TeamSide) -> Option<&SidePostMatch> {
        match side {
            TeamSide::Home => self.home.as_ref(),
            TeamSide::Away => self.away.as_ref(),
        }
    }

    /// Le registre, l'après-match de ce camp posé — ou remplacé.
    fn with(mut self, entry: &TeamPostMatch) -> Self {
        let recorded = Some(entry.post_match.clone());
        match entry.side {
            TeamSide::Home => {
                self.home = recorded;
                self.title = entry.title.clone();
            }
            TeamSide::Away => self.away = recorded,
        }
        self
    }
}

/// Le paragraphe de résumé de chaque camp (carte 658). Vide en « Je saisis
/// tout », dont le résumé est `summary_body`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SummaryParagraphs {
    pub home: Option<String>,
    pub away: Option<String>,
}

impl SummaryParagraphs {
    pub fn of(&self, side: TeamSide) -> Option<&String> {
        match side {
            TeamSide::Home => self.home.as_ref(),
            TeamSide::Away => self.away.as_ref(),
        }
    }

    fn with(mut self, side: TeamSide, paragraph: Option<String>) -> Self {
        match side {
            TeamSide::Home => self.home = paragraph,
            TeamSide::Away => self.away = paragraph,
        }
        self
    }
}

/// L'issue de l'après-match d'un camp dans l'avant-match.
#[derive(Debug)]
pub enum TeamPostMatchOutcome {
    /// L'autre camp n'a pas encore le sien.
    Pending(MatchReportPreMatch),
    /// Le second camp : le rapport est prêt à publier.
    Closed(MatchReportReadyToPublish),
}

impl MatchReportPreMatch {
    /// L'après-match d'un camp. Au second camp, rend aussi `PostMatchClosed` :
    /// les deux événements partent dans le même append (§ 12.8 de E22).
    pub fn record_team_post_match(
        &self,
        entry: TeamPostMatch,
    ) -> Result<(TeamPostMatchOutcome, Vec<MatchReportDomainEvent>), DomainError> {
        self.check_side_open(entry.side)?;
        entry.check(self.entry_mode, self.actions_for(entry.side))?;
        let event = entry.to_event();
        let pending = self.clone().apply_team_post_match(&entry);
        match pending.close_post_match() {
            Some(ready) => Ok((
                TeamPostMatchOutcome::Closed(ready),
                vec![event, MatchReportDomainEvent::PostMatchClosed],
            )),
            None => Ok((TeamPostMatchOutcome::Pending(pending), vec![event])),
        }
    }

    pub fn post_match_of(&self, side: TeamSide) -> Option<SidePostMatch> {
        self.team_post_match.of(side).cloned()
    }

    /// Le rejeu d'un `TeamPostMatchRecorded` dans l'avant-match.
    pub(crate) fn apply_team_post_match(mut self, entry: &TeamPostMatch) -> Self {
        self.team_post_match = self.team_post_match.with(entry);
        self.version += 1;
        self
    }

    /// Le passage en `ReadyToPublish`, quand les deux camps ont le leur.
    pub(crate) fn close_post_match(&self) -> Option<MatchReportReadyToPublish> {
        let (home, away) = (
            self.team_post_match.home.as_ref()?,
            self.team_post_match.away.as_ref()?,
        );
        let bonuses = NoStallingBonuses {
            home: home.no_stalling_bonus,
            away: away.no_stalling_bonus,
        };
        let title = self.team_post_match.title.clone();
        let ready = MatchReportReadyToPublish::from_pre_match(
            self,
            home.gain,
            away.gain,
            bonuses,
            home.fan_mod,
            away.fan_mod,
            title,
            None,
        );
        Some(ready.with_side_authorship(home, away))
    }
}

impl MatchReportReadyToPublish {
    /// En correction, un camp remplace ses seules valeurs (carte 658).
    pub fn record_team_post_match(
        &self,
        entry: TeamPostMatch,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        self.check_side_open(entry.side)?;
        entry.check(self.entry_mode, self.actions_of(entry.side))?;
        Ok((self.clone().apply_team_post_match(&entry), entry.to_event()))
    }

    /// L'après-match d'un camp, connu si le camp l'a enregistré lui-même.
    pub fn post_match_of(&self, side: TeamSide) -> Option<SidePostMatch> {
        let recorded_by = self.post_match_recorded_by.of(side)?;
        let (gain, no_stalling_bonus, fan_mod) = match side {
            TeamSide::Home => (
                self.home_gain,
                self.home_no_stalling_bonus,
                self.home_fan_mod,
            ),
            TeamSide::Away => (
                self.away_gain,
                self.away_no_stalling_bonus,
                self.away_fan_mod,
            ),
        };
        Some(SidePostMatch {
            gain,
            no_stalling_bonus,
            fan_mod,
            paragraph: self.summary_paragraphs.of(side).cloned(),
            recorded_by,
        })
    }

    /// Les après-matchs des deux camps, pour l'avant-match qu'on en tire.
    pub(crate) fn team_post_matches(&self) -> TeamPostMatches {
        TeamPostMatches {
            home: self.post_match_of(TeamSide::Home),
            away: self.post_match_of(TeamSide::Away),
            title: self.summary_title.clone(),
        }
    }

    /// Le rejeu d'un `TeamPostMatchRecorded` en correction.
    pub(crate) fn apply_team_post_match(mut self, entry: &TeamPostMatch) -> Self {
        let p = &entry.post_match;
        match entry.side {
            TeamSide::Home => {
                (self.home_gain, self.home_no_stalling_bonus) = (p.gain, p.no_stalling_bonus);
                self.home_fan_mod = p.fan_mod;
                self.summary_title = entry.title.clone();
            }
            TeamSide::Away => {
                (self.away_gain, self.away_no_stalling_bonus) = (p.gain, p.no_stalling_bonus);
                self.away_fan_mod = p.fan_mod;
            }
        }
        self.version += 1;
        self.with_side(entry.side, p)
    }

    fn with_side_authorship(self, home: &SidePostMatch, away: &SidePostMatch) -> Self {
        self.with_side(TeamSide::Home, home)
            .with_side(TeamSide::Away, away)
    }

    /// Le paragraphe et l'auteur d'un camp.
    fn with_side(mut self, side: TeamSide, p: &SidePostMatch) -> Self {
        self.summary_paragraphs = self.summary_paragraphs.with(side, p.paragraph.clone());
        self.post_match_recorded_by = self.post_match_recorded_by.with(side, p.recorded_by);
        self
    }

    fn actions_of(&self, side: TeamSide) -> &[MatchAction] {
        match side {
            TeamSide::Home => &self.home_actions,
            TeamSide::Away => &self.away_actions,
        }
    }
}

/// La commande que rejoue un `TeamPostMatchRecorded`.
pub(crate) fn team_post_match_of(event: &MatchReportDomainEvent) -> Option<TeamPostMatch> {
    let MatchReportDomainEvent::TeamPostMatchRecorded {
        team_side,
        gain,
        no_stalling_bonus,
        fan_mod,
        summary_title,
        summary_paragraph,
        recorded_by,
    } = event
    else {
        return None;
    };
    Some(TeamPostMatch {
        side: *team_side,
        post_match: SidePostMatch {
            gain: *gain,
            no_stalling_bonus: *no_stalling_bonus,
            fan_mod: *fan_mod,
            paragraph: summary_paragraph.clone(),
            recorded_by: *recorded_by,
        },
        title: summary_title.clone(),
    })
}

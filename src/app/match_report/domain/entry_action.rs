//! L'action de saisie d'un match pour un coach (carte 676, I7-E1-KR-T2).
//!
//! « Commencer », « Continuer » ou « Suivre » : la page « Mes matchs » montre
//! un bouton, et c'est `match_report` seul qui décide lequel. La règle
//! d'écriture n'est **pas** ici : elle reste `can_write_report`, dont le
//! verdict arrive sous la forme d'un `CoachStanding`.

/// Le bouton proposé à un coach pour un match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryAction {
    /// Le brouillon n'est pas commencé : le coach peut le démarrer.
    Start,
    /// Le rapport est commencé, et le coach y écrit.
    Continue,
    /// Commencé en « Je saisis tout » par l'autre : le coach suit en direct.
    Follow,
}

/// Où en est la saisie d'un rapport, d'après sa projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryProgress {
    /// Brouillon que personne n'a démarré.
    Unstarted,
    /// Démarré, pas encore publié.
    Started,
    /// Publié ou annulé : plus rien à saisir.
    Closed,
}

impl EntryProgress {
    /// Relit la phase écrite en projection et le démarreur. Une phase que ce
    /// code ne connaît pas vaut `Closed` : on ne propose pas de saisir ce
    /// qu'on ne sait pas lire.
    pub fn from_projection(phase: &str, started_by: Option<&str>) -> Self {
        match (phase, started_by) {
            ("Draft", None) => Self::Unstarted,
            ("Draft" | "PreMatch" | "ReadyToPublish", _) => Self::Started,
            _ => Self::Closed,
        }
    }
}

/// La place d'un coach dans un match, vue depuis la règle d'écriture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoachStanding {
    /// N'entraîne aucune des deux équipes — admin compris (I7-B1).
    Outsider,
    /// Coach du match, et `can_write_report` l'autorise.
    Writer,
    /// Coach du match, que `can_write_report` refuse : il suit.
    Reader,
}

impl EntryAction {
    /// L'action pour ce coach, `None` s'il n'en a aucune.
    pub fn decide(progress: EntryProgress, standing: CoachStanding) -> Option<Self> {
        match (progress, standing) {
            (EntryProgress::Closed, _) | (_, CoachStanding::Outsider) => None,
            (EntryProgress::Unstarted, _) => Some(Self::Start),
            (EntryProgress::Started, CoachStanding::Writer) => Some(Self::Continue),
            (EntryProgress::Started, CoachStanding::Reader) => Some(Self::Follow),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unstarted_draft_reads_as_unstarted() {
        assert_eq!(
            EntryProgress::from_projection("Draft", None),
            EntryProgress::Unstarted
        );
    }

    #[test]
    fn a_report_in_progress_reads_as_started() {
        for phase in ["PreMatch", "ReadyToPublish"] {
            assert_eq!(
                EntryProgress::from_projection(phase, Some("coach")),
                EntryProgress::Started
            );
        }
    }

    /// Un brouillon qui porte un démarreur n'est plus « non commencé ».
    #[test]
    fn a_draft_with_a_starter_reads_as_started() {
        assert_eq!(
            EntryProgress::from_projection("Draft", Some("coach")),
            EntryProgress::Started
        );
    }

    #[test]
    fn published_cancelled_and_unknown_phases_read_as_closed() {
        for phase in ["Published", "Cancelled", "Unknown"] {
            assert_eq!(
                EntryProgress::from_projection(phase, Some("coach")),
                EntryProgress::Closed
            );
        }
    }

    #[test]
    fn a_coach_starts_an_unstarted_draft() {
        let action = EntryAction::decide(EntryProgress::Unstarted, CoachStanding::Writer);
        assert_eq!(action, Some(EntryAction::Start));
    }

    #[test]
    fn a_writer_continues_a_started_report() {
        let action = EntryAction::decide(EntryProgress::Started, CoachStanding::Writer);
        assert_eq!(action, Some(EntryAction::Continue));
    }

    #[test]
    fn a_reader_follows_a_started_report() {
        let action = EntryAction::decide(EntryProgress::Started, CoachStanding::Reader);
        assert_eq!(action, Some(EntryAction::Follow));
    }

    #[test]
    fn a_closed_report_offers_nothing() {
        for standing in [CoachStanding::Writer, CoachStanding::Reader] {
            assert_eq!(EntryAction::decide(EntryProgress::Closed, standing), None);
        }
    }

    #[test]
    fn an_outsider_gets_nothing() {
        for progress in [EntryProgress::Unstarted, EntryProgress::Started] {
            assert_eq!(EntryAction::decide(progress, CoachStanding::Outsider), None);
        }
    }
}

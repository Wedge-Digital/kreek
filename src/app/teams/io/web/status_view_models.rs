//! Le statut d'une équipe, tel qu'un écran l'affiche : un libellé et la
//! variante de badge qui le colore.
//!
//! **Un seul endroit pour toute la couche web de `teams`** (carte 566). La
//! fonction vivait dans le contrôleur de la fiche ; le widget d'identité en a
//! besoin aussi, et une copie aurait laissé deux écrans nommer différemment le
//! même statut. Elle a été déplacée telle quelle.
//!
//! `my_teams_widget.rs` garde sa propre traduction, depuis les chaînes de
//! `team_proj` et non l'agrégat — duplication assumée, documentée sur place.

use crate::app::teams::domain::team::{GamePhase, ParticipationStatus, Team};

/// `(libellé, variante)` — la variante suffixe `team-status-badge--`.
pub(crate) fn status_display(team: &Team) -> (String, String) {
    match &team.participation_status {
        ParticipationStatus::Dismissed => ("Renvoyée".into(), "dismissed".into()),
        ParticipationStatus::Rejected => ("Inscription refusée".into(), "dismissed".into()),
        ParticipationStatus::PendingEnrollment => {
            ("En attente d'inscription".into(), "pending".into())
        }
        ParticipationStatus::Enrolled => match &team.game_phase {
            Some(GamePhase::ReadyToPlay) => ("Prête à jouer".into(), "ready".into()),
            Some(GamePhase::MatchReporting) => ("Rapport en cours".into(), "phase".into()),
            Some(GamePhase::PlayerImprovement) => ("Phase d'amélioration".into(), "phase".into()),
            Some(GamePhase::Recruitment) => ("Phase de recrutement".into(), "phase".into()),
            Some(GamePhase::Dismissals) => ("Phase de renvois".into(), "phase".into()),
            Some(GamePhase::CostlyMistakes) => ("Erreurs coûteuses".into(), "phase".into()),
            Some(GamePhase::TemporaryRetirement) => ("Retraite temporaire".into(), "phase".into()),
            Some(GamePhase::OffSeason) => ("Off-season".into(), "offseason".into()),
            None => ("Inscrite".into(), "ready".into()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::teams::io::web::tests::fixtures::equipe_de_test;

    fn statut(participation: ParticipationStatus, phase: Option<GamePhase>) -> (String, String) {
        let mut team = equipe_de_test();
        team.participation_status = participation;
        team.game_phase = phase;
        status_display(&team)
    }

    fn attendu(label: &str, class: &str) -> (String, String) {
        (label.into(), class.into())
    }

    #[test]
    fn les_statuts_hors_jeu() {
        use ParticipationStatus::*;
        assert_eq!(statut(Dismissed, None), attendu("Renvoyée", "dismissed"));
        assert_eq!(
            statut(Rejected, None),
            attendu("Inscription refusée", "dismissed")
        );
        assert_eq!(
            statut(PendingEnrollment, None),
            attendu("En attente d'inscription", "pending")
        );
    }

    #[test]
    fn chaque_phase_de_jeu_d_une_equipe_inscrite() {
        use GamePhase::*;
        let cas = [
            (Some(ReadyToPlay), "Prête à jouer", "ready"),
            (Some(MatchReporting), "Rapport en cours", "phase"),
            (Some(PlayerImprovement), "Phase d'amélioration", "phase"),
            (Some(Recruitment), "Phase de recrutement", "phase"),
            (Some(Dismissals), "Phase de renvois", "phase"),
            (Some(CostlyMistakes), "Erreurs coûteuses", "phase"),
            (Some(TemporaryRetirement), "Retraite temporaire", "phase"),
            (Some(OffSeason), "Off-season", "offseason"),
            (None, "Inscrite", "ready"),
        ];
        for (phase, label, class) in cas {
            assert_eq!(
                statut(ParticipationStatus::Enrolled, phase.clone()),
                attendu(label, class),
                "{phase:?}"
            );
        }
    }
}

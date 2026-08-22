//! Les quatre réglages de notification d'une saison, et ce qui les rend
//! applicables.
//!
//! # Coché et applicable sont deux choses
//!
//! `applicability()` **ne lit pas** `CompetitionNotifications`, et c'est la
//! seule chose à retenir de ce module. R6 veut qu'une notification cochée puis
//! rendue inapplicable — parce qu'on a retiré le calendrier, par exemple —
//! **reste cochée**. Mêler les deux dans une fonction unique rendrait
//! mécaniquement impossible d'afficher une case cochée et grisée.
//!
//! # Pas d'agrégat
//!
//! Les quatre booléens sont indépendants, aucune combinaison n'est interdite,
//! et R6 interdit de filtrer à l'écriture. Leur donner des méthodes de commande
//! serait inventer des invariants qui n'existent pas — c'est la même nature que
//! `CompetitionInvitations` et `CompetitionStructure`, qui n'en ont pas non
//! plus.

use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::competition_structure::{
    CompetitionStructure, ScheduledDate,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotifyRegistrationOpen(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotifyRoundEve(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotifyRoundClosing(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotifyRegistrationDeadline(pub bool);

fn allume_registration_open() -> NotifyRegistrationOpen {
    NotifyRegistrationOpen(true)
}
fn allume_round_eve() -> NotifyRoundEve {
    NotifyRoundEve(true)
}
fn allume_round_closing() -> NotifyRoundClosing {
    NotifyRoundClosing(true)
}
fn allume_registration_deadline() -> NotifyRegistrationDeadline {
    NotifyRegistrationDeadline(true)
}

/// Un champ absent vaut **allumé** : c'est le défaut d'une saison neuve (R8),
/// dont la colonne `notifications` est `NULL` à l'insertion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompetitionNotifications {
    #[serde(default = "allume_registration_open")]
    pub registration_open: NotifyRegistrationOpen,
    #[serde(default = "allume_round_eve")]
    pub round_eve: NotifyRoundEve,
    #[serde(default = "allume_round_closing")]
    pub round_closing: NotifyRoundClosing,
    #[serde(default = "allume_registration_deadline")]
    pub registration_deadline: NotifyRegistrationDeadline,
}

impl Default for CompetitionNotifications {
    fn default() -> Self {
        Self {
            registration_open: allume_registration_open(),
            round_eve: allume_round_eve(),
            round_closing: allume_round_closing(),
            registration_deadline: allume_registration_deadline(),
        }
    }
}

/// Pourquoi une notification ne peut pas se déclencher sur cette saison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inapplicable {
    /// Pas de calendrier — l'interrupteur est éteint, **ou** aucune journée n'a
    /// été saisie. Le motif affiché est le même dans les deux cas : un
    /// calendrier à zéro journée n'est pas un calendrier, et distinguer les
    /// deux ne donnerait à l'organisateur aucune action différente.
    NoSchedule,
    /// Aucune journée n'a de fenêtre à clore — que des dates fixes.
    NoTimeFrameRound,
    /// Aucune date limite d'inscription n'est fixée.
    NoDeadline,
}

/// `None` = applicable. `registration_open` n'y figure pas : elle l'est
/// toujours, une compétition ayant par construction une ouverture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationApplicability {
    pub round_eve: Option<Inapplicable>,
    pub round_closing: Option<Inapplicable>,
    pub registration_deadline: Option<Inapplicable>,
}

/// Fonction **pure et totale** : toute structure lui donne une réponse, aucune
/// entrée ne la met en échec. D'où l'absence de `Result` et de variante ajoutée
/// à `DomainError`.
pub fn applicability(
    structure: &CompetitionStructure,
    invitations: Option<&CompetitionInvitations>,
) -> NotificationApplicability {
    NotificationApplicability {
        round_eve: veille_de_journee(structure),
        round_closing: fin_de_journee(structure),
        registration_deadline: date_limite(invitations),
    }
}

fn veille_de_journee(structure: &CompetitionStructure) -> Option<Inapplicable> {
    (!a_un_calendrier(structure)).then_some(Inapplicable::NoSchedule)
}

/// Une journée `FixedDate` ne porte qu'une date de multiplexe : elle n'a pas de
/// fenêtre à clore. Une compétition dont toutes les journées sont à date fixe
/// peut donc prévenir de la veille, jamais de la clôture.
fn fin_de_journee(structure: &CompetitionStructure) -> Option<Inapplicable> {
    if !a_un_calendrier(structure) {
        return Some(Inapplicable::NoSchedule);
    }
    let a_une_fenetre = structure
        .schedule
        .scheduled_dates
        .iter()
        .any(|d| matches!(d, ScheduledDate::TimeFrame { .. }));
    (!a_une_fenetre).then_some(Inapplicable::NoTimeFrameRound)
}

/// La date vide est traitée comme absente. Le champ HTML rend `""` quand on
/// l'efface, et le JS de l'étape 4 le convertit aujourd'hui en `null` — mais une
/// règle métier qui repose sur un `|| null` dans un template est une règle qui
/// tombera au premier remaniement de ce template.
fn date_limite(invitations: Option<&CompetitionInvitations>) -> Option<Inapplicable> {
    let renseignee = invitations
        .and_then(|i| i.registration_deadline.as_deref())
        .is_some_and(|d| !d.trim().is_empty());
    (!renseignee).then_some(Inapplicable::NoDeadline)
}

/// Un calendrier activé mais sans aucune journée est l'état normal d'une saison
/// arrivée à l'étape 4 sans avoir rempli l'étape 3 : traité comme « pas de
/// calendrier ».
fn a_un_calendrier(structure: &CompetitionStructure) -> bool {
    structure.schedule.use_schedule.0 && !structure.schedule.scheduled_dates.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les structures de test sont montées depuis du JSON plutôt que champ par
    /// champ : `CompetitionStructure` en imbrique une dizaine dont aucune ne
    /// concerne l'applicabilité, et c'est de toute façon par cette porte que la
    /// donnée arrive — la colonne est un JSONB.
    fn structure(use_schedule: bool, journees: &str) -> CompetitionStructure {
        let json = format!(
            r#"{{"ranking_group":{{"use_ranking_groups":false,"ranking_groups":[]}},
                 "play_offs_phase":{{"use_playoffs_phase":false,
                                     "qualified_team_per_pool":0,
                                     "final_phase_match_for_third_place":false}},
                 "schedule":{{"use_schedule":{use_schedule},
                              "use_mail_notification":false,
                              "scheduled_dates":[{journees}]}}}}"#
        );
        serde_json::from_str(&json).expect("structure de test invalide")
    }

    const FIXE: &str = r#"{"type":"fixed_date","name":"J1","multiplexe_date":"2026-09-01"}"#;
    const FENETRE: &str = r#"{"type":"time_frame","name":"J2",
                              "start_date":"2026-09-01","end_date":"2026-09-07"}"#;

    fn invitations(deadline: Option<&str>) -> CompetitionInvitations {
        let d = deadline.map_or("null".to_string(), |v| format!("\"{v}\""));
        serde_json::from_str(&format!(
            r#"{{"max_participants":null,"registration_deadline":{d}}}"#
        ))
        .expect("invitations de test invalides")
    }

    // ── Applicabilité : les sept cas de la phase 6 ───────────────────────────

    #[test]
    fn calendrier_eteint_rend_les_deux_notifications_de_journee_inapplicables() {
        let a = applicability(&structure(false, FENETRE), None);

        assert_eq!(a.round_eve, Some(Inapplicable::NoSchedule));
        assert_eq!(a.round_closing, Some(Inapplicable::NoSchedule));
    }

    #[test]
    fn calendrier_allume_sans_aucune_journee_vaut_pas_de_calendrier() {
        let a = applicability(&structure(true, ""), None);

        assert_eq!(a.round_eve, Some(Inapplicable::NoSchedule));
        assert_eq!(a.round_closing, Some(Inapplicable::NoSchedule));
    }

    #[test]
    fn des_journees_a_date_fixe_permettent_la_veille_mais_pas_la_cloture() {
        let a = applicability(&structure(true, FIXE), None);

        assert_eq!(a.round_eve, None);
        assert_eq!(a.round_closing, Some(Inapplicable::NoTimeFrameRound));
    }

    #[test]
    fn une_seule_journee_a_fenetre_suffit_a_rendre_la_cloture_applicable() {
        let deux = format!("{FIXE},{FENETRE}");
        let a = applicability(&structure(true, &deux), None);

        assert_eq!(a.round_eve, None);
        assert_eq!(a.round_closing, None);
    }

    #[test]
    fn sans_invitations_la_date_limite_est_inapplicable() {
        let a = applicability(&structure(true, FENETRE), None);

        assert_eq!(a.registration_deadline, Some(Inapplicable::NoDeadline));
    }

    #[test]
    fn une_date_limite_vide_vaut_une_date_limite_absente() {
        let vides = invitations(Some(""));
        let a = applicability(&structure(true, FENETRE), Some(&vides));

        assert_eq!(a.registration_deadline, Some(Inapplicable::NoDeadline));
    }

    #[test]
    fn une_date_limite_renseignee_rend_la_notification_applicable() {
        let posee = invitations(Some("2026-09-15"));
        let a = applicability(&structure(true, FENETRE), Some(&posee));

        assert_eq!(a.registration_deadline, None);
    }

    // ── Sérialisation : R8, le défaut « saison neuve » ───────────────────────

    #[test]
    fn un_json_vide_allume_les_quatre_notifications() {
        let n: CompetitionNotifications = serde_json::from_str("{}").unwrap();

        assert_eq!(n.registration_open, NotifyRegistrationOpen(true));
        assert_eq!(n.round_eve, NotifyRoundEve(true));
        assert_eq!(n.round_closing, NotifyRoundClosing(true));
        assert_eq!(n.registration_deadline, NotifyRegistrationDeadline(true));
    }

    /// Deux chemins mènent au défaut : `Default`, qu'utilisera l'appelant quand
    /// le dépôt rend `None` (colonne `NULL`), et serde, quand le JSON stocké
    /// est incomplet. Ils doivent dire la même chose — sinon une saison neuve
    /// n'aurait pas les mêmes réglages selon qu'on la lit ou qu'on la crée.
    #[test]
    fn le_defaut_de_rust_et_celui_de_serde_coincident() {
        let par_serde: CompetitionNotifications = serde_json::from_str("{}").unwrap();

        assert_eq!(CompetitionNotifications::default(), par_serde);
    }

    #[test]
    fn quatre_reglages_eteints_font_un_aller_retour_fidele() {
        let json = r#"{"registration_open":false,"round_eve":false,
                       "round_closing":false,"registration_deadline":false}"#;
        let n: CompetitionNotifications = serde_json::from_str(json).unwrap();
        let relu: CompetitionNotifications =
            serde_json::from_str(&serde_json::to_string(&n).unwrap()).unwrap();

        assert_eq!(relu.registration_open, NotifyRegistrationOpen(false));
        assert_eq!(relu.round_eve, NotifyRoundEve(false));
        assert_eq!(relu.round_closing, NotifyRoundClosing(false));
        assert_eq!(
            relu.registration_deadline,
            NotifyRegistrationDeadline(false)
        );
    }
}

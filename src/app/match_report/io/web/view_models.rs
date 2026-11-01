use crate::app::match_report::domain::value_objects::{
    ActionPlayer, InjuryType, MatchAction, MatchActionType, PurchaseKind,
};
use crate::app::match_report::use_cases::inducements_recap_service::{
    RecapLine, RecapSide, RecordedBy,
};

// ── Récap — VMs pur domaine (from_domain / all_from_domain) ───────────────────

pub struct MatchResultVm {
    pub home_score: u8,
    pub away_score: u8,
    pub summary_title: Option<String>,
    pub summary_body: Option<String>,
}

impl MatchResultVm {
    pub fn from_domain(
        home_actions: &[MatchAction],
        away_actions: &[MatchAction],
        summary_title: Option<String>,
        summary_body: Option<String>,
    ) -> Self {
        Self {
            home_score: count_touchdowns(home_actions),
            away_score: count_touchdowns(away_actions),
            summary_title,
            summary_body,
        }
    }
}

fn count_touchdowns(actions: &[MatchAction]) -> u8 {
    actions
        .iter()
        .filter(|a| matches!(a.action, MatchActionType::Touchdown))
        .count() as u8
}

pub struct GainsFanVm {
    /// Le total crédité, bonus compris : le chiffre reste un seul nombre.
    pub home_gain_kpo: u32,
    pub away_gain_kpo: u32,
    /// Pris ou non — une note sous le montant dit d'où viennent les 10 kPo
    /// (carte 481). Un montant qu'on ne sait pas refaire est un montant qu'on
    /// vient contester.
    pub home_no_stalling_bonus: bool,
    pub away_no_stalling_bonus: bool,
    /// Lu dans le domaine : le gabarit n'écrit pas « 10 ».
    pub no_stalling_bonus_kpo: u32,
    pub home_fan_mod: i8,
    pub away_fan_mod: i8,
}

pub struct TimelineEventVm {
    pub turn: u8,
    pub side: &'static str,
    pub player_display_name: String,
    pub player_position: String,
    pub action_label: String,
    pub action_icon: &'static str,
    pub injury_label: Option<String>,
    pub injury_badge_class: &'static str,
}

impl TimelineEventVm {
    pub fn all_from_domain(
        home_actions: &[MatchAction],
        away_actions: &[MatchAction],
    ) -> Vec<Self> {
        let mut events: Vec<Self> = home_actions
            .iter()
            .map(|a| Self::from_action(a, "home"))
            .chain(away_actions.iter().map(|a| Self::from_action(a, "away")))
            .collect();
        events.sort_by_key(|e| e.turn);
        events
    }

    fn from_action(action: &MatchAction, side: &'static str) -> Self {
        // BR6 — injury_label/injury_badge_class jamais renseignés pour Sortie, seulement pour Blesse{injury}
        let (injury_label, injury_badge_class) = match &action.action {
            MatchActionType::Blesse { injury, .. } => {
                (Some(injury_label(injury)), injury_badge_class(injury))
            }
            _ => (None, ""),
        };
        Self {
            turn: action.turn.value(),
            side,
            player_display_name: action.player_display_name.clone(),
            player_position: action.player_position.clone(),
            action_label: action_label(&action.action),
            action_icon: action_icon(&action.action),
            injury_label,
            injury_badge_class,
        }
    }
}

fn action_label(action: &MatchActionType) -> String {
    match action {
        MatchActionType::Touchdown => "Touchdown".to_string(),
        MatchActionType::Passe => "Passe".to_string(),
        MatchActionType::Interception => "Interception".to_string(),
        MatchActionType::Agression => "Agression".to_string(),
        MatchActionType::Lancer => "Lancer".to_string(),
        MatchActionType::Sortie => "Sortie".to_string(),
        MatchActionType::Mvp => "MVP".to_string(),
        MatchActionType::Blesse { .. } => "Blessure".to_string(),
    }
}

fn action_icon(action: &MatchActionType) -> &'static str {
    match action {
        MatchActionType::Touchdown => "🏈",
        MatchActionType::Passe => "🎯",
        MatchActionType::Interception => "🛡️",
        MatchActionType::Agression => "💥",
        MatchActionType::Lancer => "🤾",
        MatchActionType::Sortie => "🩸",
        MatchActionType::Mvp => "⭐",
        MatchActionType::Blesse { .. } => "🚑",
    }
}

/// Regroupe la chronologie par mi-temps (règle Blood Bowl : 8 tours/mi-temps).
pub struct HalfTimelineVm {
    pub label: String,
    pub events: Vec<TimelineEventVm>,
}

const HALF_TURN_LIMIT: u8 = 8;

impl HalfTimelineVm {
    pub fn all_from_domain(
        home_actions: &[MatchAction],
        away_actions: &[MatchAction],
    ) -> Vec<Self> {
        let (first_half, second_half): (Vec<_>, Vec<_>) =
            TimelineEventVm::all_from_domain(home_actions, away_actions)
                .into_iter()
                .partition(|e| e.turn <= HALF_TURN_LIMIT);
        let (ht_home, ht_away) = halftime_score(home_actions, away_actions);
        vec![
            Self {
                label: "1ère mi-temps".to_string(),
                events: first_half,
            },
            Self {
                label: format!("2ème mi-temps · Mi-temps : {ht_home} — {ht_away}"),
                events: second_half,
            },
        ]
    }
}

fn halftime_score(home_actions: &[MatchAction], away_actions: &[MatchAction]) -> (u8, u8) {
    let count = |actions: &[MatchAction]| {
        actions
            .iter()
            .filter(|a| {
                a.turn.value() <= HALF_TURN_LIMIT && matches!(a.action, MatchActionType::Touchdown)
            })
            .count() as u8
    };
    (count(home_actions), count(away_actions))
}

fn injury_label(injury: &InjuryType) -> String {
    match injury {
        InjuryType::Commotion => "Commotion".to_string(),
        InjuryType::Amoche => "Amoché".to_string(),
        InjuryType::BlessureSerieuse => "Blessure Sérieuse".to_string(),
        InjuryType::Sequel { .. } => "Séquelle".to_string(),
        InjuryType::Mort => "Mort".to_string(),
    }
}

fn injury_badge_class(injury: &InjuryType) -> &'static str {
    match injury {
        InjuryType::Commotion | InjuryType::Amoche => "commo",
        InjuryType::BlessureSerieuse | InjuryType::Sequel { .. } => "serious",
        InjuryType::Mort => "death",
    }
}

pub struct MvpRowVm {
    pub side: &'static str,
    pub player_display_name: String,
    pub player_position: String,
    pub team_name: String,
}

impl MvpRowVm {
    pub fn all_from_domain(
        home_actions: &[MatchAction],
        away_actions: &[MatchAction],
        home_team_name: &str,
        away_team_name: &str,
    ) -> Vec<Self> {
        let home = home_actions
            .iter()
            .filter(|a| matches!(a.action, MatchActionType::Mvp))
            .map(|a| Self {
                side: "home",
                player_display_name: a.player_display_name.clone(),
                player_position: a.player_position.clone(),
                team_name: home_team_name.to_string(),
            });
        let away = away_actions
            .iter()
            .filter(|a| matches!(a.action, MatchActionType::Mvp))
            .map(|a| Self {
                side: "away",
                player_display_name: a.player_display_name.clone(),
                player_position: a.player_position.clone(),
                team_name: away_team_name.to_string(),
            });
        home.chain(away).collect()
    }
}

pub struct InjuryRowVm {
    pub side: &'static str,
    pub player_display_name: String,
    pub player_position: String,
    pub turn: u8,
    pub injury_label: String,
    pub injury_badge_class: &'static str,
}

impl InjuryRowVm {
    pub fn all_from_domain(
        home_actions: &[MatchAction],
        away_actions: &[MatchAction],
    ) -> Vec<Self> {
        let home = home_actions
            .iter()
            .filter_map(|a| Self::from_action(a, "home"));
        let away = away_actions
            .iter()
            .filter_map(|a| Self::from_action(a, "away"));
        home.chain(away).collect()
    }

    fn from_action(action: &MatchAction, side: &'static str) -> Option<Self> {
        match &action.action {
            MatchActionType::Blesse { injury, .. } => Some(Self {
                side,
                player_display_name: action.player_display_name.clone(),
                player_position: action.player_position.clone(),
                turn: action.turn.value(),
                injury_label: injury_label(injury),
                injury_badge_class: injury_badge_class(injury),
            }),
            _ => None,
        }
    }
}

pub fn action_player_key(player: &ActionPlayer) -> String {
    match player {
        ActionPlayer::Regular(player_id) => player_id.to_string(),
        ActionPlayer::Temp(temp_id) => temp_id.0.clone(),
    }
}

// ── Récapitulatif des coups de pouce (carte 628) ──────────────────────────────

/// Un camp au récapitulatif : tout vient du service d'hydratation, donc du
/// domaine — la vue ne fait que nommer.
pub struct RecapSideVm {
    pub team_name: String,
    pub initials: String,
    pub role_label: &'static str,
    pub lines: Vec<RecapLineVm>,
    pub spent: u32,
    pub budget: u32,
    /// « par vous », « par Morgane » ; vide quand l'auteur est inconnu.
    pub recorded_by_label: String,
}

pub struct RecapLineVm {
    pub name: String,
    pub kind_label: &'static str,
    pub price: u32,
}

impl RecapSideVm {
    pub fn from_recap(side: &RecapSide) -> Self {
        Self {
            initials: team_initials(&side.team_name),
            team_name: side.team_name.clone(),
            role_label: match side.is_topdog {
                true => "Favori · achète en premier",
                false => "Outsider · achète en second",
            },
            lines: side.lines.iter().map(RecapLineVm::from_recap).collect(),
            spent: side.spent,
            budget: side.budget,
            recorded_by_label: match &side.recorded_by {
                Some(RecordedBy::Viewer) => "par vous".to_string(),
                Some(RecordedBy::Coach(name)) => format!("par {name}"),
                None => String::new(),
            },
        }
    }
}

impl RecapLineVm {
    fn from_recap(line: &RecapLine) -> Self {
        Self {
            name: match line.qty {
                0 | 1 => line.name.clone(),
                qty => format!("{} ×{qty}", line.name),
            },
            kind_label: match line.kind {
                PurchaseKind::Inducement => "Coup de pouce",
                PurchaseKind::StarPlayer => "Star",
                PurchaseKind::Mercenary { skilled: true, .. } => "Mercenaire Niv. 1",
                PurchaseKind::Mercenary { skilled: false, .. } => "Mercenaire",
            },
            price: line.total_cost,
        }
    }
}

/// Les initiales d'une équipe, pour son logo de remplacement.
pub fn team_initials(team_name: &str) -> String {
    team_name
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

#[cfg(test)]
mod recap_tests {
    use super::*;

    fn side(recorded_by: Option<RecordedBy>, lines: Vec<RecapLine>) -> RecapSide {
        RecapSide {
            team_id: "T".into(),
            team_name: "Nains de Fer".into(),
            is_topdog: true,
            lines,
            spent: 145,
            budget: 510,
            recorded_by,
        }
    }

    fn line(name: &str, qty: u8, kind: PurchaseKind) -> RecapLine {
        RecapLine {
            name: name.into(),
            qty,
            kind,
            total_cost: 30,
        }
    }

    #[test]
    fn un_camp_se_nomme_et_vouvoie_son_lecteur() {
        let vm = RecapSideVm::from_recap(&side(Some(RecordedBy::Viewer), vec![]));
        assert_eq!(vm.initials, "ND");
        assert_eq!(vm.role_label, "Favori · achète en premier");
        assert_eq!(vm.recorded_by_label, "par vous");
        let vm = RecapSideVm::from_recap(&side(Some(RecordedBy::Coach("Morgane".into())), vec![]));
        assert_eq!(vm.recorded_by_label, "par Morgane");
        assert_eq!(
            RecapSideVm::from_recap(&side(None, vec![])).recorded_by_label,
            ""
        );
    }

    #[test]
    fn chaque_ligne_dit_sa_nature_et_sa_quantite() {
        let lines = vec![
            line("Chariot de Vivres", 2, PurchaseKind::Inducement),
            line("Morg", 1, PurchaseKind::StarPlayer),
            line(
                "Bloqueur nain",
                1,
                PurchaseKind::Mercenary {
                    position_uid: "p".into(),
                    skilled: true,
                },
            ),
            line(
                "Bloqueur nain",
                1,
                PurchaseKind::Mercenary {
                    position_uid: "p".into(),
                    skilled: false,
                },
            ),
        ];
        let vm = RecapSideVm::from_recap(&side(None, lines));
        let rendered: Vec<_> = vm
            .lines
            .iter()
            .map(|l| (l.name.as_str(), l.kind_label))
            .collect();
        assert_eq!(
            rendered,
            [
                ("Chariot de Vivres ×2", "Coup de pouce"),
                ("Morg", "Star"),
                ("Bloqueur nain", "Mercenaire Niv. 1"),
                ("Bloqueur nain", "Mercenaire"),
            ]
        );
    }
}

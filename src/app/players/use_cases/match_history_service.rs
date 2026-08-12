use crate::app::players::domain::events::PlayerDomainEvent;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchHistoryActionKind {
    Touchdown,
    Pass,
    Interception,
    Casualty,
    Mvp,
    Foul,
    Injury,
}

#[derive(Debug, Clone)]
pub struct MatchHistoryAction {
    pub kind: MatchHistoryActionKind,
    pub spp_earned: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct MatchHistoryEntry {
    pub match_report_id: String,
    pub round_label: String,
    pub opponent_team_name: String,
    pub team_score: u8,
    pub opponent_score: u8,
    pub actions: Vec<MatchHistoryAction>,
}

/// Reconstruit l'historique de matchs d'un joueur à partir de ses events bruts
/// — rien n'est stocké en plus sur l'agrégat (BR établie à la conception du
/// domaine match-impact). `MatchConcluded` fournit l'en-tête de chaque match
/// (adversaire, journée, score) ; les events d'action du même
/// `context.match_report_id` fournissent les lignes de détail. Tolérant à
/// l'ordre relatif action/`MatchConcluded` (les actions peuvent arriver avant
/// l'event de fin de match dans le flux d'émission du publisher). Retourné du
/// plus récent au plus ancien.
pub fn build_match_history(events: &[PlayerDomainEvent]) -> Vec<MatchHistoryEntry> {
    let mut order: Vec<String> = Vec::new();
    let mut entries: HashMap<String, MatchHistoryEntry> = HashMap::new();

    for event in events {
        // Le match a été dépublié pour correction : sa carte disparaît de
        // l'historique. Elle réapparaîtra au rejeu de la re-publication.
        if let PlayerDomainEvent::MatchImpactReverted {
            match_report_id, ..
        } = event
        {
            entries.remove(&match_report_id.0);
            order.retain(|id| id != &match_report_id.0);
            continue;
        }

        let Some(match_report_id) = event_match_report_id(event) else {
            continue;
        };

        entries.entry(match_report_id.clone()).or_insert_with(|| {
            order.push(match_report_id.clone());
            MatchHistoryEntry {
                match_report_id: match_report_id.clone(),
                round_label: String::new(),
                opponent_team_name: String::new(),
                team_score: 0,
                opponent_score: 0,
                actions: vec![],
            }
        });
        let entry = entries.get_mut(&match_report_id).expect("juste inséré");
        apply_event(entry, event);
    }

    let mut result: Vec<MatchHistoryEntry> = order
        .into_iter()
        .filter_map(|id| entries.remove(&id))
        .collect();
    result.reverse();
    result
}

fn event_match_report_id(event: &PlayerDomainEvent) -> Option<String> {
    match event {
        // Fait d'équipe, hors de tout historique de match.
        PlayerDomainEvent::InitialRosterCompleted { .. } => None,
        PlayerDomainEvent::TouchdownScored { context, .. }
        | PlayerDomainEvent::PassCompleted { context, .. }
        | PlayerDomainEvent::InterceptionMade { context, .. }
        | PlayerDomainEvent::CasualtyInflicted { context, .. }
        | PlayerDomainEvent::MatchMvpNamed { context, .. }
        | PlayerDomainEvent::FoulCommitted { context, .. }
        | PlayerDomainEvent::InjurySustained { context, .. }
        | PlayerDomainEvent::MatchConcluded { context, .. } => Some(context.match_report_id.0.clone()),
        PlayerDomainEvent::PlayerCreated { .. }
        | PlayerDomainEvent::InitialSkillEarned { .. }
        | PlayerDomainEvent::PlayerAvailabilityRestored { .. }
        | PlayerDomainEvent::PlayerSkillPurchased { .. }
        | PlayerDomainEvent::PlayerStatIncreased { .. }
        // Traité en amont de la boucle : il retire une entrée au lieu d'en
        // alimenter une.
        | PlayerDomainEvent::MatchImpactReverted { .. }
        // Décisions de coach, hors de tout match : rien à rattacher.
        | PlayerDomainEvent::PlayerDismissed { .. }
        | PlayerDomainEvent::PlayerRenamed { .. }
        | PlayerDomainEvent::PlayerJerseyChanged { .. }
        | PlayerDomainEvent::PlayerReordered { .. } => None,
    }
}

fn apply_event(entry: &mut MatchHistoryEntry, event: &PlayerDomainEvent) {
    match event {
        PlayerDomainEvent::InitialRosterCompleted { .. } => {}
        PlayerDomainEvent::MatchConcluded {
            context,
            team_score,
            opponent_score,
            ..
        } => {
            entry.round_label = context.round_label.clone();
            entry.opponent_team_name = context.opponent_team_name.clone();
            entry.team_score = *team_score;
            entry.opponent_score = *opponent_score;
        }
        PlayerDomainEvent::TouchdownScored { spp_earned, .. } => push(
            entry,
            MatchHistoryActionKind::Touchdown,
            Some(spp_earned.into_inner()),
        ),
        PlayerDomainEvent::PassCompleted { spp_earned, .. } => push(
            entry,
            MatchHistoryActionKind::Pass,
            Some(spp_earned.into_inner()),
        ),
        PlayerDomainEvent::InterceptionMade { spp_earned, .. } => push(
            entry,
            MatchHistoryActionKind::Interception,
            Some(spp_earned.into_inner()),
        ),
        PlayerDomainEvent::CasualtyInflicted { spp_earned, .. } => push(
            entry,
            MatchHistoryActionKind::Casualty,
            Some(spp_earned.into_inner()),
        ),
        PlayerDomainEvent::MatchMvpNamed { spp_earned, .. } => push(
            entry,
            MatchHistoryActionKind::Mvp,
            Some(spp_earned.into_inner()),
        ),
        PlayerDomainEvent::FoulCommitted { .. } => push(entry, MatchHistoryActionKind::Foul, None),
        PlayerDomainEvent::InjurySustained { .. } => {
            push(entry, MatchHistoryActionKind::Injury, None)
        }
        PlayerDomainEvent::PlayerCreated { .. }
        | PlayerDomainEvent::InitialSkillEarned { .. }
        | PlayerDomainEvent::PlayerAvailabilityRestored { .. }
        | PlayerDomainEvent::PlayerSkillPurchased { .. }
        | PlayerDomainEvent::PlayerStatIncreased { .. }
        | PlayerDomainEvent::MatchImpactReverted { .. }
        | PlayerDomainEvent::PlayerDismissed { .. }
        | PlayerDomainEvent::PlayerRenamed { .. }
        | PlayerDomainEvent::PlayerJerseyChanged { .. }
        | PlayerDomainEvent::PlayerReordered { .. } => {}
    }
}

fn push(entry: &mut MatchHistoryEntry, kind: MatchHistoryActionKind, spp_earned: Option<u32>) {
    entry.actions.push(MatchHistoryAction { kind, spp_earned });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::match_impact::{
        MatchContext, MatchReportId, RoundId, SppEarned,
    };
    use crate::app::players::domain::player::{PlayerId, TeamId};

    fn context(match_report_id: &str, round_label: &str, opponent: &str) -> MatchContext {
        MatchContext {
            match_report_id: MatchReportId(match_report_id.into()),
            round_id: RoundId("r".into()),
            round_label: round_label.into(),
            opponent_team_id: TeamId("opp".into()),
            opponent_team_name: opponent.into(),
        }
    }

    fn touchdown(match_report_id: &str) -> PlayerDomainEvent {
        PlayerDomainEvent::TouchdownScored {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            context: context(match_report_id, "Journée 1", "Bone Crushers"),
            spp_earned: SppEarned::try_new(3).unwrap(),
        }
    }

    fn concluded(
        match_report_id: &str,
        round_label: &str,
        opponent: &str,
        team_score: u8,
        opponent_score: u8,
    ) -> PlayerDomainEvent {
        PlayerDomainEvent::MatchConcluded {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            context: context(match_report_id, round_label, opponent),
            team_score,
            opponent_score,
        }
    }

    #[test]
    fn groups_events_by_match_report_id() {
        let events = vec![
            touchdown("mr1"),
            concluded("mr1", "Journée 1", "Bone Crushers", 2, 1),
            touchdown("mr2"),
            concluded("mr2", "Journée 2", "Green Machine", 1, 1),
        ];
        let history = build_match_history(&events);
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn most_recent_match_first() {
        let events = vec![
            touchdown("mr1"),
            concluded("mr1", "Journée 1", "Bone Crushers", 2, 1),
            touchdown("mr2"),
            concluded("mr2", "Journée 2", "Green Machine", 1, 1),
        ];
        let history = build_match_history(&events);
        assert_eq!(history[0].match_report_id, "mr2");
        assert_eq!(history[1].match_report_id, "mr1");
    }

    #[test]
    fn actions_attached_to_correct_match() {
        let events = vec![
            touchdown("mr1"),
            touchdown("mr1"),
            concluded("mr1", "Journée 1", "Bone Crushers", 2, 1),
            touchdown("mr2"),
            concluded("mr2", "Journée 2", "Green Machine", 1, 1),
        ];
        let history = build_match_history(&events);
        let mr1 = history.iter().find(|m| m.match_report_id == "mr1").unwrap();
        let mr2 = history.iter().find(|m| m.match_report_id == "mr2").unwrap();
        assert_eq!(mr1.actions.len(), 2);
        assert_eq!(mr2.actions.len(), 1);
    }

    #[test]
    fn match_without_action_still_appears_via_match_concluded_alone() {
        let events = vec![concluded("mr1", "Journée 1", "Bone Crushers", 0, 2)];
        let history = build_match_history(&events);
        assert_eq!(history.len(), 1);
        assert!(history[0].actions.is_empty());
        assert_eq!(history[0].opponent_team_name, "Bone Crushers");
    }

    #[test]
    fn header_populated_even_if_action_arrives_before_match_concluded() {
        // ordre réel du publisher : actions d'abord, TeamMatchConcluded ensuite
        let events = vec![
            touchdown("mr1"),
            concluded("mr1", "Journée 1", "Bone Crushers", 2, 1),
        ];
        let history = build_match_history(&events);
        assert_eq!(history[0].round_label, "Journée 1");
        assert_eq!(history[0].team_score, 2);
        assert_eq!(history[0].actions.len(), 1);
    }
}

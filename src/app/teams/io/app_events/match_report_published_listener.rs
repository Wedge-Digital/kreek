use crate::app::shared_kernel::app_events::match_report_app_events::{
    MatchReportAppEvent, MatchReportPublishedPayload,
};
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::teams::domain::team::{Team, TeamDomainEvent};
use crate::app::teams::domain::value_objects::{Kpo, MatchResult};
use crate::app::teams::ports::ITeamRepository;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use std::cmp::Ordering;
use std::sync::Arc;
use tracing::Instrument;

/// Conséquences du rapport publié pour une équipe — jamais construit en
/// mélangeant les champs home/away (cf. `derive_team_effects`).
struct TeamMatchEffect {
    team_id: String,
    match_report_id: String,
    result: MatchResult,
    fan_mod: i8,
    gain_kpo: u32,
    /// Ce que les coups de pouce retirent à la caisse — déjà net de la petite
    /// monnaie de l'underdog, calculé par `match_report` qui seul connaissait
    /// l'écart de valeur d'équipe.
    inducement_spending_kpo: u32,
}

pub fn init(app_event_bus: &EventBus, team_repo: Arc<dyn ITeamRepository>) {
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(MatchReportAppEvent::MatchReportPublished(payload)) =
                        serde_json::from_value::<MatchReportAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };

                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    let (home, away) = derive_team_effects(&payload);
                    async {
                        handle_team(&team_repo, home).await;
                        handle_team(&team_repo, away).await;
                    }
                    .instrument(span)
                    .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("match_report_published_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

fn derive_result(own_score: u8, opponent_score: u8) -> MatchResult {
    match own_score.cmp(&opponent_score) {
        Ordering::Greater => MatchResult::Win,
        Ordering::Equal => MatchResult::Draw,
        Ordering::Less => MatchResult::Loss,
    }
}

fn derive_team_effects(
    payload: &MatchReportPublishedPayload,
) -> (TeamMatchEffect, TeamMatchEffect) {
    let home = TeamMatchEffect {
        team_id: payload.home_team_id.clone(),
        match_report_id: payload.match_report_id.clone(),
        result: derive_result(payload.home_score, payload.away_score),
        fan_mod: payload.home_fan_mod,
        gain_kpo: payload.home_gain_kpo,
        inducement_spending_kpo: payload.home_inducement_spending_kpo,
    };
    let away = TeamMatchEffect {
        team_id: payload.away_team_id.clone(),
        match_report_id: payload.match_report_id.clone(),
        result: derive_result(payload.away_score, payload.home_score),
        fan_mod: payload.away_fan_mod,
        gain_kpo: payload.away_gain_kpo,
        inducement_spending_kpo: payload.away_inducement_spending_kpo,
    };
    (home, away)
}

async fn handle_team(team_repo: &Arc<dyn ITeamRepository>, effect: TeamMatchEffect) {
    let team_id = &effect.team_id;
    let team = match team_repo.find_by_id(team_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            tracing::warn!("match_report_published_listener: team {team_id} not found");
            return;
        }
        Err(e) => {
            tracing::error!("match_report_published_listener: find_by_id {team_id}: {e}");
            return;
        }
    };

    // spp_gains reste hors périmètre (cartes 35/145/154).
    let sequence = match team.start_post_match_sequence(
        effect.result.clone(),
        effect.fan_mod,
        Kpo(effect.gain_kpo),
        vec![],
    ) {
        Ok(event) => event,
        Err(e) => {
            tracing::warn!(
                "match_report_published_listener: start_post_match_sequence {team_id}: {e}"
            );
            return;
        }
    };

    let lot = build_lot(&team, sequence, &effect);
    if let Err(e) = team_repo.append_batch(team_id, &lot, team.version).await {
        tracing::error!("match_report_published_listener: append {team_id}: {e}");
    } else {
        tracing::info!("match_report_published_listener: team {team_id} → PlayerImprovement");
    }
}

/// Le paiement des coups de pouce **suit** la séquence, et ce n'est pas
/// indifférent : c'est `PostMatchSequenceStarted` qui pose l'instantané de
/// compensation, et `InducementsPaid` qui vient y inscrire son montant. Dans
/// l'autre ordre, la dépublication ne saurait pas quoi rembourser.
///
/// Un lot, pour que gains et coups de pouce atterrissent ensemble : une panne
/// entre les deux laisserait la trésorerie à mi-chemin.
fn build_lot(
    team: &Team,
    sequence: TeamDomainEvent,
    effect: &TeamMatchEffect,
) -> Vec<TeamDomainEvent> {
    let mut lot = vec![sequence];
    if effect.inducement_spending_kpo == 0 {
        return lot;
    }
    // Le budget a été vérifié à l'achat contre cette même caisse, donc le cas
    // ne devrait pas se produire. S'il se produit, `treasury_movement` écrête à
    // zéro et l'équipe paie moins que dû : on le dit plutôt que de le laisser
    // passer sans trace.
    if effect.inducement_spending_kpo > team.treasury.0 {
        tracing::warn!(
            "match_report_published_listener: coups de pouce de {} kPo pour une trésorerie de {} kPo (équipe {}) — débit écrêté à zéro",
            effect.inducement_spending_kpo,
            team.treasury.0,
            effect.team_id,
        );
    }
    if let Ok(mr_id) = MatchReportId::try_new(&effect.match_report_id) {
        lot.push(team.pay_inducements(mr_id, Kpo(effect.inducement_spending_kpo)));
    } else {
        tracing::error!(
            "match_report_published_listener: identifiant de rapport illisible ({}) — coups de pouce non débités",
            effect.match_report_id
        );
    }
    lot
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_result_victoire() {
        assert_eq!(derive_result(2, 1), MatchResult::Win);
    }

    #[test]
    fn derive_result_defaite() {
        assert_eq!(derive_result(1, 2), MatchResult::Loss);
    }

    #[test]
    fn derive_result_nul() {
        assert_eq!(derive_result(1, 1), MatchResult::Draw);
    }

    fn sample_payload() -> MatchReportPublishedPayload {
        MatchReportPublishedPayload {
            match_report_id: "mr1".into(),
            space_id: "sp1".into(),
            competition_id: "c1".into(),
            season_id: "s1".into(),
            round_id: "r1".into(),
            pairing_id: None,
            published_at: chrono::Utc::now(),
            home_team_id: "home-team".into(),
            away_team_id: "away-team".into(),
            home_score: 2,
            away_score: 1,
            home_gain_kpo: 150_000,
            home_inducement_spending_kpo: 0,
            away_inducement_spending_kpo: 0,
            away_gain_kpo: 90_000,
            home_fan_mod: 1,
            away_fan_mod: -2,
            home_actions: vec![],
            away_actions: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
        }
    }

    #[test]
    fn derive_team_effects_never_crosses_home_and_away() {
        let (home, away) = derive_team_effects(&sample_payload());

        assert_eq!(home.team_id, "home-team");
        assert_eq!(home.result, MatchResult::Win);
        assert_eq!(home.fan_mod, 1);
        assert_eq!(home.gain_kpo, 150_000);

        assert_eq!(away.team_id, "away-team");
        assert_eq!(away.result, MatchResult::Loss);
        assert_eq!(away.fan_mod, -2);
        assert_eq!(away.gain_kpo, 90_000);
    }
}

//! Le match a changé de journée : l'historique de chaque joueur qui l'a joué
//! le date désormais de là (carte 557).
//!
//! Appelé depuis `player_match_impact_listener`, dans la même tâche
//! séquentielle que les autres faits de match — deux tâches se disputeraient
//! la version des mêmes joueurs, et l'une des deux perdrait en silence.
//!
//! **Seuls les joueurs qui ont conclu ce match** reçoivent l'événement : un
//! joueur arrivé depuis n'a pas ce match dans son historique, et lui écrire
//! une relocalisation daterait un match qu'il n'a pas joué.

use crate::app::players::domain::match_impact::{MatchReportId, RoundId};
use crate::app::players::ports::IPlayerRepository;

pub(crate) async fn handle_team_match_relocated(
    player_repo: &dyn IPlayerRepository,
    team_id: &str,
    match_report_id: &str,
    round_id: &str,
    round_label: &str,
) {
    let team = crate::app::players::domain::player::TeamId(team_id.to_string());
    let ids = match player_repo.find_ids_by_match(&team, match_report_id).await {
        Ok(ids) => ids,
        Err(e) => {
            tracing::error!("team_match_relocated_listener: find_ids_by_match {team_id}: {e}");
            return;
        }
    };

    for player_id in &ids {
        let player = match player_repo.find_by_id(player_id).await {
            Ok(Some(p)) => p,
            Ok(None) => continue,
            Err(e) => {
                tracing::error!(
                    "team_match_relocated_listener: find_by_id {}: {e}",
                    player_id.0
                );
                continue;
            }
        };
        let event = player.relocate_match(
            MatchReportId(match_report_id.to_string()),
            RoundId(round_id.to_string()),
            round_label.to_string(),
        );
        if let Err(e) = player_repo
            .append(&player.id, &player.team_id, &event, player.version + 1)
            .await
        {
            tracing::error!(
                "team_match_relocated_listener: append MatchRelocated {}: {e}",
                player.id.0
            );
        }
    }
}

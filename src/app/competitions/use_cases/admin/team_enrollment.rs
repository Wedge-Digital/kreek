use crate::app::competitions::domain::match_day::MatchDay;
use crate::app::competitions::domain::match_day_repository_port::NewPairingProjection;
use crate::app::competitions::ports::{ITeamInfoPort, TeamInfoDto};
use std::collections::HashMap;

/// Charge les équipes réellement `Enrolled` d'une saison, indexées par id —
/// seule source de vérité pour savoir si une équipe peut être appariée (BR :
/// un pairing n'est jamais créé pour une équipe non enrôlée).
// arch:no-instrument — lecture appelée depuis d'autres use cases, déjà instrumentés
pub async fn load_enrolled_teams(
    season_id: &str,
    team_port: &dyn ITeamInfoPort,
) -> Result<HashMap<String, TeamInfoDto>, String> {
    let enrolled = team_port.find_enrolled_teams(season_id).await?;
    Ok(enrolled
        .into_iter()
        .map(|t| (t.team_id.clone(), t))
        .collect())
}

/// Ne garde que les ids présents dans `enrolled` ; les autres sont renvoyés à
/// part pour permettre d'avertir l'admin de leur exclusion.
pub fn filter_enrolled_team_ids(
    team_ids: &[String],
    enrolled: &HashMap<String, TeamInfoDto>,
) -> (Vec<String>, Vec<String>) {
    let mut kept = Vec::new();
    let mut skipped = Vec::new();
    for id in team_ids {
        if enrolled.contains_key(id) {
            kept.push(id.clone());
        } else {
            skipped.push(id.clone());
        }
    }
    (kept, skipped)
}

/// Résout des noms d'affichage pour un ensemble d'ids d'équipes exclues (donc
/// absentes de la map des enrôlées), pour construire un message d'avertissement.
// arch:no-instrument — lecture de noms d'affichage pour un message d'avertissement
pub async fn resolve_team_names(
    mut team_ids: Vec<String>,
    team_port: &dyn ITeamInfoPort,
) -> Vec<String> {
    team_ids.sort();
    team_ids.dedup();
    if team_ids.is_empty() {
        return vec![];
    }
    team_port
        .find_team_names(&team_ids)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|t| t.team_name)
        .collect()
}

/// Construit les données de projection d'un pairing à partir de la journée et
/// des équipes déjà résolues — utilisé par tous les créateurs de pairing
/// (ajout manuel, génération par journée/saison) pour l'écriture atomique
/// pairing + projection (cf. `IMatchDayRepository::save_pairing`).
pub fn build_new_pairing_projection(
    home: &str,
    away: &str,
    season_id: &str,
    match_day: &MatchDay,
    team_display: &HashMap<String, TeamInfoDto>,
) -> NewPairingProjection {
    let home_info = team_display
        .get(home)
        .expect("home team vérifié enrôlé avant projection");
    let away_info = team_display
        .get(away)
        .expect("away team vérifié enrôlé avant projection");
    NewPairingProjection {
        season_id: season_id.to_string(),
        round_name: match_day.name.to_string(),
        round_position: match_day.position.into_inner(),
        round_date_start: match_day.date_start.as_ref().map(|d| d.to_string()),
        round_date_end: match_day.date_end.as_ref().map(|d| d.to_string()),
        round_day_type: match_day.day_type.as_str().to_string(),
        home_team_name: home_info.team_name.clone(),
        home_roster_name: home_info.roster_name.clone(),
        home_coach_name: home_info.coach_name.clone(),
        home_logo_url: home_info.logo_url.clone(),
        away_team_name: away_info.team_name.clone(),
        away_roster_name: away_info.roster_name.clone(),
        away_coach_name: away_info.coach_name.clone(),
        away_logo_url: away_info.logo_url.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dto(id: &str) -> TeamInfoDto {
        TeamInfoDto {
            team_id: id.to_string(),
            team_name: format!("Team {id}"),
            coach_id: String::new(),
            coach_name: String::new(),
            roster_name: String::new(),
            logo_url: None,
        }
    }

    #[test]
    fn filter_enrolled_team_ids_splits_present_and_missing() {
        let mut enrolled = HashMap::new();
        enrolled.insert("t1".to_string(), dto("t1"));

        let (kept, skipped) = filter_enrolled_team_ids(&["t1".into(), "t2".into()], &enrolled);

        assert_eq!(kept, vec!["t1".to_string()]);
        assert_eq!(skipped, vec!["t2".to_string()]);
    }

    #[test]
    fn filter_enrolled_team_ids_empty_input_yields_empty_output() {
        let enrolled = HashMap::new();
        let (kept, skipped) = filter_enrolled_team_ids(&[], &enrolled);
        assert!(kept.is_empty());
        assert!(skipped.is_empty());
    }
}

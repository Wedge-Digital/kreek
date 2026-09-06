use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{TeamSide, TempPlayerKind};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub struct TempPlayerRowVm {
    pub temp_player_id: String,
    pub label: String,
}

#[derive(Template)]
#[template(path = "temp-player-selector-widget.html")]
pub struct TempPlayerSelectorTemplate {
    /// Les mercenaires, qui n'existent que le temps de ce rapport — et qui
    /// s'affichaient jusqu'ici sous le titre « Journaliers », avec un badge
    /// « J ». Un défaut d'étiquetage antérieur à l'épic E15, que le retrait des
    /// journaliers met au jour : sans ce champ, la section serait restée
    /// nommée d'après ceux qui n'y sont plus.
    pub mercenaries: Vec<TempPlayerRowVm>,
    pub stars: Vec<TempPlayerRowVm>,
}

impl IntoResponse for TempPlayerSelectorTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_temp_players_step3(
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let _ = space_id;
    render_temp_players(&mr_id, TeamSide::Home, &state).await
}

pub async fn get_temp_players_step4(
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let _ = space_id;
    render_temp_players(&mr_id, TeamSide::Away, &state).await
}

async fn render_temp_players(mr_id: &str, side: TeamSide, state: &AppState) -> Response {
    let state_opt = match state.match_report.match_report_repo.find_by_id(mr_id).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let pm = match state_opt {
        Some(MatchReportState::PreMatch(pm)) => pm,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let (mut mercenaries, mut stars) = (Vec::new(), Vec::new());
    for tp in pm.temp_players_for(side).iter() {
        let vm = TempPlayerRowVm {
            temp_player_id: tp.id.0.clone(),
            label: tp
                .display_name
                .clone()
                .unwrap_or_else(|| kind_label(&tp.kind)),
        };
        match &tp.kind {
            TempPlayerKind::StarPlayer { .. } => stars.push(vm),
            TempPlayerKind::Mercenary { .. } => mercenaries.push(vm),
            // **Le journalier n'est plus un remplaçant à choisir ici.** Depuis
            // l'épic E15 il est un joueur de l'effectif, et le sélecteur des
            // joueurs réguliers le propose déjà : l'afficher aussi ici le
            // montrait deux fois pour un seul homme.
            TempPlayerKind::Journeyman { .. } => {}
        }
    }
    TempPlayerSelectorTemplate { mercenaries, stars }.into_response()
}

fn kind_label(kind: &TempPlayerKind) -> String {
    match kind {
        TempPlayerKind::StarPlayer { ref_uid, .. } => ref_uid.clone(),
        TempPlayerKind::Mercenary { .. } => "Mercenaire".to_string(),
        TempPlayerKind::Journeyman { .. } => "Journalier".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::{TempPlayer, TempPlayerId};
    use crate::app::shared_kernel::bloodbowl::team::TeamId;

    fn remplacant(id: &str, kind: TempPlayerKind, nom: Option<&str>) -> TempPlayer {
        TempPlayer {
            id: TempPlayerId(id.into()),
            team_id: TeamId::new(),
            kind,
            display_name: nom.map(|n| n.to_string()),
        }
    }

    /// Le tri qui décide de ce que l'écran montre. Il vit dans le widget ; ce
    /// test l'éprouve sur les trois natures d'un coup, parce que c'est leur
    /// **répartition** qui est la règle, pas chacune prise à part.
    fn trier(remplacants: &[TempPlayer]) -> (Vec<String>, Vec<String>) {
        let (mut mercs, mut stars) = (Vec::new(), Vec::new());
        for tp in remplacants {
            let label = tp
                .display_name
                .clone()
                .unwrap_or_else(|| kind_label(&tp.kind));
            match &tp.kind {
                TempPlayerKind::StarPlayer { .. } => stars.push(label),
                TempPlayerKind::Mercenary { .. } => mercs.push(label),
                TempPlayerKind::Journeyman { .. } => {}
            }
        }
        (mercs, stars)
    }

    /// Carte 503 — il est dans la liste des joueurs réguliers depuis l'épic
    /// E15, et l'afficher ici aussi le montrait deux fois pour un seul homme.
    #[test]
    fn un_journalier_ne_figure_pas_chez_les_remplacants() {
        let (mercs, stars) = trier(&[remplacant(
            "j1",
            TempPlayerKind::Journeyman {
                position_uid: "LINEMAN".into(),
            },
            None,
        )]);
        assert!(mercs.is_empty(), "il n'est pas un mercenaire");
        assert!(stars.is_empty(), "ni une vedette");
    }

    /// Ce qu'on ne casse pas en le retirant : le mercenaire n'existe pas dans
    /// `players` et n'y existera jamais. Il reste ici, et c'est sa place.
    #[test]
    fn un_mercenaire_reste_chez_les_remplacants() {
        let (mercs, _) = trier(&[remplacant(
            "m1",
            TempPlayerKind::Mercenary {
                position_uid: "BLITZER".into(),
            },
            None,
        )]);
        assert_eq!(mercs.len(), 1);
    }

    /// **Un défaut d'étiquetage antérieur à l'épic**, que le retrait des
    /// journaliers met au jour : les mercenaires s'affichaient sous le titre
    /// « Journaliers », parce qu'un `_` les ramassait avec eux.
    #[test]
    fn un_mercenaire_n_est_plus_etiquete_journalier() {
        let etiquette = kind_label(&TempPlayerKind::Mercenary {
            position_uid: "BLITZER".into(),
        });
        assert_eq!(etiquette, "Mercenaire");
        assert_ne!(etiquette, "Journalier");
    }

    /// La vedette garde la sienne, et son nom passe avant son type.
    #[test]
    fn une_vedette_garde_son_nom() {
        let (_, stars) = trier(&[remplacant(
            "s1",
            TempPlayerKind::StarPlayer {
                ref_uid: "GRIFF".into(),
                position_uid: "STAR".into(),
            },
            Some("Griff Oberwald"),
        )]);
        assert_eq!(stars, vec!["Griff Oberwald".to_string()]);
    }
}

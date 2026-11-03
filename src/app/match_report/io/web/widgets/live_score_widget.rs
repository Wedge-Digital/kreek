//! Le score du match en direct, en widget : `GET /{id}/live/score` (carte 646).
//!
//! Sorti de la page `/live` pour que le direct puisse le recharger : le canal
//! temps réel n'envoie qu'un signal (« le match X a changé »), et la page
//! recharge ce widget par sa route, derrière `garde_suivi` comme la page.
//!
//! Le calcul est celui de la page avant la carte : `compute_score` et
//! `compute_cas` du domaine, sur l'agrégat relu.
//!
//! **Un rapport qui n'est plus en cours rend 204** : htmx ne remplace rien, le
//! dernier score reste affiché. La page, elle, redirige un brouillon ou un
//! rapport publié ; un widget qui suivrait cette redirection injecterait une
//! page entière dans le bandeau.

use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// Le score et les sorties, tels que le domaine les compte.
pub struct LiveScoreVm {
    pub home: u8,
    pub away: u8,
    pub home_cas: u8,
    pub away_cas: u8,
}

impl LiveScoreVm {
    pub fn from_domain(pm: &MatchReportPreMatch) -> Self {
        let (home, away) = pm.compute_score();
        let (home_cas, away_cas) = pm.compute_cas();
        Self {
            home,
            away,
            home_cas,
            away_cas,
        }
    }
}

#[derive(Template)]
#[template(path = "live-score-widget.html")]
pub struct LiveScoreTemplate {
    pub score: LiveScoreVm,
}

impl IntoResponse for LiveScoreTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("live-score-widget.html: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn get_live_score(
    Path((_space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    match load_score(&state, &mr_id).await {
        Ok(Some(score)) => LiveScoreTemplate { score }.into_response(),
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Err(status) => status.into_response(),
    }
}

async fn load_score(state: &AppState, mr_id: &str) -> Result<Option<LiveScoreVm>, StatusCode> {
    let repo = state.match_report.match_report_repo.as_ref();
    match repo.find_by_id(mr_id).await {
        Ok(Some(found)) => Ok(score_of(found)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("live score find_by_id {mr_id}: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Le score d'un match en cours ou prêt à publier ; `None` pour tout autre
/// état — le widget ne remplace alors rien.
pub fn score_of(found: MatchReportState) -> Option<LiveScoreVm> {
    match found {
        MatchReportState::PreMatch(pm) => Some(LiveScoreVm::from_domain(&pm)),
        MatchReportState::ReadyToPublish(rtp) => {
            Some(LiveScoreVm::from_domain(&rtp.into_pre_match()))
        }
        MatchReportState::Draft(_)
        | MatchReportState::Published(_)
        | MatchReportState::Cancelled(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
    use crate::app::match_report::domain::match_report_state::MatchReportCancelled;
    use crate::app::match_report::domain::value_objects::MatchReportOrigin;
    use crate::app::shared_kernel::bloodbowl::ids::{
        CompetitionId, MatchReportId, RoundId, SeasonId,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};

    fn draft() -> MatchReportDraft {
        MatchReportDraft::create(
            MatchReportId::new(),
            SpaceId::new(),
            CompetitionId::new(),
            SeasonId::new(),
            RoundId::new(),
            TeamId::new(),
            TeamId::new(),
            CoachId::new(),
            MatchReportOrigin::Pairing,
            None,
        )
        .expect("deux équipes distinctes")
        .0
    }

    /// Déplacé de `live_match_controller` (carte 646) : le widget rend les
    /// chiffres que la page rendait.
    #[test]
    fn the_score_is_the_domain_score() {
        let score = LiveScoreVm {
            home: 2,
            away: 1,
            home_cas: 1,
            away_cas: 0,
        };
        let html = LiveScoreTemplate { score }.render().unwrap();
        assert!(html.contains(r#"<span class="live-score-value">2</span>"#));
        assert!(html.contains(r#"<span class="live-score-value">1</span>"#));
        assert!(html.contains("<span>1</span><span class=\"live-score-cas-sep\">"));
    }

    /// Règle 3 des widgets : rien de la page hôte ne s'hérite.
    #[test]
    fn the_widget_root_blocks_htmx_inheritance() {
        let score = LiveScoreVm {
            home: 0,
            away: 0,
            home_cas: 0,
            away_cas: 0,
        };
        let html = LiveScoreTemplate { score }.render().unwrap();
        assert!(html.contains(r#"class="live-score-center" hx-disinherit="*""#));
    }

    #[test]
    fn a_draft_renders_no_score() {
        assert!(score_of(MatchReportState::Draft(draft())).is_none());
    }

    #[test]
    fn a_cancelled_report_renders_no_score() {
        let cancelled = MatchReportCancelled {
            id: MatchReportId::new(),
            reason: "test".to_string(),
        };
        assert!(score_of(MatchReportState::Cancelled(cancelled)).is_none());
    }
}

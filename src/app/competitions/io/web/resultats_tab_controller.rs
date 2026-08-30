use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::io::web::competition_detail::{full_page, load_page_base};
use crate::app::competitions::io::web::resultats_view::{
    build_journees, compute_authorization, load_resultats, JourneeResultatsVm, MatchOutcome,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TabCursorQuery {
    pub cursor: Option<i32>,
}

#[derive(Template)]
#[template(path = "competition-tab-resultats.html")]
pub struct ResultatsTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub journees: Vec<JourneeResultatsVm>,
    pub next_cursor: Option<i32>,
    pub is_initial: bool,
}

impl IntoResponse for ResultatsTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(h) => Html(h).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_resultats_tab(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    Query(query): Query<TabCursorQuery>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let rows = match load_resultats(&state, &season_id, query.cursor).await {
        Ok(r) => r,
        Err(r) => return r,
    };

    let space_id_vo = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let competition_id_vo = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let authz =
        compute_authorization(&state, &user, &space_id_vo, &competition_id_vo, &season_id).await;

    let (journees, next_cursor) = build_journees(rows, 3, &authz);
    let is_htmx = headers.contains_key("hx-request");

    if is_htmx {
        return ResultatsTabTemplate {
            app_routes: AppRoutes::default(),
            space_id,
            competition_id,
            season_id,
            journees,
            next_cursor,
            is_initial: query.cursor.is_none(),
        }
        .into_response();
    }

    render_full_page(
        space_id,
        competition_id,
        season_id,
        journees,
        next_cursor,
        &state,
    )
    .await
}

async fn render_full_page(
    space_id: String,
    competition_id: String,
    season_id: String,
    journees: Vec<JourneeResultatsVm>,
    next_cursor: Option<i32>,
    state: &AppState,
) -> Response {
    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let pb = match load_page_base(&cid, &sid, state, &competition_id).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let _ = (journees, next_cursor);
    full_page(
        pb,
        space_id,
        competition_id,
        season_id,
        "resultats",
        false,
        vec![],
        vec![],
        vec![],
        vec![],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::io::web::resultats_view::MatchResultatVm;

    /// Un match joué, avec logos, score et lien de rapport — le cas nominal de
    /// l'onglet compétition, où `round_label` et `outcome` valent `None`.
    fn match_joue() -> MatchResultatVm {
        MatchResultatVm {
            home_name: "Les Granitiers".into(),
            home_roster: "Nains".into(),
            home_coach: "Colonel Castor".into(),
            home_logo: Some("https://exemple.test/g.png".into()),
            home_initials: "LG".into(),
            away_name: "Les Zéphyriens".into(),
            away_roster: "Elfes".into(),
            away_coach: "Dame Brume".into(),
            away_logo: None,
            away_initials: "LZ".into(),
            is_in_progress: false,
            is_completed: true,
            home_score: Some(2),
            away_score: Some(1),
            home_cas: Some(3),
            away_cas: Some(0),
            report_url: Some("/rapport/1".into()),
            round_label: None,
            outcome: None,
        }
    }

    fn rendu(matches: Vec<MatchResultatVm>) -> String {
        ResultatsTabTemplate {
            app_routes: Default::default(),
            space_id: "01ARZ3NDEKTSV4RRFFQ69G5FAW".into(),
            competition_id: "01ARZ3NDEKTSV4RRFFQ69G5FAX".into(),
            season_id: "01ARZ3NDEKTSV4RRFFQ69G5FAY".into(),
            journees: vec![JourneeResultatsVm {
                label: "Journée 3".into(),
                matches,
            }],
            next_cursor: None,
            is_initial: true,
        }
        .render()
        .expect("l'onglet doit se rendre")
    }

    /// **Les deux champs neufs valent `None` ici, et le gabarit ne rend rien.**
    ///
    /// C'est ce qui rend l'extraction invisible sur cet écran : le composant
    /// sait afficher une journée et une pastille, l'onglet compétition ne lui
    /// en donne aucune. Sans ce test, un `{% if %}` inversé ajouterait une
    /// pastille vide sur chaque ligne de toutes les compétitions.
    #[test]
    fn l_onglet_competition_ne_rend_ni_pastille_ni_journee() {
        let html = rendu(vec![match_joue()]);

        assert!(!html.contains("match-outcome"), "{html}");
        assert!(!html.contains("match-round"), "{html}");
        // Contre-épreuve : le bloc est bien rendu, ce n'est pas un gabarit vide
        // qui satisferait les deux absences.
        assert!(html.contains("match-widget"));
    }

    /// Les deux champs, quand ils sont donnés — c'est ce dont la carte 477 se
    /// servira, et le seul endroit qui l'exerce avant elle.
    #[test]
    fn le_composant_rend_la_journee_et_la_pastille_quand_on_les_lui_donne() {
        let mut m = match_joue();
        m.round_label = Some("Journée 3".into());
        m.outcome = Some(MatchOutcome::Win);

        let html = rendu(vec![m]);

        assert!(
            html.contains(r#"<div class="match-round">Journée 3</div>"#),
            "{html}"
        );
        assert!(html.contains("match-outcome--win"), "{html}");
        assert!(html.contains(">V</div>"), "{html}");
    }

    /// Les trois issues ne se confondent pas : une inversion donnerait une
    /// pastille fausse sans jamais rien casser.
    #[test]
    fn chaque_issue_a_sa_lettre_et_sa_classe() {
        for (issue, lettre, classe) in [
            (MatchOutcome::Win, ">V</div>", "match-outcome--win"),
            (MatchOutcome::Draw, ">N</div>", "match-outcome--draw"),
            (MatchOutcome::Loss, ">D</div>", "match-outcome--loss"),
        ] {
            let mut m = match_joue();
            m.outcome = Some(issue);
            let html = rendu(vec![m]);

            assert!(html.contains(lettre), "{issue:?} : lettre — {html}");
            assert!(html.contains(classe), "{issue:?} : classe — {html}");
        }
    }

    /// Le bloc extrait est **complet** : score, blessures, noms, rosters,
    /// coachs, et le repli sur les initiales quand un logo manque.
    #[test]
    fn le_composant_rend_le_score_et_les_blessures() {
        let html = rendu(vec![match_joue()]);

        for marqueur in [
            "Les Granitiers",
            "Nains · Colonel Castor",
            "Dame Brume · Elfes",
            r#"<span class="match-score-num">2</span>"#,
            r#"<span class="match-score-num">1</span>"#,
            r#"<span class="match-cas-num">3</span>"#,
            "match-cas-sep",
            r#"src="https://exemple.test/g.png""#,
            r#"<div class="match-team-logo-sm match-team-logo-sm--initials">LZ</div>"#,
        ] {
            assert!(html.contains(marqueur), "« {marqueur} » manque — {html}");
        }
    }

    /// Le second état du bloc : ni score ni blessures, un badge à la place.
    #[test]
    fn le_composant_rend_le_badge_en_cours_de_saisie() {
        let mut m = match_joue();
        m.is_completed = false;
        m.is_in_progress = true;

        let html = rendu(vec![m]);

        assert!(html.contains("match-status-badge--in-progress"));
        assert!(html.contains("En cours de saisie"));
        assert!(html.contains("match-widget--in-progress"));
        // Le bloc de score cède la place — il ne s'ajoute pas au badge.
        assert!(!html.contains("match-score-tds"), "{html}");
    }

    /// Le troisième `Option` : sans URL, pas d'ancre de recouvrement **et** pas
    /// de classe cliquable. Les deux vont ensemble — une ligne au curseur
    /// « main » qui ne mène nulle part se lit comme une panne.
    #[test]
    fn le_composant_omet_le_lien_sans_url_de_rapport() {
        let mut m = match_joue();
        m.report_url = None;

        let html = rendu(vec![m]);

        assert!(!html.contains("match-widget-link"), "{html}");
        assert!(!html.contains("match-widget--clickable"), "{html}");
        // Contre-épreuve : avec l'URL, les deux sont là.
        let avec = rendu(vec![match_joue()]);
        assert!(avec.contains("match-widget-link"));
        assert!(avec.contains("match-widget--clickable"));
    }
}

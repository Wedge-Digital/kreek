//! L'annexe des compétences d'une impression (carte 606) — une à plusieurs
//! feuilles A4.
//!
//! **Le fragment rend ses propres feuilles** (`.print-slot` › `.print-frame` ›
//! `.print-sheet`) : seul le serveur sait combien il en faut. La page hôte de
//! `teams` pose ces classes et les style ; c'est le contrat des fragments
//! d'impression qui peuvent courir sur plusieurs pages.
//!
//! Le découpage en pages est de la présentation : il suit l'orientation et la
//! densité choisies. Pas de ligne « portée par… » : `players` ne connaît pas le
//! nom des équipes.

use crate::app::players::use_cases::skills_annex_service::{self, AnnexSkill};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    #[default]
    Normal,
    Compact,
}

#[derive(Debug, Default, Deserialize)]
pub struct SkillsAnnexQuery {
    #[serde(default)]
    pub orientation: Orientation,
    #[serde(default)]
    pub density: Density,
}

impl SkillsAnnexQuery {
    /// Combien d'entrées tiennent sur une feuille : deux colonnes en portrait,
    /// trois en paysage, plus serrées en compact.
    fn per_page(&self) -> usize {
        match (self.orientation, self.density) {
            (Orientation::Portrait, Density::Normal) => 26,
            (Orientation::Portrait, Density::Compact) => 36,
            (Orientation::Landscape, Density::Normal) => 24,
            (Orientation::Landscape, Density::Compact) => 33,
        }
    }
}

pub struct SkillsAnnexVm {
    pub total: usize,
    pub pages: Vec<Vec<AnnexSkill>>,
}

impl SkillsAnnexVm {
    pub fn build(skills: Vec<AnnexSkill>, query: &SkillsAnnexQuery) -> Self {
        let total = skills.len();
        let pages = skills
            .chunks(query.per_page())
            .map(|page| page.to_vec())
            .collect();
        Self { total, pages }
    }
}

#[derive(Template)]
#[template(path = "players-print-skills.html")]
pub struct SkillsAnnexTemplate {
    pub vm: SkillsAnnexVm,
}

impl IntoResponse for SkillsAnnexTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("players print skills render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

async fn render(state: &AppState, team_ids: &[&str], query: &SkillsAnnexQuery) -> Response {
    let ctx = &state.players;
    let skills = skills_annex_service::collect(
        ctx.projection_repository.as_ref(),
        ctx.skill_catalog.as_ref(),
        team_ids,
    )
    .await;
    SkillsAnnexTemplate {
        vm: SkillsAnnexVm::build(skills, query),
    }
    .into_response()
}

pub async fn get_skills_annex(
    Path((_space_id, team_id)): Path<(String, String)>,
    Query(query): Query<SkillsAnnexQuery>,
    State(state): State<AppState>,
) -> Response {
    render(&state, &[&team_id], &query).await
}

/// `{other_team_id}` est contrôlé par `space_scope` (carte 594).
pub async fn get_skills_annex_with(
    Path((_space_id, team_id, other_team_id)): Path<(String, String, String)>,
    Query(query): Query<SkillsAnnexQuery>,
    State(state): State<AppState>,
) -> Response {
    render(&state, &[&team_id, &other_team_id], &query).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn competences(n: usize) -> Vec<AnnexSkill> {
        (0..n)
            .map(|i| AnnexSkill {
                name: format!("Compétence {i:02}"),
                category_label: "Général".into(),
                description: String::new(),
            })
            .collect()
    }

    #[test]
    fn l_annexe_se_decoupe_en_pages_selon_la_mise_en_page() {
        let portrait = SkillsAnnexQuery::default();
        let vm = SkillsAnnexVm::build(competences(30), &portrait);
        assert_eq!((vm.total, vm.pages.len()), (30, 2));
        assert_eq!(vm.pages[0].len(), 26);

        let compacte = SkillsAnnexQuery {
            density: Density::Compact,
            ..Default::default()
        };
        assert_eq!(
            SkillsAnnexVm::build(competences(30), &compacte).pages.len(),
            1
        );
    }

    #[test]
    fn sans_competence_aucune_feuille() {
        let vm = SkillsAnnexVm::build(vec![], &SkillsAnnexQuery::default());
        assert!(vm.pages.is_empty());
    }
}

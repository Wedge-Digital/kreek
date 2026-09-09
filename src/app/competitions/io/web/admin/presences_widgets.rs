//! La barre latérale des journées, et le conteneur du panneau.
//!
//! # `etat` et `resume` viennent du domaine, pas du gabarit
//!
//! Le gabarit choisit une pastille et imprime une phrase ; il ne décide pas
//! laquelle. `etat` sort de `statut_de(...)` — la fonction **libre** de la carte
//! 511, et c'est précisément pourquoi elle est libre : cette barre latérale lit
//! des DTO de la base, pas des agrégats, et doit répondre à la même question que
//! `PresenceSurvey::statut`. Une règle calculée à deux endroits finit par l'être
//! de deux façons, et celle-ci croise deux champs et une horloge.
//!
//! # La garde vaut aussi sur les fragments
//!
//! `require_admin_access` sur chaque handler, celui-ci compris : sans quoi le
//! chemin htmx du changement d'onglet contournerait le contrôle. `space_scope`
//! garantit qu'une ressource appartient à l'espace de l'URL, **pas** que
//! l'appelant en est administrateur — et il n'a aucun résolveur pour `round_id`,
//! qui passe donc librement (carte 416).

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::presence_survey::{
    statut_de, Fermeture, SurveyDeadline, SurveyStatus,
};
use crate::app::competitions::domain::presence_survey_repository_port::SurveySummaryDto;
use crate::app::competitions::io::web::admin::admin_page::require_admin_access;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use time::macros::format_description;
use time::OffsetDateTime;

// ── La barre latérale ────────────────────────────────────────────────────────

pub struct PresenceRoundItemVm {
    pub round_id: String,
    pub name: String,
    /// `repos` · `aucun` · `en_cours` · `clos` · `apparie`
    ///
    /// **Pas de `defection`** : le désaccord se calcule par `desaccord`, qui exige
    /// les appariements de la journée — que ce DTO ne porte pas. La barre latérale
    /// dit donc « appariée », et c'est le panneau qui dira « défection à traiter »
    /// (cartes 520 et 521). Enrichir `list_summaries` pour l'afficher ici coûterait
    /// une jointure par journée, pour une nuance que l'écran voisin porte déjà.
    pub etat: String,
    pub resume: String,
    pub is_rest: bool,
}

impl PresenceRoundItemVm {
    pub fn all_from_domain(resumes: &[SurveySummaryDto], aujourd_hui: &DateString) -> Vec<Self> {
        resumes
            .iter()
            .map(|dto| Self::from_domain(dto, aujourd_hui))
            .collect()
    }

    fn from_domain(dto: &SurveySummaryDto, aujourd_hui: &DateString) -> Self {
        let etat = etat_de(dto, aujourd_hui);
        Self {
            round_id: dto.round_id.clone(),
            name: dto.round_name.clone(),
            resume: resume_de(dto, &etat),
            etat,
            is_rest: dto.is_rest,
        }
    }
}

fn etat_de(dto: &SurveySummaryDto, aujourd_hui: &DateString) -> String {
    if dto.is_rest {
        return "repos".to_string();
    }
    let Some(deadline) = dto.deadline.as_ref() else {
        return "aucun".to_string();
    };
    if dto.appariee == Some(true) {
        return "apparie".to_string();
    }
    match statut_de(
        &fermeture_de(dto),
        &echeance(deadline, &dto.round_id),
        aujourd_hui,
    ) {
        SurveyStatus::Ouverte => "en_cours".to_string(),
        SurveyStatus::Close(_) => "clos".to_string(),
    }
}

/// Une échéance illisible **est signalée**, pas escamotée : elle ne peut venir
/// que d'une écriture hors du domaine, et un `unwrap_or_default` silencieux ferait
/// disparaître une campagne de la barre latérale sans une ligne de journal.
fn echeance(brut: &str, round_id: &str) -> SurveyDeadline {
    SurveyDeadline::try_new(brut.to_string()).unwrap_or_else(|_| {
        tracing::error!(round_id, deadline = brut, "échéance de campagne illisible");
        SurveyDeadline::try_new("1970-01-01".to_string()).expect("date de repli valide")
    })
}

fn fermeture_de(dto: &SurveySummaryDto) -> Fermeture {
    match dto.close_le.as_ref() {
        None => Fermeture::Aucune,
        Some(le) => {
            crate::app::competitions::domain::presence_survey::FermeeLe::try_new(le.to_string())
                .map(|le| Fermeture::Decidee { le })
                .unwrap_or_else(|_| {
                    tracing::error!(round_id = dto.round_id, close_le = le, "clôture illisible");
                    Fermeture::Aucune
                })
        }
    }
}

/// **Aucun nombre inventé.** Le résumé d'une journée appariée ne dit pas combien
/// de matchs elle porte : `SurveySummaryDto` ne le sait pas, et le déduire de
/// `presents / 2` serait faux dès qu'une équipe est exemptée. La maquette annonce
/// « 4 matchs créés » ; l'afficher demandera que le DTO compte les appariements.
fn resume_de(dto: &SurveySummaryDto, etat: &str) -> String {
    match etat {
        "repos" => "Journée de repos".to_string(),
        "aucun" => "Aucun sondage".to_string(),
        "apparie" => "Journée appariée".to_string(),
        "clos" => format!("{} présents sur {}", dto.presents, dto.attendues),
        _ => format!("{} réponses sur {}", dto.reponses, dto.attendues),
    }
}

#[derive(Template)]
#[template(path = "admin/widgets/presences-rounds.html")]
pub struct PresenceRoundsTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub rounds: Vec<PresenceRoundItemVm>,
}

impl IntoResponse for PresenceRoundsTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("presences rounds render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn presences_rounds_widget(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin_access(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &state,
    )
    .await
    {
        return resp;
    }

    // Une seule requête pour toute la saison. Les compter journée par journée
    // ferait vingt allers-retours pour une colonne.
    let resumes = match state
        .competitions
        .presence_survey_repository
        .list_summaries(&season_id)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("list_summaries: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    PresenceRoundsTemplate {
        app_routes: AppRoutes::default(),
        space_id,
        competition_id,
        season_id,
        rounds: PresenceRoundItemVm::all_from_domain(&resumes, &aujourd_hui()),
    }
    .into_response()
}

/// L'horloge est lue **ici**, au bord IO, et non dans un use case : la convention
/// de `send_due_notifications_use_case` — « `today` est une entrée, pas une
/// lecture d'horloge » — vise les use cases, qu'elle rend testables. Un rendu, lui,
/// doit bien la lire quelque part.
fn aujourd_hui() -> DateString {
    let brut = OffsetDateTime::now_utc()
        .date()
        .format(format_description!("[year]-[month]-[day]"))
        .unwrap_or_default();
    DateString::try_new(brut).unwrap_or_default()
}

// ── Le conteneur du panneau ──────────────────────────────────────────────────

/// **Provisoire** : la carte 520 remplace ce corps par les six états du panneau.
/// Il rend aujourd'hui l'invite qui vaut quand aucune journée n'est choisie — ce
/// qui est aussi le vrai comportement au premier chargement.
#[derive(Template)]
#[template(path = "admin/widgets/presences-panel-empty.html")]
pub struct PresencePanelPlaceholderTemplate {}

impl IntoResponse for PresencePanelPlaceholderTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("presences panel render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn presences_panel_widget(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin_access(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &state,
    )
    .await
    {
        return resp;
    }
    PresencePanelPlaceholderTemplate {}.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dto(
        deadline: Option<&str>,
        close_le: Option<&str>,
        appariee: Option<bool>,
    ) -> SurveySummaryDto {
        SurveySummaryDto {
            round_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            round_name: "Journée 3".to_string(),
            round_position: 2,
            is_rest: false,
            deadline: deadline.map(str::to_string),
            close_le: close_le.map(str::to_string),
            appariee,
            attendues: 14,
            reponses: 10,
            presents: 8,
        }
    }

    fn date(s: &str) -> DateString {
        DateString::try_new(s.to_string()).unwrap()
    }

    fn vm(dto: &SurveySummaryDto, aujourd_hui: &str) -> PresenceRoundItemVm {
        PresenceRoundItemVm::from_domain(dto, &date(aujourd_hui))
    }

    #[test]
    fn une_journee_sans_campagne_n_a_pas_de_sondage() {
        let v = vm(&dto(None, None, None), "2026-10-05");

        assert_eq!(v.etat, "aucun");
        assert_eq!(v.resume, "Aucun sondage");
    }

    #[test]
    fn une_campagne_ouverte_affiche_l_avancement() {
        let v = vm(&dto(Some("2026-10-10"), None, Some(false)), "2026-10-05");

        assert_eq!(v.etat, "en_cours");
        assert_eq!(v.resume, "10 réponses sur 14");
    }

    /// R23 — la clôture est **calculée**. Le jour de l'échéance on répond encore ;
    /// le lendemain la campagne est close, sans que rien ne l'ait écrite.
    #[test]
    fn une_campagne_echue_est_close_le_lendemain() {
        let d = dto(Some("2026-10-10"), None, Some(false));

        assert_eq!(vm(&d, "2026-10-10").etat, "en_cours");
        assert_eq!(vm(&d, "2026-10-11").etat, "clos");
        assert_eq!(vm(&d, "2026-10-11").resume, "8 présents sur 14");
    }

    #[test]
    fn une_cloture_decidee_prime_sur_l_echeance() {
        let v = vm(
            &dto(Some("2026-10-10"), Some("2026-10-03"), Some(false)),
            "2026-10-05",
        );

        assert_eq!(v.etat, "clos", "close par décision, échéance à venir");
    }

    /// L'appariement prime sur le statut : une journée appariée se lit comme telle,
    /// que sa campagne soit close par décision ou par échéance.
    #[test]
    fn une_journee_appariee_le_dit_plutot_que_son_statut() {
        let v = vm(&dto(Some("2026-10-10"), None, Some(true)), "2026-10-05");

        assert_eq!(v.etat, "apparie");
        assert_eq!(
            v.resume, "Journée appariée",
            "aucun nombre inventé : le DTO ne compte pas les appariements"
        );
    }

    /// R2 — il n'y a rien à sonder une journée de repos. Elle reste listée pour ne
    /// pas faire un trou dans le calendrier.
    #[test]
    fn une_journee_de_repos_est_signalee_comme_telle() {
        let mut d = dto(None, None, None);
        d.is_rest = true;

        let v = vm(&d, "2026-10-05");

        assert_eq!(v.etat, "repos");
        assert_eq!(v.resume, "Journée de repos");
        assert!(v.is_rest);
    }

    /// Une échéance illisible ne peut venir que d'une écriture hors du domaine.
    /// Elle est journalisée, et la journée reste affichée — l'escamoter ferait
    /// disparaître une campagne de la barre latérale sans une ligne de journal.
    #[test]
    fn une_echeance_illisible_ne_fait_pas_disparaitre_la_journee() {
        let v = vm(&dto(Some("pas une date"), None, Some(false)), "2026-10-05");

        assert_eq!(v.etat, "clos", "le repli de 1970 la met dans le passé");
        assert_eq!(v.name, "Journée 3");
    }

    #[test]
    fn la_liste_conserve_l_ordre_du_depot() {
        let mut premiere = dto(None, None, None);
        premiere.round_name = "Journée 1".to_string();
        let mut seconde = dto(Some("2026-10-10"), None, Some(false));
        seconde.round_name = "Journée 2".to_string();

        let vms = PresenceRoundItemVm::all_from_domain(&[premiere, seconde], &date("2026-10-05"));

        assert_eq!(vms.len(), 2);
        assert_eq!(vms[0].name, "Journée 1");
        assert_eq!(vms[1].name, "Journée 2");
    }
}

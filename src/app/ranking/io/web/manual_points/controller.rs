//! La page des points manuels : attribuer, lister, retirer.
//!
//! # Deux widgets et non un fragment unique
//!
//! Non pas à cause du nombre de sections, mais parce que le formulaire doit
//! **garder son état** pendant qu'on attribue plusieurs lignes d'affilée. C'est
//! le geste réel : un commissaire traite les forfaits d'une journée en quatre
//! attributions. Un fragment unique les rejouerait toutes depuis zéro.
//!
//! Le formulaire n'écoute donc rien ; c'est la liste qui se rafraîchit sur
//! `manualPointsChanged`.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::ranking::domain::manual_points::{ManualPoints, ManualPointsReason};
use crate::app::ranking::io::web::manual_points::builders::{build_teams, libelle_releve, Chemin};
use crate::app::ranking::io::web::manual_points::view_models::{
    ManualPointsFormVm, ManualPointsListVm,
};
use crate::app::ranking::use_cases::manual_points::ManualPointsError;
use crate::app::ranking::use_cases::{award_manual_points_use_case, revoke_manual_points_use_case};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::{Form, Json};
use serde::{Deserialize, Serialize};

// ── Gabarits ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "manual-points/page.html")]
pub struct ManualPointsPageTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub form_url: String,
    pub list_url: String,
}

#[derive(Template)]
#[template(path = "manual-points/form.html")]
pub struct ManualPointsFormTemplate {
    pub vm: ManualPointsFormVm,
    pub post_url: String,
    pub teams_json_url: String,
}

#[derive(Template)]
#[template(path = "manual-points/list.html")]
pub struct ManualPointsListTemplate {
    pub vm: ManualPointsListVm,
}

macro_rules! rendu {
    ($t:ty, $nom:literal) => {
        impl IntoResponse for $t {
            fn into_response(self) -> Response {
                match self.render() {
                    Ok(html) => Html(html).into_response(),
                    Err(e) => {
                        tracing::error!("{} render: {e}", $nom);
                        StatusCode::INTERNAL_SERVER_ERROR.into_response()
                    }
                }
            }
        }
    };
}
rendu!(ManualPointsPageTemplate, "manual points page");
rendu!(ManualPointsFormTemplate, "manual points form");
rendu!(ManualPointsListTemplate, "manual points list");

// ── Autorisation ──────────────────────────────────────────────────────────────

/// **Les lectures sont ouvertes à tout membre connecté** : les points manuels
/// sont publics, ils s'affichent déjà dans le classement. Réserver la page
/// donnerait à croire qu'ils se cachent.
fn membre(auth: &AuthSession) -> Result<String, Response> {
    match &auth.user {
        Some(u) => Ok(u.id.to_string()),
        None => Err(StatusCode::UNAUTHORIZED.into_response()),
    }
}

async fn peut_gerer(state: &AppState, user_id: &str, competition_id: &str, space_id: &str) -> bool {
    crate::app::ranking::use_cases::manual_points::autorise(
        state.ranking.admin_port.as_ref(),
        user_id,
        competition_id,
        space_id,
    )
    .await
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn manual_points_page(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
) -> Response {
    if let Err(refus) = membre(&auth_session) {
        return refus;
    }
    let routes = AppRoutes::default();
    ManualPointsPageTemplate {
        form_url: routes
            .ranking
            .manual_points_form(&space_id, &competition_id, &season_id),
        list_url: routes
            .ranking
            .manual_points_list(&space_id, &competition_id, &season_id),
        app_routes: routes,
        space_id,
        competition_id,
        season_id,
    }
    .into_response()
}

pub async fn manual_points_form(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    let user_id = match membre(&auth_session) {
        Ok(id) => id,
        Err(refus) => return refus,
    };
    rendre_formulaire(
        &state,
        &user_id,
        &space_id,
        &competition_id,
        &season_id,
        None,
        None,
    )
    .await
}

pub async fn manual_points_list(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    let user_id = match membre(&auth_session) {
        Ok(id) => id,
        Err(refus) => return refus,
    };
    rendre_liste(&state, &user_id, &space_id, &competition_id, &season_id).await
}

/// Les équipes inscrites, pour le `kreek-select` du formulaire.
#[derive(Serialize)]
pub struct TeamOption {
    pub id: String,
    pub name: String,
}

pub async fn manual_points_teams_json(
    auth_session: AuthSession,
    Path((_space_id, _competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    if let Err(refus) = membre(&auth_session) {
        return refus;
    }
    let equipes: Vec<TeamOption> = state
        .ranking
        .competition_port
        .find_enrolled_teams(&season_id)
        .await
        .into_iter()
        .map(|t| TeamOption {
            id: t.team_id,
            name: t.team_name,
        })
        .collect();
    Json(equipes).into_response()
}

/// **Le sens est une bascule, pas un signe tapé.** Un `-3` se tape aussi bien
/// par erreur qu'exprès ; le champ ne prend que des entiers positifs et le
/// handler compose. Un `direction` inconnu est un `400`, jamais un repli.
#[derive(Deserialize)]
pub struct AwardForm {
    pub team_id: String,
    pub direction: String,
    pub points: i32,
    pub reason: String,
}

pub async fn post_manual_points(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Form(form): Form<AwardForm>,
) -> Response {
    let user_id = match membre(&auth_session) {
        Ok(id) => id,
        Err(refus) => return refus,
    };

    let team_id = form.team_id.clone();
    let cmd = match construire(&form, &space_id, &competition_id, &season_id, &user_id) {
        Ok(cmd) => cmd,
        Err(Refus::Protocole(code)) => return code.into_response(),
        Err(Refus::Champ(motif)) => {
            return refus_affiche(
                &state,
                &user_id,
                &space_id,
                &competition_id,
                &season_id,
                &team_id,
                &motif,
            )
            .await
        }
    };

    match award_manual_points_use_case::execute(
        cmd,
        state.ranking.repository.as_ref(),
        state.ranking.admin_port.as_ref(),
        state.ranking.competition_port.as_ref(),
    )
    .await
    {
        Ok(()) => {
            let corps = rendre_formulaire(
                &state,
                &user_id,
                &space_id,
                &competition_id,
                &season_id,
                Some(&team_id),
                None,
            )
            .await;
            avec_evenement(corps)
        }
        Err(ManualPointsError::Forbidden) => StatusCode::FORBIDDEN.into_response(),
        Err(ManualPointsError::TeamNotEnrolled) => {
            refus_affiche(
                &state,
                &user_id,
                &space_id,
                &competition_id,
                &season_id,
                &team_id,
                "Cette équipe n'est pas inscrite à la saison.",
            )
            .await
        }
        Err(cause) => {
            tracing::error!("post manual points {competition_id}: {cause:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_manual_point(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id, point_id)): Path<(String, String, String, i64)>,
    State(state): State<AppState>,
) -> Response {
    let user_id = match membre(&auth_session) {
        Ok(id) => id,
        Err(refus) => return refus,
    };

    let cmd = revoke_manual_points_use_case::RevokeManualPointsCommand {
        id: point_id,
        season_id,
        competition_id: competition_id.clone(),
        space_id,
        user_id,
    };

    match revoke_manual_points_use_case::execute(
        cmd,
        state.ranking.repository.as_ref(),
        state.ranking.admin_port.as_ref(),
    )
    .await
    {
        // `204` et non le fragment : la liste se recharge sur l'événement, et
        // renvoyer un corps ferait deux rendus pour un seul retrait.
        Ok(()) => avec_evenement(StatusCode::NO_CONTENT.into_response()),
        Err(ManualPointsError::Forbidden) => StatusCode::FORBIDDEN.into_response(),
        Err(ManualPointsError::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(cause) => {
            tracing::error!("delete manual point {competition_id}: {cause:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// ── Composition ───────────────────────────────────────────────────────────────

enum Refus {
    /// Le formulaire n'a pas pu être compris : ce n'est pas une saisie fautive.
    Protocole(StatusCode),
    /// Un motif à montrer sous le champ, avec le formulaire re-rendu.
    Champ(String),
}

fn construire(
    form: &AwardForm,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    user_id: &str,
) -> Result<award_manual_points_use_case::AwardManualPointsCommand, Refus> {
    let signe = match form.direction.as_str() {
        "bonus" => 1,
        "penalty" => -1,
        // Un repli sur « bonus » transformerait une pénalité en récompense.
        _ => return Err(Refus::Protocole(StatusCode::BAD_REQUEST)),
    };
    if form.points <= 0 {
        return Err(Refus::Champ(
            "Le nombre de points doit être strictement positif.".to_string(),
        ));
    }
    let points = ManualPoints::try_new(signe * form.points)
        .map_err(|_| Refus::Champ("Le nombre de points doit valoir au plus 100.".to_string()))?;
    // **Un champ vide vaut une absence, pas un refus.** Le motif est facultatif
    // à l'écran ; ce n'est qu'une fois saisi qu'il doit être valide.
    let reason = match form.reason.trim().is_empty() {
        true => None,
        false => Some(
            ManualPointsReason::try_new(form.reason.clone()).map_err(|_| {
                Refus::Champ("Ce motif contient des caractères refusés.".to_string())
            })?,
        ),
    };

    Ok(award_manual_points_use_case::AwardManualPointsCommand {
        season_id: season_id.to_string(),
        competition_id: competition_id.to_string(),
        space_id: space_id.to_string(),
        team_id: form.team_id.clone(),
        user_id: user_id.to_string(),
        points,
        reason,
    })
}

/// L'en-tête que la liste écoute. Posé sur les **deux** mutations : sans lui sur
/// la suppression, la ligne resterait à l'écran jusqu'au rechargement suivant.
fn avec_evenement(mut r: Response) -> Response {
    r.headers_mut().insert(
        "HX-Trigger",
        axum::http::HeaderValue::from_static("manualPointsChanged"),
    );
    r
}

async fn refus_affiche(
    state: &AppState,
    user_id: &str,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    team_id: &str,
    motif: &str,
) -> Response {
    let corps = rendre_formulaire(
        state,
        user_id,
        space_id,
        competition_id,
        season_id,
        Some(team_id),
        Some(motif.to_string()),
    )
    .await;
    // `422` et non `400` : la requête est bien formée, c'est son contenu que le
    // domaine refuse. Htmx échange tout de même le fragment.
    (StatusCode::UNPROCESSABLE_ENTITY, corps).into_response()
}

async fn rendre_formulaire(
    state: &AppState,
    user_id: &str,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    selected_team_id: Option<&str>,
    error: Option<String>,
) -> Response {
    let routes = AppRoutes::default();
    ManualPointsFormTemplate {
        vm: ManualPointsFormVm {
            selected_team_id: selected_team_id.map(str::to_string),
            error,
            can_manage: peut_gerer(state, user_id, competition_id, space_id).await,
        },
        post_url: routes
            .ranking
            .manual_points(space_id, competition_id, season_id),
        teams_json_url: routes.ranking.manual_points_teams_json(
            space_id,
            competition_id,
            season_id,
        ),
    }
    .into_response()
}

async fn rendre_liste(
    state: &AppState,
    user_id: &str,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
) -> Response {
    let (lignes, equipes, can_manage) = tokio::join!(
        state.ranking.repository.list_manual_points(season_id),
        state
            .ranking
            .competition_port
            .find_enrolled_teams(season_id),
        peut_gerer(state, user_id, competition_id, space_id),
    );

    let lignes = match lignes {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("manual points list {season_id}: {e:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let chemin = Chemin {
        space_id,
        competition_id,
        season_id,
    };
    let nb_lignes = lignes.len();
    let teams = build_teams(lignes, &equipes, &chemin);
    ManualPointsListTemplate {
        vm: ManualPointsListVm {
            teams_label: libelle_releve(nb_lignes, teams.len()),
            teams,
            can_manage,
        },
    }
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ranking::io::web::manual_points::builders::Chemin;
    use crate::app::ranking::io::web::manual_points::view_models::{
        ManualPointVm, ManualPointsTeamVm,
    };

    fn formulaire(direction: &str, points: i32, reason: &str) -> AwardForm {
        AwardForm {
            team_id: "T1".into(),
            direction: direction.into(),
            points,
            reason: reason.into(),
        }
    }

    fn construire_test(
        f: &AwardForm,
    ) -> Result<award_manual_points_use_case::AwardManualPointsCommand, Refus> {
        construire(f, "E1", "C1", "S1", "U1")
    }

    #[test]
    fn direction_penalty_donne_un_point_negatif() {
        let cmd = construire_test(&formulaire("penalty", 3, "sanction"))
            .ok()
            .unwrap();

        assert_eq!(cmd.points.into_inner(), -3);
    }

    #[test]
    fn direction_bonus_donne_un_point_positif() {
        let cmd = construire_test(&formulaire("bonus", 3, "forfait adverse"))
            .ok()
            .unwrap();

        assert_eq!(cmd.points.into_inner(), 3);
    }

    /// **Pas de repli silencieux.** Se rabattre sur « bonus » transformerait une
    /// pénalité en récompense — l'écran afficherait un enregistrement réussi, et
    /// l'équipe sanctionnée y gagnerait des points.
    #[test]
    fn un_direction_inconnu_est_un_400() {
        let r = construire_test(&formulaire("recompense", 3, "motif"));

        assert!(matches!(r, Err(Refus::Protocole(StatusCode::BAD_REQUEST))));
    }

    /// Le champ n'accepte que des entiers positifs : le signe vient de la
    /// bascule, jamais du clavier.
    #[test]
    fn un_nombre_de_points_negatif_est_refuse_sous_le_champ() {
        assert!(matches!(
            construire_test(&formulaire("bonus", -3, "motif")),
            Err(Refus::Champ(_))
        ));
        assert!(matches!(
            construire_test(&formulaire("bonus", 0, "motif")),
            Err(Refus::Champ(_))
        ));
    }

    #[test]
    fn au_dela_de_cent_points_le_motif_vise_le_champ() {
        assert!(matches!(
            construire_test(&formulaire("bonus", 101, "motif")),
            Err(Refus::Champ(_))
        ));
    }

    // ── Le rendu conditionnel de la suppression ──────────────────────────────

    fn liste(can_manage: bool) -> String {
        let chemin = Chemin {
            space_id: "E1",
            competition_id: "C1",
            season_id: "S1",
        };
        let _ = &chemin;
        ManualPointsListTemplate {
            vm: ManualPointsListVm {
                teams: vec![ManualPointsTeamVm {
                    team_id: "T1".into(),
                    team_name: "Trolls".into(),
                    total: "+3".into(),
                    total_class: "plus",
                    line_count: 1,
                    line_label: "1 ligne".into(),
                    lines: vec![ManualPointVm {
                        id: 42,
                        delete_url: "/retrait/42".into(),
                        points: "+3".into(),
                        points_class: "plus",
                        reason: Some("forfait".into()),
                        awarded_by: "DevCoach".into(),
                        awarded_at: "19 août".into(),
                    }],
                }],
                teams_label: "1 ligne · 1 équipe concernée".into(),
                can_manage,
            },
        }
        .render()
        .unwrap()
    }

    /// **Le gabarit rend, il ne décide pas.** Sans ce test, la colonne de
    /// suppression pourrait s'afficher pour tout le monde : le `DELETE` la
    /// refuserait bien, mais l'écran aurait promis un geste impossible.
    #[test]
    fn can_manage_faux_ne_rend_pas_la_suppression() {
        let sans = liste(false);

        assert!(
            !sans.contains("/retrait/42"),
            "l'URL de retrait ne doit pas paraître"
        );
        assert!(!sans.contains("mp-icon-btn"));
        // Contre-épreuve : sans elle, un gabarit vide passerait ce test.
        assert!(
            sans.contains("Trolls"),
            "le relevé doit tout de même s'afficher"
        );
    }

    #[test]
    fn can_manage_vrai_rend_la_suppression() {
        let avec = liste(true);

        assert!(avec.contains("/retrait/42"));
        assert!(avec.contains("mp-icon-btn"));
    }

    /// Le `colspan` de la bande de groupe suit le nombre de colonnes : une
    /// colonne de moins sans `colspan` ajusté laisserait la bande dépasser du
    /// tableau.
    #[test]
    fn le_colspan_de_la_bande_suit_la_colonne_de_suppression() {
        assert!(liste(true).contains(r#"colspan="5""#));
        assert!(liste(false).contains(r#"colspan="4""#));
    }
}

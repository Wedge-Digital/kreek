//! Le fragment « Matchs d'une équipe » (carte 477).
//!
//! **C'est `competitions` qui le sert, et la fiche d'équipe qui le compose.**
//! `competition_match_display_proj` porte déjà, par match, les identifiants et
//! les noms dénormalisés des deux camps, leurs rosters, leurs coachs, leurs
//! logos, le score, les blessures et l'URL du rapport — et cette table
//! appartient à ce BC. Le patron est celui du widget joueurs, déjà composé par
//! la même fiche.
//!
//! Le prix est un aller-retour de plus au premier clic sur l'onglet. C'est
//! celui de la souveraineté des données, et il est déjà payé ailleurs.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::io::web::resultats_view::MatchOutcome;
use crate::app::competitions::io::web::resultats_view::{
    build_team_matches, compute_authorization, MatchResultatVm,
};
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "team-matches-widget.html")]
pub struct TeamMatchesWidgetTemplate {
    pub matches: Vec<MatchResultatVm>,
}

impl IntoResponse for TeamMatchesWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("team_matches_widget render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

/// # Le `competition_id` ne se prend pas dans l'URL
///
/// Le baker serait naturel — c'est ce que la règle 4 des widgets demande pour
/// un paramètre contextuel. Il ne faut pas : un admin de la compétition X
/// ouvrant la fiche d'une équipe de Y, et forçant `?competition_id=X`,
/// obtiendrait `is_comp_admin = vrai` et les liens vers les rapports de Y.
/// `space_scope` ne l'attrape pas, X et Y vivant dans le même espace.
///
/// Il se résout donc **depuis le `team_id`**, seul identifiant du chemin — et
/// `{team_id}` est déjà cloisonné par `TeamSpaceOwnership`, qui consulte les
/// deux sources depuis les cartes 320-321.
pub async fn get_team_matches_widget(
    auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(space_id_vo) = SpaceId::try_new(&space_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    // Sans inscription — un brouillon non soumis — l'équipe n'a joué aucun
    // match : une liste vide, et non une erreur, dit la vérité.
    let inscription = match state
        .competitions
        .team_info_port
        .find_team_enrollment(&team_id)
        .await
    {
        Ok(Some(e)) => e,
        Ok(None) => return TeamMatchesWidgetTemplate { matches: vec![] }.into_response(),
        Err(e) => {
            tracing::error!("team_matches_widget: find_team_enrollment {team_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let Ok(competition_id) = CompetitionId::try_new(&inscription.competition_id) else {
        tracing::error!("team_matches_widget: competition_id illisible en base pour {team_id}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let rows = match state
        .competitions
        .match_day_repository
        .list_team_matches(&inscription.season_id, &team_id)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("team_matches_widget: list_team_matches {team_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // La même fonction que l'onglet Résultats, sans une ligne d'adaptation :
    // appliquée aux matchs d'une seule équipe, elle rend exactement la règle
    // voulue — et laisse au coach de l'adversaire l'accès aux rapports de *ses*
    // matchs, où qu'il les regarde.
    let authz = compute_authorization(
        &state,
        &user,
        &space_id_vo,
        &competition_id,
        &inscription.season_id,
    )
    .await;

    TeamMatchesWidgetTemplate {
        matches: build_team_matches(rows, &authz, &team_id),
    }
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day_repository_port::PairingDisplayDto;
    use crate::app::competitions::io::web::resultats_view::{
        build_team_matches, ResultAuthorization,
    };

    fn rencontre(pairing_id: &str, statut: &str) -> PairingDisplayDto {
        PairingDisplayDto {
            pairing_id: pairing_id.into(),
            round_id: "r1".into(),
            round_name: "Journée 1".into(),
            round_position: 1,
            round_date_start: None,
            round_date_end: None,
            round_day_type: "fixed_date".into(),
            home_team_id: "A".into(),
            home_team_name: "Les Granitiers".into(),
            home_roster_name: "Nains".into(),
            home_coach_name: "Castor".into(),
            home_logo_url: None,
            home_initials: "LG".into(),
            away_team_id: "B".into(),
            away_team_name: "Les Zéphyriens".into(),
            away_roster_name: "Elfes".into(),
            away_coach_name: "Brume".into(),
            away_logo_url: None,
            away_initials: "LZ".into(),
            match_status: statut.into(),
            home_score: Some(3),
            away_score: Some(1),
            home_casualties: Some(2),
            away_casualties: Some(0),
            match_report_url: None,
        }
    }

    fn rendu(rows: Vec<PairingDisplayDto>) -> String {
        TeamMatchesWidgetTemplate {
            matches: build_team_matches(rows, &ResultAuthorization::unrestricted(), "A"),
        }
        .render()
        .expect("le fragment doit se rendre")
    }

    /// **Une équipe sans inscription rend une liste vide, pas une erreur.**
    ///
    /// Un brouillon non soumis n'a joué aucun match — c'est vrai, et c'est ce
    /// que le contrôleur traduit en rendant ce fragment sans aucune ligne. Le
    /// même bloc couvre l'équipe inscrite dont le calendrier n'est pas encore
    /// généré.
    #[test]
    fn une_equipe_sans_inscription_rend_une_liste_vide() {
        let html = rendu(vec![]);

        assert!(html.contains("Aucun match pour le moment."), "{html}");
        assert!(!html.contains("match-widget"), "aucun bloc : {html}");
    }

    /// La liste est **plate** : un seul conteneur, aucun en-tête de journée.
    /// Grouper donnerait quinze groupes d'un match, chacun titré « 1 match ».
    #[test]
    fn la_liste_est_plate_et_le_libelle_de_journee_entre_dans_le_bloc() {
        let html = rendu(vec![
            rencontre("p1", "completed"),
            rencontre("p2", "upcoming"),
        ]);

        assert_eq!(html.matches("matches-list").count(), 1, "{html}");
        assert!(!html.contains("matches-list-header"), "{html}");
        assert_eq!(html.matches(r#"class="match-round""#).count(), 2, "{html}");
        assert!(!html.contains("Aucun match"), "{html}");
    }

    /// La pastille se rend pour le match joué, et pas pour celui à venir.
    #[test]
    fn seul_un_match_joue_porte_sa_pastille() {
        let html = rendu(vec![
            rencontre("p1", "completed"),
            rencontre("p2", "upcoming"),
        ]);

        assert_eq!(html.matches("match-outcome--win").count(), 1, "{html}");
        assert_eq!(html.matches(r#"class="match-outcome"#).count(), 1, "{html}");
    }
}

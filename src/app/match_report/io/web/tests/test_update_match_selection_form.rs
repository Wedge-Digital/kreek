//! Le corps du POST de la phase 1 sur un rapport existant (carte 564).
//!
//! Quand la sélection est figée, le formulaire n'a plus aucun champ : le
//! navigateur envoie un corps **vide**. L'extraction doit l'accepter — c'est le
//! handler qui décide alors de reprendre les équipes du brouillon.

use crate::app::match_report::io::web::match_selection_controller::UpdateMatchSelectionForm;
use axum::body::Body;
use axum::extract::FromRequest;
use axum::http::Request;
use axum::Form;

async fn extraire(corps: &'static str) -> UpdateMatchSelectionForm {
    let requete = Request::post("/")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::from(corps))
        .unwrap();
    let Form(form) = Form::<UpdateMatchSelectionForm>::from_request(requete, &())
        .await
        .unwrap_or_else(|e| panic!("le corps « {corps} » doit être accepté : {e}"));
    form
}

#[tokio::test]
async fn un_corps_vide_est_accepte_sans_equipes() {
    let form = extraire("").await;
    assert_eq!(form.home_team_id, None);
    assert_eq!(form.away_team_id, None);
}

#[tokio::test]
async fn le_corps_du_widget_rend_les_deux_equipes() {
    let form =
        extraire("competition_id=c&season_id=s&round_id=r&home_team_id=h&away_team_id=a").await;
    assert_eq!(form.home_team_id.as_deref(), Some("h"));
    assert_eq!(form.away_team_id.as_deref(), Some("a"));
}

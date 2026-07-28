use crate::app::shared_kernel::identity::ids::EntityId;
use crate::app::team_creation::io::web::view_models::CartVm;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "widgets/cart-widget.html")]
pub struct CartWidgetTemplate {
    pub cart: Option<CartVm>,
}

impl IntoResponse for CartWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn cart_widget(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cart = match state
        .team_creation
        .roster_repository
        .find_by_id(&team_id_val)
        .await
    {
        Ok(Some(team)) => Some(CartVm::from_domain(&team)),
        _ => None,
    };

    CartWidgetTemplate { cart }.into_response()
}

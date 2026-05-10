use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Form;
use axum::http::StatusCode;
use serde::Deserialize;
use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::authorization::SpaceAuthorization;
use crate::app::shared_kernel::common_types::SpaceId;
use crate::app::spaces::domain::ports::SpaceRepositoryError;
use crate::state::AppState;

fn string_or_vec<'de, D>(de: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{SeqAccess, Visitor};

    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("string or sequence of strings")
        }
        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Vec<String>, E> {
            Ok(vec![v.to_owned()])
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<String>, A::Error> {
            let mut out = Vec::new();
            while let Some(v) = seq.next_element()? { out.push(v); }
            Ok(out)
        }
    }
    de.deserialize_any(V)
}

#[derive(Deserialize)]
pub struct JoinSpacesForm {
    #[serde(default, deserialize_with = "string_or_vec")]
    pub space_ids: Vec<String>,
}

pub async fn join_many_spaces(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(payload): Form<JoinSpacesForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let mut first_joined: Option<String> = None;

    for raw_id in &payload.space_ids {
        let space_id = match SpaceId::from_string(raw_id) {
            Ok(id) => id,
            Err(_) => continue,
        };

        match state.space_repository.add_member(&space_id, &user.id, &SpaceAuthorization::SimpleUser).await {
            Ok(()) => {
                if first_joined.is_none() {
                    first_joined = Some(raw_id.clone());
                }
            }
            Err(SpaceRepositoryError::CoachAlreadyMember) => {}
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }

    let redirect_to = match first_joined {
        Some(id) => crate::app::news::routes::path::APP_HOME.replace("{space_id}", &id),
        None     => crate::app::spaces::routes::path::SPACE_ALL.to_string(),
    };

    Response::builder()
        .header("HX-Redirect", redirect_to)
        .body(axum::body::Body::empty())
        .unwrap()
}
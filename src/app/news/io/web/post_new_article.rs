use crate::app::auth::auth_backend::AuthSession;
use crate::app::news::domain::article::ArticleParagraph;
use crate::app::news::io::web::new_article::NewArticleTemplate;
use crate::app::news::routes::path;
use crate::app::news::use_cases::create_article::{self, CreateArticleCommand, CreateArticleError};
use crate::app::shared_kernel::common_types::SpaceId;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct NewArticleFormPayload {
    pub title: String,
    pub tag: Option<String>,
    #[serde(rename = "abstract")]
    pub abstract_: Option<String>,
    pub image_url: Option<String>,
    pub content_json: String,
}

pub async fn post_new_article(
    Path(space_id_raw): Path<String>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(payload): Form<NewArticleFormPayload>,
) -> Response {
    let Some(user) = auth_session.user else {
        return Response::builder().status(401).body(Body::empty()).unwrap();
    };

    let Ok(space_id) = SpaceId::try_new(&space_id_raw) else {
        return Response::builder().status(400).body(Body::empty()).unwrap();
    };

    let paragraphs: Vec<ArticleParagraph> =
        serde_json::from_str(&payload.content_json).unwrap_or_default();

    let tags = payload
        .tag
        .as_deref()
        .filter(|t| !t.is_empty())
        .map(|t| vec![t.to_string()])
        .unwrap_or_default();

    let cmd = CreateArticleCommand {
        space_id,
        author_id: user.id,
        author_name: user.coach_name.into_inner(),
        title: payload.title.clone(),
        abstract_: payload.abstract_.clone().unwrap_or_default(),
        tags,
        image: payload.image_url.clone().filter(|s| !s.is_empty()),
        paragraphs,
    };

    match create_article::execute(cmd, state.news.article_repository.as_ref()).await {
        Ok(_) => {
            let redirect = path::APP_HOME.replace("{space_id}", &space_id_raw);
            Response::builder()
                .header("HX-Redirect", redirect)
                .body(Body::empty())
                .unwrap()
        }

        Err(e) => {
            let mut tmpl = NewArticleTemplate {
                space_id: space_id_raw,
                image_url_value: payload.image_url.unwrap_or_default(),
                title_value: payload.title,
                tag_value: payload.tag.unwrap_or_default(),
                abstract_value: payload.abstract_.unwrap_or_default(),
                ..Default::default()
            };
            match e {
                CreateArticleError::EmptyTitle => tmpl.title_error = Some(e.to_string()),
                CreateArticleError::EmptyContent => tmpl.content_error = Some(e.to_string()),
                CreateArticleError::Database(_) => tmpl.title_error = Some(e.to_string()),
            }
            tmpl.into_response()
        }
    }
}

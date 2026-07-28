use crate::app::auth::auth_backend::AuthSession;
use crate::app::news::domain::comment::Comment;
use crate::app::news::routes::path;
use crate::app::shared_kernel::bloodbowl::ids::{ArticleId, CommentId};
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::response::Response;
use axum::Form;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CommentPayload {
    pub content: String,
}

pub async fn post_comment(
    Path((space_id_raw, article_id_raw)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(payload): Form<CommentPayload>,
) -> Response {
    let Some(user) = auth_session.user else {
        return Response::builder().status(401).body(Body::empty()).unwrap();
    };

    let Ok(article_id) = ArticleId::try_new(&article_id_raw) else {
        return Response::builder().status(400).body(Body::empty()).unwrap();
    };

    let content = payload.content.trim().to_string();
    if content.is_empty() {
        let redirect = path::APP_ARTICLE
            .replace("{space_id}", &space_id_raw)
            .replace("{article_id}", &article_id_raw);
        return Response::builder()
            .header("HX-Redirect", redirect)
            .body(Body::empty())
            .unwrap();
    }

    let comment = Comment {
        id: CommentId::new(),
        article_id,
        author_id: user.id,
        author_name: user.coach_name.into_inner(),
        content,
        created_at: time::OffsetDateTime::now_utc(),
    };

    let _ = state.news.comment_repository.save(&comment).await;

    let redirect = path::APP_ARTICLE
        .replace("{space_id}", &space_id_raw)
        .replace("{article_id}", &article_id_raw);
    Response::builder()
        .header("HX-Redirect", redirect)
        .body(Body::empty())
        .unwrap()
}

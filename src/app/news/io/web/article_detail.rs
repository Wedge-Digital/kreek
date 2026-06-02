use crate::app::auth::auth_backend::AuthSession;
use crate::app::news::domain::article::ArticleParagraph;
use crate::app::news::domain::comment::Comment;
use crate::app::news::routes::Routes as NewsRoutes;
use crate::app::shared_kernel::common_types::ArticleId;
use crate::state::AppState;
use crate::web::routes::Routes as WebRoutes;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub struct ParagraphViewModel {
    pub title: String,
    pub content: String,
}

impl From<ArticleParagraph> for ParagraphViewModel {
    fn from(p: ArticleParagraph) -> Self {
        Self {
            title: p.title,
            content: p.content,
        }
    }
}

pub struct CommentViewModel {
    pub author_initial: String,
    pub author_name: String,
    pub avatar_class: &'static str,
    pub content: String,
    pub created_at: String,
}

const AVATAR_CLASSES: &[&str] = &[
    "avatar--blue",
    "avatar--green",
    "avatar--orange",
    "avatar--purple",
    "avatar--red",
];

fn avatar_class(name: &str) -> &'static str {
    let idx = name.chars().next().map(|c| c as usize).unwrap_or(0) % AVATAR_CLASSES.len();
    AVATAR_CLASSES[idx]
}

fn format_date(dt: &time::OffsetDateTime) -> String {
    let months = [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ];
    format!(
        "{} {} {}",
        dt.day(),
        months[dt.month() as usize - 1],
        dt.year()
    )
}

impl From<Comment> for CommentViewModel {
    fn from(c: Comment) -> Self {
        let initial = c
            .author_name
            .chars()
            .next()
            .unwrap_or('?')
            .to_uppercase()
            .next()
            .unwrap_or('?')
            .to_string();
        Self {
            avatar_class: avatar_class(&c.author_name),
            author_initial: initial,
            author_name: c.author_name,
            content: c.content,
            created_at: format_date(&c.created_at),
        }
    }
}

#[derive(Template)]
#[template(path = "article-detail.html")]
pub struct ArticleDetailTemplate {
    pub web_routes: WebRoutes,
    pub news_routes: NewsRoutes,
    pub space_id: String,
    pub article_id: String,
    pub title: String,
    pub abstract_: String,
    pub tags: Vec<String>,
    pub image: Option<String>,
    pub author_initial: String,
    pub author_name: String,
    pub created_at: String,
    pub read_time: u32,
    pub paragraphs: Vec<ParagraphViewModel>,
    pub comments: Vec<CommentViewModel>,
    pub current_user_initial: String,
}

impl IntoResponse for ArticleDetailTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_article(
    Path((space_id_raw, article_id_raw)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Ok(article_id) = ArticleId::try_new(&article_id_raw) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let Some(article) = state
        .news
        .article_repository
        .find_by_id(&article_id)
        .await
        .unwrap_or(None)
    else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let comments_raw = state
        .news
        .comment_repository
        .find_by_article(&article_id)
        .await
        .unwrap_or_default();

    let word_count: usize = article
        .content
        .iter()
        .map(|p| p.content.split_whitespace().count() + p.title.split_whitespace().count())
        .sum();
    let read_time = ((word_count as f32 / 200.0).ceil() as u32).max(1);

    let author_initial = article
        .author_name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .next()
        .unwrap_or('?')
        .to_string();

    let current_user_initial = auth_session
        .user
        .as_ref()
        .and_then(|u| u.coach_name.clone().into_inner().chars().next())
        .map(|c: char| c.to_uppercase().next().unwrap_or('?').to_string())
        .unwrap_or_else(|| "?".to_string());

    ArticleDetailTemplate {
        web_routes: Default::default(),
        news_routes: Default::default(),
        space_id: space_id_raw,
        article_id: article_id_raw,
        author_initial,
        author_name: article.author_name,
        title: article.title,
        abstract_: article.abstract_,
        tags: article.tags,
        image: article.image,
        created_at: format_date(&article.created_at),
        read_time,
        paragraphs: article.content.into_iter().map(Into::into).collect(),
        comments: comments_raw.into_iter().map(Into::into).collect(),
        current_user_initial,
    }
    .into_response()
}

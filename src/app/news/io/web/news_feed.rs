use crate::app::news::domain::article::Article;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::common_types::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

const PER_PAGE: i64 = 10;

pub struct ArticleViewModel {
    pub id: String,
    pub title: String,
    pub abstract_: String,
    pub tag: String,
    pub image: Option<String>,
    pub author_name: String,
    pub created_at: String,
}

impl From<Article> for ArticleViewModel {
    fn from(a: Article) -> Self {
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
        let created_at = format!(
            "{} {} {}",
            a.created_at.day(),
            months[a.created_at.month() as usize - 1],
            a.created_at.year(),
        );
        Self {
            id: a.id.to_string(),
            title: a.title.into_inner(),
            abstract_: a.abstract_,
            tag: a.tags.into_iter().next().map(|t| t.to_string()).unwrap_or_default(),
            image: a.image,
            author_name: a.author_name,
            created_at,
        }
    }
}

pub struct PaginationItem {
    pub page: Option<i64>, // None → ellipsis
    pub is_active: bool,
}

fn build_pagination(page: i64, total_pages: i64) -> Vec<PaginationItem> {
    let mut pages: Vec<i64> = vec![1, total_pages];
    if page > 1 {
        pages.push(page - 1);
    }
    pages.push(page);
    if page < total_pages {
        pages.push(page + 1);
    }

    pages.sort_unstable();
    pages.dedup();

    let mut items = Vec::new();
    for (i, &p) in pages.iter().enumerate() {
        if i > 0 {
            let gap = p - pages[i - 1];
            if gap == 2 {
                let mid = pages[i - 1] + 1;
                items.push(PaginationItem {
                    page: Some(mid),
                    is_active: mid == page,
                });
            } else if gap > 2 {
                items.push(PaginationItem {
                    page: None,
                    is_active: false,
                });
            }
        }
        items.push(PaginationItem {
            page: Some(p),
            is_active: p == page,
        });
    }
    items
}

#[derive(Template, Default)]
#[template(path = "news-feed.html")]
pub struct NewsFeedTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub featured: Option<ArticleViewModel>,
    pub articles: Vec<ArticleViewModel>,
    pub page: i64,
    pub total_pages: i64,
    pub pagination_items: Vec<PaginationItem>,
}

impl IntoResponse for NewsFeedTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Deserialize, Default)]
pub struct PageQuery {
    pub page: Option<i64>,
}

pub async fn get_news_feed(
    Path(space_id_raw): Path<String>,
    Query(query): Query<PageQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);

    let Ok(space_id) = SpaceId::try_new(&space_id_raw) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let (all_articles, total) = state
        .news
        .article_repository
        .find_by_space(&space_id, page, PER_PAGE)
        .await
        .unwrap_or_else(|_| (vec![], 0));

    let total_pages = ((total + PER_PAGE - 1) / PER_PAGE).max(1);
    let mut view_models: Vec<ArticleViewModel> = all_articles.into_iter().map(Into::into).collect();

    let featured = if page == 1 {
        view_models.drain(..1.min(view_models.len())).next()
    } else {
        None
    };
    let pagination_items = build_pagination(page, total_pages);

    NewsFeedTemplate {
        app_routes: Default::default(),
        space_id: space_id_raw,
        featured,
        articles: view_models,
        page,
        total_pages,
        pagination_items,
    }
    .into_response()
}

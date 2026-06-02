use crate::app::shared_kernel::common_types::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use std::collections::HashSet;

pub struct MemberItem {
    pub id: String,
    pub name: String,
    pub selected: bool,
}

#[derive(Template)]
#[template(path = "space-members-widget.html")]
pub struct SpaceMembersWidgetTemplate {
    pub members: Vec<MemberItem>,
    /// Sélection unique (false = multi-select par défaut)
    pub single: bool,
    /// Nom du champ form (`admin_ids[]` par défaut, `coach_id` pour single)
    pub field_name: String,
}

impl IntoResponse for SpaceMembersWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Deserialize, Default)]
pub struct MembersWidgetQuery {
    pub selected: Option<String>,
    /// `?single=true` → sélection unique, champ `coach_id`
    pub single: Option<bool>,
    /// Nom du champ form à utiliser (surcharge le défaut)
    pub field_name: Option<String>,
}

pub async fn get_members_widget(
    Path(space_id): Path<String>,
    Query(query): Query<MembersWidgetQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let sid = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let single = query.single.unwrap_or(false);
    let field_name = query.field_name.unwrap_or_else(|| {
        if single {
            "coach_id".into()
        } else {
            "admin_ids[]".into()
        }
    });

    let selected_ids: HashSet<String> = query
        .selected
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    let cached = state
        .spaces
        .user_cache_repository
        .list_members_for_space(&sid)
        .await
        .unwrap_or_default();

    let members = cached
        .into_iter()
        .map(|u| {
            let id = u.id.to_string();
            let selected = selected_ids.contains(&id);
            MemberItem {
                id,
                name: u.name.into_inner(),
                selected,
            }
        })
        .collect();

    SpaceMembersWidgetTemplate {
        members,
        single,
        field_name,
    }
    .into_response()
}

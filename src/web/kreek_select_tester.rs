use askama::Template;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};

#[derive(Template)]
#[template(path = "kreek-select-tester.html")]
pub struct KreekSelectTesterTemplate;

impl IntoResponse for KreekSelectTesterTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_kreek_select_tester() -> impl IntoResponse {
    KreekSelectTesterTemplate.into_response()
}

#[derive(Serialize)]
pub struct FruitOption {
    pub id: String,
    pub name: String,
    pub color: String,
}

fn all_fruits() -> Vec<FruitOption> {
    vec![
        FruitOption {
            id: "apple".into(),
            name: "Pomme".into(),
            color: "rouge".into(),
        },
        FruitOption {
            id: "banana".into(),
            name: "Banane".into(),
            color: "jaune".into(),
        },
        FruitOption {
            id: "cherry".into(),
            name: "Cerise".into(),
            color: "rouge".into(),
        },
        FruitOption {
            id: "grape".into(),
            name: "Raisin".into(),
            color: "violet".into(),
        },
        FruitOption {
            id: "kiwi".into(),
            name: "Kiwi".into(),
            color: "vert".into(),
        },
        FruitOption {
            id: "lemon".into(),
            name: "Citron".into(),
            color: "jaune".into(),
        },
        FruitOption {
            id: "mango".into(),
            name: "Mangue".into(),
            color: "orange".into(),
        },
        FruitOption {
            id: "orange".into(),
            name: "Orange".into(),
            color: "orange".into(),
        },
        FruitOption {
            id: "peach".into(),
            name: "Pêche".into(),
            color: "rose".into(),
        },
        FruitOption {
            id: "pear".into(),
            name: "Poire".into(),
            color: "vert".into(),
        },
        FruitOption {
            id: "pineapple".into(),
            name: "Ananas".into(),
            color: "jaune".into(),
        },
        FruitOption {
            id: "strawberry".into(),
            name: "Fraise".into(),
            color: "rouge".into(),
        },
        FruitOption {
            id: "watermelon".into(),
            name: "Pastèque".into(),
            color: "vert".into(),
        },
    ]
}

#[derive(Deserialize, Default)]
pub struct FruitQuery {
    pub color: Option<String>,
}

pub async fn get_kreek_select_test_data(Query(q): Query<FruitQuery>) -> impl IntoResponse {
    let fruits = all_fruits();
    let filtered: Vec<FruitOption> = match q.color {
        Some(ref c) if !c.is_empty() => fruits.into_iter().filter(|f| f.color == *c).collect(),
        _ => fruits,
    };
    Json(filtered)
}

#[derive(Serialize)]
pub struct ColorOption {
    pub id: String,
    pub name: String,
}

pub async fn get_kreek_select_test_colors() -> impl IntoResponse {
    let colors = vec![
        ColorOption {
            id: "rouge".into(),
            name: "Rouge".into(),
        },
        ColorOption {
            id: "jaune".into(),
            name: "Jaune".into(),
        },
        ColorOption {
            id: "vert".into(),
            name: "Vert".into(),
        },
        ColorOption {
            id: "orange".into(),
            name: "Orange".into(),
        },
        ColorOption {
            id: "violet".into(),
            name: "Violet".into(),
        },
        ColorOption {
            id: "rose".into(),
            name: "Rose".into(),
        },
    ];
    Json(colors)
}

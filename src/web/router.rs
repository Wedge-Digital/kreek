use crate::state::AppState;
use crate::web::app_layout::app_layout;
use crate::web::app_menu::app_menu;
use crate::web::kreek_select_tester::{
    get_kreek_select_test_colors, get_kreek_select_test_data, get_kreek_select_tester,
};
use crate::web::routes::path;
use crate::web::spaces_widget_tester::get_space_widget_tester;
use axum::{routing::get, Router};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::APP_LAYOUT, get(app_layout))
        .route(path::MENU, get(app_menu))
        .route(path::SPACES_WIDGET_TESTER, get(get_space_widget_tester))
        .route(path::KREEK_SELECT_TESTER, get(get_kreek_select_tester))
        .route(
            path::KREEK_SELECT_TESTER_DATA,
            get(get_kreek_select_test_data),
        )
        .route(
            path::KREEK_SELECT_TESTER_COLORS,
            get(get_kreek_select_test_colors),
        )
}

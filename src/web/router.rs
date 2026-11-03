use crate::state::AppState;
use crate::web::app_layout::app_layout;
use crate::web::app_menu::app_menu;
use crate::web::kreek_select_tester::{
    get_kreek_select_test_colors, get_kreek_select_test_data, get_kreek_select_tester,
};
use crate::web::realtime::close_streams_controller::post_close_my_streams;
use crate::web::realtime::presence_controller::post_presence;
use crate::web::realtime::presence_widget::get_presence_widget;
use crate::web::realtime::stream_controller::get_realtime_stream;
use crate::web::routes::path;
use crate::web::spaces_widget_tester::get_space_widget_tester;
use axum::{
    routing::{get, post},
    Router,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::APP_LAYOUT, get(app_layout))
        .route(path::MENU, get(app_menu))
        .route(path::REALTIME_STREAM, get(get_realtime_stream))
        .route(path::REALTIME_PRESENCE, post(post_presence))
        .route(path::REALTIME_MATCH_PRESENCE, get(get_presence_widget))
        // Carte 646 : réservée aux e2e, 404 hors `bypass_auth`.
        .route(
            path::REALTIME_TEST_CLOSE_STREAMS,
            post(post_close_my_streams),
        )
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

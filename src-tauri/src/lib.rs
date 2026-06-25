mod api;

use api::{
    ComicCommentsResult, ComicDetailResult, HomeFeedResult, RemoteSettingResult,
    SearchAlbumsResult, WeekFiltersResult, WeekItemsResult,
};

#[tauri::command]
async fn get_remote_setting(endpoint: Option<String>) -> Result<RemoteSettingResult, String> {
    api::get_remote_setting(endpoint)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn search_comics(
    query: String,
    page: Option<u32>,
    endpoint: Option<String>,
) -> Result<SearchAlbumsResult, String> {
    api::search_comics(query, page, endpoint)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_home_feed(endpoint: Option<String>) -> Result<HomeFeedResult, String> {
    api::get_home_feed(endpoint)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_week_filters(endpoint: Option<String>) -> Result<WeekFiltersResult, String> {
    api::get_week_filters(endpoint)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_week_items(
    page: Option<u32>,
    category_id: String,
    type_id: String,
    endpoint: Option<String>,
) -> Result<WeekItemsResult, String> {
    api::get_week_items(page, category_id, type_id, endpoint)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_comic_detail(
    comic_id: String,
    endpoint: Option<String>,
) -> Result<ComicDetailResult, String> {
    api::get_comic_detail(comic_id, endpoint)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_comic_comments(
    comic_id: String,
    page: Option<u32>,
    endpoint: Option<String>,
) -> Result<ComicCommentsResult, String> {
    api::get_comic_comments(comic_id, page, endpoint)
        .await
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_remote_setting,
            search_comics,
            get_home_feed,
            get_week_filters,
            get_week_items,
            get_comic_detail,
            get_comic_comments
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

mod cache;
mod cache_index;
mod image_decode;
mod manifest;
mod page;
mod types;

use crate::api::{ApiError, ApiErrorKind, ApiResult};
use cache::{map_cache_error, normalize_cache_limit, reader_cache_root, reader_cache_stats};
use cache_index::clear_reader_cache_entries;
use manifest::clear_manifest_cache;
use page::materialize_reader_page;
use std::fs;
use tauri::AppHandle;

pub(crate) use manifest::get_or_load_manifest;
pub(crate) use page::{materialize_reader_page_to_path, reader_page_output_extension};
pub use types::{ComicReadManifestResult, ComicReadPageResult, ReaderCacheStatsResult};

pub async fn get_comic_read_manifest(
    read_id: String,
    endpoint: Option<String>,
) -> ApiResult<ComicReadManifestResult> {
    let manifest = get_or_load_manifest(read_id, endpoint).await?;

    Ok(manifest.to_result())
}

pub async fn get_comic_read_page(
    app: &AppHandle,
    read_id: String,
    index: u32,
    endpoint: Option<String>,
    request_origin: Option<String>,
    cache_limit_bytes: Option<u64>,
) -> ApiResult<ComicReadPageResult> {
    let manifest = get_or_load_manifest(read_id, endpoint).await?;

    materialize_reader_page(
        app,
        &manifest,
        index,
        normalize_cache_limit(cache_limit_bytes),
        request_origin,
    )
    .await
}

pub async fn get_reader_cache_stats(
    app: &AppHandle,
    cache_limit_bytes: Option<u64>,
) -> ApiResult<ReaderCacheStatsResult> {
    let cache_root = reader_cache_root(app)?;
    let cache_limit_bytes = normalize_cache_limit(cache_limit_bytes);
    reader_cache_stats(cache_root, cache_limit_bytes).await
}

pub async fn clear_reader_cache(
    app: &AppHandle,
    cache_limit_bytes: Option<u64>,
) -> ApiResult<ReaderCacheStatsResult> {
    let cache_root = reader_cache_root(app)?;
    let cache_limit_bytes = normalize_cache_limit(cache_limit_bytes);

    if cache_root.exists() {
        fs::remove_dir_all(&cache_root).map_err(map_cache_error)?;
    }

    clear_reader_cache_entries().await?;
    clear_manifest_cache();
    reader_cache_stats(cache_root, cache_limit_bytes).await
}

pub fn open_reader_cache_dir(app: &AppHandle) -> ApiResult<()> {
    let cache_root = reader_cache_root(app)?;

    fs::create_dir_all(&cache_root).map_err(map_cache_error)?;
    tauri_plugin_opener::open_path(&cache_root, None::<&str>)
        .map_err(|error| ApiError::new(ApiErrorKind::Cache, error.to_string()))
}

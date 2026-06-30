use super::cache::{
    cleanup_reader_cache, file_size_bytes, map_cache_error, reader_cache_root,
    reader_page_cache_extension, reader_page_cache_path, write_temp_reader_cache_file,
};
use super::cache_index::{
    delete_reader_cache_entry, find_reader_cache_entry, touch_reader_cache_entry,
    upsert_reader_cache_entry, ReaderCacheEntryInput,
};
use super::image_decode::{
    decode_scrambled_image, encode_scrambled_webp_cache, map_image_error, should_decode_image,
    source_extension,
};
use super::manifest::url_host;
use super::types::{ComicReadPageResult, ReaderManifest, ReaderPage, ReaderPageMaterializeOrigin};
use crate::api::{build_http_client, ApiError, ApiErrorKind, ApiResult};
use crate::diagnostics;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tokio::sync::Mutex as AsyncMutex;

static PAGE_MATERIALIZE_LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<AsyncMutex<()>>>>> =
    OnceLock::new();

pub(crate) async fn materialize_reader_page(
    app: &AppHandle,
    manifest: &ReaderManifest,
    index: u32,
    cache_limit_bytes: u64,
    request_origin: Option<String>,
) -> ApiResult<ComicReadPageResult> {
    materialize_reader_page_inner(app, manifest, index, cache_limit_bytes, request_origin).await
}

pub(crate) async fn materialize_reader_page_to_path(
    manifest: &ReaderManifest,
    index: u32,
    target_path: PathBuf,
) -> ApiResult<(u32, u32, bool)> {
    let page = manifest
        .pages
        .get(index as usize)
        .ok_or_else(|| ApiError::new(ApiErrorKind::MissingData, "Reader page is out of range"))?
        .clone();
    let materialize_lock = reader_page_materialize_lock(&target_path);
    let _materialize_guard = materialize_lock.lock().await;

    if target_path.exists() {
        match fs::metadata(&target_path) {
            Ok(metadata) if metadata.is_file() => return Ok((0, 0, true)),
            Ok(_) => {
                let _ = fs::remove_file(&target_path);
            }
            Err(_) => {
                let _ = fs::remove_file(&target_path);
            }
        }
    }

    let client = build_http_client()?;
    let bytes = download_image_bytes(&client, &page.source_url, &manifest.endpoint).await?;
    let manifest_for_decode = manifest.clone();
    let page_for_decode = page.clone();
    let target_path_for_decode = target_path.clone();

    let (width, height) = tokio::task::spawn_blocking(move || {
        write_reader_page_file(
            &target_path_for_decode,
            &manifest_for_decode,
            &page_for_decode,
            &bytes,
        )
    })
    .await
    .map_err(|error| {
        ApiError::new(
            ApiErrorKind::Decode,
            format!("Failed to decode download page: {error}"),
        )
    })??;

    Ok((width, height, false))
}

async fn materialize_reader_page_inner(
    app: &AppHandle,
    manifest: &ReaderManifest,
    index: u32,
    cache_limit_bytes: u64,
    request_origin: Option<String>,
) -> ApiResult<ComicReadPageResult> {
    let materialize_started_at = Instant::now();
    let origin = resolve_reader_materialize_origin(request_origin);
    let page = manifest
        .pages
        .get(index as usize)
        .ok_or_else(|| ApiError::new(ApiErrorKind::MissingData, "Reader page is out of range"))?
        .clone();
    let cache_root = reader_cache_root(app)?;
    let cache_path = reader_page_cache_path(&cache_root, manifest, &page)?;
    let materialize_lock = reader_page_materialize_lock(&cache_path);
    let lock_started_at = Instant::now();
    let _materialize_guard = materialize_lock.lock().await;
    let lock_wait_elapsed = lock_started_at.elapsed();

    if let Some(entry) = find_reader_cache_entry(manifest, &page).await? {
        let indexed_path = PathBuf::from(&entry.path);
        match fs::metadata(&indexed_path) {
            Ok(metadata) if metadata.is_file() => {
                touch_reader_cache_entry(manifest, &page).await?;
                diagnostics::debug(format!(
                    "Reader page cache hit read_id={} page={} origin={} lock_wait_ms={:.1} total_ms={:.1}",
                    manifest.read_id,
                    index + 1,
                    origin.as_str(),
                    elapsed_ms(lock_wait_elapsed),
                    elapsed_ms(materialize_started_at.elapsed()),
                ));

                return Ok(page_result(
                    manifest,
                    index,
                    indexed_path,
                    entry.width,
                    entry.height,
                    true,
                ));
            }
            Ok(_) => {
                diagnostics::warn(
                    "Failed to read cached reader page, refreshing it: cached path is not a file",
                );
                let _ = fs::remove_file(&indexed_path);
                delete_reader_cache_entry(manifest, &page).await?;
            }
            Err(error) => {
                diagnostics::warn(format!(
                    "Failed to read cached reader page, refreshing it: {error}"
                ));
                let _ = fs::remove_file(&indexed_path);
                delete_reader_cache_entry(manifest, &page).await?;
            }
        }
    }

    if cache_path.exists() {
        let _ = fs::remove_file(&cache_path);
    }

    let client = build_http_client()?;
    let download_started_at = Instant::now();
    let bytes = download_image_bytes(&client, &page.source_url, &manifest.endpoint).await?;
    let download_elapsed = download_started_at.elapsed();

    let page_for_decode = page.clone();
    let manifest_for_decode = manifest.clone();
    let cache_path_for_decode = cache_path.clone();

    let write_result = tokio::task::spawn_blocking(move || {
        write_reader_page_cache(
            &cache_path_for_decode,
            &manifest_for_decode,
            &page_for_decode,
            &bytes,
        )
    })
    .await
    .map_err(|error| {
        ApiError::new(
            ApiErrorKind::Decode,
            format!("Failed to decode reader page: {error}"),
        )
    })??;
    let output_bytes =
        file_size_bytes(&cache_path).unwrap_or(write_result.output_bytes.unwrap_or(0));
    upsert_reader_cache_entry(ReaderCacheEntryInput {
        endpoint: manifest.endpoint.clone(),
        read_id: manifest.read_id.clone(),
        page_index: page.index,
        path: cache_path.to_string_lossy().to_string(),
        size_bytes: output_bytes,
        width: write_result.width,
        height: write_result.height,
        extension: reader_page_cache_extension(manifest, &page).to_string(),
        is_scrambled: should_decode_image(manifest, &page),
    })
    .await?;
    let cleanup_started_at = Instant::now();
    cleanup_reader_cache(cache_limit_bytes).await?;
    let cleanup_elapsed = cleanup_started_at.elapsed();
    log_reader_cache_timing(
        manifest,
        &page,
        origin,
        write_result.mode,
        write_result.source_bytes,
        Some(output_bytes),
        &write_result.timings_with_cleanup(cleanup_elapsed),
    );
    let cache_elapsed = download_started_at
        .elapsed()
        .saturating_sub(download_elapsed);

    diagnostics::debug(format!(
        "Reader page materialized read_id={} page={} origin={} lock_wait_ms={:.1} download_ms={:.1} cache_ms={:.1} total_ms={:.1}",
        manifest.read_id,
        index + 1,
        origin.as_str(),
        elapsed_ms(lock_wait_elapsed),
        elapsed_ms(download_elapsed),
        elapsed_ms(cache_elapsed),
        elapsed_ms(materialize_started_at.elapsed())
    ));

    Ok(page_result(
        manifest,
        index,
        cache_path,
        write_result.width,
        write_result.height,
        false,
    ))
}

fn page_result(
    manifest: &ReaderManifest,
    index: u32,
    path: PathBuf,
    width: u32,
    height: u32,
    is_cached: bool,
) -> ComicReadPageResult {
    ComicReadPageResult {
        read_id: manifest.read_id.clone(),
        index,
        path: path.to_string_lossy().to_string(),
        width,
        height,
        aspect_ratio: if height == 0 {
            1.0
        } else {
            width as f32 / height as f32
        },
        is_cached,
    }
}

fn resolve_reader_materialize_origin(
    request_origin: Option<String>,
) -> ReaderPageMaterializeOrigin {
    match request_origin.as_deref() {
        Some("prefetch") => ReaderPageMaterializeOrigin::Prefetch,
        _ => ReaderPageMaterializeOrigin::Visible,
    }
}

async fn download_image_bytes(
    client: &reqwest::Client,
    source_url: &str,
    _endpoint: &str,
) -> ApiResult<Vec<u8>> {
    let host = url_host(source_url);
    let response = client
        .get(source_url)
        .header("Host", host)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|error| ApiError::new(ApiErrorKind::Network, error.to_string()))?;

    if !response.status().is_success() {
        return Err(ApiError::new(
            ApiErrorKind::Http,
            format!("Image CDN returned HTTP {}", response.status()),
        ));
    }

    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|error| ApiError::new(ApiErrorKind::Network, error.to_string()))
}

fn write_reader_page_cache(
    cache_path: &Path,
    manifest: &ReaderManifest,
    page: &ReaderPage,
    bytes: &[u8],
) -> ApiResult<ReaderCacheWriteResult> {
    let total_started_at = Instant::now();

    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_err(map_cache_error)?;
    }

    if !should_decode_image(manifest, page) {
        let write_started_at = Instant::now();
        write_temp_reader_cache_file(cache_path, |temp_path| {
            fs::write(temp_path, bytes).map_err(map_cache_error)
        })?;
        let output_bytes = file_size_bytes(cache_path);
        let write_elapsed = write_started_at.elapsed();
        return Ok(ReaderCacheWriteResult::new(
            0,
            0,
            output_bytes,
            "direct",
            bytes.len(),
            vec![
                ("write_ms", write_elapsed),
                ("total_ms", total_started_at.elapsed()),
            ],
        ));
    }

    let load_started_at = Instant::now();
    let original = image::load_from_memory(bytes).map_err(map_image_error)?;
    let load_elapsed = load_started_at.elapsed();
    let decode_started_at = Instant::now();
    let decoded = decode_scrambled_image(original, manifest.read_id_number, &page.page_name)?;
    let decode_elapsed = decode_started_at.elapsed();
    let (decoded_width, decoded_height) = decoded.dimensions();

    let encode_started_at = Instant::now();
    let webp_bytes = encode_scrambled_webp_cache(&decoded);
    let encode_elapsed = encode_started_at.elapsed();
    let write_started_at = Instant::now();
    write_temp_reader_cache_file(cache_path, |temp_path| {
        fs::write(temp_path, &webp_bytes).map_err(map_cache_error)
    })?;
    let write_elapsed = write_started_at.elapsed();
    let output_bytes = file_size_bytes(cache_path);
    Ok(ReaderCacheWriteResult::new(
        decoded_width,
        decoded_height,
        output_bytes,
        "scrambled_webp_q75",
        bytes.len(),
        vec![
            ("load_ms", load_elapsed),
            ("reorder_ms", decode_elapsed),
            ("encode_ms", encode_elapsed),
            ("write_ms", write_elapsed),
            ("total_ms", total_started_at.elapsed()),
        ],
    ))
}

fn write_reader_page_file(
    target_path: &Path,
    manifest: &ReaderManifest,
    page: &ReaderPage,
    bytes: &[u8],
) -> ApiResult<(u32, u32)> {
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(map_cache_error)?;
    }

    if !should_decode_image(manifest, page) {
        write_temp_reader_cache_file(target_path, |temp_path| {
            fs::write(temp_path, bytes).map_err(map_cache_error)
        })?;

        return Ok((0, 0));
    }

    let original = image::load_from_memory(bytes).map_err(map_image_error)?;
    let decoded = decode_scrambled_image(original, manifest.read_id_number, &page.page_name)?;
    let (decoded_width, decoded_height) = decoded.dimensions();
    let webp_bytes = encode_scrambled_webp_cache(&decoded);
    write_temp_reader_cache_file(target_path, |temp_path| {
        fs::write(temp_path, &webp_bytes).map_err(map_cache_error)
    })?;

    Ok((decoded_width, decoded_height))
}

pub(crate) fn reader_page_output_extension(
    manifest: &ReaderManifest,
    index: u32,
) -> ApiResult<&'static str> {
    let page = manifest
        .pages
        .get(index as usize)
        .ok_or_else(|| ApiError::new(ApiErrorKind::MissingData, "Reader page is out of range"))?;

    Ok(if should_decode_image(manifest, page) {
        "webp"
    } else {
        source_extension(&page.source_url)
    })
}

fn log_reader_cache_timing(
    manifest: &ReaderManifest,
    page: &ReaderPage,
    origin: ReaderPageMaterializeOrigin,
    mode: &str,
    source_bytes: usize,
    output_bytes: Option<u64>,
    timings: &[(&str, Duration)],
) {
    let size_info = reader_cache_size_log(source_bytes as u64, output_bytes);
    let timings = timings
        .iter()
        .map(|(name, duration)| format!("{name}={:.1}", elapsed_ms(*duration)))
        .collect::<Vec<_>>()
        .join(" ");

    diagnostics::debug(format!(
        "Reader cache write read_id={} page={} origin={} mode={} {} {}",
        manifest.read_id,
        page.index + 1,
        origin.as_str(),
        mode,
        size_info,
        timings
    ));
}

fn reader_cache_size_log(source_bytes: u64, output_bytes: Option<u64>) -> String {
    let source_kb = bytes_to_kb(source_bytes);

    if let Some(output_bytes) = output_bytes {
        let output_kb = bytes_to_kb(output_bytes);
        let ratio = if source_bytes == 0 {
            0.0
        } else {
            output_bytes as f64 / source_bytes as f64
        };

        format!(
            "source_kb={source_kb:.1} output_kb={output_kb:.1} ratio={ratio:.2} delta_kb={:+.1}",
            output_kb - source_kb
        )
    } else {
        format!("source_kb={source_kb:.1} output_kb=? ratio=? delta_kb=?")
    }
}

fn bytes_to_kb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0
}

fn elapsed_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn reader_page_materialize_lock(cache_path: &Path) -> Arc<AsyncMutex<()>> {
    let mut locks = PAGE_MATERIALIZE_LOCKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    locks
        .entry(cache_path.to_path_buf())
        .or_insert_with(|| Arc::new(AsyncMutex::new(())))
        .clone()
}

#[derive(Debug)]
struct ReaderCacheWriteResult {
    width: u32,
    height: u32,
    output_bytes: Option<u64>,
    mode: &'static str,
    source_bytes: usize,
    timings: Vec<(&'static str, Duration)>,
}

impl ReaderCacheWriteResult {
    fn new(
        width: u32,
        height: u32,
        output_bytes: Option<u64>,
        mode: &'static str,
        source_bytes: usize,
        timings: Vec<(&'static str, Duration)>,
    ) -> Self {
        Self {
            width,
            height,
            output_bytes,
            mode,
            source_bytes,
            timings,
        }
    }

    fn timings_with_cleanup(&self, cleanup_elapsed: Duration) -> Vec<(&'static str, Duration)> {
        let mut timings = self.timings.clone();
        timings.push(("cleanup_ms", cleanup_elapsed));
        timings
    }
}

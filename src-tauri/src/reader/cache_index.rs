use super::types::{ReaderManifest, ReaderPage};
use crate::api::{ApiError, ApiErrorKind, ApiResult};
use crate::storage;
use sqlx::{Row, SqlitePool};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub(crate) struct ReaderCacheEntry {
    pub(crate) path: String,
    pub(crate) size_bytes: u64,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug)]
pub(crate) struct ReaderCacheEntryInput {
    pub(crate) endpoint: String,
    pub(crate) read_id: String,
    pub(crate) page_index: u32,
    pub(crate) path: String,
    pub(crate) size_bytes: u64,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) extension: String,
    pub(crate) is_scrambled: bool,
}

pub(crate) async fn find_reader_cache_entry(
    manifest: &ReaderManifest,
    page: &ReaderPage,
) -> ApiResult<Option<ReaderCacheEntry>> {
    let pool = pool()?;
    let row = sqlx::query(
        r#"
        SELECT path, size_bytes, width, height
        FROM reader_cache_entries
        WHERE endpoint = ? AND read_id = ? AND page_index = ?
        "#,
    )
    .bind(&manifest.endpoint)
    .bind(&manifest.read_id)
    .bind(u32_to_i64(page.index))
    .fetch_optional(pool)
    .await
    .map_err(map_sqlx_error)?;

    Ok(row.map(|row| ReaderCacheEntry {
        path: row.get("path"),
        size_bytes: i64_to_u64(row.get("size_bytes")),
        width: i64_to_u32(row.get("width")),
        height: i64_to_u32(row.get("height")),
    }))
}

pub(crate) async fn upsert_reader_cache_entry(input: ReaderCacheEntryInput) -> ApiResult<()> {
    let pool = pool()?;
    let mut transaction = pool.begin().await.map_err(map_sqlx_error)?;

    sqlx::query(
        r#"
        DELETE FROM reader_cache_entries
        WHERE path = ?
          AND NOT (endpoint = ? AND read_id = ? AND page_index = ?)
        "#,
    )
    .bind(&input.path)
    .bind(&input.endpoint)
    .bind(&input.read_id)
    .bind(u32_to_i64(input.page_index))
    .execute(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?;

    sqlx::query(
        r#"
        INSERT INTO reader_cache_entries (
            endpoint, read_id, page_index, path, size_bytes, width, height,
            extension, is_scrambled, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(endpoint, read_id, page_index) DO UPDATE SET
            path = excluded.path,
            size_bytes = excluded.size_bytes,
            width = excluded.width,
            height = excluded.height,
            extension = excluded.extension,
            is_scrambled = excluded.is_scrambled,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(input.endpoint)
    .bind(input.read_id)
    .bind(u32_to_i64(input.page_index))
    .bind(input.path)
    .bind(u64_to_i64(input.size_bytes))
    .bind(u32_to_i64(input.width))
    .bind(u32_to_i64(input.height))
    .bind(input.extension)
    .bind(if input.is_scrambled { 1_i64 } else { 0_i64 })
    .bind(current_timestamp())
    .execute(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?;

    transaction.commit().await.map_err(map_sqlx_error)
}

pub(crate) async fn touch_reader_cache_entry(
    manifest: &ReaderManifest,
    page: &ReaderPage,
) -> ApiResult<()> {
    let pool = pool()?;
    sqlx::query(
        r#"
        UPDATE reader_cache_entries
        SET updated_at = ?
        WHERE endpoint = ? AND read_id = ? AND page_index = ?
        "#,
    )
    .bind(current_timestamp())
    .bind(&manifest.endpoint)
    .bind(&manifest.read_id)
    .bind(u32_to_i64(page.index))
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(map_sqlx_error)
}

pub(crate) async fn delete_reader_cache_entry(
    manifest: &ReaderManifest,
    page: &ReaderPage,
) -> ApiResult<()> {
    let pool = pool()?;
    sqlx::query(
        r#"
        DELETE FROM reader_cache_entries
        WHERE endpoint = ? AND read_id = ? AND page_index = ?
        "#,
    )
    .bind(&manifest.endpoint)
    .bind(&manifest.read_id)
    .bind(u32_to_i64(page.index))
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(map_sqlx_error)
}

pub(crate) async fn delete_reader_cache_entry_by_path(path: &str) -> ApiResult<()> {
    let pool = pool()?;
    sqlx::query("DELETE FROM reader_cache_entries WHERE path = ?")
        .bind(path)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
}

pub(crate) async fn clear_reader_cache_entries() -> ApiResult<()> {
    let pool = pool()?;
    sqlx::query("DELETE FROM reader_cache_entries")
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
}

pub(crate) async fn reader_cache_index_stats() -> ApiResult<(u64, u32)> {
    let pool = pool()?;
    let row = sqlx::query(
        r#"
        SELECT COALESCE(SUM(size_bytes), 0) AS total_bytes, COUNT(*) AS file_count
        FROM reader_cache_entries
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(map_sqlx_error)?;

    Ok((
        i64_to_u64(row.get("total_bytes")),
        i64_to_u32(row.get::<i64, _>("file_count")),
    ))
}

pub(crate) async fn list_reader_cache_entries_by_age() -> ApiResult<Vec<ReaderCacheEntry>> {
    let pool = pool()?;
    let rows = sqlx::query(
        r#"
        SELECT path, size_bytes, width, height
        FROM reader_cache_entries
        ORDER BY updated_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_error)?;

    Ok(rows
        .into_iter()
        .map(|row| ReaderCacheEntry {
            path: row.get("path"),
            size_bytes: i64_to_u64(row.get("size_bytes")),
            width: i64_to_u32(row.get("width")),
            height: i64_to_u32(row.get("height")),
        })
        .collect())
}

fn pool() -> ApiResult<&'static SqlitePool> {
    storage::pool().map_err(|error| ApiError::new(ApiErrorKind::Cache, error))
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or_default()
}

fn map_sqlx_error(error: sqlx::Error) -> ApiError {
    ApiError::new(
        ApiErrorKind::Cache,
        format!("SQLite reader cache index error: {error}"),
    )
}

fn i64_to_u32(value: i64) -> u32 {
    value.max(0).min(u32::MAX as i64) as u32
}

fn i64_to_u64(value: i64) -> u64 {
    value.max(0) as u64
}

fn u32_to_i64(value: u32) -> i64 {
    i64::from(value)
}

fn u64_to_i64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

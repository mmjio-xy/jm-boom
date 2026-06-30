mod db;
mod migrations;

use sqlx::SqlitePool;
use std::sync::OnceLock;
use tauri::AppHandle;

static STORAGE_POOL: OnceLock<SqlitePool> = OnceLock::new();

pub(crate) async fn init(app: &AppHandle) -> Result<(), String> {
    if STORAGE_POOL.get().is_some() {
        return Ok(());
    }

    let pool = db::connect(app).await?;
    migrations::run(&pool).await?;

    let _ = STORAGE_POOL.set(pool);

    Ok(())
}

#[allow(dead_code)]
pub(crate) fn pool() -> Result<&'static SqlitePool, String> {
    STORAGE_POOL
        .get()
        .ok_or_else(|| "SQLite storage is not initialized".to_string())
}
